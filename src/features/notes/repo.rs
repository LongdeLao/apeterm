use std::collections::HashSet;

use rusqlite::{Connection, Result, params};

#[derive(Debug, Clone, PartialEq)]
pub struct NoteRow {
    pub id: i64,
    pub body: String,
    pub tickers: Vec<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn list_all(connection: &Connection) -> Result<Vec<NoteRow>> {
    let mut statement = connection.prepare(
        "
        SELECT id, body, tickers, tags, pinned, created_at, updated_at
        FROM notes
        ORDER BY pinned DESC, updated_at DESC
        ",
    )?;
    let rows = statement.query_map([], map_note)?;
    rows.collect()
}

pub fn insert(
    connection: &Connection,
    body: &str,
    tickers: &[String],
    tags: &[String],
    now: i64,
) -> Result<i64> {
    connection.execute(
        "
        INSERT INTO notes(body, tickers, tags, pinned, created_at, updated_at)
        VALUES (?1, ?2, ?3, 0, ?4, ?4)
        ",
        params![body, join_list(tickers), join_list(tags), now],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn update(
    connection: &Connection,
    id: i64,
    body: &str,
    tickers: &[String],
    tags: &[String],
    now: i64,
) -> Result<()> {
    connection.execute(
        "
        UPDATE notes
        SET body = ?2, tickers = ?3, tags = ?4, updated_at = ?5
        WHERE id = ?1
        ",
        params![id, body, join_list(tickers), join_list(tags), now],
    )?;
    Ok(())
}

pub fn delete(connection: &Connection, id: i64) -> Result<()> {
    connection.execute("DELETE FROM notes WHERE id = ?1", [id])?;
    Ok(())
}

pub fn set_pinned(connection: &Connection, id: i64, pinned: bool) -> Result<()> {
    connection.execute(
        "UPDATE notes SET pinned = ?2 WHERE id = ?1",
        params![id, pinned as i64],
    )?;
    Ok(())
}

pub fn search_fts(connection: &Connection, query: &str) -> Result<Vec<i64>> {
    let Some(match_query) = fts_match_query(query) else {
        return Ok(Vec::new());
    };

    let mut statement = connection.prepare(
        "
        SELECT notes.id
        FROM notes_fts
        JOIN notes ON notes.id = notes_fts.rowid
        WHERE notes_fts MATCH ?1
        ORDER BY bm25(notes_fts) ASC
        ",
    )?;
    let rows = statement.query_map([match_query], |row| row.get(0))?;
    rows.collect()
}

pub fn all_ticker_symbols(connection: &Connection) -> Result<HashSet<String>> {
    let mut statement = connection
        .prepare("SELECT tickers FROM notes WHERE tickers IS NOT NULL AND tickers != ''")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let mut symbols = HashSet::new();
    for row in rows {
        for symbol in row?.split(',') {
            let symbol = symbol.trim();
            if !symbol.is_empty() {
                symbols.insert(symbol.to_string());
            }
        }
    }
    Ok(symbols)
}

fn map_note(row: &rusqlite::Row<'_>) -> Result<NoteRow> {
    Ok(NoteRow {
        id: row.get(0)?,
        body: row.get(1)?,
        tickers: split_list(row.get::<_, Option<String>>(2)?),
        tags: split_list(row.get::<_, Option<String>>(3)?),
        pinned: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn join_list(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join(","))
    }
}

fn split_list(value: Option<String>) -> Vec<String> {
    value
        .as_deref()
        .unwrap_or_default()
        .split([',', ' '])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
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
