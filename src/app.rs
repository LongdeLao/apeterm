use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use crate::{
    config::AppConfig,
    i18n::{I18n, Key, Locale},
    market::{MarketEvent, MarketSession},
    quotes::{Quote, update_market_quotes},
    search::{self, InstrumentDetails, LiveInstrumentDetails, SearchResult},
};

const SEARCH_PAGE_SIZE: usize = 30;
const SEARCH_VISIBLE_ROWS: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Onboarding,
    Dashboard,
    Search,
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    Language,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Dark,
    Light,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    News,
    Watchlist,
    Calendar,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    News,
    Watchlist,
    Calendar,
    Notes,
    Picker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelContents {
    pub news: WindowKind,
    pub watchlist: WindowKind,
    pub calendar: WindowKind,
    pub notes: WindowKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAssetKind {
    Stocks,
    Etfs,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub page: Page,
    pub onboarding_step: OnboardingStep,
    pub onboarding_complete: bool,
    pub logged_in: bool,
    pub locale: Locale,
    pub i18n: I18n,
    pub theme_name: ThemeName,
    pub dashboard_layout: DashboardLayout,
    pub focused_panel: PanelId,
    pub closed_panels: Vec<PanelId>,
    pub show_help: bool,
    pub pending_split: bool,
    pub panel_contents: PanelContents,
    pub window_picker_index: usize,
    pub crypto_quotes: Vec<Quote>,
    pub stock_quotes: Vec<Quote>,
    pub stock_market_session: MarketSession,
    pub ticker_db_path: PathBuf,
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_selection: usize,
    pub search_scroll: usize,
    pub search_limit: usize,
    pub search_asset_kind: SearchAssetKind,
    pub selected_details: Option<InstrumentDetails>,
    pub selected_live_details: Option<LiveInstrumentDetails>,
    pub live_details_loading: bool,
    live_details_receiver: Option<Receiver<Option<LiveInstrumentDetails>>>,
    pub search_message: Option<String>,
    config: AppConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardLayout {
    pub top_left_width_percent: u16,
    pub bottom_left_width_percent: u16,
    pub top_height_percent: u16,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let locale = config.locale.clone();
        Self {
            should_quit: false,
            page: Page::Onboarding,
            onboarding_step: OnboardingStep::Welcome,
            onboarding_complete: false,
            logged_in: false,
            locale: locale.clone(),
            i18n: I18n::new(locale),
            theme_name: ThemeName::Dark,
            dashboard_layout: DashboardLayout::default(),
            focused_panel: PanelId::News,
            closed_panels: Vec::new(),
            show_help: false,
            pending_split: false,
            panel_contents: PanelContents::default(),
            window_picker_index: 0,
            crypto_quotes: Vec::new(),
            stock_quotes: Vec::new(),
            stock_market_session: MarketSession::AfterHours,
            ticker_db_path: config.ticker_db_path.clone(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selection: 0,
            search_scroll: 0,
            search_limit: SEARCH_PAGE_SIZE,
            search_asset_kind: SearchAssetKind::Stocks,
            selected_details: None,
            selected_live_details: None,
            live_details_loading: false,
            live_details_receiver: None,
            search_message: None,
            config,
        }
    }

    pub fn t(&self, key: Key) -> &str {
        self.i18n.t(key)
    }

    pub fn advance_onboarding(&mut self) {
        self.onboarding_step = match self.onboarding_step {
            OnboardingStep::Welcome => OnboardingStep::Language,
            OnboardingStep::Language => OnboardingStep::Theme,
            OnboardingStep::Theme => {
                self.page = Page::Dashboard;
                self.onboarding_complete = true;
                OnboardingStep::Theme
            }
        };
    }

    pub fn move_selection(&mut self, direction: SelectionDirection) {
        match self.onboarding_step {
            OnboardingStep::Welcome => {}
            OnboardingStep::Language => {
                let locales = self.i18n.available_locales();
                if locales.is_empty() {
                    return;
                }
                let current = locales
                    .iter()
                    .position(|locale| locale == &self.locale)
                    .unwrap_or(0);
                let next = match direction {
                    SelectionDirection::Previous => {
                        if current == 0 {
                            locales.len() - 1
                        } else {
                            current - 1
                        }
                    }
                    SelectionDirection::Next => (current + 1) % locales.len(),
                };
                self.set_locale(locales[next].clone());
            }
            OnboardingStep::Theme => self.theme_name = self.theme_name.move_to(direction),
        }
    }

    pub fn handle_market_event(&mut self, event: MarketEvent) {
        update_market_quotes(
            &mut self.crypto_quotes,
            &mut self.stock_quotes,
            &mut self.stock_market_session,
            event,
        );
    }

    pub fn focus_panel(&mut self, panel_id: PanelId) {
        if self.is_panel_open(panel_id) {
            self.focused_panel = panel_id;
        }
    }

    pub fn focus_next_panel(&mut self) {
        self.focus_by_offset(1);
    }

    pub fn focus_previous_panel(&mut self) {
        self.focus_by_offset(PanelId::ALL.len() - 1);
    }

    pub fn focus_panel_in_direction(&mut self, direction: MoveDirection) {
        let next = match (self.focused_panel, direction) {
            (PanelId::News, MoveDirection::Right) => PanelId::Watchlist,
            (PanelId::News, MoveDirection::Down) => PanelId::Calendar,
            (PanelId::Watchlist, MoveDirection::Left) => PanelId::News,
            (PanelId::Watchlist, MoveDirection::Down) => PanelId::Notes,
            (PanelId::Calendar, MoveDirection::Up) => PanelId::News,
            (PanelId::Calendar, MoveDirection::Right) => PanelId::Notes,
            (PanelId::Notes, MoveDirection::Up) => PanelId::Watchlist,
            (PanelId::Notes, MoveDirection::Left) => PanelId::Calendar,
            _ => self.focused_panel,
        };

        self.focus_panel(next);
    }

    pub fn close_focused_panel(&mut self) {
        if self.open_panel_count() <= 1 || !self.is_panel_open(self.focused_panel) {
            return;
        }

        self.closed_panels.push(self.focused_panel);
        self.pending_split = false;
        self.focus_next_panel();
    }

    pub fn reset_dashboard(&mut self) {
        self.dashboard_layout = DashboardLayout::default();
        self.closed_panels.clear();
        self.focused_panel = PanelId::News;
        self.show_help = false;
        self.pending_split = false;
        self.panel_contents = PanelContents::default();
        self.window_picker_index = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.pending_split = false;
    }

    pub fn toggle_locale(&mut self) {
        self.set_locale(self.i18n.next_locale(&self.locale));
    }

    pub fn close_help(&mut self) {
        self.show_help = false;
        self.pending_split = false;
        if self.page == Page::Details {
            self.page = Page::Search;
            self.selected_details = None;
            self.selected_live_details = None;
            self.live_details_loading = false;
            self.live_details_receiver = None;
        } else if self.page == Page::Search {
            self.page = Page::Dashboard;
        }
    }

    pub fn is_panel_open(&self, panel_id: PanelId) -> bool {
        !self.closed_panels.contains(&panel_id)
    }

    pub fn is_panel_focused(&self, panel_id: PanelId) -> bool {
        self.focused_panel == panel_id && self.is_panel_open(panel_id)
    }

    pub fn resize_dashboard(&mut self, direction: MoveDirection) {
        if self.page != Page::Dashboard {
            return;
        }

        match (self.focused_panel, direction) {
            (PanelId::News | PanelId::Watchlist, MoveDirection::Left) => {
                self.dashboard_layout.resize_top_left_width(-5)
            }
            (PanelId::News | PanelId::Watchlist, MoveDirection::Right) => {
                self.dashboard_layout.resize_top_left_width(5)
            }
            (PanelId::Calendar | PanelId::Notes, MoveDirection::Left) => {
                self.dashboard_layout.resize_bottom_left_width(-5)
            }
            (PanelId::Calendar | PanelId::Notes, MoveDirection::Right) => {
                self.dashboard_layout.resize_bottom_left_width(5)
            }
            (PanelId::News | PanelId::Calendar, MoveDirection::Up) => {
                self.dashboard_layout.resize_top_height(-5)
            }
            (PanelId::News | PanelId::Calendar, MoveDirection::Down) => {
                self.dashboard_layout.resize_top_height(5)
            }
            (PanelId::Watchlist | PanelId::Notes, MoveDirection::Up) => {
                self.dashboard_layout.resize_top_height(-5)
            }
            (PanelId::Watchlist | PanelId::Notes, MoveDirection::Down) => {
                self.dashboard_layout.resize_top_height(5)
            }
        }
    }

    pub fn begin_split_command(&mut self) {
        if self.page == Page::Dashboard {
            self.pending_split = true;
        }
    }

    pub fn split_focused_panel(&mut self, direction: SplitDirection) {
        if self.page != Page::Dashboard {
            return;
        }

        self.pending_split = false;

        let panel_id = match (self.focused_panel, direction) {
            (PanelId::News, SplitDirection::Horizontal) => PanelId::Watchlist,
            (PanelId::Watchlist, SplitDirection::Horizontal) => PanelId::News,
            (PanelId::Calendar, SplitDirection::Horizontal) => PanelId::Notes,
            (PanelId::Notes, SplitDirection::Horizontal) => PanelId::Calendar,
            (PanelId::News, SplitDirection::Vertical) => PanelId::Calendar,
            (PanelId::Calendar, SplitDirection::Vertical) => PanelId::News,
            (PanelId::Watchlist, SplitDirection::Vertical) => PanelId::Notes,
            (PanelId::Notes, SplitDirection::Vertical) => PanelId::Watchlist,
        };

        self.open_panel(panel_id);
        self.set_panel_content(panel_id, WindowKind::Picker);
        self.window_picker_index = 0;
    }

    pub fn add_panel(&mut self) {
        self.pending_split = false;

        let Some(panel_id) = self.closed_panels.pop() else {
            return;
        };

        self.focused_panel = panel_id;
        self.set_panel_content(panel_id, WindowKind::Picker);
        self.window_picker_index = 0;
    }

    pub fn change_focused_panel_content(&mut self) {
        if self.page != Page::Dashboard || !self.is_panel_open(self.focused_panel) {
            return;
        }

        let current = self.panel_content(self.focused_panel);
        self.window_picker_index = WindowKind::CHOICES
            .iter()
            .position(|window_kind| *window_kind == current)
            .unwrap_or(0);
        self.set_panel_content(self.focused_panel, WindowKind::Picker);
    }

    pub fn cancel_pending_command(&mut self) {
        self.pending_split = false;
    }

    pub fn panel_content(&self, panel_id: PanelId) -> WindowKind {
        self.panel_contents.get(panel_id)
    }

    pub fn is_choosing_window(&self) -> bool {
        self.panel_content(self.focused_panel) == WindowKind::Picker
    }

    pub fn move_window_picker(&mut self, direction: SelectionDirection) {
        let choices = WindowKind::CHOICES.len();
        self.window_picker_index = match direction {
            SelectionDirection::Previous => {
                if self.window_picker_index == 0 {
                    choices - 1
                } else {
                    self.window_picker_index - 1
                }
            }
            SelectionDirection::Next => (self.window_picker_index + 1) % choices,
        };
    }

    pub fn confirm_window_picker(&mut self) {
        if !self.is_choosing_window() {
            return;
        }

        let window_kind = WindowKind::CHOICES[self.window_picker_index];
        self.set_panel_content(self.focused_panel, window_kind);
    }

    pub fn open_search(&mut self) {
        self.page = Page::Search;
        self.show_help = false;
        self.pending_split = false;
        self.refresh_search();
    }

    pub fn push_search_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        self.search_query.push(character);
        self.reset_search_window();
        self.refresh_search();
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.reset_search_window();
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

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::details(&connection, &result.symbol))
        {
            Ok(details) => {
                self.selected_details = details;
                self.selected_live_details = None;
                self.live_details_receiver = None;
                self.live_details_loading = false;
                self.page = Page::Details;
                self.search_message = None;
                self.spawn_live_details_fetch(result.symbol.clone());
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

    fn spawn_live_details_fetch(&mut self, symbol: String) {
        let (sender, receiver) = mpsc::channel();
        self.live_details_receiver = Some(receiver);
        self.live_details_loading = true;
        thread::spawn(move || {
            let details = search::live_details(&symbol);
            let _ = sender.send(details);
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

    fn refresh_search(&mut self) {
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

    fn set_locale(&mut self, locale: Locale) {
        self.locale = locale.clone();
        self.i18n.set_active(locale.clone());
        self.config.locale = locale;
        let _ = self.config.save();
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

    fn reset_search_window(&mut self) {
        self.search_selection = 0;
        self.search_scroll = 0;
        self.search_limit = SEARCH_PAGE_SIZE;
    }

    fn focus_by_offset(&mut self, offset: usize) {
        let current_index = PanelId::ALL
            .iter()
            .position(|panel_id| *panel_id == self.focused_panel)
            .unwrap_or(0);

        for step in 1..=PanelId::ALL.len() {
            let index = (current_index + offset * step) % PanelId::ALL.len();
            let panel_id = PanelId::ALL[index];
            if self.is_panel_open(panel_id) {
                self.focused_panel = panel_id;
                break;
            }
        }
    }

    fn open_panel_count(&self) -> usize {
        PanelId::ALL
            .iter()
            .filter(|panel_id| self.is_panel_open(**panel_id))
            .count()
    }

    fn open_panel(&mut self, panel_id: PanelId) {
        self.closed_panels
            .retain(|closed_panel_id| *closed_panel_id != panel_id);
        self.focused_panel = panel_id;
    }

    fn set_panel_content(&mut self, panel_id: PanelId, window_kind: WindowKind) {
        self.panel_contents.set(panel_id, window_kind);
    }
}

impl PanelId {
    pub const ALL: [Self; 4] = [Self::News, Self::Watchlist, Self::Calendar, Self::Notes];
}

impl WindowKind {
    pub const CHOICES: [Self; 4] = [Self::News, Self::Watchlist, Self::Calendar, Self::Notes];

    pub fn label_key(self) -> Key {
        match self {
            Self::News => Key::PanelTitleNews,
            Self::Watchlist => Key::PanelTitleWatchlist,
            Self::Calendar => Key::PanelTitleCalendar,
            Self::Notes => Key::PanelTitleNotes,
            Self::Picker => Key::PanelTitlePicker,
        }
    }
}

impl Default for PanelContents {
    fn default() -> Self {
        Self {
            news: WindowKind::News,
            watchlist: WindowKind::Watchlist,
            calendar: WindowKind::Calendar,
            notes: WindowKind::Notes,
        }
    }
}

impl PanelContents {
    fn get(self, panel_id: PanelId) -> WindowKind {
        match panel_id {
            PanelId::News => self.news,
            PanelId::Watchlist => self.watchlist,
            PanelId::Calendar => self.calendar,
            PanelId::Notes => self.notes,
        }
    }

    fn set(&mut self, panel_id: PanelId, window_kind: WindowKind) {
        match panel_id {
            PanelId::News => self.news = window_kind,
            PanelId::Watchlist => self.watchlist = window_kind,
            PanelId::Calendar => self.calendar = window_kind,
            PanelId::Notes => self.notes = window_kind,
        }
    }
}

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            top_left_width_percent: 50,
            bottom_left_width_percent: 50,
            top_height_percent: 50,
        }
    }
}

impl DashboardLayout {
    const MIN_PERCENT: u16 = 15;
    const MAX_PERCENT: u16 = 85;

    pub fn top_divider_column(self, width: u16) -> u16 {
        percent_to_position(width, self.top_left_width_percent)
    }

    pub fn bottom_divider_column(self, width: u16) -> u16 {
        percent_to_position(width, self.bottom_left_width_percent)
    }

    pub fn divider_row(self, height: u16) -> u16 {
        percent_to_position(height, self.top_height_percent)
    }

    fn resize_top_left_width(&mut self, amount: i16) {
        self.top_left_width_percent = adjust_percent(self.top_left_width_percent, amount);
    }

    fn resize_bottom_left_width(&mut self, amount: i16) {
        self.bottom_left_width_percent = adjust_percent(self.bottom_left_width_percent, amount);
    }

    fn resize_top_height(&mut self, amount: i16) {
        self.top_height_percent = adjust_percent(self.top_height_percent, amount);
    }
}

fn percent_to_position(size: u16, percent: u16) -> u16 {
    size.saturating_sub(1).saturating_mul(percent) / 100
}

fn adjust_percent(percent: u16, amount: i16) -> u16 {
    percent
        .saturating_add_signed(amount)
        .clamp(DashboardLayout::MIN_PERCENT, DashboardLayout::MAX_PERCENT)
}

impl ThemeName {
    fn move_to(self, direction: SelectionDirection) -> Self {
        match direction {
            SelectionDirection::Previous => self.previous(),
            SelectionDirection::Next => self.next(),
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Dark => Self::Transparent,
            Self::Light => Self::Dark,
            Self::Transparent => Self::Light,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Transparent,
            Self::Transparent => Self::Dark,
        }
    }

    pub fn label_key(self) -> Key {
        match self {
            Self::Dark => Key::AppThemeDark,
            Self::Light => Key::AppThemeLight,
            Self::Transparent => Key::AppThemeTransparent,
        }
    }
}
