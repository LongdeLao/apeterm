use crate::app::*;
use crate::{
    db,
    features::news::feed::{
        self as news, NewsCategory, NewsItem, NewsPriority, NewsRuntimeConfig, WatchlistMatcher,
    },
    features::search::engine as search,
    i18n::Key,
};
use chrono::{DateTime, Local};
use std::{
    collections::HashSet,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

impl App {
    pub fn news_fetch_on_startup(&self) -> bool {
        self.config.news.fetch_on_startup
    }
    pub fn news_refresh_interval(&self) -> Duration {
        Duration::from_secs(self.config.news.refresh_interval_seconds.max(1))
    }
    pub fn refresh_news(&mut self) {
        if self.news_loading {
            return;
        }

        let runtime_config = NewsRuntimeConfig {
            feeds: self.config.news.feeds.clone(),
            stock_symbols: self.stock_watchlist().to_vec(),
            crypto_symbols: self.crypto_watchlist().to_vec(),
            stock_matchers: self.build_watchlist_matchers(WatchlistKind::Stock),
            crypto_matchers: self.build_watchlist_matchers(WatchlistKind::Crypto),
            enable_rss: self.config.news.enable_rss,
            enable_financial_juice: self.config.news.enable_financial_juice
                && !self.financial_juice_in_cooldown(),
            enable_nasdaq: self.config.news.enable_nasdaq,
        };
        self.news_loading = true;
        self.last_news_refresh = Some(Instant::now());
        self.news_status = Some(self.t(Key::NewsStatusLoading).to_string());
        self.news_connection_status = "reconnecting...".to_string();

        let (sender, receiver) = mpsc::channel();
        self.news_receiver = Some(receiver);
        thread::spawn(move || {
            let result = news::stream_all_news(&runtime_config, |result, done| {
                let _ = sender.send(NewsEvent::Loaded { result, done });
            });
            if let Err(error) = result {
                let _ = sender.send(NewsEvent::Error(error));
            }
        });
    }
    pub fn poll_news(&mut self) {
        if let Some(receiver) = &self.news_receiver {
            match receiver.try_recv() {
                Ok(NewsEvent::Loaded { result, done }) => {
                    let selected_id = self.selected_news.as_ref().map(|item| item.id.clone());
                    self.news_items = result.items;
                    self.news_source_label = result.source_label;
                    self.news_connection_status = result.connection_status;
                    self.news_source_counts = result.source_counts;
                    self.sync_collapsed_watchlist_news();
                    self.news_selection = self
                        .news_selection
                        .min(self.news_visible_rows().len().saturating_sub(1));
                    self.sync_news_scroll(6);
                    self.news_loading = !done;
                    if done {
                        self.news_receiver = None;
                    }
                    self.selected_news = selected_id
                        .and_then(|id| self.news_items.iter().find(|item| item.id == id).cloned());
                    self.news_status = result.status.or_else(|| {
                        if self.news_items.is_empty() {
                            Some(self.t(Key::NewsEmpty).to_string())
                        } else {
                            None
                        }
                    });
                    self.update_financial_juice_backoff();
                }
                Ok(NewsEvent::Error(error)) => {
                    self.news_loading = false;
                    self.news_receiver = None;
                    self.news_connection_status = "reconnecting...".to_string();
                    self.news_status =
                        Some(self.t(Key::NewsStatusError).replace("{error}", &error));
                    self.update_financial_juice_backoff();
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.news_loading = false;
                    self.news_receiver = None;
                    self.news_connection_status = "reconnecting...".to_string();
                    self.news_status = Some(self.t(Key::NewsStatusInterrupted).to_string());
                    self.update_financial_juice_backoff();
                }
            }
        }

        self.maybe_auto_refresh_news();
    }
    pub(crate) fn maybe_auto_refresh_news(&mut self) {
        if self.news_loading || !self.onboarding_complete {
            return;
        }

        let Some(last_refresh) = self.last_news_refresh else {
            self.refresh_news();
            return;
        };

        if last_refresh.elapsed() >= self.news_refresh_interval() {
            self.refresh_news();
        }
    }
    pub fn news_priority_label(&self, priority: NewsPriority) -> &'static str {
        match priority {
            NewsPriority::Critical => "critical",
            NewsPriority::High => "high",
            NewsPriority::Medium => "medium",
            NewsPriority::Low => "low",
        }
    }
    pub fn news_symbols_label(&self, symbols: &[String]) -> Option<String> {
        if symbols.is_empty() {
            None
        } else {
            Some(format!("[{}]", symbols.join(" ")))
        }
    }
    pub fn move_news_selection(&mut self, direction: SelectionDirection) {
        let visible = self.news_filtered_indices();
        if self.news_filter_tab == NewsFilterTab::Watchlist {
            let row_count = self.news_visible_rows().len();
            if row_count == 0 {
                self.news_selection = 0;
                self.news_scroll = 0;
                return;
            }

            self.news_selection = match direction {
                SelectionDirection::Previous => self.news_selection.saturating_sub(1),
                SelectionDirection::Next => {
                    (self.news_selection + 1).min(row_count.saturating_sub(1))
                }
            };
            self.sync_news_scroll(6);
            return;
        }

        if visible.is_empty() {
            self.news_selection = 0;
            self.news_scroll = 0;
            return;
        }

        self.news_selection = match direction {
            SelectionDirection::Previous => self.news_selection.saturating_sub(1),
            SelectionDirection::Next => {
                (self.news_selection + 1).min(visible.len().saturating_sub(1))
            }
        };
        self.sync_news_scroll(6);
    }
    pub fn open_selected_news(&mut self) {
        if self.news_filter_tab == NewsFilterTab::Watchlist {
            match self.news_visible_rows().get(self.news_selection) {
                Some(NewsListRow::Group { symbol, .. }) => {
                    if !self.collapsed_watchlist_news.remove(symbol) {
                        self.collapsed_watchlist_news.insert(symbol.clone());
                    }
                    self.selected_news = None;
                    self.sync_news_scroll(6);
                }
                Some(NewsListRow::Item(index)) => {
                    self.selected_news = self.news_items.get(*index).cloned();
                }
                None => {
                    self.selected_news = None;
                }
            }
            return;
        }

        self.selected_news = self
            .news_filtered_indices()
            .get(self.news_selection)
            .and_then(|index| self.news_items.get(*index))
            .cloned();
    }
    pub fn open_selected_news_in_browser(&mut self) {
        let item = if self.news_filter_tab == NewsFilterTab::Watchlist {
            self.news_visible_rows()
                .get(self.news_selection)
                .and_then(|row| match row {
                    NewsListRow::Item(index) => self.news_items.get(*index),
                    NewsListRow::Group { .. } => None,
                })
        } else {
            self.news_filtered_indices()
                .get(self.news_selection)
                .and_then(|index| self.news_items.get(*index))
        };
        let Some(item) = item else {
            return;
        };

        match open_url(&item.url) {
            Ok(()) => {
                self.news_status = Some(
                    self.t(Key::NewsStatusOpened)
                        .replace("{source}", item.source.as_str()),
                );
            }
            Err(error) => {
                self.news_status =
                    Some(self.t(Key::NewsStatusOpenError).replace("{error}", &error));
            }
        }
    }
    pub fn news_timestamp(&self, timestamp: Option<DateTime<chrono::Utc>>) -> String {
        let Some(value) = timestamp else {
            return String::new();
        };
        let delta = Local::now().signed_duration_since(value.with_timezone(&Local));
        if delta.num_minutes() < 1 {
            "now".to_string()
        } else if delta.num_minutes() < 60 {
            format!("{}m", delta.num_minutes())
        } else if delta.num_hours() < 24 {
            format!("{}h", delta.num_hours())
        } else if delta.num_days() == 1 {
            "yday".to_string()
        } else if delta.num_days() < 7 {
            format!("{}d", delta.num_days())
        } else if delta.num_days() < 31 {
            format!("{}w", delta.num_days() / 7)
        } else if delta.num_days() < 365 {
            format!("{}mo", delta.num_days() / 30)
        } else {
            "1y+".to_string()
        }
    }
    pub fn news_absolute_timestamp(&self, timestamp: Option<DateTime<chrono::Utc>>) -> String {
        timestamp
            .map(|value| {
                value
                    .with_timezone(&Local)
                    .format("%b %d %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| self.t(Key::NewsStatusUndated).to_string())
    }
    pub fn news_filtered_indices(&self) -> Vec<usize> {
        self.news_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| self.news_matches_filter(item).then_some(index))
            .collect()
    }
    pub fn news_visible_rows(&self) -> Vec<NewsListRow> {
        let filtered = self.news_filtered_indices();
        if self.news_filter_tab != NewsFilterTab::Watchlist {
            return filtered.into_iter().map(NewsListRow::Item).collect();
        }

        let mut rows = Vec::new();
        let mut seen = HashSet::new();

        for index in filtered {
            let Some(item) = self.news_items.get(index) else {
                continue;
            };
            let Some(symbol) = item.symbols.first() else {
                continue;
            };

            if seen.insert(symbol.clone()) {
                let count = self
                    .news_items
                    .iter()
                    .filter(|candidate| self.news_matches_filter(candidate))
                    .filter(|candidate| candidate.symbols.first() == Some(symbol))
                    .count();
                let expanded = !self.collapsed_watchlist_news.contains(symbol);
                rows.push(NewsListRow::Group {
                    symbol: symbol.clone(),
                    count,
                    expanded,
                });
            }

            if !self.collapsed_watchlist_news.contains(symbol) {
                rows.push(NewsListRow::Item(index));
            }
        }

        rows
    }
    pub(crate) fn news_matches_filter(&self, item: &NewsItem) -> bool {
        match self.news_filter_tab {
            NewsFilterTab::All => true,
            NewsFilterTab::Watchlist => item.relevant,
            NewsFilterTab::Macro => item.category == NewsCategory::Macro,
            NewsFilterTab::Reddit => item.category == NewsCategory::Reddit,
            NewsFilterTab::Crypto => item.category == NewsCategory::Crypto,
        }
    }
    pub fn cycle_news_filter(&mut self, direction: SelectionDirection) {
        self.news_filter_tab = match (self.news_filter_tab, direction) {
            (NewsFilterTab::All, SelectionDirection::Previous) => NewsFilterTab::Crypto,
            (NewsFilterTab::All, SelectionDirection::Next) => NewsFilterTab::Watchlist,
            (NewsFilterTab::Watchlist, SelectionDirection::Previous) => NewsFilterTab::All,
            (NewsFilterTab::Watchlist, SelectionDirection::Next) => NewsFilterTab::Macro,
            (NewsFilterTab::Macro, SelectionDirection::Previous) => NewsFilterTab::Watchlist,
            (NewsFilterTab::Macro, SelectionDirection::Next) => NewsFilterTab::Reddit,
            (NewsFilterTab::Reddit, SelectionDirection::Previous) => NewsFilterTab::Macro,
            (NewsFilterTab::Reddit, SelectionDirection::Next) => NewsFilterTab::Crypto,
            (NewsFilterTab::Crypto, SelectionDirection::Previous) => NewsFilterTab::Reddit,
            (NewsFilterTab::Crypto, SelectionDirection::Next) => NewsFilterTab::All,
        };
        self.news_selection = 0;
        self.news_scroll = 0;
    }
    pub fn sync_news_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.news_scroll = self.news_selection;
            return;
        }

        if self.news_selection < self.news_scroll {
            self.news_scroll = self.news_selection;
        } else if self.news_selection >= self.news_scroll + visible_rows {
            self.news_scroll = self.news_selection + 1 - visible_rows;
        }
    }
    pub(crate) fn sync_collapsed_watchlist_news(&mut self) {
        let symbols = self
            .news_items
            .iter()
            .filter(|item| item.relevant)
            .filter_map(|item| item.symbols.first().cloned())
            .collect::<HashSet<_>>();
        self.collapsed_watchlist_news
            .retain(|symbol| symbols.contains(symbol));
        self.known_watchlist_news_symbols
            .retain(|symbol| symbols.contains(symbol));
        for symbol in symbols {
            if !self.known_watchlist_news_symbols.contains(&symbol) {
                self.collapsed_watchlist_news.insert(symbol.clone());
            }
            self.known_watchlist_news_symbols.insert(symbol);
        }
    }
    pub fn news_empty_message(&self) -> &str {
        if self.news_items.is_empty() {
            return self
                .news_status
                .as_deref()
                .unwrap_or(self.t(Key::NewsEmpty));
        }
        if self.news_filter_tab == NewsFilterTab::Watchlist {
            if self.stock_watchlist().is_empty() && self.crypto_watchlist().is_empty() {
                return self.t(Key::NewsEmptyWatchlistConfig);
            }
            return self.t(Key::NewsEmptyWatchlistMatches);
        }
        self.news_status
            .as_deref()
            .unwrap_or(self.t(Key::NewsEmpty))
    }
    pub(crate) fn build_watchlist_matchers(&self, kind: WatchlistKind) -> Vec<WatchlistMatcher> {
        let symbols = match kind {
            WatchlistKind::Stock => self.stock_watchlist(),
            WatchlistKind::Crypto => self.crypto_watchlist(),
        };

        let mut instrument_names = Vec::new();
        if let Ok(connection) = db::open(&self.ticker_db_path) {
            for symbol in symbols {
                let name = search::details(&connection, symbol)
                    .ok()
                    .flatten()
                    .map(|details| details.name);
                instrument_names.push((symbol.clone(), name));
            }
        }

        symbols
            .iter()
            .map(|symbol| {
                let mut terms = vec![symbol.clone()];
                if let Some(alias) = self.watchlist_display_name(symbol) {
                    if !alias.trim().is_empty() {
                        terms.push(alias.trim().to_string());
                    }
                }
                if let Some((_, Some(name))) = instrument_names
                    .iter()
                    .find(|(candidate, _)| candidate == symbol)
                {
                    terms.extend(name_match_terms(name));
                }
                terms.sort();
                terms.dedup();
                WatchlistMatcher {
                    symbol: symbol.clone(),
                    terms,
                }
            })
            .collect()
    }
    pub(crate) fn financial_juice_in_cooldown(&self) -> bool {
        self.financial_juice_cooldown_until
            .is_some_and(|until| Instant::now() < until)
    }
    pub(crate) fn update_financial_juice_backoff(&mut self) {
        let hit_rate_limit = self
            .news_status
            .as_deref()
            .is_some_and(|status| status.contains("FinancialJuice") && status.contains("429"));

        if hit_rate_limit {
            self.financial_juice_cooldown_until =
                Some(Instant::now() + Duration::from_secs(30 * 60));
            if self.news_items.is_empty() {
                self.news_status =
                    Some("FinancialJuice paused for 30m after 429; using other feeds.".to_string());
            }
        } else if self
            .financial_juice_cooldown_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.financial_juice_cooldown_until = None;
        }
    }
}

fn name_match_terms(name: &str) -> Vec<String> {
    let cleaned_text = name.replace(',', " ").replace('.', " ").replace('&', " ");
    let cleaned = cleaned_text
        .split_whitespace()
        .filter(|token| {
            !matches!(
                token.to_ascii_lowercase().as_str(),
                "inc"
                    | "corp"
                    | "corporation"
                    | "company"
                    | "co"
                    | "holdings"
                    | "holding"
                    | "group"
                    | "plc"
                    | "ltd"
                    | "limited"
                    | "class"
                    | "common"
                    | "stock"
                    | "ordinary"
                    | "shares"
                    | "the"
            )
        })
        .collect::<Vec<_>>();

    let mut terms = Vec::new();
    if !cleaned.is_empty() {
        terms.push(cleaned.join(" "));
        if cleaned[0].len() >= 4 {
            terms.push(cleaned[0].to_string());
        }
        if cleaned.len() == 1 {
            terms.push(cleaned[0].to_string());
        }
    }
    terms.retain(|term| term.len() >= 3);
    terms
}

fn open_url(url: &str) -> Result<(), String> {
    let (command, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[url])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start", "", url])
    } else {
        ("xdg-open", &[url])
    };

    std::process::Command::new(command)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
