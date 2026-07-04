pub mod client;
pub mod form4;
pub mod repo;
pub mod state;
pub mod submissions;
pub mod sync;
pub mod thirteenf;
pub mod types;
pub mod view;

use rusqlite::{Connection, params};

pub use types::*;

pub fn ensure_seeded(connection: &Connection) -> rusqlite::Result<()> {
    let seeds = serde_json::from_str::<Vec<types::SeedEntity>>(include_str!(
        "../../../assets/sec_watchlist.json"
    ))
    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;

    let transaction = connection.unchecked_transaction()?;
    for seed in seeds {
        transaction.execute(
            "
            INSERT INTO sec_entities(kind, name, filer_cik, issuer_ticker, subtitle)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(filer_cik) DO UPDATE SET
              kind = excluded.kind,
              name = excluded.name,
              issuer_ticker = excluded.issuer_ticker,
              subtitle = excluded.subtitle
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
