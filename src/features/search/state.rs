use crate::app::*;
use crate::features::watchlist::state::normalize_symbol;
use crate::{
    features::search::engine::{self as search},
    i18n::Key,
    metrics::{MetricId, visible_key_stats},
};
use std::{sync::mpsc, thread};

pub(crate) const SEARCH_PAGE_SIZE: usize = 30;
pub(crate) const SEARCH_VISIBLE_ROWS: usize = 24;

/// UI + runtime state owned by the search/details feature, including the
/// per-symbol backend insight shown on the details page.
#[derive(Debug)]
pub struct SearchFeature {
    pub query: String,
    pub results: Vec<search::SearchResult>,
    pub selection: usize,
    pub scroll: usize,
    pub limit: usize,
    pub asset_kind: SearchAssetKind,
    pub message: Option<String>,
    pub selected_details: Option<search::InstrumentDetails>,
    pub selected_live_details: Option<search::LiveInstrumentDetails>,
    pub live_details_loading: bool,
    pub(crate) live_details_receiver: Option<mpsc::Receiver<Option<search::LiveInstrumentDetails>>>,
    pub detail_timeframe: DetailTimeframe,
    pub detail_sidebar_scroll: usize,
    pub detail_metric_selection: usize,
    pub detail_description_expanded: bool,
    pub detail_context_expanded: bool,
    pub backend_insight: Option<crate::backend::BackendInsight>,
    pub backend_insight_loading: bool,
    pub backend_insight_status: Option<String>,
    pub(crate) backend_insight_receiver: Option<mpsc::Receiver<BackendInsightEvent>>,
}

impl Default for SearchFeature {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selection: 0,
            scroll: 0,
            limit: SEARCH_PAGE_SIZE,
            asset_kind: SearchAssetKind::Stocks,
            message: None,
            selected_details: None,
            selected_live_details: None,
            live_details_loading: false,
            live_details_receiver: None,
            detail_timeframe: DetailTimeframe::ThreeMonths,
            detail_sidebar_scroll: 0,
            detail_metric_selection: 0,
            detail_description_expanded: false,
            detail_context_expanded: false,
            backend_insight: None,
            backend_insight_loading: false,
            backend_insight_status: None,
            backend_insight_receiver: None,
        }
    }
}

