use crate::{
    app::App,
    backend::BackendInsight,
    db,
    features::{search::engine as search, watchlist::quotes::Quote},
};

use super::{
    NEWS_LIMIT,
    format::{
        format_backend_insight, format_news_items, format_notes, join_or_none, money_opt,
        number_opt, option_label, percent, quote_rank, relative_volume_label, trim_chars,
    },
};

impl App {
    pub fn agent_summarize_watchlist(&self) -> Result<String, String> {
        let symbols = self.agent_active_watchlist_symbols();
        if symbols.is_empty() {
            return Err("active watchlist is empty".to_string());
        }

        let mut lines = vec![format!(
            "active watchlist `{}`: {}",
            self.active_watchlist().name,
            symbols.join(", ")
        )];
        lines.push("quotes:".to_string());
        lines.extend(
            symbols
                .iter()
                .map(|symbol| format!("  {}", self.agent_symbol_quote_line(symbol))),
        );
        lines.push(format!(
            "missing quotes: {}",
            join_or_none(&self.agent_missing_quotes(&symbols))
        ));
        lines.push("fresh watchlist news:".to_string());
        lines.extend(format_news_items(
            &self.agent_news_for_symbols(&symbols, NEWS_LIMIT),
            2,
        ));
        lines.push("notes coverage:".to_string());
        lines.extend(
            self.agent_note_counts_for_symbols(&symbols)
                .into_iter()
                .map(|(symbol, count)| {
                    format!(
                        "  {symbol}: {count} note{}",
                        if count == 1 { "" } else { "s" }
                    )
                }),
        );
        Ok(lines.join("\n"))
    }

    pub fn agent_explain_watchlist_move(&self) -> Result<String, String> {
        let symbols = self.agent_active_watchlist_symbols();
        if symbols.is_empty() {
            return Err("active watchlist is empty".to_string());
        }

        let mut movers = self.agent_quotes_for_symbols(&symbols);
        movers.sort_by(|left, right| {
            right
                .change_percent
                .abs()
                .total_cmp(&left.change_percent.abs())
        });

        let mut lines = vec!["watchlist move evidence:".to_string()];
        for quote in movers.into_iter().take(8) {
            let news_items = self.agent_news_for_symbol(&quote.symbol, 3);
            lines.push(format!(
                "- {}: {} ({})",
                quote.symbol,
                percent(quote.change_percent),
                relative_volume_label(quote.relative_volume)
            ));
            lines.extend(format_news_items(&news_items, 4));
            if let Some(insight) = self.agent_current_backend_insight_for(&quote.symbol) {
                lines.extend(format_backend_insight(insight, 2));
            }
        }
        if lines.len() == 1 {
            lines.push("no quote data is loaded for active watchlist symbols".to_string());
        }
        Ok(lines.join("\n"))
    }

    pub fn agent_find_watchlist_outliers(&self) -> Result<String, String> {
        let symbols = self.agent_active_watchlist_symbols();
        if symbols.is_empty() {
            return Err("active watchlist is empty".to_string());
        }

        let quotes = self.agent_quotes_for_symbols(&symbols);
        let mut gainers = quotes.clone();
        gainers.sort_by(|left, right| right.change_percent.total_cmp(&left.change_percent));
        let mut losers = quotes.clone();
        losers.sort_by(|left, right| left.change_percent.total_cmp(&right.change_percent));
        let mut volume = quotes
            .iter()
            .filter(|quote| quote.relative_volume.is_some())
            .cloned()
            .collect::<Vec<_>>();
        volume.sort_by(|left, right| {
            right
                .relative_volume
                .unwrap_or_default()
                .total_cmp(&left.relative_volume.unwrap_or_default())
        });
        let no_news = quotes
            .iter()
            .filter(|quote| {
                quote.change_percent.abs() >= 1.0
                    && self.agent_news_for_symbol(&quote.symbol, 1).is_empty()
            })
            .map(|quote| format!("{} {}", quote.symbol, percent(quote.change_percent)))
            .collect::<Vec<_>>();

        Ok([
            format!("top gainers: {}", quote_rank(gainers.iter().take(5))),
            format!("top losers: {}", quote_rank(losers.iter().take(5))),
            format!(
                "relative-volume spikes: {}",
                quote_rank(volume.iter().take(5))
            ),
            format!(
                "moves without matching local news: {}",
                join_or_none(&no_news)
            ),
            format!(
                "missing quotes: {}",
                join_or_none(&self.agent_missing_quotes(&symbols))
            ),
        ]
        .join("\n"))
    }

