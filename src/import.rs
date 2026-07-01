use std::collections::{HashMap, HashSet};

use std::error::Error;

use chrono::Utc;
use rusqlite::{Connection, Result, params};

const NASDAQ_LISTED_URL: &str = "https://www.nasdaqtrader.com/dynamic/SymDir/nasdaqlisted.txt";
const OTHER_LISTED_URL: &str = "https://www.nasdaqtrader.com/dynamic/SymDir/otherlisted.txt";
const MAJOR_ETFS: &[&str] = &[
    "ARKK", "BIL", "BND", "DIA", "EEM", "EFA", "GLD", "HYG", "IEF", "IJH", "IJR", "IWM", "IVV",
    "IYR", "JEPI", "LQD", "QQQ", "SCHD", "SHY", "SLV", "SMH", "SOXX", "SPY", "TLT", "UNG", "USO",
    "VEA", "VGT", "VNQ", "VOO", "VTI", "VTV", "VUG", "VXUS", "XLB", "XLC", "XLE", "XLF", "XLI",
    "XLK", "XLP", "XLRE", "XLU", "XLV", "XLY",
];

#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentImport {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub asset_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub fetched: usize,
    pub upserted: usize,
    pub deactivated: usize,
}

pub type ImportResult<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

pub async fn import_nasdaq_directory(connection: &mut Connection) -> ImportResult<ImportSummary> {
    let client = reqwest::Client::new();
    let nasdaq = client.get(NASDAQ_LISTED_URL).send().await?.text().await?;
    let other = client.get(OTHER_LISTED_URL).send().await?.text().await?;

    let mut instruments = parse_nasdaq_listed(&nasdaq);
    instruments.extend(parse_other_listed(&other));
    Ok(upsert_imports(connection, instruments)?)
}

pub fn parse_nasdaq_listed(text: &str) -> Vec<InstrumentImport> {
    parse_directory(text, DirectoryKind::Nasdaq)
}

pub fn parse_other_listed(text: &str) -> Vec<InstrumentImport> {
    parse_directory(text, DirectoryKind::Other)
}

pub fn upsert_imports(
    connection: &mut Connection,
    imports: Vec<InstrumentImport>,
) -> Result<ImportSummary> {
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for import in imports {
        if seen.insert(import.symbol.clone()) {
            deduped.push(import);
        }
    }

    let active_symbols: HashSet<String> = deduped
        .iter()
        .map(|instrument| instrument.symbol.clone())
        .collect();
    let tx = connection.transaction()?;
    let now = Utc::now().to_rfc3339();

    for instrument in &deduped {
        tx.execute(
            "
            INSERT INTO instruments(symbol, name, exchange, asset_type, active, last_updated)
            VALUES (?1, ?2, ?3, ?4, 1, ?5)
            ON CONFLICT(symbol) DO UPDATE SET
              name = excluded.name,
              exchange = excluded.exchange,
              asset_type = excluded.asset_type,
              active = 1
            ",
            params![
                instrument.symbol,
                instrument.name,
                instrument.exchange,
                instrument.asset_type,
                now
            ],
        )?;
    }

    let mut deactivated = 0;
    let existing = {
        let mut statement = tx.prepare("SELECT symbol FROM instruments WHERE active = 1")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>>>()?
    };

    for symbol in existing {
        if !active_symbols.contains(&symbol) {
            deactivated += tx.execute(
                "UPDATE instruments SET active = 0 WHERE symbol = ?1",
                params![symbol],
            )?;
        }
    }

    tx.commit()?;
    Ok(ImportSummary {
        fetched: active_symbols.len(),
        upserted: active_symbols.len(),
        deactivated,
    })
}

pub fn normalize_symbol(raw: &str) -> Option<String> {
    let symbol = raw.trim().to_uppercase();
    if symbol.is_empty()
        || symbol.starts_with("FILE CREATION TIME")
        || symbol.contains(' ')
        || symbol.contains('|')
    {
        return None;
    }
    Some(symbol)
}

