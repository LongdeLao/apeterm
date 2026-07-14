use chrono::Datelike;
use regex::Regex;
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{
    config::SecConfig,
    db,
    sec::{
        client::SecClient,
        form4::parse_form4,
        submissions::new_accessions,
        thirteenf::parse_information_table,
        types::{EntityKind, ParsedCongressFiling, ParsedHolding, ParsedInsiderTx, SecEntity},
    },
};

const LOCAL_PYTHON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/bin/python");
const HOUSE_PTR_SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/house_ptr_extract.py");
const ENV_PYTHON: &str = "APETERM_PYTHON";
const ENV_SCRIPT_DIR: &str = "APETERM_SCRIPT_DIR";

#[derive(Debug, Deserialize)]
struct FilingIndex {
    directory: FilingDirectory,
}

#[derive(Debug, Deserialize)]
struct FilingDirectory {
    item: Vec<FilingFile>,
}

#[derive(Debug, Deserialize)]
struct FilingFile {
    name: String,
}

pub fn sync_all(db_path: &std::path::Path, config: &SecConfig) -> Result<usize, String> {
    let connection = db::open(db_path).map_err(|error| error.to_string())?;
    crate::features::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entities =
        crate::features::sec::repo::list_all_entities(&connection).map_err(|e| e.to_string())?;
    sync_entities(&connection, config, &entities)
}

pub fn sync_entity(
    db_path: &std::path::Path,
    config: &SecConfig,
    entity_id: i64,
) -> Result<usize, String> {
    let connection = db::open(db_path).map_err(|error| error.to_string())?;
    crate::features::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entity = crate::features::sec::repo::get_entity(&connection, entity_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("SEC entity {entity_id} not found"))?;
    sync_entities(&connection, config, &[entity])
}

fn sync_entities(
    connection: &Connection,
    config: &SecConfig,
    entities: &[SecEntity],
) -> Result<usize, String> {
    let client = SecClient::new(config)?;
    let mut synced = 0;

    for entity in entities {
        match sync_single_entity(connection, &client, entity) {
            Ok(()) => synced += 1,
            Err(error) => {
                if std::env::var("APETERM_SEC_DEBUG").is_ok() {
                    eprintln!("sec sync failed entity={} error={error}", entity.name);
                }
            }
        }
    }

    Ok(synced)
}

