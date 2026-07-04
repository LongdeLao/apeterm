use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use rusqlite::{Connection, OptionalExtension, Result, params};
use serde::Deserialize;
use std::{
    env,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const LOCAL_PYTHON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/bin/python");
const YFINANCE_DETAILS_SCRIPT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/yfinance_details.py");
const ENV_PYTHON: &str = "APETERM_PYTHON";
const ENV_SCRIPT_DIR: &str = "APETERM_SCRIPT_DIR";

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub symbol: String,
    pub name: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub exchange: Option<String>,
    pub asset_type: Option<String>,
    pub rank: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentDetails {
    pub symbol: String,
    pub name: String,
    pub exchange: Option<String>,
    pub asset_type: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub currency: Option<String>,
    pub active: bool,
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct HistoryPoint {
    pub ts: i64,
    pub close: f64,
    #[serde(default)]
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct LiveInstrumentDetails {
    pub price: Option<f64>,
    pub previous_close: Option<f64>,
    pub day_volume: Option<f64>,
    pub open: Option<f64>,
    pub day_high: Option<f64>,
    pub day_low: Option<f64>,
    pub market_cap: Option<f64>,
    pub avg_volume: Option<f64>,
    pub extended_price: Option<f64>,
    pub extended_change_percent: Option<f64>,
    pub week_52_high: Option<f64>,
    pub week_52_low: Option<f64>,
    pub trailing_pe: Option<f64>,
    pub forward_pe: Option<f64>,
    pub dividend_yield: Option<f64>,
    pub beta: Option<f64>,
    pub next_earnings_days: Option<i64>,
    pub summary: Option<String>,
    pub summary_de: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub website: Option<String>,
    pub full_time_employees: Option<f64>,
    #[serde(default)]
    pub history: Vec<HistoryPoint>,
}

pub fn search(
    connection: &Connection,
    query: &str,
    asset_type: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    search_assets(connection, query, &[asset_type], limit)
}

pub fn search_assets(
    connection: &Connection,
    query: &str,
    asset_types: &[&str],
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let query = query.trim();
    if asset_types.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let per_asset_limit = limit.max(6);
    let mut results: Vec<SearchResult> = Vec::new();

    for asset_type in asset_types {
        let mut asset_results = if query.is_empty() {
            popular(connection, asset_type, per_asset_limit)?
        } else {
            let mut matches = fts_prefix_search(connection, query, asset_type, per_asset_limit)?;
            if matches.len() < per_asset_limit.min(5) {
                merge_fuzzy_results(connection, query, asset_type, per_asset_limit, &mut matches)?;
            }
            matches
        };

        for result in asset_results.drain(..) {
            if !results
                .iter()
                .any(|existing| existing.symbol == result.symbol)
            {
                results.push(result);
            }
        }
    }

    sort_results_for_query(query, &mut results);
    results.truncate(limit);
    Ok(results)
}

pub fn details(connection: &Connection, symbol: &str) -> Result<Option<InstrumentDetails>> {
    connection
        .query_row(
            "
            SELECT symbol, name, exchange, asset_type, sector, industry, currency,
                   active, last_updated
            FROM instruments
            WHERE symbol = ?1
            ",
            params![symbol],
            |row| {
                Ok(InstrumentDetails {
                    symbol: row.get(0)?,
                    name: row.get(1)?,
                    exchange: row.get(2)?,
                    asset_type: row.get(3)?,
                    sector: row.get(4)?,
                    industry: row.get(5)?,
                    currency: row.get(6)?,
                    active: row.get::<_, i64>(7)? == 1,
                    last_updated: row.get(8)?,
                })
            },
        )
        .optional()
}

pub fn live_details(symbol: &str) -> Option<LiveInstrumentDetails> {
    let mut child = Command::new(python_command())
        .arg("-u")
        .arg(yfinance_details_script())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{symbol}");
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

/// Fast, indexed prefix lookup over `instruments` directly (no FTS5 join),
/// used by Spotlight where keystroke-to-keystroke latency matters more than
/// full-text ranking. Falls back to `popular()` for an empty query.
pub fn spotlight_prefix_search(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let query = query.trim();
    if query.is_empty() {
        return popular(connection, "stock", limit);
    }

    let mut statement = connection.prepare(
        "
        SELECT symbol, name, sector, industry, exchange, asset_type, 0.0 as rank
        FROM instruments
        WHERE active = 1 AND (symbol LIKE ?1 || '%' OR name LIKE ?1 || '%')
        ORDER BY (symbol LIKE ?1 || '%') DESC, symbol ASC
        LIMIT ?2
        ",
    )?;
    rows_to_search_results(statement.query_map(params![query, limit as i64], read_search_result)?)
}

fn popular(connection: &Connection, asset_type: &str, limit: usize) -> Result<Vec<SearchResult>> {
    let mut statement = connection.prepare(
        "
        SELECT symbol, name, sector, industry, exchange, asset_type,
               0.0 as rank
        FROM instruments
        WHERE active = 1 AND asset_type = ?1
        ORDER BY symbol ASC
        LIMIT ?2
        ",
    )?;
    rows_to_search_results(
        statement.query_map(params![asset_type, limit as i64], read_search_result)?,
    )
}

fn fts_prefix_search(
    connection: &Connection,
    query: &str,
    asset_type: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let Some(match_query) = fts_match_query(query) else {
        return Ok(Vec::new());
    };

    let mut statement = connection.prepare(
        "
        SELECT i.symbol, i.name, i.sector, i.industry, i.exchange, i.asset_type,
               bm25(instruments_fts, 12.0, 5.0, 1.5, 1.0) as rank
        FROM instruments_fts
        JOIN instruments i ON i.rowid = instruments_fts.rowid
        WHERE instruments_fts MATCH ?1 AND i.active = 1 AND i.asset_type = ?2
        ORDER BY rank ASC, length(i.symbol) ASC, i.symbol ASC
        LIMIT ?3
        ",
    )?;

    rows_to_search_results(statement.query_map(
        params![match_query, asset_type, limit as i64],
        read_search_result,
    )?)
}

fn merge_fuzzy_results(
    connection: &Connection,
    query: &str,
    asset_type: &str,
    limit: usize,
    results: &mut Vec<SearchResult>,
) -> Result<()> {
    let matcher = SkimMatcherV2::default();
    let mut statement = connection.prepare(
        "
        SELECT symbol, name, sector, industry, exchange, asset_type, 0.0
        FROM instruments
        WHERE active = 1 AND asset_type = ?1
        ",
    )?;
    let rows = statement.query_map(params![asset_type], read_search_result)?;
    let mut fuzzy = Vec::new();

    for row in rows {
        let mut result = row?;
        if results
            .iter()
            .any(|existing| existing.symbol == result.symbol)
        {
            continue;
        }

        let haystack = format!("{} {}", result.symbol, result.name);
        if let Some(score) = matcher.fuzzy_match(&haystack, query) {
            result.rank = -(score as f64);
            fuzzy.push(result);
        }
    }

    fuzzy.sort_by(|left, right| {
        left.rank
            .partial_cmp(&right.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.symbol.len().cmp(&right.symbol.len()))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });

    results.extend(fuzzy.into_iter().take(limit.saturating_sub(results.len())));
    Ok(())
}

fn python_command() -> String {
    if let Ok(value) = env::var(ENV_PYTHON) {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if Path::new(LOCAL_PYTHON).exists() {
        LOCAL_PYTHON.to_string()
    } else {
        "python3".to_string()
    }
}

fn yfinance_details_script() -> PathBuf {
    if let Ok(value) = env::var(ENV_SCRIPT_DIR) {
        let path = Path::new(value.trim()).join("yfinance_details.py");
        if path.exists() {
            return path;
        }
    }
    PathBuf::from(YFINANCE_DETAILS_SCRIPT)
}

fn rows_to_search_results(
    rows: impl Iterator<Item = Result<SearchResult>>,
) -> Result<Vec<SearchResult>> {
    rows.collect()
}

fn read_search_result(row: &rusqlite::Row<'_>) -> Result<SearchResult> {
    Ok(SearchResult {
        symbol: row.get(0)?,
        name: row.get(1)?,
        sector: row.get(2)?,
        industry: row.get(3)?,
        exchange: row.get(4)?,
        asset_type: row.get(5)?,
        rank: row.get(6)?,
    })
}

fn sort_results_for_query(query: &str, results: &mut [SearchResult]) {
    let normalized = query.trim().to_ascii_uppercase();
    results.sort_by(|left, right| {
        exact_symbol_match(right, &normalized)
            .cmp(&exact_symbol_match(left, &normalized))
            .then_with(|| {
                left.rank
                    .partial_cmp(&right.rank)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.symbol.len().cmp(&right.symbol.len()))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
}

fn exact_symbol_match(result: &SearchResult, query: &str) -> bool {
    !query.is_empty() && result.symbol.eq_ignore_ascii_case(query)
}

fn fts_match_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|term| {
            let cleaned: String = term
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect();
            if cleaned.is_empty() {
                None
            } else {
                Some(format!("{cleaned}*"))
            }
        })
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::db;

    #[test]
    fn ranks_symbol_matches_before_name_matches() {
        let connection = db::open_memory().unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["TECH", "Techne Corp", "Healthcare", "Diagnostics"],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["AAPL", "Apple Inc.", "Technology", "Consumer Electronics"],
            )
            .unwrap();

        let rows = search(&connection, "tech", "stock", 5).unwrap();

        assert_eq!(rows[0].symbol, "TECH");
        assert!(rows.iter().any(|row| row.symbol == "AAPL"));
    }

    #[test]
    fn fuzzy_fallback_handles_typos() {
        let connection = db::open_memory().unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["NVDA", "NVIDIA Corporation", "Technology", "Semiconductors"],
            )
            .unwrap();

        let rows = search(&connection, "nvdia", "stock", 5).unwrap();

        assert_eq!(rows[0].symbol, "NVDA");
    }

    #[test]
    fn exact_symbol_matches_rank_first() {
        let connection = db::open_memory().unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["MRVI", "Maravai LifeSciences Holdings, Inc.", "Life Sciences", "Pharmaceutical Preparations"],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["MRVL", "Marvell Technology, Inc.", "Manufacturing", "Semiconductors & Related Devices"],
            )
            .unwrap();

        let rows = search(&connection, "mrvl", "stock", 5).unwrap();

        assert_eq!(rows[0].symbol, "MRVL");
    }

    #[test]
    fn can_search_across_asset_types() {
        let connection = db::open_memory().unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'stock', ?3, ?4, 1)",
                params!["MRVL", "Marvell Technology, Inc.", "Manufacturing", "Semiconductors & Related Devices"],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO instruments(symbol, name, asset_type, sector, industry, active) VALUES (?1, ?2, 'etf', ?3, ?4, 1)",
                params!["XLC", "Communication Services Select Sector SPDR Fund", "ETF", "Sector"],
            )
            .unwrap();

        let rows = search_assets(&connection, "mrvl", &["stock", "etf"], 5).unwrap();

        assert_eq!(rows[0].symbol, "MRVL");
    }
}