#[cfg(test)]
pub fn was_updated_recently(
    connection: &Connection,
    symbol: &str,
    max_age_hours: i64,
) -> Result<bool> {
    use rusqlite::OptionalExtension;

    let value = connection
        .query_row(
            "SELECT last_updated FROM instruments WHERE symbol = ?1",
            params![symbol],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();

    let Some(value) = value else {
        return Ok(false);
    };
    let Ok(updated) = chrono::DateTime::parse_from_rfc3339(&value) else {
        return Ok(false);
    };

    Ok(Utc::now()
        .signed_duration_since(updated.with_timezone(&Utc))
        .num_hours()
        < max_age_hours)
}

#[derive(Debug, Clone, Copy)]
enum DirectoryKind {
    Nasdaq,
    Other,
}

fn parse_directory(text: &str, kind: DirectoryKind) -> Vec<InstrumentImport> {
    let mut lines = text.lines();
    let Some(header_line) = lines.next() else {
        return Vec::new();
    };
    let headers = header_map(header_line);

    lines
        .filter_map(|line| parse_line(line, kind, &headers))
        .collect()
}

fn parse_line(
    line: &str,
    kind: DirectoryKind,
    headers: &HashMap<String, usize>,
) -> Option<InstrumentImport> {
    if line.starts_with("File Creation Time") {
        return None;
    }

    let fields: Vec<&str> = line.split('|').collect();
    let test_issue = get_field(&fields, headers, "Test Issue").unwrap_or("Y");
    if test_issue != "N" {
        return None;
    }

    let symbol_header = match kind {
        DirectoryKind::Nasdaq => "Symbol",
        DirectoryKind::Other => "ACT Symbol",
    };
    let symbol = normalize_symbol(get_field(&fields, headers, symbol_header)?)?;
    let name = clean_security_name(get_field(&fields, headers, "Security Name")?);
    let etf = get_field(&fields, headers, "ETF").unwrap_or("N");
    let exchange = match kind {
        DirectoryKind::Nasdaq => "NASDAQ".to_string(),
        DirectoryKind::Other => {
            exchange_name(get_field(&fields, headers, "Exchange").unwrap_or(""))
        }
    };
    let asset_type = if etf == "Y" { "etf" } else { "stock" }.to_string();
    if !is_useful_instrument(&symbol, &name, &asset_type) {
        return None;
    }

    Some(InstrumentImport {
        symbol,
        name,
        exchange,
        asset_type,
    })
}

pub fn is_useful_instrument(symbol: &str, name: &str, asset_type: &str) -> bool {
    if asset_type == "etf" {
        return MAJOR_ETFS.contains(&symbol);
    }

    let lower_name = name.to_ascii_lowercase();
    let symbol_has_structured_suffix = symbol.contains('$') || symbol.contains('.');
    let is_derivative_or_special_class = [
        " warrant",
        " warrants",
        " right",
        " rights",
        " unit",
        " units",
        "preferred",
        "preference",
        "blank check",
        "acquisition",
        " senior notes",
        " notes due",
        " debenture",
        " bond",
    ]
    .iter()
    .any(|needle| lower_name.contains(needle));

    !symbol_has_structured_suffix && !is_derivative_or_special_class
}

pub fn clean_security_name(raw: &str) -> String {
    let mut name = raw.trim().trim_end_matches('.').to_string();
    for marker in [
        " - Common Stock",
        " - Common Shares",
        " - Ordinary Shares",
        " - Class A Common Stock",
        " - Class B Common Stock",
        " - Class A Ordinary Shares",
        " - Class B Ordinary Shares",
        " Common Stock",
        " Common Shares",
        " Ordinary Shares",
        " Class A Common Stock",
        " Class B Common Stock",
        " Class A Ordinary Shares",
        " Class B Ordinary Shares",
    ] {
        if let Some(stripped) = name.strip_suffix(marker) {
            name = stripped.trim().to_string();
            break;
        }
    }

    if let Some((prefix, _)) = name.split_once(" American Depositary Shares") {
        name = prefix.trim().to_string();
    } else if let Some((prefix, _)) = name.split_once(" American Depository Shares") {
        name = prefix.trim().to_string();
    }

    name
}

fn header_map(header: &str) -> HashMap<String, usize> {
    header
        .split('|')
        .enumerate()
        .map(|(index, name)| (name.trim().to_string(), index))
        .collect()
}

fn get_field<'a>(
    fields: &'a [&str],
    headers: &HashMap<String, usize>,
    name: &str,
) -> Option<&'a str> {
    fields.get(*headers.get(name)?).copied()
}

