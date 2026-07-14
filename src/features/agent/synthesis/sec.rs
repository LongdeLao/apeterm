use std::collections::HashSet;

use crate::{
    app::{App, SecTab},
    db,
    features::sec::{self, HoldingDeltaKind},
};

use super::{
    NEWS_LIMIT, SEC_LIMIT,
    format::{compact_number, money_opt, normalize_agent_symbol, timestamp_label, trim_chars},
};

impl App {
    pub fn agent_build_symbol_timeline(&self, symbol: &str) -> Result<String, String> {
        let symbol = normalize_agent_symbol(symbol)?;
        let mut events = Vec::new();
        for note in self.agent_notes_for_symbol(&symbol) {
            events.push((
                note.updated_at,
                format!(
                    "note: {}",
                    trim_chars(note.body.lines().next().unwrap_or(""), 140)
                ),
            ));
        }
        for item in self.agent_news_for_symbol(&symbol, NEWS_LIMIT) {
            let ts = item
                .published_at
                .map(|date| date.timestamp())
                .unwrap_or_default();
            events.push((
                ts,
                format!("news: {} ({})", trim_chars(&item.title, 140), item.source),
            ));
        }
        events.extend(self.agent_sec_events_for_symbol(&symbol)?);
        events.sort_by_key(|event| std::cmp::Reverse(event.0));
        if events.is_empty() {
            return Ok(format!("no local timeline events found for {symbol}"));
        }

        let mut lines = vec![format!("timeline for {symbol}:")];
        for (ts, text) in events.into_iter().take(14) {
            lines.push(format!("- {} · {text}", timestamp_label(ts)));
        }
        Ok(lines.join("\n"))
    }

