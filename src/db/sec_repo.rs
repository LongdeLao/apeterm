use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use crate::sec::types::{
    EntityKind, HoldingDelta, HoldingDeltaKind, HoldingRow, InsiderTx, SecEntity,
};

pub fn list_entities(connection: &Connection, kind: EntityKind) -> rusqlite::Result<Vec<SecEntity>> {
    let mut statement = connection.prepare(
        "
        SELECT id, kind, name, filer_cik, issuer_ticker, subtitle
        FROM sec_entities
        WHERE kind = ?1
        ORDER BY name
        ",
    )?;
    let rows = statement.query_map([kind.as_db_str()], map_entity)?;
    rows.collect()
}

pub fn list_all_entities(connection: &Connection) -> rusqlite::Result<Vec<SecEntity>> {
    let mut statement = connection.prepare(
        "
        SELECT id, kind, name, filer_cik, issuer_ticker, subtitle
        FROM sec_entities
        ORDER BY kind, name
        ",
    )?;
    let rows = statement.query_map([], map_entity)?;
    rows.collect()
}

pub fn get_entity(connection: &Connection, entity_id: i64) -> rusqlite::Result<Option<SecEntity>> {
    connection
        .query_row(
            "
            SELECT id, kind, name, filer_cik, issuer_ticker, subtitle
            FROM sec_entities
            WHERE id = ?1
            ",
            [entity_id],
            map_entity,
        )
        .optional()
}