impl App {
    pub fn open_search(&mut self) {
        if self.page != Page::Search {
            self.return_page = Some(self.page);
        }
        self.begin_text_input(InputTarget::Search);
        self.page = Page::Search;
        self.show_help = false;
        self.dashboard.pending_split = false;
        self.news.selected = None;
        self.refresh_search();
    }
    pub fn move_search_selection(&mut self, direction: SelectionDirection) {
        if self.search.results.is_empty() {
            self.search.selection = 0;
            self.search.scroll = 0;
            return;
        }

        match direction {
            SelectionDirection::Previous => {
                self.search.selection = self.search.selection.saturating_sub(1);
            }
            SelectionDirection::Next => {
                if self.search.selection + 1 < self.search.results.len() {
                    self.search.selection += 1;
                } else {
                    let previous_len = self.search.results.len();
                    self.search.limit += SEARCH_PAGE_SIZE;
                    self.refresh_search();
                    if self.search.results.len() > previous_len {
                        self.search.selection += 1;
                    }
                }
            }
        }
        self.sync_search_scroll(SEARCH_VISIBLE_ROWS);
    }
    pub fn open_selected_details(&mut self) {
        let Some(result) = self.search.results.get(self.search.selection) else {
            return;
        };
        let symbol = result.symbol.clone();

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::details(&connection, &symbol))
        {
            Ok(details) => {
                self.search.selected_details = details;
                self.search.selected_live_details = None;
                self.search.live_details_receiver = None;
                self.search.live_details_loading = false;
                self.reset_detail_view_state();
                self.search.backend_insight = None;
                self.search.backend_insight_loading = false;
                self.search.backend_insight_status = None;
                self.search.backend_insight_receiver = None;
                self.page = Page::Details;
                self.search.message = None;
                self.spawn_live_details_fetch(symbol.clone());
                self.spawn_backend_insight_fetch(symbol);
            }
            Err(error) => {
                self.search.message = Some(
                    self.t(Key::SearchErrorDetailsUnavailable)
                        .replace("{error}", &error.to_string()),
                );
            }
        }
    }
    pub fn poll_live_details(&mut self) {
        let Some(receiver) = &self.search.live_details_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(details) => {
                self.search.selected_live_details = details;
                self.search.live_details_loading = false;
                self.search.live_details_receiver = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.search.live_details_loading = false;
                self.search.live_details_receiver = None;
            }
        }
    }
    pub fn cycle_detail_timeframe(&mut self, direction: SelectionDirection) {
        let frames = DetailTimeframe::ALL;
        let index = frames
            .iter()
            .position(|timeframe| *timeframe == self.search.detail_timeframe)
            .unwrap_or(0);
        self.search.detail_timeframe = match direction {
            SelectionDirection::Previous => frames[(index + frames.len() - 1) % frames.len()],
            SelectionDirection::Next => frames[(index + 1) % frames.len()],
        };
    }
    pub fn select_detail_timeframe(&mut self, index: usize) {
        if let Some(timeframe) = DetailTimeframe::ALL.get(index).copied() {
            self.search.detail_timeframe = timeframe;
        }
    }
    pub fn move_detail_sidebar_scroll(&mut self, direction: SelectionDirection) {
        self.search.detail_sidebar_scroll = match direction {
            SelectionDirection::Previous => self.search.detail_sidebar_scroll.saturating_sub(1),
            SelectionDirection::Next => self.search.detail_sidebar_scroll.saturating_add(1),
        };
    }
    pub fn cycle_detail_metric_focus(&mut self, direction: SelectionDirection) {
        let count = visible_key_stats(self.preferences.experience).len();
        if count == 0 {
            self.search.detail_metric_selection = 0;
            return;
        }
        self.search.detail_metric_selection = match direction {
            SelectionDirection::Previous => {
                (self.search.detail_metric_selection + count - 1) % count
            }
            SelectionDirection::Next => (self.search.detail_metric_selection + 1) % count,
        };
    }
    pub fn focused_detail_metric(&self) -> Option<MetricId> {
        let metrics = visible_key_stats(self.preferences.experience);
        metrics
            .get(
                self.search
                    .detail_metric_selection
                    .min(metrics.len().saturating_sub(1)),
            )
            .copied()
    }
    pub fn toggle_detail_description(&mut self) {
        self.search.detail_description_expanded = !self.search.detail_description_expanded;
    }
    pub fn toggle_detail_context(&mut self) {
        self.search.detail_context_expanded = !self.search.detail_context_expanded;
    }
    pub(crate) fn reset_detail_view_state(&mut self) {
        self.search.detail_timeframe = DetailTimeframe::ThreeMonths;
        self.search.detail_sidebar_scroll = 0;
        self.search.detail_metric_selection = 0;
        self.search.detail_description_expanded = false;
        self.search.detail_context_expanded = false;
    }
    pub fn poll_backend_insight(&mut self) {
        let Some(receiver) = &self.search.backend_insight_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(BackendInsightEvent::Loaded { symbol, insight }) => {
                let is_current = self
                    .search
                    .selected_details
                    .as_ref()
                    .map(|details| details.symbol == symbol)
                    .unwrap_or(false);
                if is_current {
                    self.search.backend_insight = insight;
                    self.search.backend_insight_status = None;
                }
                self.search.backend_insight_loading = false;
                self.search.backend_insight_receiver = None;
            }
            Ok(BackendInsightEvent::Error { symbol, message }) => {
                let is_current = self
                    .search
                    .selected_details
                    .as_ref()
                    .map(|details| details.symbol == symbol)
                    .unwrap_or(false);
                if is_current {
                    self.search.backend_insight = None;
                    self.search.backend_insight_status = Some(message);
                }
                self.search.backend_insight_loading = false;
                self.search.backend_insight_receiver = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.search.backend_insight_loading = false;
                self.search.backend_insight_receiver = None;
            }
        }
    }
    pub(crate) fn spawn_live_details_fetch(&mut self, symbol: String) {
        let (sender, receiver) = mpsc::channel();
        self.search.live_details_receiver = Some(receiver);
        self.search.live_details_loading = true;
        thread::spawn(move || {
            let details = search::live_details(&symbol);
            let _ = sender.send(details);
        });
    }
    pub(crate) fn spawn_backend_insight_fetch(&mut self, symbol: String) {
        let backend_config = self.config.backend.clone();
        let (sender, receiver) = mpsc::channel();
        self.search.backend_insight = None;
        self.search.backend_insight_status = None;
        self.search.backend_insight_receiver = Some(receiver);
        self.search.backend_insight_loading = true;
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
        self.search.asset_kind = match self.search.asset_kind {
            SearchAssetKind::Stocks => SearchAssetKind::Etfs,
            SearchAssetKind::Etfs => SearchAssetKind::Stocks,
        };
        self.reset_search_window();
        self.refresh_search();
    }
    pub fn search_asset_type(&self) -> &'static str {
        match self.search.asset_kind {
            SearchAssetKind::Stocks => "stock",
            SearchAssetKind::Etfs => "etf",
        }
    }
    pub(crate) fn refresh_search(&mut self) {
        match crate::db::open(&self.ticker_db_path).and_then(|connection| {
            search::search(
                &connection,
                &self.search.query,
                self.search_asset_type(),
                self.search.limit,
            )
        }) {
            Ok(results) => {
                self.search.results = results;
                self.search.selection = self
                    .search
                    .selection
                    .min(self.search.results.len().saturating_sub(1));
                self.sync_search_scroll(SEARCH_VISIBLE_ROWS);
                self.search.message = None;
            }
            Err(error) => {
                self.search.results.clear();
                self.search.selection = 0;
                self.search.scroll = 0;
                self.search.message = Some(
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
                self.search.selected_details = Some(details);
                self.search.selected_live_details = None;
                self.search.live_details_loading = false;
                self.reset_detail_view_state();
                self.page = Page::Details;
                self.search.message = None;
                self.news.selected = None;
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
            self.search.scroll = self.search.selection;
            return;
        }

        if self.search.selection < self.search.scroll {
            self.search.scroll = self.search.selection;
        } else if self.search.selection >= self.search.scroll + visible_rows {
            self.search.scroll = self.search.selection + 1 - visible_rows;
        }
    }
    pub(crate) fn reset_search_window(&mut self) {
        self.search.selection = 0;
        self.search.scroll = 0;
        self.search.limit = SEARCH_PAGE_SIZE;
    }
}
