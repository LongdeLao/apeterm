pub mod client;
pub mod form4;
pub mod submissions;
pub mod sync;
pub mod thirteenf;
pub mod types;

use rusqlite::{Connection, params};

pub use types::*;

pub fn ensure_seeded(connection: &Connection) -> rusqlite::Result<()> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM sec_entities", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let seeds = serde_json::from_str::<Vec<types::SeedEntity>>(include_str!("../../assets/sec_watchlist.json"))
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;

    let transaction = connection.unchecked_transaction()?;
    for seed in seeds {
        transaction.execute(
            "
            INSERT OR IGNORE INTO sec_entities(kind, name, filer_cik, issuer_ticker, subtitle)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                seed.kind.as_db_str(),
                seed.name,
                seed.filer_cik,
                seed.issuer_ticker,
                seed.subtitle
            ],
        )?;
    }
    transaction.commit()
}