    pub fn agent_summarize_sec_activity(&self) -> Result<String, String> {
        let entity_id = self
            .selected_sec_entity_id()
            .ok_or_else(|| "no selected SEC entity".to_string())?;
        let connection = db::open(&self.ticker_db_path).map_err(|error| error.to_string())?;
        let entity = sec::repo::get_entity(&connection, entity_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "selected SEC entity not found".to_string())?;
        let mut lines = vec![format!("SEC activity for {}:", entity.name)];

        match self.sec.tab {
            SecTab::Institutional => {
                let holdings = sec::repo::latest_holdings(&connection, entity.id)
                    .map_err(|error| error.to_string())?;
                let deltas = sec::repo::holding_deltas(&connection, entity.id)
                    .map_err(|error| error.to_string())?;
                lines.push(format!(
                    "top holdings: {}",
                    holdings
                        .iter()
                        .take(SEC_LIMIT)
                        .map(|row| format!(
                            "{} ${}",
                            row.ticker.as_deref().unwrap_or(row.cusip.as_str()),
                            compact_number(row.value_usd as f64)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                lines.push(format!(
                    "notable deltas: {}",
                    deltas
                        .iter()
                        .filter(|row| row.kind != HoldingDeltaKind::Unchanged)
                        .take(SEC_LIMIT)
                        .map(|row| format!(
                            "{} {:?} {} -> {}",
                            row.ticker.as_deref().unwrap_or(row.cusip.as_str()),
                            row.kind,
                            row.previous_shares,
                            row.current_shares
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            SecTab::Ceos => {
                let txs = sec::repo::recent_insider_txs(&connection, entity.id, SEC_LIMIT)
                    .map_err(|error| error.to_string())?;
                lines.extend(txs.into_iter().map(|tx| {
                    format!(
                        "- {} {} {} shares at {} on {}",
                        tx.ticker,
                        tx.code,
                        tx.shares,
                        money_opt(tx.price_usd),
                        tx.transaction_date
                    )
                }));
            }
            SecTab::Congress => {
                let txs = sec::repo::recent_congress_txs(&connection, entity.id, SEC_LIMIT)
                    .map_err(|error| error.to_string())?;
                lines.extend(txs.into_iter().map(|tx| {
                    format!(
                        "- {} {} {} {} on {}",
                        tx.ticker.as_deref().unwrap_or(tx.asset_name.as_str()),
                        tx.transaction_type,
                        tx.amount_range,
                        tx.chamber,
                        tx.transaction_date
                    )
                }));
            }
        }
        Ok(lines.join("\n"))
    }

    pub fn agent_find_sec_watchlist_matches(&self) -> Result<String, String> {
        let symbols = self
            .agent_active_watchlist_symbols()
            .into_iter()
            .collect::<HashSet<_>>();
        if symbols.is_empty() {
            return Err("active watchlist is empty".to_string());
        }

        let connection = db::open(&self.ticker_db_path).map_err(|error| error.to_string())?;
        let entities =
            sec::repo::list_all_entities(&connection).map_err(|error| error.to_string())?;
        let mut matches = Vec::new();
        for entity in entities {
            for delta in sec::repo::holding_deltas(&connection, entity.id).unwrap_or_default() {
                if delta
                    .ticker
                    .as_ref()
                    .is_some_and(|ticker| symbols.contains(ticker))
                    && delta.kind != HoldingDeltaKind::Unchanged
                {
                    matches.push(format!(
                        "{}: {} {:?} {} -> {}",
                        entity.name,
                        delta.ticker.unwrap_or(delta.cusip),
                        delta.kind,
                        delta.previous_shares,
                        delta.current_shares
                    ));
                }
            }
            for tx in sec::repo::recent_insider_txs(&connection, entity.id, 5).unwrap_or_default() {
                if symbols.contains(&tx.ticker) {
                    matches.push(format!(
                        "{}: insider {} {} shares of {} on {}",
                        entity.name, tx.code, tx.shares, tx.ticker, tx.transaction_date
                    ));
                }
            }
            for tx in sec::repo::recent_congress_txs(&connection, entity.id, 5).unwrap_or_default()
            {
                if tx
                    .ticker
                    .as_ref()
                    .is_some_and(|ticker| symbols.contains(ticker))
                {
                    matches.push(format!(
                        "{}: congress {} {} {} on {}",
                        entity.name,
                        tx.transaction_type,
                        tx.amount_range,
                        tx.ticker.unwrap_or(tx.asset_name),
                        tx.transaction_date
                    ));
                }
            }
        }
        matches.sort();
        matches.dedup();
        if matches.is_empty() {
            return Ok("no SEC activity found for active watchlist symbols".to_string());
        }
        Ok(format!(
            "SEC/watchlist matches:\n{}",
            matches
                .into_iter()
                .take(14)
                .map(|row| format!("- {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }

    fn agent_sec_events_for_symbol(&self, symbol: &str) -> Result<Vec<(i64, String)>, String> {
        let connection = db::open(&self.ticker_db_path).map_err(|error| error.to_string())?;
        let entities =
            sec::repo::list_all_entities(&connection).map_err(|error| error.to_string())?;
        let mut events = Vec::new();
        for entity in entities {
            for tx in sec::repo::recent_insider_txs(&connection, entity.id, 5).unwrap_or_default() {
                if tx.ticker.eq_ignore_ascii_case(symbol) {
                    events.push((
                        0,
                        format!(
                            "SEC insider: {} {} {} shares on {}",
                            entity.name, tx.code, tx.shares, tx.transaction_date
                        ),
                    ));
                }
            }
            for tx in sec::repo::recent_congress_txs(&connection, entity.id, 5).unwrap_or_default()
            {
                if tx
                    .ticker
                    .as_deref()
                    .is_some_and(|ticker| ticker.eq_ignore_ascii_case(symbol))
                {
                    events.push((
                        0,
                        format!(
                            "SEC congress: {} {} {} on {}",
                            entity.name, tx.transaction_type, tx.amount_range, tx.transaction_date
                        ),
                    ));
                }
            }
        }
        Ok(events)
    }
}
