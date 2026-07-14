use std::collections::HashSet;

use crate::{
    app::App,
    features::news::feed::{self as news, NewsItem},
};

use super::{
    NEWS_LIMIT,
    format::{
        format_news_items, normalize_agent_symbol, priority_label, priority_score, trim_chars,
    },
};

impl App {
    pub fn agent_summarize_symbol_news(&self, symbol: &str) -> Result<String, String> {
        let symbol = normalize_agent_symbol(symbol)?;
        let items = self.agent_news_for_symbol(&symbol, NEWS_LIMIT);
        if items.is_empty() {
            return Ok(format!("no local news found for {symbol}"));
        }
        let mut lines = vec![format!("recent news for {symbol}:")];
        lines.extend(format_news_items(&items, 0));
        Ok(lines.join("\n"))
    }

    pub fn agent_find_news_without_position(&self) -> Result<String, String> {
        let watchlist = self
            .agent_active_watchlist_symbols()
            .into_iter()
            .collect::<HashSet<_>>();
        let mut rows = Vec::new();
        for item in &self.news.items {
            for symbol in &item.symbols {
                if !watchlist.contains(symbol) {
                    rows.push((symbol.clone(), item));
                }
            }
        }
        rows.sort_by(|left, right| {
            priority_score(right.1.priority).cmp(&priority_score(left.1.priority))
        });
        rows.dedup_by(|left, right| left.0 == right.0 && left.1.id == right.1.id);

        if rows.is_empty() {
            return Ok("no important local news symbols outside the active watchlist".to_string());
        }

        let mut lines = vec!["news symbols not on active watchlist:".to_string()];
        for (symbol, item) in rows.into_iter().take(NEWS_LIMIT) {
            lines.push(format!(
                "- {symbol}: [{}] {} ({})",
                priority_label(item.priority),
                trim_chars(&item.title, 120),
                item.source
            ));
        }
        Ok(lines.join("\n"))
    }

    pub fn agent_surface_attention_list(&self) -> Result<String, String> {
        let symbols = self.agent_active_watchlist_symbols();
        if symbols.is_empty() && self.news.items.is_empty() {
            return Ok("no local watchlist or news signals are loaded yet".to_string());
        }

        let mut rows = Vec::new();
        for quote in self.agent_quotes_for_symbols(&symbols) {
            let news_count = self.agent_news_for_symbol(&quote.symbol, 3).len();
            let mut score = quote.change_percent.abs();
            score += quote.relative_volume.unwrap_or_default().max(1.0) - 1.0;
            score += news_count as f64 * 0.5;
            rows.push((
                score,
                format!(
                    "{}: {} · {} · {} matching news",
                    quote.symbol,
                    super::format::percent(quote.change_percent),
                    super::format::relative_volume_label(quote.relative_volume),
                    news_count
                ),
            ));
        }
        for item in &self.news.items {
            let score = priority_score(item.priority) as f64;
            if score >= 3.0 {
                let symbols = if item.symbols.is_empty() {
                    "macro".to_string()
                } else {
                    item.symbols.join(",")
                };
                rows.push((
                    score,
                    format!(
                        "news {symbols}: [{}] {}",
                        priority_label(item.priority),
                        trim_chars(&item.title, 110)
                    ),
                ));
            }
        }

        rows.sort_by(|left, right| right.0.total_cmp(&left.0));
        rows.dedup_by(|left, right| left.1 == right.1);
        if rows.is_empty() {
            return Ok("no high-priority local attention signals found".to_string());
        }

        Ok(format!(
            "attention list:\n{}",
            rows.into_iter()
                .take(10)
                .map(|(_, row)| format!("- {row}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }

    pub(super) fn agent_news_for_symbols(&self, symbols: &[String], limit: usize) -> Vec<NewsItem> {
        let symbol_set = symbols.iter().collect::<HashSet<_>>();
        let mut items = self
            .news
            .items
            .iter()
            .filter(|item| {
                item.symbols
                    .iter()
                    .any(|symbol| symbol_set.contains(symbol))
                    || symbols
                        .iter()
                        .any(|symbol| news::contains_symbol(&item.title, symbol))
            })
            .cloned()
            .collect::<Vec<_>>();
        sort_news(&mut items);
        items.truncate(limit);
        items
    }

    pub(super) fn agent_news_for_symbol(&self, symbol: &str, limit: usize) -> Vec<NewsItem> {
        let mut items = self
            .news
            .items
            .iter()
            .filter(|item| {
                item.symbols
                    .iter()
                    .any(|item_symbol| item_symbol.eq_ignore_ascii_case(symbol))
                    || news::contains_symbol(&item.title, symbol)
            })
            .cloned()
            .collect::<Vec<_>>();
        sort_news(&mut items);
        items.truncate(limit);
        items
    }
}

fn sort_news(items: &mut [NewsItem]) {
    items.sort_by(|left, right| {
        priority_score(right.priority)
            .cmp(&priority_score(left.priority))
            .then_with(|| right.published_at.cmp(&left.published_at))
    });
}
