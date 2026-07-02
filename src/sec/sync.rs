use rusqlite::{Connection, params};
use serde::Deserialize;

use crate::{
    config::SecConfig,
    db,
    sec::{
        client::SecClient,
        form4::parse_form4,
        submissions::new_accessions,
        thirteenf::parse_information_table,
        types::{EntityKind, ParsedHolding, ParsedInsiderTx, SecEntity},
    },
};

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
    crate::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entities = crate::db::sec_repo::list_all_entities(&connection).map_err(|e| e.to_string())?;
    sync_entities(&connection, config, &entities)
}

pub fn sync_entity(
    db_path: &std::path::Path,
    config: &SecConfig,
    entity_id: i64,
) -> Result<usize, String> {
    let connection = db::open(db_path).map_err(|error| error.to_string())?;
    crate::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entity = crate::db::sec_repo::get_entity(&connection, entity_id)
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
    maybe_reset_institution_backfill(connection, entity)?;
    let last_seen = crate::db::sec_repo::last_accession_seen(connection, entity.id)
        .map_err(|error| error.to_string())?;
    let filings = new_accessions(client, entity, last_seen.as_deref())?;
    let mut latest_success = last_seen;

    for filing in filings {
        let result = (|| -> Result<(), String> {
            let xml =
                fetch_filing_xml(client, entity, &filing.accession_no, &filing.primary_document)?;
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
        } else if std::env::var("APETERM_SEC_DEBUG").is_ok() {
            if let Err(error) = result {
                eprintln!(
                    "sec filing failed entity={} accession={} error={error}",
                    entity.name, filing.accession_no
                );
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
            params![entity.id, latest_success, now],
        )
        .map_err(|error| error.to_string())?;

    Ok(())
}

pub fn sync_all_verbose(db_path: &std::path::Path, config: &SecConfig) -> Result<usize, String> {
    let connection = db::open(db_path).map_err(|error| error.to_string())?;
    crate::sec::ensure_seeded(&connection).map_err(|error| error.to_string())?;
    let entities = crate::db::sec_repo::list_all_entities(&connection).map_err(|e| e.to_string())?;
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
        .execute("DELETE FROM thirteenf_holdings WHERE entity_id = ?1", [entity.id])
        .map_err(|error| error.to_string())?;
    connection
        .execute("DELETE FROM sec_sync_state WHERE entity_id = ?1", [entity.id])
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
    let base = format!(
        "https://www.sec.gov/Archives/edgar/data/{cik_trimmed}/{accession_compact}"
    );
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

    Err(format!("no XML document found for accession {accession_no}"))
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
    let transaction = connection.unchecked_transaction().map_err(|e| e.to_string())?;
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
        .filter(|tx| issuer_filter.as_ref().is_none_or(|value| &tx.ticker == value))
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        return Ok(());
    }

    let transaction = connection.unchecked_transaction().map_err(|e| e.to_string())?;
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