fn exchange_name(code: &str) -> String {
    match code {
        "A" => "NYSE American",
        "N" => "NYSE",
        "P" => "NYSE Arca",
        "Z" => "Cboe BZX",
        "V" => "IEX",
        "Q" => "NASDAQ",
        other => other,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn normalizes_and_dedupes_symbols() {
        let rows = vec![
            InstrumentImport {
                symbol: normalize_symbol(" aapl ").unwrap(),
                name: "Apple Inc.".to_string(),
                exchange: "NASDAQ".to_string(),
                asset_type: "stock".to_string(),
            },
            InstrumentImport {
                symbol: normalize_symbol("AAPL").unwrap(),
                name: "Apple Inc. Duplicate".to_string(),
                exchange: "NASDAQ".to_string(),
                asset_type: "stock".to_string(),
            },
        ];
        let mut connection = db::open_memory().unwrap();
        let summary = upsert_imports(&mut connection, rows).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM instruments", [], |row| row.get(0))
            .unwrap();

        assert_eq!(summary.upserted, 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn parses_headers_and_skips_test_issues() {
        let text = "Symbol|Security Name|Market Category|Test Issue|ETF\nAAPL|Apple Inc.|Q|N|N\nTEST|Test Co|Q|Y|N\nFile Creation Time: 0701202606:00||||\n";
        let rows = parse_nasdaq_listed(text);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "AAPL");
        assert_eq!(rows[0].asset_type, "stock");
    }

    #[test]
    fn filters_niche_etfs_and_structured_symbols() {
        assert!(is_useful_instrument(
            "AAPL",
            "Apple Inc. - Common Stock",
            "stock"
        ));
        assert!(is_useful_instrument(
            "SPY",
            "State Street SPDR S&P 500 ETF Trust",
            "etf"
        ));
        assert!(!is_useful_instrument(
            "AAPU",
            "Direxion Daily AAPL Bull 2X ETF",
            "etf"
        ));
        assert!(!is_useful_instrument(
            "AACIW",
            "Armada Acquisition Corp. III - Warrant",
            "stock"
        ));
        assert!(!is_useful_instrument(
            "AACB",
            "Artius II Acquisition Inc. - Class A Ordinary Shares",
            "stock"
        ));
        assert!(!is_useful_instrument(
            "ABXL",
            "Abacus Global Management, Inc. 9.875% Fixed Rate Senior Notes due 2028",
            "stock"
        ));
        assert!(!is_useful_instrument(
            "ABR$D",
            "Arbor Realty Trust Preferred Stock",
            "stock"
        ));
    }

    #[test]
    fn cleans_directory_security_names() {
        assert_eq!(
            clean_security_name("NVIDIA Corporation - Common Stock"),
            "NVIDIA Corporation"
        );
        assert_eq!(
            clean_security_name("Apple Inc. - Common Stock"),
            "Apple Inc."
        );
        assert_eq!(
            clean_security_name(
                "Ambev S.A. American Depositary Shares (Each representing 1 Common Share)"
            ),
            "Ambev S.A."
        );
    }

    #[test]
    fn detects_recent_updates_for_resume_logic() {
        let connection = db::open_memory().unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, active, last_updated) VALUES (?1, ?2, 1, ?3)",
                rusqlite::params!["AAPL", "Apple Inc.", chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();

        assert!(was_updated_recently(&connection, "AAPL", 24).unwrap());
    }
}
