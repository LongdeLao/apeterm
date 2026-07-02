use std::path::Path;

use rusqlite::{Connection, Result};

use crate::config;

#[path = "db/sec_repo.rs"]
pub mod sec_repo;
#[path = "db/notes_repo.rs"]
pub mod notes_repo;

pub fn open(path: &Path) -> Result<Connection> {
    config::ensure_parent(path)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    let connection = Connection::open(path)?;
    initialize(&connection)?;
    Ok(connection)
}

#[cfg(test)]
pub fn open_memory() -> Result<Connection> {
    let connection = Connection::open_in_memory()?;
    initialize(&connection)?;
    Ok(connection)
}

pub fn initialize(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS instruments (
          symbol TEXT PRIMARY KEY,
          name TEXT,
          exchange TEXT,
          asset_type TEXT,
          sector TEXT,
          industry TEXT,
          currency TEXT,
          active INTEGER DEFAULT 1,
          last_updated TEXT
        );

        CREATE INDEX IF NOT EXISTS instruments_active_symbol_idx
          ON instruments(active, symbol);
        CREATE INDEX IF NOT EXISTS instruments_last_updated_idx
          ON instruments(last_updated);
        CREATE INDEX IF NOT EXISTS instruments_sector_idx
          ON instruments(sector);

        CREATE VIEW IF NOT EXISTS stocks AS
          SELECT symbol, name, exchange, sector, industry, currency, active, last_updated
          FROM instruments
          WHERE asset_type = 'stock';

        CREATE VIEW IF NOT EXISTS etfs AS
          SELECT symbol, name, exchange, sector, industry, currency, active, last_updated
          FROM instruments
          WHERE asset_type = 'etf';

        CREATE VIRTUAL TABLE IF NOT EXISTS instruments_fts USING fts5(
          symbol,
          name,
          sector,
          industry,
          content='instruments',
          content_rowid='rowid',
          tokenize='unicode61'
        );

        CREATE TRIGGER IF NOT EXISTS instruments_ai
        AFTER INSERT ON instruments BEGIN
          INSERT INTO instruments_fts(rowid, symbol, name, sector, industry)
          VALUES (new.rowid, new.symbol, new.name, new.sector, new.industry);
        END;

        CREATE TRIGGER IF NOT EXISTS instruments_ad
        AFTER DELETE ON instruments BEGIN
          INSERT INTO instruments_fts(instruments_fts, rowid, symbol, name, sector, industry)
          VALUES ('delete', old.rowid, old.symbol, old.name, old.sector, old.industry);
        END;

        CREATE TRIGGER IF NOT EXISTS instruments_au
        AFTER UPDATE ON instruments BEGIN
          INSERT INTO instruments_fts(instruments_fts, rowid, symbol, name, sector, industry)
          VALUES ('delete', old.rowid, old.symbol, old.name, old.sector, old.industry);
          INSERT INTO instruments_fts(rowid, symbol, name, sector, industry)
          VALUES (new.rowid, new.symbol, new.name, new.sector, new.industry);
        END;

        CREATE TABLE IF NOT EXISTS update_runs (
          name TEXT PRIMARY KEY,
          last_started TEXT,
          last_finished TEXT,
          status TEXT
        );

        CREATE TABLE IF NOT EXISTS sec_entities (
          id INTEGER PRIMARY KEY,
          kind TEXT NOT NULL CHECK(kind IN ('institution','ceo')),
          name TEXT NOT NULL,
          filer_cik TEXT NOT NULL UNIQUE,
          issuer_ticker TEXT,
          subtitle TEXT
        );

        CREATE TABLE IF NOT EXISTS thirteenf_holdings (
          id INTEGER PRIMARY KEY,
          entity_id INTEGER NOT NULL REFERENCES sec_entities(id),
          period_of_report TEXT NOT NULL,
          cusip TEXT NOT NULL,
          ticker TEXT,
          shares INTEGER NOT NULL,
          value_usd INTEGER NOT NULL,
          accession_no TEXT NOT NULL,
          UNIQUE(entity_id, period_of_report, cusip)
        );

        CREATE TABLE IF NOT EXISTS insider_transactions (
          id INTEGER PRIMARY KEY,
          entity_id INTEGER NOT NULL REFERENCES sec_entities(id),
          ticker TEXT NOT NULL,
          filed_at TEXT NOT NULL,
          transaction_date TEXT NOT NULL,
          code TEXT NOT NULL,
          shares REAL NOT NULL,
          price_usd REAL,
          shares_owned_after REAL,
          accession_no TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS congress_transactions (
          id INTEGER PRIMARY KEY,
          entity_id INTEGER NOT NULL REFERENCES sec_entities(id),
          filing_id TEXT NOT NULL,
          transaction_index INTEGER NOT NULL,
          chamber TEXT NOT NULL,
          source_url TEXT NOT NULL,
          filed_at TEXT,
          transaction_date TEXT NOT NULL,
          notification_date TEXT,
          owner_code TEXT,
          asset_name TEXT NOT NULL,
          ticker TEXT,
          transaction_type TEXT NOT NULL,
          amount_range TEXT NOT NULL,
          description TEXT,
          UNIQUE(entity_id, filing_id, transaction_index)
        );

        CREATE TABLE IF NOT EXISTS sec_sync_state (
          entity_id INTEGER PRIMARY KEY REFERENCES sec_entities(id),
          last_accession_seen TEXT,
          last_polled_at TEXT
        );

        CREATE TABLE IF NOT EXISTS notes (
          id INTEGER PRIMARY KEY,
          body TEXT NOT NULL,
          tickers TEXT,
          tags TEXT,
          pinned INTEGER NOT NULL DEFAULT 0,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
          body,
          content='notes',
          content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS notes_ai
        AFTER INSERT ON notes BEGIN
          INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
        END;

        CREATE TRIGGER IF NOT EXISTS notes_ad
        AFTER DELETE ON notes BEGIN
          INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
        END;

        CREATE TRIGGER IF NOT EXISTS notes_au
        AFTER UPDATE ON notes BEGIN
          INSERT INTO notes_fts(notes_fts, rowid, body) VALUES ('delete', old.id, old.body);
          INSERT INTO notes_fts(rowid, body) VALUES (new.id, new.body);
        END;
        ",
    )?;

    static FTS_REBUILT: std::sync::Once = std::sync::Once::new();
    FTS_REBUILT.call_once(|| {
        let _ = connection.execute("INSERT INTO instruments_fts(instruments_fts) VALUES('rebuild')", []);
    });
    Ok(())
}
