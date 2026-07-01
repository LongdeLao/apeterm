use std::path::Path;

use rusqlite::{Connection, Result};

use crate::config;

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
        ",
    )?;

    connection.execute(
        "INSERT INTO instruments_fts(instruments_fts) VALUES('rebuild')",
        [],
    )?;
    Ok(())
}