    pub fn agent_compare_symbols(&self, symbols: &[String]) -> Result<String, String> {
        if !(2..=5).contains(&symbols.len()) {
            return Err("compare_symbols requires 2 to 5 symbols".to_string());
        }

        let connection = db::open(&self.ticker_db_path).map_err(|error| error.to_string())?;
        let mut lines = vec![format!("comparison: {}", symbols.join(", "))];
        for symbol in symbols {
            lines.push(format!("\n{symbol}:"));
            match search::details(&connection, symbol).map_err(|error| error.to_string())? {
                Some(details) => lines.push(format!(
                    "  profile: {} · {} · {} · {}",
                    details.name,
                    option_label(details.exchange.as_deref()),
                    option_label(details.sector.as_deref()),
                    option_label(details.industry.as_deref())
                )),
                None => lines.push("  profile: not found in local ticker db".to_string()),
            }
            lines.push(format!("  quote: {}", self.agent_symbol_quote_line(symbol)));
            lines.push(format!(
                "  notes: {}",
                self.agent_notes_for_symbol(symbol).len()
            ));
            lines.extend(format_news_items(&self.agent_news_for_symbol(symbol, 3), 4));
            if let Some(insight) = self.agent_current_backend_insight_for(symbol) {
                lines.extend(format_backend_insight(insight, 2));
            }
        }
        Ok(lines.join("\n"))
    }

    pub fn agent_brief_selected_symbol(&self) -> Result<String, String> {
        let details = self
            .search
            .selected_details
            .as_ref()
            .ok_or_else(|| "no selected symbol; open a symbol first".to_string())?;
        let symbol = details.symbol.as_str();

        let mut lines = vec![
            format!("selected symbol: {symbol}"),
            format!(
                "profile: {} · {} · {} · {}",
                details.name,
                option_label(details.exchange.as_deref()),
                option_label(details.sector.as_deref()),
                option_label(details.industry.as_deref())
            ),
            format!("quote: {}", self.agent_symbol_quote_line(symbol)),
        ];
        if let Some(live) = &self.search.selected_live_details {
            lines.push(format!(
                "live metrics: market cap {} · trailing PE {} · forward PE {} · beta {} · next earnings {}",
                money_opt(live.market_cap),
                number_opt(live.trailing_pe),
                number_opt(live.forward_pe),
                number_opt(live.beta),
                live.next_earnings_days
                    .map(|days| format!("{days}d"))
                    .unwrap_or_else(|| "n/a".to_string())
            ));
            if let Some(summary) = live
                .summary
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                lines.push(format!("business summary: {}", trim_chars(summary, 360)));
            }
        }
        let notes = self.agent_notes_for_symbol(symbol);
        lines.push(format!("notes: {}", notes.len()));
        lines.extend(format_notes(&notes, 2));
        lines.push("recent news:".to_string());
        lines.extend(format_news_items(
            &self.agent_news_for_symbol(symbol, NEWS_LIMIT),
            2,
        ));
        if let Some(insight) = self.agent_current_backend_insight_for(symbol) {
            lines.extend(format_backend_insight(insight, 0));
        }
        Ok(lines.join("\n"))
    }

    pub(super) fn agent_active_watchlist_symbols(&self) -> Vec<String> {
        self.stock_watchlist()
            .iter()
            .chain(self.crypto_watchlist())
            .cloned()
            .collect()
    }

    pub(super) fn agent_quotes_for_symbols(&self, symbols: &[String]) -> Vec<Quote> {
        symbols
            .iter()
            .filter_map(|symbol| self.agent_quote_for_symbol(symbol).cloned())
            .collect()
    }

    pub(super) fn agent_quote_for_symbol(&self, symbol: &str) -> Option<&Quote> {
        self.watchlist
            .stock_quotes
            .iter()
            .chain(self.watchlist.crypto_quotes.iter())
            .find(|quote| quote.symbol.eq_ignore_ascii_case(symbol))
    }

    pub(super) fn agent_symbol_quote_line(&self, symbol: &str) -> String {
        match self.agent_quote_for_symbol(symbol) {
            Some(quote) => format!(
                "{} ${:.2} {} · {}",
                quote.symbol,
                quote.price,
                percent(quote.change_percent),
                relative_volume_label(quote.relative_volume)
            ),
            None => format!("{symbol}: no loaded quote"),
        }
    }

    pub(super) fn agent_current_backend_insight_for(
        &self,
        symbol: &str,
    ) -> Option<&BackendInsight> {
        self.search
            .backend_insight
            .as_ref()
            .filter(|insight| insight.ticker.eq_ignore_ascii_case(symbol))
    }

    fn agent_missing_quotes(&self, symbols: &[String]) -> Vec<String> {
        symbols
            .iter()
            .filter(|symbol| self.agent_quote_for_symbol(symbol).is_none())
            .cloned()
            .collect()
    }
}
