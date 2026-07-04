use crate::app::*;
use crate::features::watchlist::state::normalize_symbol;
use crate::{
    i18n::Key,
    metrics::{MetricId, visible_key_stats},
    search::{self},
};
use std::{sync::mpsc, thread};

pub(crate) const SEARCH_PAGE_SIZE: usize = 30;
pub(crate) const SEARCH_VISIBLE_ROWS: usize = 24;

impl App {
    pub fn open_search(&mut self) {
        if self.page != Page::Search {
            self.return_page = Some(self.page);
        }
        self.begin_text_input(InputTarget::Search);
        self.page = Page::Search;
        self.show_help = false;
        self.pending_split = false;
        self.selected_news = None;
        self.refresh_search();
    }
    pub fn move_search_selection(&mut self, direction: SelectionDirection) {
        if self.search_results.is_empty() {
            self.search_selection = 0;
            self.search_scroll = 0;
            return;
        }

        match direction {
            SelectionDirection::Previous => {
                self.search_selection = self.search_selection.saturating_sub(1);
            }
            SelectionDirection::Next => {
                if self.search_selection + 1 < self.search_results.len() {
                    self.search_selection += 1;
                } else {
                    let previous_len = self.search_results.len();
                    self.search_limit += SEARCH_PAGE_SIZE;
                    self.refresh_search();
                    if self.search_results.len() > previous_len {
                        self.search_selection += 1;
                    }
                }
            }
        }
        self.sync_search_scroll(SEARCH_VISIBLE_ROWS);
    }
    pub fn open_selected_details(&mut self) {
        let Some(result) = self.search_results.get(self.search_selection) else {
            return;
        };
        let symbol = result.symbol.clone();

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::details(&connection, &symbol))
        {
            Ok(details) => {
                self.selected_details = details;
                self.selected_live_details = None;
                self.live_details_receiver = None;
                self.live_details_loading = false;
                self.reset_detail_view_state();
                self.backend_insight = None;
                self.backend_insight_loading = false;
                self.backend_insight_status = None;
                self.backend_insight_receiver = None;
                self.page = Page::Details;
                self.search_message = None;
                self.spawn_live_details_fetch(symbol.clone());
                self.spawn_backend_insight_fetch(symbol);
            }
            Err(error) => {
                self.search_message = Some(
                    self.t(Key::SearchErrorDetailsUnavailable)
                        .replace("{error}", &error.to_string()),
                );
            }
        }
    }
    pub fn poll_live_details(&mut self) {
        let Some(receiver) = &self.live_details_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(details) => {
                self.selected_live_details = details;
                self.live_details_loading = false;
                self.live_details_receiver = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.live_details_loading = false;
                self.live_details_receiver = None;
            }
        }
    }
    pub fn cycle_detail_timeframe(&mut self, direction: SelectionDirection) {
        let frames = DetailTimeframe::ALL;
        let index = frames
            .iter()
            .position(|timeframe| *timeframe == self.detail_timeframe)
            .unwrap_or(0);
        self.detail_timeframe = match direction {
            SelectionDirection::Previous => frames[(index + frames.len() - 1) % frames.len()],
            SelectionDirection::Next => frames[(index + 1) % frames.len()],
        };
    }
    pub fn select_detail_timeframe(&mut self, index: usize) {
        if let Some(timeframe) = DetailTimeframe::ALL.get(index).copied() {
            self.detail_timeframe = timeframe;
        }
    }
    pub fn move_detail_sidebar_scroll(&mut self, direction: SelectionDirection) {
        self.detail_sidebar_scroll = match direction {
            SelectionDirection::Previous => self.detail_sidebar_scroll.saturating_sub(1),
            SelectionDirection::Next => self.detail_sidebar_scroll.saturating_add(1),
        };
    }
    pub fn cycle_detail_metric_focus(&mut self, direction: SelectionDirection) {
        let count = visible_key_stats(self.preferences.experience).len();
        if count == 0 {
            self.detail_metric_selection = 0;
            return;
        }
        self.detail_metric_selection = match direction {
            SelectionDirection::Previous => (self.detail_metric_selection + count - 1) % count,
            SelectionDirection::Next => (self.detail_metric_selection + 1) % count,
        };
    }
    pub fn focused_detail_metric(&self) -> Option<MetricId> {
        let metrics = visible_key_stats(self.preferences.experience);
        metrics
            .get(
                self.detail_metric_selection
                    .min(metrics.len().saturating_sub(1)),
            )
            .copied()
    }
    pub fn toggle_detail_description(&mut self) {
        self.detail_description_expanded = !self.detail_description_expanded;
    }
    pub fn toggle_detail_context(&mut self) {
        self.detail_context_expanded = !self.detail_context_expanded;
    }
    pub(crate) fn reset_detail_view_state(&mut self) {
        self.detail_timeframe = DetailTimeframe::ThreeMonths;
        self.detail_sidebar_scroll = 0;
        self.detail_metric_selection = 0;
        self.detail_description_expanded = false;
        self.detail_context_expanded = false;
    }
    pub fn poll_backend_insight(&mut self) {
        let Some(receiver) = &self.backend_insight_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(BackendInsightEvent::Loaded { symbol, insight }) => {
                let is_current = self
                    .selected_details
                    .as_ref()
                    .map(|details| details.symbol == symbol)
                    .unwrap_or(false);
                if is_current {
                    self.backend_insight = insight;
                    self.backend_insight_status = None;
                }
                self.backend_insight_loading = false;
                self.backend_insight_receiver = None;
            }
            Ok(BackendInsightEvent::Error { symbol, message }) => {
                let is_current = self
                    .selected_details
                    .as_ref()
                    .map(|details| details.symbol == symbol)
                    .unwrap_or(false);
                if is_current {
                    self.backend_insight = None;
                    self.backend_insight_status = Some(message);
                }
                self.backend_insight_loading = false;
                self.backend_insight_receiver = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.backend_insight_loading = false;
                self.backend_insight_receiver = None;
            }
        }
    }
    pub(crate) fn spawn_live_details_fetch(&mut self, symbol: String) {
        let (sender, receiver) = mpsc::channel();
        self.live_details_receiver = Some(receiver);
        self.live_details_loading = true;
        thread::spawn(move || {
            let details = search::live_details(&symbol);
            let _ = sender.send(details);
        });
    }
    pub(crate) fn spawn_backend_insight_fetch(&mut self, symbol: String) {
        let backend_config = self.config.backend.clone();
        let (sender, receiver) = mpsc::channel();
        self.backend_insight = None;
        self.backend_insight_status = None;
        self.backend_insight_receiver = Some(receiver);
        self.backend_insight_loading = true;
        thread::spawn(move || {
            let event = match crate::backend::BackendClient::new(&backend_config)
                .and_then(|client| client.fetch_insight(&symbol))
            {
                Ok(insight) => BackendInsightEvent::Loaded { symbol, insight },
                Err(message) => BackendInsightEvent::Error { symbol, message },
            };
            let _ = sender.send(event);
        });
    }
    pub fn toggle_search_asset_kind(&mut self) {
        self.search_asset_kind = match self.search_asset_kind {
            SearchAssetKind::Stocks => SearchAssetKind::Etfs,
            SearchAssetKind::Etfs => SearchAssetKind::Stocks,
        };
        self.reset_search_window();
        self.refresh_search();
    }
    pub fn search_asset_type(&self) -> &'static str {
        match self.search_asset_kind {
            SearchAssetKind::Stocks => "stock",
            SearchAssetKind::Etfs => "etf",
        }
    }
    pub(crate) fn refresh_search(&mut self) {
        match crate::db::open(&self.ticker_db_path).and_then(|connection| {
            search::search(
                &connection,
                &self.search_query,
                self.search_asset_type(),
                self.search_limit,
            )
        }) {
            Ok(results) => {
                self.search_results = results;
                self.search_selection = self
                    .search_selection
                    .min(self.search_results.len().saturating_sub(1));
                self.sync_search_scroll(SEARCH_VISIBLE_ROWS);
                self.search_message = None;
            }
            Err(error) => {
                self.search_results.clear();
                self.search_selection = 0;
                self.search_scroll = 0;
                self.search_message = Some(
                    self.t(Key::SearchErrorDatabaseUnavailable)
                        .replace("{error}", &error.to_string()),
                );
            }
        }
    }
    pub fn agent_open_symbol(&mut self, symbol: &str) -> Result<String, String> {
        let Some(symbol) = normalize_symbol(symbol) else {
            return Err("symbol must not be empty".to_string());
        };

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::details(&connection, &symbol))
        {
            Ok(Some(details)) => {
                self.selected_details = Some(details);
                self.selected_live_details = None;
                self.live_details_loading = false;
                self.reset_detail_view_state();
                self.page = Page::Details;
                self.search_message = None;
                self.selected_news = None;
                self.show_help = false;
                self.spawn_live_details_fetch(symbol.clone());
                Ok(format!("opened details for {symbol}"))
            }
            Ok(None) => Err(format!("symbol {symbol} not found in the local ticker db")),
            Err(error) => Err(format!("lookup failed: {error}")),
        }
    }
    pub fn sync_search_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.search_scroll = self.search_selection;
            return;
        }

        if self.search_selection < self.search_scroll {
            self.search_scroll = self.search_selection;
        } else if self.search_selection >= self.search_scroll + visible_rows {
            self.search_scroll = self.search_selection + 1 - visible_rows;
        }
    }
    pub(crate) fn reset_search_window(&mut self) {
        self.search_selection = 0;
        self.search_scroll = 0;
        self.search_limit = SEARCH_PAGE_SIZE;
    }
}