pub fn latest_holdings(connection: &Connection, entity_id: i64) -> rusqlite::Result<Vec<HoldingRow>> {
    let period = latest_period(connection, entity_id)?;
    let Some(period) = period else {
        return Ok(Vec::new());
    };

    let mut statement = connection.prepare(
        "
        SELECT cusip, ticker, shares, value_usd
        FROM thirteenf_holdings
        WHERE entity_id = ?1 AND period_of_report = ?2
        ORDER BY value_usd DESC, ticker, cusip
        ",
    )?;
    let rows = statement.query_map(params![entity_id, period], |row| {
        Ok(HoldingRow {
            cusip: row.get(0)?,
            ticker: row.get(1)?,
            shares: row.get(2)?,
            value_usd: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn holding_deltas(connection: &Connection, entity_id: i64) -> rusqlite::Result<Vec<HoldingDelta>> {
    let periods = latest_two_periods(connection, entity_id)?;
    let Some(current_period) = periods.first() else {
        return Ok(Vec::new());
    };
    let current = holdings_for_period(connection, entity_id, current_period)?;
    let previous = periods
        .get(1)
        .map(|period| holdings_for_period(connection, entity_id, period))
        .transpose()?
        .unwrap_or_default();

    let mut previous_by_cusip = previous
        .into_iter()
        .map(|row| (row.cusip.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut deltas = Vec::new();

    for row in current {
        let previous_row = previous_by_cusip.remove(&row.cusip);
        let previous_shares = previous_row.as_ref().map(|value| value.shares).unwrap_or(0);
        let kind = match previous_row {
            None => HoldingDeltaKind::New,
            Some(ref prior) if row.shares > prior.shares => HoldingDeltaKind::Increased,
            Some(ref prior) if row.shares < prior.shares => HoldingDeltaKind::Decreased,
            Some(_) => HoldingDeltaKind::Unchanged,
        };
        deltas.push(HoldingDelta {
            cusip: row.cusip,
            ticker: row.ticker,
            current_shares: row.shares,
            previous_shares,
            kind,
        });
    }

    for (_, row) in previous_by_cusip {
        deltas.push(HoldingDelta {
            cusip: row.cusip,
            ticker: row.ticker,
            current_shares: 0,
            previous_shares: row.shares,
            kind: HoldingDeltaKind::Exited,
        });
    }

    deltas.sort_by(|left, right| {
        right
            .current_shares
            .cmp(&left.current_shares)
            .then_with(|| left.ticker.cmp(&right.ticker))
            .then_with(|| left.cusip.cmp(&right.cusip))
    });
    Ok(deltas)
}

pub fn portfolio_value_history(
    connection: &Connection,
    entity_id: i64,
) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut statement = connection.prepare(
        "
        SELECT period_of_report, SUM(value_usd) AS total_value
        FROM thirteenf_holdings
        WHERE entity_id = ?1
        GROUP BY period_of_report
        ORDER BY period_of_report ASC
        ",
    )?;
    let rows = statement.query_map([entity_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn recent_insider_txs(
    connection: &Connection,
    entity_id: i64,
    limit: usize,
) -> rusqlite::Result<Vec<InsiderTx>> {
    let mut statement = connection.prepare(
        "
        SELECT ticker, filed_at, transaction_date, code, shares, price_usd, shares_owned_after, accession_no
        FROM insider_transactions
        WHERE entity_id = ?1
        ORDER BY transaction_date DESC, accession_no DESC
        LIMIT ?2
        ",
    )?;
    let rows = statement.query_map(params![entity_id, limit as i64], |row| {
        Ok(InsiderTx {
            ticker: row.get(0)?,
            filed_at: row.get(1)?,
            transaction_date: row.get(2)?,
            code: row.get(3)?,
            shares: row.get(4)?,
            price_usd: row.get(5)?,
            shares_owned_after: row.get(6)?,
            accession_no: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn last_accession_seen(connection: &Connection, entity_id: i64) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT last_accession_seen FROM sec_sync_state WHERE entity_id = ?1",
            [entity_id],
            |row| row.get(0),
        )
        .optional()
}

pub fn last_polled_at(connection: &Connection, entity_id: i64) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT last_polled_at FROM sec_sync_state WHERE entity_id = ?1",
            [entity_id],
            |row| row.get(0),
        )
        .optional()
}

pub fn latest_transaction_code(
    connection: &Connection,
    entity_id: i64,
) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "
            SELECT code
            FROM insider_transactions
            WHERE entity_id = ?1
            ORDER BY transaction_date DESC, accession_no DESC
            LIMIT 1
            ",
            [entity_id],
            |row| row.get(0),
        )
        .optional()
}

fn latest_period(connection: &Connection, entity_id: i64) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "
            SELECT period_of_report
            FROM thirteenf_holdings
            WHERE entity_id = ?1
            ORDER BY period_of_report DESC
            LIMIT 1
            ",
            [entity_id],
            |row| row.get(0),
        )
        .optional()
}

fn latest_two_periods(connection: &Connection, entity_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare(
        "
        SELECT DISTINCT period_of_report
        FROM thirteenf_holdings
        WHERE entity_id = ?1
        ORDER BY period_of_report DESC
        LIMIT 2
        ",
    )?;
    let rows = statement.query_map([entity_id], |row| row.get(0))?;
    rows.collect()
}

fn holdings_for_period(
    connection: &Connection,
    entity_id: i64,
    period: &str,
) -> rusqlite::Result<Vec<HoldingRow>> {
    let mut statement = connection.prepare(
        "
        SELECT cusip, ticker, shares, value_usd
        FROM thirteenf_holdings
        WHERE entity_id = ?1 AND period_of_report = ?2
        ",
    )?;
    let rows = statement.query_map(params![entity_id, period], |row| {
        Ok(HoldingRow {
            cusip: row.get(0)?,
            ticker: row.get(1)?,
            shares: row.get(2)?,
            value_usd: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn map_entity(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecEntity> {
    let kind = match row.get::<_, String>(1)?.as_str() {
        "institution" => EntityKind::Institution,
        _ => EntityKind::Ceo,
    };
    Ok(SecEntity {
        id: row.get(0)?,
        kind,
        name: row.get(2)?,
        filer_cik: row.get(3)?,
        issuer_ticker: row.get(4)?,
        subtitle: row.get(5)?,
    })
}