fn sync_single_entity(
    connection: &Connection,
    client: &SecClient,
    entity: &SecEntity,
) -> Result<(), String> {
    if entity.kind == EntityKind::Ceo && entity.filer_cik.starts_with("congress:") {
        return sync_congress_entity(connection, client, entity);
    }

    maybe_reset_institution_backfill(connection, entity)?;
    let last_seen = crate::features::sec::repo::last_accession_seen(connection, entity.id)
        .map_err(|error| error.to_string())?;
    let filings = new_accessions(client, entity, last_seen.as_deref())?;
    let mut latest_success = last_seen;

    for filing in filings {
        let result = (|| -> Result<(), String> {
            let xml = fetch_filing_xml(
                client,
                entity,
                &filing.accession_no,
                &filing.primary_document,
            )?;
            match entity.kind {
                EntityKind::Institution => {
                    let holdings = parse_information_table(&xml)?;
                    upsert_holdings(
                        connection,
                        entity.id,
                        &filing.accession_no,
                        &filing.filed_at,
                        holdings,
                    )?;
                }
                EntityKind::Ceo => {
                    let transactions = parse_form4(&xml, &filing.filed_at)?;
                    upsert_transactions(connection, entity, &filing.accession_no, transactions)?;
                }
            }
            Ok(())
        })();

        if result.is_ok() {
            latest_success = Some(filing.accession_no);
        } else if std::env::var("APETERM_SEC_DEBUG").is_ok()
            && let Err(error) = result
        {
            eprintln!(
                "sec filing failed entity={} accession={} error={error}",
                entity.name, filing.accession_no
            );
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "
            INSERT INTO sec_sync_state(entity_id, last_accession_seen, last_polled_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(entity_id) DO UPDATE SET
              last_accession_seen = excluded.last_accession_seen,
              last_polled_at = excluded.last_polled_at
            ",
            params![entity.id, latest_success, now],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn sync_congress_entity(
    connection: &Connection,
    client: &SecClient,
    entity: &SecEntity,
) -> Result<(), String> {
    if entity.subtitle.as_deref() != Some("House") {
        let now = chrono::Utc::now().to_rfc3339();
        connection
            .execute(
                "
                INSERT INTO sec_sync_state(entity_id, last_accession_seen, last_polled_at)
                VALUES (?1, NULL, ?2)
                ON CONFLICT(entity_id) DO UPDATE SET
                  last_polled_at = excluded.last_polled_at
                ",
                params![entity.id, now],
            )
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    let current_year = chrono::Utc::now().year();
    let years = [current_year, current_year - 1];
    let mut latest_filing_id = None;

    for year in years {
        let filings = search_house_ptrs(client, entity, year)?;
        for filing in filings {
            let pdf_bytes = client.get_bytes(&filing.source_url)?;
            let parsed = parse_house_ptr_pdf(&pdf_bytes)?;
            upsert_congress_transactions(connection, entity, &filing, parsed)?;
            if latest_filing_id
                .as_ref()
                .is_none_or(|existing| filing.filing_id > *existing)
            {
                latest_filing_id = Some(filing.filing_id.clone());
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    connection
        .execute(
            "
            INSERT INTO sec_sync_state(entity_id, last_accession_seen, last_polled_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(entity_id) DO UPDATE SET
              last_accession_seen = excluded.last_accession_seen,
              last_polled_at = excluded.last_polled_at
            ",
            params![entity.id, latest_filing_id, now],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[derive(Debug, Clone)]
struct HouseSearchResult {
    filing_id: String,
    source_url: String,
}

fn search_house_ptrs(
    client: &SecClient,
    entity: &SecEntity,
    year: i32,
) -> Result<Vec<HouseSearchResult>, String> {
    let search_html =
        client.get_text("https://disclosures-clerk.house.gov/FinancialDisclosure/ViewSearch")?;
    let token = Regex::new(r#"name="__RequestVerificationToken" type="hidden" value="([^"]+)""#)
        .map_err(|error| error.to_string())?
        .captures(&search_html)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
        .ok_or_else(|| "missing House Clerk CSRF token".to_string())?;

    let last_name = entity
        .name
        .split_whitespace()
        .last()
        .ok_or_else(|| format!("invalid congress member name: {}", entity.name))?;
    let form = vec![
        ("LastName".to_string(), last_name.to_string()),
        ("FilingYear".to_string(), year.to_string()),
        ("State".to_string(), String::new()),
        ("District".to_string(), String::new()),
        ("__RequestVerificationToken".to_string(), token),
    ];
    let results_html = client.post_form(
        "https://disclosures-clerk.house.gov/FinancialDisclosure/ViewMemberSearchResult",
        &form,
        "https://disclosures-clerk.house.gov/FinancialDisclosure/ViewSearch",
    )?;
    parse_house_search_results(&results_html, entity, year)
}

fn parse_house_search_results(
    html: &str,
    entity: &SecEntity,
    _year: i32,
) -> Result<Vec<HouseSearchResult>, String> {
    let pattern = Regex::new(
        r#"(?s)<tr role="row">.*?<a href="(?P<href>[^"]+)"[^>]*>(?P<name>[^<]+)</a>.*?<td data-label="Office">(?P<office>[^<]*)</td>.*?<td data-label="Filing Year">(?P<year>[^<]*)</td>.*?<td data-label="Filing">(?P<filing>[^<]*)</td>.*?</tr>"#,
    )
    .map_err(|error| error.to_string())?;
    let expected = normalized_name_tokens(&entity.name);
    let mut results = Vec::new();

    for captures in pattern.captures_iter(html) {
        let href = captures
            .name("href")
            .map(|value| value.as_str())
            .unwrap_or_default();
        let filing = captures
            .name("filing")
            .map(|value| value.as_str())
            .unwrap_or_default();
        if !filing.contains("PTR") {
            continue;
        }
        let matched_name = captures
            .name("name")
            .map(|value| value.as_str())
            .unwrap_or_default();
        if !name_matches_entity(matched_name, &expected) {
            continue;
        }
        let source_url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://disclosures-clerk.house.gov/{href}")
        };
        let filing_id = source_url
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".pdf")
            .to_string();
        results.push(HouseSearchResult {
            filing_id,
            source_url,
        });
    }

    results.sort_by(|left, right| right.filing_id.cmp(&left.filing_id));
    results.dedup_by(|left, right| left.filing_id == right.filing_id);
    Ok(results)
}

fn normalized_name_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn name_matches_entity(candidate: &str, expected: &[String]) -> bool {
    let normalized = normalized_name_tokens(candidate);
    expected
        .iter()
        .all(|token| normalized.iter().any(|part| part == token))
}

fn parse_house_ptr_pdf(bytes: &[u8]) -> Result<ParsedCongressFiling, String> {
    let temp_path = temp_pdf_path();
    fs::write(&temp_path, bytes).map_err(|error| error.to_string())?;
    let result = (|| {
        let child = Command::new(python_command())
            .arg("-u")
            .arg(house_ptr_script())
            .arg(&temp_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        let output = child
            .wait_with_output()
            .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err("house pdf parser failed".to_string());
        }
        serde_json::from_slice::<ParsedCongressFiling>(&output.stdout)
            .map_err(|error| error.to_string())
    })();
    let _ = fs::remove_file(&temp_path);
    result
}

fn upsert_congress_transactions(
    connection: &Connection,
    entity: &SecEntity,
    filing: &HouseSearchResult,
    parsed: ParsedCongressFiling,
) -> Result<(), String> {
    if parsed.transactions.is_empty() {
        return Ok(());
    }
    let filed_at = parsed.filed_at;
    let filing_id = parsed.filing_id.unwrap_or_else(|| filing.filing_id.clone());
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    for (index, tx) in parsed.transactions.into_iter().enumerate() {
        transaction
            .execute(
                "
                INSERT OR IGNORE INTO congress_transactions(
                  entity_id, filing_id, transaction_index, chamber, source_url, filed_at,
                  transaction_date, notification_date, owner_code, asset_name, ticker,
                  transaction_type, amount_range, description
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                ",
                params![
                    entity.id,
                    filing_id,
                    index as i64,
                    entity
                        .subtitle
                        .clone()
                        .unwrap_or_else(|| "House".to_string()),
                    filing.source_url,
                    filed_at,
                    normalize_us_date(&tx.transaction_date),
                    tx.notification_date.as_deref().map(normalize_us_date),
                    tx.owner_code,
                    tx.asset_name,
                    tx.ticker,
                    tx.transaction_type,
                    tx.amount_range,
                    tx.description,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn normalize_us_date(value: &str) -> String {
    chrono::NaiveDate::parse_from_str(value, "%m/%d/%Y")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| value.to_string())
}

fn temp_pdf_path() -> PathBuf {
    let filename = format!(
        "apeterm-congress-{}-{}.pdf",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    std::env::temp_dir().join(filename)
}

fn python_command() -> String {
    if let Ok(value) = env::var(ENV_PYTHON)
        && !value.trim().is_empty()
    {
        return value;
    }
    if Path::new(LOCAL_PYTHON).exists() {
        LOCAL_PYTHON.to_string()
    } else {
        "python3".to_string()
    }
}

fn house_ptr_script() -> PathBuf {
    if let Ok(value) = env::var(ENV_SCRIPT_DIR) {
        let path = Path::new(value.trim()).join("house_ptr_extract.py");
        if path.exists() {
            return path;
        }
    }
    PathBuf::from(HOUSE_PTR_SCRIPT)
}

pub fn sync_all_verbose(db_path: &std::path::Path, config: &SecConfig) -> Result<usize, String> {
    let connection = db::open(db_path).map_err(|error| error.to_string())?;
    crate::features::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entities =
        crate::features::sec::repo::list_all_entities(&connection).map_err(|e| e.to_string())?;
    let client = SecClient::new(config)?;
    let mut synced = 0;

    for entity in entities {
        eprintln!("syncing {}", entity.name);
        match sync_single_entity(&connection, &client, &entity) {
            Ok(()) => {
                synced += 1;
                eprintln!("ok {}", entity.name);
            }
            Err(error) => {
                eprintln!("failed {}: {}", entity.name, error);
            }
        }
    }

    Ok(synced)
}

fn maybe_reset_institution_backfill(
    connection: &Connection,
    entity: &SecEntity,
) -> Result<(), String> {
    if entity.kind != EntityKind::Institution {
        return Ok(());
    }

    let missing_labels: i64 = connection
        .query_row(
            "
            SELECT COUNT(*)
            FROM thirteenf_holdings
            WHERE entity_id = ?1 AND (ticker IS NULL OR ticker = '')
            ",
            [entity.id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    if missing_labels == 0 {
        return Ok(());
    }

    connection
        .execute(
            "DELETE FROM thirteenf_holdings WHERE entity_id = ?1",
            [entity.id],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM sec_sync_state WHERE entity_id = ?1",
            [entity.id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn fetch_filing_xml(
    client: &SecClient,
    entity: &SecEntity,
    accession_no: &str,
    primary_document: &str,
) -> Result<String, String> {
    let cik_trimmed = entity.filer_cik.trim_start_matches('0');
    let accession_compact = accession_no.replace('-', "");
    let base = format!("https://www.sec.gov/Archives/edgar/data/{cik_trimmed}/{accession_compact}");
    let index: FilingIndex = client.get_json(&format!("{base}/index.json"))?;

    let mut candidates = index
        .directory
        .item
        .into_iter()
        .map(|file| file.name)
        .filter(|name| name.ends_with(".xml"))
        .collect::<Vec<_>>();

    candidates.sort_by_key(|name| file_priority(name, primary_document));

    for candidate in candidates {
        let url = format!("{base}/{candidate}");
        if let Ok(xml) = client.get_text(&url) {
            let lower = xml.to_ascii_lowercase();
            let valid = match entity.kind {
                EntityKind::Institution => lower.contains("<informationtable"),
                EntityKind::Ceo => lower.contains("<ownershipdocument"),
            };
            if valid {
                return Ok(xml);
            }
        }
    }

    Err(format!(
        "no XML document found for accession {accession_no}"
    ))
}

fn file_priority(name: &str, primary_document: &str) -> usize {
    let lower = name.to_ascii_lowercase();
    if lower.contains("infotable") || lower.contains("information") {
        return 0;
    }
    if name == primary_document {
        return 1;
    }
    if lower.contains("form4") || lower.contains("ownership") || lower.contains("primary_doc") {
        return 2;
    }
    3
}

fn upsert_holdings(
    connection: &Connection,
    entity_id: i64,
    accession_no: &str,
    filed_at: &str,
    holdings: Vec<ParsedHolding>,
) -> Result<(), String> {
    if holdings.is_empty() {
        return Err(format!("empty 13F holdings for {accession_no}"));
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    for holding in holdings {
        transaction
            .execute(
                "
                INSERT OR IGNORE INTO thirteenf_holdings(
                  entity_id, period_of_report, cusip, ticker, shares, value_usd, accession_no
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ",
                params![
                    entity_id,
                    filed_at,
                    holding.cusip,
                    holding.ticker,
                    holding.shares,
                    holding.value_usd,
                    accession_no
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}

fn upsert_transactions(
    connection: &Connection,
    entity: &SecEntity,
    accession_no: &str,
    transactions: Vec<ParsedInsiderTx>,
) -> Result<(), String> {
    let issuer_filter = entity
        .issuer_ticker
        .as_ref()
        .map(|value| value.to_ascii_uppercase());
    let filtered = transactions
        .into_iter()
        .filter(|tx| {
            issuer_filter
                .as_ref()
                .is_none_or(|value| &tx.ticker == value)
        })
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return Ok(());
    }

    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    for (index, row) in filtered.into_iter().enumerate() {
        let unique_accession = if index == 0 {
            accession_no.to_string()
        } else {
            format!("{accession_no}:{}", index + 1)
        };
        transaction
            .execute(
                "
                INSERT OR IGNORE INTO insider_transactions(
                  entity_id, ticker, filed_at, transaction_date, code, shares, price_usd,
                  shares_owned_after, accession_no
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ",
                params![
                    entity.id,
                    row.ticker,
                    row.filed_at,
                    row.transaction_date,
                    row.code,
                    row.shares,
                    row.price_usd,
                    row.shares_owned_after,
                    unique_accession
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())
}
