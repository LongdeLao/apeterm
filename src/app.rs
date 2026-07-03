use std::{
    collections::HashSet,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::{
    agent::AgentController,
    config::{AppConfig, NamedWatchlist},
    db,
    i18n::{I18n, Key, Locale},
    market::{MarketEvent, MarketSession},
    news::{
        self, FetchResult, NewsCategory, NewsItem, NewsPriority, NewsRuntimeConfig,
        WatchlistMatcher,
    },
    quotes::{Quote, update_market_quotes},
    sec::{self, EntityKind},
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
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    TextInput(InputTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTarget {
    Agent,
    Search,
    ResetConfirmation,
    Watchlist,
    Notes,
    NotesSearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    Language,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
    Transparent,
    Bloomberg,
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
    Sec,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsFilterTab {
    All,
    Watchlist,
    Macro,
    Reddit,
    Crypto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotesFilterTab {
    #[default]
    All,
    Tickers,
    Journal,
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecTab {
    Institutional,
    Ceos,
    Congress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    Language,
    Theme,
    Onboarding,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchlistKind {
    Stock,
    Crypto,
}

#[derive(Debug, Clone)]
pub enum WatchlistEditMode {
    Add {
        kind: WatchlistKind,
        input: String,
    },
    EditAlias {
        symbol: String,
        input: String,
    },
    ChangeTicker {
        kind: WatchlistKind,
        index: usize,
        input: String,
    },
    CreateWatchlist {
        input: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchlistEditRow {
    AddStock,
    Stock(usize),
    AddCrypto,
    Crypto(usize),
}

#[derive(Debug, Clone)]
pub struct WatchlistEditor {
    pub selection: usize,
    pub mode: Option<WatchlistEditMode>,
}

#[derive(Debug, Clone)]
pub struct TextInput {
    pub input: String,
}

#[derive(Debug, Clone)]
pub struct NotesDraft {
    pub note_id: i64,
    pub body: String,
}

#[derive(Debug)]
enum NewsEvent {
    Loaded { result: FetchResult, done: bool },
    Error(String),
}

#[derive(Debug)]
enum SecEvent {
    Done(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum NewsListRow {
    Group {
        symbol: String,
        count: usize,
        expanded: bool,
    },
    Item(usize),
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub page: Page,
    pub onboarding_step: OnboardingStep,
    pub onboarding_complete: bool,
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
    pub stock_market_session: Option<MarketSession>,
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
    pub settings_selection: usize,
    pub reset_confirmation: Option<TextInput>,
    pub watchlist_editor: Option<WatchlistEditor>,
    pub watchlist_suggestions: Vec<SearchResult>,
    pub watchlist_suggestion_selection: usize,
    pub market_refresh_requested: bool,
    pub news_items: Vec<NewsItem>,
    pub news_selection: usize,
    pub news_scroll: usize,
    pub news_loading: bool,
    pub news_status: Option<String>,
    pub selected_news: Option<NewsItem>,
    pub news_filter_tab: NewsFilterTab,
    pub news_source_label: String,
    pub news_connection_status: String,
    pub news_source_counts: Vec<(String, usize)>,
    pub collapsed_watchlist_news: HashSet<String>,
    known_watchlist_news_symbols: HashSet<String>,
    last_news_refresh: Option<Instant>,
    pub notes_tab: NotesFilterTab,
    pub notes_selection: usize,
    pub notes_scroll: usize,
    pub notes_search_query: String,
    pub notes_ticker_filter: Option<String>,
    pub notes_insert_mode: bool,
    pub notes_draft: Option<NotesDraft>,
    pub notes_suggestions: Vec<SearchResult>,
    pub notes_suggestion_selection: usize,
    pub pending_note_delete: Option<i64>,
    pub sec_tab: SecTab,
    pub sec_institutional_selection: usize,
    pub sec_ceo_selection: usize,
    pub sec_congress_selection: usize,
    pub sec_status: Option<String>,
    pub sec_loading: bool,
    sec_receiver: Option<Receiver<SecEvent>>,
    last_sec_refresh: Option<Instant>,
    financial_juice_cooldown_until: Option<Instant>,
    pub agent: AgentController,
    news_receiver: Option<Receiver<NewsEvent>>,
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
        let onboarding_complete = config.onboarding.completed;
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            page: if onboarding_complete {
                Page::Dashboard
            } else {
                Page::Onboarding
            },
            onboarding_step: OnboardingStep::Welcome,
            onboarding_complete,
            locale: locale.clone(),
            i18n: I18n::new(locale),
            theme_name: config.theme,
            dashboard_layout: DashboardLayout::default(),
            focused_panel: PanelId::News,
            closed_panels: Vec::new(),
            show_help: false,
            pending_split: false,
            panel_contents: PanelContents::default(),
            window_picker_index: 0,
            crypto_quotes: Vec::new(),
            stock_quotes: Vec::new(),
            stock_market_session: None,
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
            settings_selection: 0,
            reset_confirmation: None,
            watchlist_editor: None,
            watchlist_suggestions: Vec::new(),
            watchlist_suggestion_selection: 0,
            market_refresh_requested: false,
            news_items: Vec::new(),
            news_selection: 0,
            news_scroll: 0,
            news_loading: false,
            news_status: None,
            selected_news: None,
            news_filter_tab: NewsFilterTab::All,
            news_source_label: "news feed".to_string(),
            news_connection_status: "connecting...".to_string(),
            news_source_counts: Vec::new(),
            collapsed_watchlist_news: HashSet::new(),
            known_watchlist_news_symbols: HashSet::new(),
            last_news_refresh: None,
            notes_tab: NotesFilterTab::All,
            notes_selection: 0,
            notes_scroll: 0,
            notes_search_query: String::new(),
            notes_ticker_filter: None,
            notes_insert_mode: false,
            notes_draft: None,
            notes_suggestions: Vec::new(),
            notes_suggestion_selection: 0,
            pending_note_delete: None,
            sec_tab: SecTab::Institutional,
            sec_institutional_selection: 0,
            sec_ceo_selection: 0,
            sec_congress_selection: 0,
            sec_status: None,
            sec_loading: false,
            sec_receiver: None,
            last_sec_refresh: None,
            financial_juice_cooldown_until: None,
            agent: AgentController::new(&config.llm),
            news_receiver: None,
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
                self.config.onboarding.completed = true;
                let _ = self.config.save();
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
            OnboardingStep::Theme => self.set_theme(self.theme_name.move_to(direction)),
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

    pub fn take_market_refresh_request(&mut self) -> bool {
        let requested = self.market_refresh_requested;
        self.market_refresh_requested = false;
        requested
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
        if self.selected_news.is_some() {
            self.selected_news = None;
            return;
        }

        self.show_help = false;
        self.pending_split = false;
        if self.page == Page::Details {
            self.mode = AppMode::Normal;
            self.page = Page::Search;
            self.selected_details = None;
            self.selected_live_details = None;
            self.live_details_loading = false;
            self.live_details_receiver = None;
        } else if self.page == Page::Search {
            self.mode = AppMode::Normal;
            self.page = Page::Dashboard;
        } else if self.page == Page::Settings {
            if self.reset_confirmation.is_some() {
                self.reset_confirmation = None;
                self.clear_text_input_mode();
            } else {
                self.page = Page::Dashboard;
            }
        } else if self.is_editing_watchlist() {
            self.close_watchlist_editor();
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
        self.begin_text_input(InputTarget::Search);
        self.page = Page::Search;
        self.show_help = false;
        self.pending_split = false;
        self.selected_news = None;
        self.refresh_search();
    }

    pub fn open_settings(&mut self) {
        self.mode = AppMode::Normal;
        self.page = Page::Settings;
        self.show_help = false;
        self.pending_split = false;
        self.reset_confirmation = None;
        self.selected_news = None;
    }

    /// Opens the agent panel if closed; focuses its input either way.
    pub fn open_agent(&mut self) {
        self.agent.panel_open = true;
        self.begin_text_input(InputTarget::Agent);
        self.show_help = false;
        self.pending_split = false;
        self.watchlist_editor = None;
        self.agent.auto_scroll = true;
    }

    pub fn close_agent(&mut self) {
        if self.is_text_input_target(InputTarget::Agent) {
            self.mode = AppMode::Normal;
        }
        self.agent.panel_open = false;
    }

    pub fn agent_panel_open(&self) -> bool {
        self.agent.panel_open
    }

    pub fn begin_text_input(&mut self, target: InputTarget) {
        self.mode = AppMode::TextInput(target);
    }

    pub fn clear_text_input_mode(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn is_text_input_active(&self) -> bool {
        matches!(self.mode, AppMode::TextInput(_))
    }

    pub fn is_text_input_target(&self, target: InputTarget) -> bool {
        self.mode == AppMode::TextInput(target)
    }

    pub fn cancel_text_input(&mut self) {
        match self.mode {
            // Esc blurs the agent input but keeps the panel open; a second
            // Esc (handled in event.rs) closes the panel.
            AppMode::TextInput(InputTarget::Agent) => {
                self.clear_text_input_mode();
            }
            AppMode::TextInput(InputTarget::Search) => self.clear_text_input_mode(),
            AppMode::TextInput(InputTarget::ResetConfirmation) => {
                self.reset_confirmation = None;
                self.clear_text_input_mode();
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                if let Some(editor) = &mut self.watchlist_editor {
                    editor.mode = None;
                }
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
                self.clear_text_input_mode();
            }
            // Notes editing is fully handled in event.rs before this dispatcher runs.
            AppMode::TextInput(InputTarget::Notes) => {}
            AppMode::TextInput(InputTarget::NotesSearch) => {
                self.notes_search_query.clear();
                self.notes_ticker_filter = None;
                self.notes_selection = 0;
                self.notes_scroll = 0;
                self.clear_text_input_mode();
            }
            AppMode::Normal => {}
        }
    }

    pub fn submit_text_input(&mut self) {
        match self.mode {
            AppMode::TextInput(InputTarget::Agent) => self.send_agent_message(),
            AppMode::TextInput(InputTarget::Search) => {
                self.clear_text_input_mode();
                self.open_selected_details();
            }
            AppMode::TextInput(InputTarget::ResetConfirmation) => {
                if self
                    .reset_confirmation
                    .as_ref()
                    .is_some_and(|input| input.input == "reset")
                {
                    self.reset_settings_to_defaults();
                }
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                self.save_watchlist_input();
                self.clear_text_input_mode();
            }
            AppMode::TextInput(InputTarget::Notes) => {}
            AppMode::TextInput(InputTarget::NotesSearch) => self.clear_text_input_mode(),
            AppMode::Normal => {}
        }
    }

    pub fn push_text_input_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }

        match self.mode {
            AppMode::TextInput(InputTarget::Agent) => self.agent.input.push(character),
            AppMode::TextInput(InputTarget::Search) => {
                self.search_query.push(character);
                self.reset_search_window();
                self.refresh_search();
            }
            AppMode::TextInput(InputTarget::ResetConfirmation) => {
                if let Some(input) = &mut self.reset_confirmation {
                    input.input.push(character);
                }
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                match self
                    .watchlist_editor
                    .as_mut()
                    .and_then(|editor| editor.mode.as_mut())
                {
                    Some(WatchlistEditMode::Add { input, .. })
                    | Some(WatchlistEditMode::EditAlias { input, .. })
                    | Some(WatchlistEditMode::ChangeTicker { input, .. })
                    | Some(WatchlistEditMode::CreateWatchlist { input }) => input.push(character),
                    None => {}
                }
                self.refresh_watchlist_suggestions();
            }
            AppMode::TextInput(InputTarget::Notes) => {}
            AppMode::TextInput(InputTarget::NotesSearch) => {
                self.notes_search_query.push(character);
                self.notes_ticker_filter = None;
                self.notes_selection = 0;
                self.notes_scroll = 0;
            }
            AppMode::Normal => {}
        }
    }

    pub fn pop_text_input_char(&mut self) {
        match self.mode {
            AppMode::TextInput(InputTarget::Agent) => {
                self.agent.input.pop();
            }
            AppMode::TextInput(InputTarget::Search) => {
                self.search_query.pop();
                self.reset_search_window();
                self.refresh_search();
            }
            AppMode::TextInput(InputTarget::ResetConfirmation) => {
                if let Some(input) = &mut self.reset_confirmation {
                    input.input.pop();
                }
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                match self
                    .watchlist_editor
                    .as_mut()
                    .and_then(|editor| editor.mode.as_mut())
                {
                    Some(WatchlistEditMode::Add { input, .. })
                    | Some(WatchlistEditMode::EditAlias { input, .. })
                    | Some(WatchlistEditMode::ChangeTicker { input, .. })
                    | Some(WatchlistEditMode::CreateWatchlist { input }) => {
                        input.pop();
                    }
                    None => {}
                }
                self.refresh_watchlist_suggestions();
            }
            AppMode::TextInput(InputTarget::Notes) => {}
            AppMode::TextInput(InputTarget::NotesSearch) => {
                self.notes_search_query.pop();
                self.notes_ticker_filter = None;
                self.notes_selection = 0;
                self.notes_scroll = 0;
            }
            AppMode::Normal => {}
        }
    }

    pub fn send_agent_message(&mut self) {
        let context = crate::agent::context::build_context(self);
        self.agent.submit(&context);
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

    /// Drives the agent turn: when the model requested a tool, execute it
    /// against the app and feed the result back into the conversation.
    pub fn poll_agent_response(&mut self) {
        while let Some(call) = self.agent.poll() {
            let result = crate::agent::tools::execute(self, call);
            self.agent.push_tool_result(result);
        }
    }

    pub fn move_agent_scroll(&mut self, direction: SelectionDirection) {
        self.agent.scroll_by(match direction {
            SelectionDirection::Previous => -1,
            SelectionDirection::Next => 1,
        });
    }

    pub fn page_agent_scroll(&mut self, direction: SelectionDirection) {
        self.agent.scroll_by(match direction {
            SelectionDirection::Previous => -6,
            SelectionDirection::Next => 6,
        });
    }

    pub fn stick_agent_scroll_to_bottom(&mut self) {
        self.agent.stick_scroll_to_bottom();
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

    pub fn set_theme(&mut self, theme_name: ThemeName) {
        self.theme_name = theme_name;
        self.config.theme = theme_name;
        let _ = self.config.save();
    }

    pub fn stock_watchlist(&self) -> &[String] {
        &self.active_watchlist().stock_symbols
    }

    pub fn crypto_watchlist(&self) -> &[String] {
        &self.active_watchlist().crypto_symbols
    }

    pub fn watchlists(&self) -> &[NamedWatchlist] {
        &self.config.watchlist.lists
    }

    pub fn active_watchlist_index(&self) -> usize {
        self.config.watchlist.active
    }

    pub fn cycle_active_watchlist(&mut self, direction: SelectionDirection) {
        let count = self.config.watchlist.lists.len();
        if count <= 1 {
            return;
        }

        self.config.watchlist.active = match direction {
            SelectionDirection::Previous => {
                if self.config.watchlist.active == 0 {
                    count - 1
                } else {
                    self.config.watchlist.active - 1
                }
            }
            SelectionDirection::Next => (self.config.watchlist.active + 1) % count,
        };
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
    }

    pub fn delete_active_watchlist(&mut self) {
        if self.config.watchlist.lists.len() <= 1 {
            return;
        }

        let active = self.config.watchlist.active;
        self.config.watchlist.lists.remove(active);
        if self.config.watchlist.active >= self.config.watchlist.lists.len() {
            self.config.watchlist.active = self.config.watchlist.lists.len().saturating_sub(1);
        }
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.request_market_refresh();
        let _ = self.config.save();
    }

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

    pub fn sec_refresh_interval(&self) -> Duration {
        Duration::from_secs(self.config.sec.refresh_interval_seconds.max(1))
    }

    pub fn refresh_sec(&mut self) {
        if self.sec_loading {
            return;
        }

        let db_path = self.ticker_db_path.clone();
        let config = self.config.sec.clone();
        self.sec_loading = true;
        self.last_sec_refresh = Some(Instant::now());
        self.sec_status = Some("SEC sync running".to_string());

        let (sender, receiver) = mpsc::channel();
        self.sec_receiver = Some(receiver);
        thread::spawn(move || match sec::sync::sync_all(&db_path, &config) {
            Ok(count) => {
                let _ = sender.send(SecEvent::Done(format!("SEC synced {count} entities")));
            }
            Err(error) => {
                let _ = sender.send(SecEvent::Error(error));
            }
        });
    }

    pub fn refresh_selected_sec_entity(&mut self) {
        if self.sec_loading {
            return;
        }
        let Some(entity_id) = self.selected_sec_entity_id() else {
            return;
        };

        let db_path = self.ticker_db_path.clone();
        let config = self.config.sec.clone();
        self.sec_loading = true;
        self.sec_status = Some("SEC entity sync running".to_string());

        let (sender, receiver) = mpsc::channel();
        self.sec_receiver = Some(receiver);
        thread::spawn(move || match sec::sync::sync_entity(&db_path, &config, entity_id) {
            Ok(_) => {
                let _ = sender.send(SecEvent::Done("SEC entity synced".to_string()));
            }
            Err(error) => {
                let _ = sender.send(SecEvent::Error(error));
            }
        });
    }

    pub fn poll_sec(&mut self) {
        if let Some(receiver) = &self.sec_receiver {
            match receiver.try_recv() {
                Ok(SecEvent::Done(status)) => {
                    self.sec_loading = false;
                    self.sec_receiver = None;
                    self.sec_status = Some(status);
                }
                Ok(SecEvent::Error(error)) => {
                    self.sec_loading = false;
                    self.sec_receiver = None;
                    self.sec_status = Some(format!("SEC sync error: {error}"));
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.sec_loading = false;
                    self.sec_receiver = None;
                }
            }
        }

        self.maybe_auto_refresh_sec();
    }

    fn maybe_auto_refresh_sec(&mut self) {
        if self.sec_loading || !self.onboarding_complete {
            return;
        }

        let Some(last_refresh) = self.last_sec_refresh else {
            self.refresh_sec();
            return;
        };

        if last_refresh.elapsed() >= self.sec_refresh_interval() {
            self.refresh_sec();
        }
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

    fn maybe_auto_refresh_news(&mut self) {
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

    pub fn cycle_sec_tab(&mut self, direction: SelectionDirection) {
        self.sec_tab = match (self.sec_tab, direction) {
            (SecTab::Institutional, SelectionDirection::Previous) => SecTab::Congress,
            (SecTab::Institutional, SelectionDirection::Next) => SecTab::Ceos,
            (SecTab::Ceos, SelectionDirection::Previous) => SecTab::Institutional,
            (SecTab::Ceos, SelectionDirection::Next) => SecTab::Congress,
            (SecTab::Congress, SelectionDirection::Previous) => SecTab::Ceos,
            (SecTab::Congress, SelectionDirection::Next) => SecTab::Institutional,
        };
    }

    pub fn move_sec_selection(&mut self, direction: SelectionDirection) {
        let max_index = self.sec_entity_count().saturating_sub(1);
        let selection = match self.sec_tab {
            SecTab::Institutional => &mut self.sec_institutional_selection,
            SecTab::Ceos => &mut self.sec_ceo_selection,
            SecTab::Congress => &mut self.sec_congress_selection,
        };
        match direction {
            SelectionDirection::Previous => {
                *selection = selection.saturating_sub(1);
            }
            SelectionDirection::Next => {
                *selection = selection.saturating_add(1).min(max_index);
            }
        }
    }

    pub fn active_sec_selection(&self) -> usize {
        match self.sec_tab {
            SecTab::Institutional => self.sec_institutional_selection,
            SecTab::Ceos => self.sec_ceo_selection,
            SecTab::Congress => self.sec_congress_selection,
        }
    }

    pub fn selected_sec_entity_id(&self) -> Option<i64> {
        let connection = db::open(&self.ticker_db_path).ok()?;
        let entities = match self.sec_tab {
            SecTab::Institutional => db::sec_repo::list_entities(&connection, EntityKind::Institution).ok()?,
            SecTab::Ceos => db::sec_repo::list_ceo_entities(&connection, false).ok()?,
            SecTab::Congress => db::sec_repo::list_ceo_entities(&connection, true).ok()?,
        };
        let index = self.active_sec_selection().min(entities.len().saturating_sub(1));
        entities.get(index).map(|entity| entity.id)
    }

    fn sec_entity_count(&self) -> usize {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return 0;
        };
        match self.sec_tab {
            SecTab::Institutional => db::sec_repo::list_entities(&connection, EntityKind::Institution),
            SecTab::Ceos => db::sec_repo::list_ceo_entities(&connection, false),
            SecTab::Congress => db::sec_repo::list_ceo_entities(&connection, true),
        }
            .map(|entities| entities.len())
            .unwrap_or(0)
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

    fn news_matches_filter(&self, item: &NewsItem) -> bool {
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

    pub fn watchlist_display_name(&self, symbol: &str) -> Option<&str> {
        self.active_watchlist()
            .display_names
            .get(symbol)
            .map(String::as_str)
            .filter(|name| !name.trim().is_empty())
    }

    pub fn notes_visible(&self) -> Vec<db::notes_repo::NoteRow> {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return Vec::new();
        };
        let all = db::notes_repo::list_all(&connection).unwrap_or_default();

        let mut filtered: Vec<db::notes_repo::NoteRow> = if let Some(symbol) = &self.notes_ticker_filter
        {
            all.into_iter()
                .filter(|note| note.tickers.iter().any(|ticker| ticker == symbol))
                .collect()
        } else {
            all.into_iter()
                .filter(|note| self.notes_matches_tab(note))
                .collect()
        };

        let query = self.notes_search_query.trim();
        if !query.is_empty() {
            let mut ids = db::notes_repo::search_fts(&connection, query).unwrap_or_default();
            if ids.is_empty() {
                let lowered = query.to_ascii_lowercase();
                ids = filtered
                    .iter()
                    .filter(|note| note.body.to_ascii_lowercase().contains(&lowered))
                    .map(|note| note.id)
                    .collect();
            }
            filtered.retain(|note| ids.contains(&note.id));
        }

        filtered
    }

    fn notes_matches_tab(&self, note: &db::notes_repo::NoteRow) -> bool {
        match self.notes_tab {
            NotesFilterTab::All => true,
            NotesFilterTab::Tickers => !note.tickers.is_empty(),
            NotesFilterTab::Journal => note.tickers.is_empty(),
            NotesFilterTab::Pinned => note.pinned,
        }
    }

    pub fn notes_selected_row(&self) -> Option<db::notes_repo::NoteRow> {
        let rows = self.notes_visible();
        rows.get(self.notes_selection.min(rows.len().saturating_sub(1)))
            .cloned()
    }

    pub fn cycle_notes_tab(&mut self, direction: SelectionDirection) {
        self.notes_tab = match (self.notes_tab, direction) {
            (NotesFilterTab::All, SelectionDirection::Previous) => NotesFilterTab::Pinned,
            (NotesFilterTab::All, SelectionDirection::Next) => NotesFilterTab::Tickers,
            (NotesFilterTab::Tickers, SelectionDirection::Previous) => NotesFilterTab::All,
            (NotesFilterTab::Tickers, SelectionDirection::Next) => NotesFilterTab::Journal,
            (NotesFilterTab::Journal, SelectionDirection::Previous) => NotesFilterTab::Tickers,
            (NotesFilterTab::Journal, SelectionDirection::Next) => NotesFilterTab::Pinned,
            (NotesFilterTab::Pinned, SelectionDirection::Previous) => NotesFilterTab::Journal,
            (NotesFilterTab::Pinned, SelectionDirection::Next) => NotesFilterTab::All,
        };
        self.notes_ticker_filter = None;
        self.notes_selection = 0;
        self.notes_scroll = 0;
    }

    pub fn move_notes_selection(&mut self, direction: SelectionDirection) {
        let count = self.notes_visible().len();
        if count == 0 {
            self.notes_selection = 0;
            self.notes_scroll = 0;
            return;
        }

        self.notes_selection = match direction {
            SelectionDirection::Previous => self.notes_selection.saturating_sub(1),
            SelectionDirection::Next => (self.notes_selection + 1).min(count - 1),
        };
        self.sync_notes_scroll(6);
    }

    pub fn sync_notes_scroll(&mut self, visible_rows: usize) {
        if visible_rows == 0 {
            self.notes_scroll = self.notes_selection;
            return;
        }

        if self.notes_selection < self.notes_scroll {
            self.notes_scroll = self.notes_selection;
        } else if self.notes_selection >= self.notes_scroll + visible_rows {
            self.notes_scroll = self.notes_selection + 1 - visible_rows;
        }
    }

    /// Creates an empty note, inserts it into the list, selects it, and
    /// drops straight into insert mode — mirrors "New Note" in Apple Notes.
    pub fn create_new_note(&mut self) {
        let now = chrono::Utc::now().timestamp();
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return;
        };
        let Ok(id) = db::notes_repo::insert(&connection, "", &[], &[], now) else {
            return;
        };

        self.notes_tab = NotesFilterTab::All;
        self.notes_ticker_filter = None;
        self.notes_search_query.clear();

        let rows = self.notes_visible();
        self.notes_selection = rows.iter().position(|note| note.id == id).unwrap_or(0);
        self.sync_notes_scroll(6);

        self.notes_draft = Some(NotesDraft {
            note_id: id,
            body: String::new(),
        });
        self.notes_suggestions.clear();
        self.notes_suggestion_selection = 0;
        self.notes_insert_mode = true;
        self.begin_text_input(InputTarget::Notes);
    }

    /// Enters insert mode on the currently selected note (Enter / `i`).
    /// Falls back to creating a new note if the list is empty.
    pub fn enter_note_insert_mode(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            self.create_new_note();
            return;
        };
        self.notes_draft = Some(NotesDraft {
            note_id: note.id,
            body: note.body,
        });
        self.notes_suggestions.clear();
        self.notes_suggestion_selection = 0;
        self.notes_insert_mode = true;
        self.begin_text_input(InputTarget::Notes);
    }

    /// Leaves insert mode (Esc): persists the draft, recomputes
    /// tickers/tags, and drops empty notes rather than leaving clutter.
    pub fn exit_note_insert_mode(&mut self) {
        self.finalize_note_draft();
        self.notes_insert_mode = false;
        self.notes_suggestions.clear();
        self.notes_suggestion_selection = 0;
        self.clear_text_input_mode();
    }

    fn finalize_note_draft(&mut self) {
        let Some(draft) = self.notes_draft.take() else {
            return;
        };
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return;
        };

        if draft.body.trim().is_empty() {
            let _ = db::notes_repo::delete(&connection, draft.note_id);
        } else {
            let tickers = self.extract_note_tickers(&draft.body);
            let tags = extract_note_tags(&draft.body);
            let now = chrono::Utc::now().timestamp();
            let _ = db::notes_repo::update(&connection, draft.note_id, &draft.body, &tickers, &tags, now);
        }

        let visible = self.notes_visible().len();
        if self.notes_selection >= visible {
            self.notes_selection = visible.saturating_sub(1);
        }
        self.sync_notes_scroll(6);
    }

    pub fn insert_note_draft_newline(&mut self) {
        if let Some(draft) = &mut self.notes_draft {
            draft.body.push('\n');
        }
        self.refresh_note_suggestions();
    }

    pub fn push_note_draft_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        if let Some(draft) = &mut self.notes_draft {
            draft.body.push(character);
        }
        self.refresh_note_suggestions();
    }

    pub fn pop_note_draft_char(&mut self) {
        if let Some(draft) = &mut self.notes_draft {
            draft.body.pop();
        }
        self.refresh_note_suggestions();
    }

    fn refresh_note_suggestions(&mut self) {
        let Some(draft) = &self.notes_draft else {
            self.notes_suggestions.clear();
            self.notes_suggestion_selection = 0;
            return;
        };

        let last_token = draft.body.split_whitespace().last().unwrap_or("");
        let Some(query) = last_token
            .strip_prefix('$')
            .filter(|query| !query.is_empty())
        else {
            self.notes_suggestions.clear();
            self.notes_suggestion_selection = 0;
            return;
        };

        match db::open(&self.ticker_db_path)
            .and_then(|connection| search::search_assets(&connection, query, &["stock", "etf"], 6))
        {
            Ok(results) => {
                self.notes_suggestions = results;
                self.notes_suggestion_selection = self
                    .notes_suggestion_selection
                    .min(self.notes_suggestions.len().saturating_sub(1));
            }
            Err(_) => {
                self.notes_suggestions.clear();
                self.notes_suggestion_selection = 0;
            }
        }
    }

    pub fn move_note_suggestion(&mut self, direction: SelectionDirection) {
        if self.notes_suggestions.is_empty() {
            self.notes_suggestion_selection = 0;
            return;
        }

        self.notes_suggestion_selection = match direction {
            SelectionDirection::Previous => self.notes_suggestion_selection.saturating_sub(1),
            SelectionDirection::Next => (self.notes_suggestion_selection + 1)
                .min(self.notes_suggestions.len().saturating_sub(1)),
        };
    }

    pub fn accept_note_suggestion(&mut self) {
        let Some(suggestion) = self
            .notes_suggestions
            .get(self.notes_suggestion_selection)
            .cloned()
        else {
            return;
        };
        let Some(draft) = &mut self.notes_draft else {
            return;
        };

        match draft.body.rfind(char::is_whitespace) {
            Some(index) => draft.body.truncate(index + 1),
            None => draft.body.clear(),
        }
        draft.body.push('$');
        draft.body.push_str(&suggestion.symbol);
        draft.body.push(' ');

        self.notes_suggestions.clear();
        self.notes_suggestion_selection = 0;
    }

    fn extract_note_tickers(&self, body: &str) -> Vec<String> {
        let Ok(connection) = db::open(&self.ticker_db_path) else {
            return Vec::new();
        };
        let Ok(mut statement) =
            connection.prepare("SELECT symbol FROM instruments WHERE active = 1")
        else {
            return Vec::new();
        };
        let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(0)) else {
            return Vec::new();
        };

        let mut matched: Vec<String> = rows
            .filter_map(std::result::Result::ok)
            .filter(|symbol| news::contains_symbol(body, symbol))
            .collect();
        matched.sort();
        matched.dedup();
        matched
    }

    pub fn toggle_selected_note_pin(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            return;
        };
        if let Ok(connection) = db::open(&self.ticker_db_path) {
            let _ = db::notes_repo::set_pinned(&connection, note.id, !note.pinned);
        }
    }

    pub fn begin_delete_selected_note(&mut self) {
        let Some(note) = self.notes_selected_row() else {
            return;
        };
        self.pending_note_delete = Some(note.id);
    }

    pub fn confirm_delete_note(&mut self) {
        let Some(id) = self.pending_note_delete.take() else {
            return;
        };
        if let Ok(connection) = db::open(&self.ticker_db_path) {
            let _ = db::notes_repo::delete(&connection, id);
        }

        let visible = self.notes_visible().len();
        if self.notes_selection >= visible {
            self.notes_selection = visible.saturating_sub(1);
        }
        self.sync_notes_scroll(6);
    }

    pub fn cancel_delete_note(&mut self) {
        self.pending_note_delete = None;
    }

    pub fn begin_notes_search(&mut self) {
        self.begin_text_input(InputTarget::NotesSearch);
    }

    pub fn notes_ticker_symbols(&self) -> std::collections::HashSet<String> {
        db::open(&self.ticker_db_path)
            .and_then(|connection| db::notes_repo::all_ticker_symbols(&connection))
            .unwrap_or_default()
    }

    pub fn jump_to_notes_for_symbol(&mut self, symbol: &str) {
        self.notes_tab = NotesFilterTab::Tickers;
        self.notes_ticker_filter = Some(symbol.to_string());
        self.notes_search_query.clear();
        self.notes_selection = 0;
        self.notes_scroll = 0;
        self.set_panel_content(PanelId::Notes, WindowKind::Notes);
        self.focus_panel(PanelId::Notes);
    }

    pub fn jump_to_selected_watchlist_row_notes(&mut self) {
        let symbol = match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => self.stock_watchlist().get(index).cloned(),
            Some(WatchlistEditRow::Crypto(index)) => self.crypto_watchlist().get(index).cloned(),
            _ => None,
        };
        let Some(symbol) = symbol else {
            return;
        };
        self.close_watchlist_editor();
        self.jump_to_notes_for_symbol(&symbol);
    }

    pub fn selected_settings_item(&self) -> SettingsItem {
        SettingsItem::ALL[self.settings_selection.min(SettingsItem::ALL.len() - 1)]
    }

    pub fn move_settings_selection(&mut self, direction: SelectionDirection) {
        if self.reset_confirmation.is_some() {
            return;
        }

        self.settings_selection = match direction {
            SelectionDirection::Previous => {
                if self.settings_selection == 0 {
                    SettingsItem::ALL.len() - 1
                } else {
                    self.settings_selection - 1
                }
            }
            SelectionDirection::Next => (self.settings_selection + 1) % SettingsItem::ALL.len(),
        };
    }

    pub fn activate_settings_item(&mut self) {
        if let Some(input) = &self.reset_confirmation {
            if input.input == "reset" {
                self.reset_settings_to_defaults();
            }
            return;
        }

        match self.selected_settings_item() {
            SettingsItem::Language => self.toggle_locale(),
            SettingsItem::Theme => self.set_theme(self.theme_name.next()),
            SettingsItem::Onboarding => self.toggle_onboarding_preference(),
            SettingsItem::Reset => {
                self.reset_confirmation = Some(TextInput {
                    input: String::new(),
                });
                self.begin_text_input(InputTarget::ResetConfirmation);
            }
        }
    }

    pub fn adjust_settings_item(&mut self, direction: SelectionDirection) {
        if self.reset_confirmation.is_some() {
            return;
        }

        match self.selected_settings_item() {
            SettingsItem::Language => {
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
            SettingsItem::Theme => self.set_theme(self.theme_name.move_to(direction)),
            SettingsItem::Onboarding => self.toggle_onboarding_preference(),
            SettingsItem::Reset => {}
        }
    }

    pub fn is_editing_watchlist(&self) -> bool {
        self.watchlist_editor.is_some()
    }

    pub fn open_watchlist_editor(&mut self) {
        self.clear_text_input_mode();
        self.watchlist_editor = Some(WatchlistEditor {
            selection: 0,
            mode: None,
        });
    }

    pub fn close_watchlist_editor(&mut self) {
        if let Some(editor) = &mut self.watchlist_editor {
            if editor.mode.is_some() {
                editor.mode = None;
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
                self.clear_text_input_mode();
                return;
            }
        }

        self.watchlist_editor = None;
    }

    pub fn watchlist_rows(&self) -> Vec<WatchlistEditRow> {
        let mut rows = Vec::new();
        rows.push(WatchlistEditRow::AddStock);
        rows.extend((0..self.stock_watchlist().len()).map(WatchlistEditRow::Stock));
        rows.push(WatchlistEditRow::AddCrypto);
        rows.extend((0..self.crypto_watchlist().len()).map(WatchlistEditRow::Crypto));
        rows
    }

    pub fn selected_watchlist_row(&self) -> Option<WatchlistEditRow> {
        let editor = self.watchlist_editor.as_ref()?;
        self.watchlist_rows().get(editor.selection).copied()
    }

    pub fn move_watchlist_selection(&mut self, direction: SelectionDirection) {
        if self
            .watchlist_editor
            .as_ref()
            .is_some_and(|editor| editor.mode.is_some())
        {
            return;
        }

        let count = self.watchlist_rows().len();
        let Some(editor) = &mut self.watchlist_editor else {
            return;
        };

        editor.selection = match direction {
            SelectionDirection::Previous => {
                if editor.selection == 0 {
                    count - 1
                } else {
                    editor.selection - 1
                }
            }
            SelectionDirection::Next => (editor.selection + 1) % count,
        };
    }

    pub fn activate_watchlist_editor(&mut self) {
        if self
            .watchlist_editor
            .as_ref()
            .is_some_and(|editor| editor.mode.is_some())
        {
            self.save_watchlist_input();
            return;
        }

        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::AddStock) => self.begin_watchlist_add(WatchlistKind::Stock),
            Some(WatchlistEditRow::AddCrypto) => self.begin_watchlist_add(WatchlistKind::Crypto),
            Some(WatchlistEditRow::Stock(index)) => {
                self.begin_watchlist_alias_edit(WatchlistKind::Stock, index)
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                self.begin_watchlist_alias_edit(WatchlistKind::Crypto, index)
            }
            None => {}
        }
    }

    pub fn delete_selected_watchlist_symbol(&mut self) {
        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => {
                if index < self.stock_watchlist().len() {
                    self.active_watchlist_mut().stock_symbols.remove(index);
                }
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                if index < self.crypto_watchlist().len() {
                    self.active_watchlist_mut().crypto_symbols.remove(index);
                }
            }
            _ => return,
        }

        self.cleanup_watchlist_aliases();
        self.clamp_watchlist_selection();
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
    }

    pub fn begin_watchlist_add(&mut self, kind: WatchlistKind) {
        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::Add {
                kind,
                input: String::new(),
            });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.refresh_watchlist_suggestions();
    }

    pub fn begin_watchlist_create(&mut self) {
        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::CreateWatchlist {
                input: String::new(),
            });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
    }

    fn begin_watchlist_alias_edit(&mut self, kind: WatchlistKind, index: usize) {
        let symbol = match kind {
            WatchlistKind::Stock => self.stock_watchlist().get(index),
            WatchlistKind::Crypto => self.crypto_watchlist().get(index),
        }
        .cloned()
        .unwrap_or_default();
        let input = self
            .watchlist_display_name(&symbol)
            .unwrap_or(symbol.as_str())
            .to_string();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::EditAlias { symbol, input });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
    }

    pub fn begin_selected_watchlist_ticker_change(&mut self) {
        match self.selected_watchlist_row() {
            Some(WatchlistEditRow::Stock(index)) => {
                self.begin_watchlist_ticker_change(WatchlistKind::Stock, index)
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                self.begin_watchlist_ticker_change(WatchlistKind::Crypto, index)
            }
            _ => {}
        }
    }

    fn begin_watchlist_ticker_change(&mut self, kind: WatchlistKind, index: usize) {
        let input = match kind {
            WatchlistKind::Stock => self.stock_watchlist().get(index),
            WatchlistKind::Crypto => self.crypto_watchlist().get(index),
        }
        .cloned()
        .unwrap_or_default();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::ChangeTicker { kind, index, input });
        }
        self.begin_text_input(InputTarget::Watchlist);
        self.refresh_watchlist_suggestions();
    }

    pub fn move_watchlist_suggestion(&mut self, direction: SelectionDirection) {
        if self.watchlist_suggestions.is_empty() {
            self.watchlist_suggestion_selection = 0;
            return;
        }

        self.watchlist_suggestion_selection = match direction {
            SelectionDirection::Previous => self.watchlist_suggestion_selection.saturating_sub(1),
            SelectionDirection::Next => (self.watchlist_suggestion_selection + 1)
                .min(self.watchlist_suggestions.len().saturating_sub(1)),
        };
    }

    fn selected_watchlist_input_symbol(&self, input: &str) -> Option<String> {
        self.watchlist_suggestions
            .get(self.watchlist_suggestion_selection)
            .map(|suggestion| suggestion.symbol.clone())
            .or_else(|| normalize_symbol(input))
    }

    fn save_watchlist_input(&mut self) {
        let Some(mode) = self
            .watchlist_editor
            .as_mut()
            .and_then(|editor| editor.mode.take())
        else {
            return;
        };

        match mode {
            WatchlistEditMode::Add { kind, input } => {
                if let Some(symbol) = self.selected_watchlist_input_symbol(&input) {
                    let list = self.watchlist_mut(kind);
                    if !list.contains(&symbol) {
                        list.push(symbol);
                    }
                }
            }
            WatchlistEditMode::EditAlias { symbol, input } => {
                let alias = input.trim();
                if alias.is_empty() || alias.eq_ignore_ascii_case(&symbol) {
                    self.active_watchlist_mut().display_names.remove(&symbol);
                } else {
                    self.active_watchlist_mut()
                        .display_names
                        .insert(symbol, alias.to_string());
                }
            }
            WatchlistEditMode::ChangeTicker { kind, index, input } => {
                if let Some(symbol) = self.selected_watchlist_input_symbol(&input) {
                    let alias_migration = {
                        let list = self.watchlist_mut(kind);
                        if index < list.len() {
                            let old_symbol = std::mem::replace(&mut list[index], symbol);
                            let new_symbol = list[index].clone();
                            Some((old_symbol, new_symbol))
                        } else {
                            None
                        }
                    };
                    if let Some((old_symbol, new_symbol)) = alias_migration {
                        self.migrate_watchlist_alias(&old_symbol, &new_symbol);
                    }
                }
            }
            WatchlistEditMode::CreateWatchlist { input } => {
                let name = input.trim();
                if !name.is_empty() {
                    self.config.watchlist.lists.push(NamedWatchlist {
                        name: name.to_string(),
                        crypto_symbols: Vec::new(),
                        stock_symbols: Vec::new(),
                        display_names: std::collections::HashMap::new(),
                    });
                    self.config.watchlist.active = self.config.watchlist.lists.len() - 1;
                }
            }
        }

        self.active_watchlist_mut().stock_symbols.sort();
        self.active_watchlist_mut().stock_symbols.dedup();
        self.active_watchlist_mut().crypto_symbols.sort();
        self.active_watchlist_mut().crypto_symbols.dedup();
        self.cleanup_watchlist_aliases();
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
        self.clear_text_input_mode();
        self.request_market_refresh();
        let _ = self.config.save();
    }

    pub fn agent_create_watchlist(&mut self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("watchlist name must not be empty".to_string());
        }
        if self
            .config
            .watchlist
            .lists
            .iter()
            .any(|list| list.name.eq_ignore_ascii_case(name))
        {
            return Err(format!("watchlist `{name}` already exists"));
        }

        self.config.watchlist.lists.push(NamedWatchlist {
            name: name.to_string(),
            crypto_symbols: Vec::new(),
            stock_symbols: Vec::new(),
            display_names: std::collections::HashMap::new(),
        });
        self.config.watchlist.active = self.config.watchlist.lists.len() - 1;
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("created watchlist `{name}` and made it active"))
    }

    pub fn agent_add_symbol_to_watchlist(&mut self, symbol: &str) -> Result<String, String> {
        let Some(symbol) = normalize_symbol(symbol) else {
            return Err("symbol must not be empty".to_string());
        };

        let kind = if symbol.contains('-') {
            WatchlistKind::Crypto
        } else {
            WatchlistKind::Stock
        };
        let list = self.watchlist_mut(kind);
        if list.contains(&symbol) {
            return Err(format!("{symbol} is already on the active watchlist"));
        }
        list.push(symbol.clone());
        list.sort();

        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("added {symbol} to the active watchlist"))
    }

    pub fn agent_remove_symbol_from_watchlist(&mut self, symbol: &str) -> Result<String, String> {
        let Some(symbol) = normalize_symbol(symbol) else {
            return Err("symbol must not be empty".to_string());
        };

        let mut removed = false;
        for kind in [WatchlistKind::Stock, WatchlistKind::Crypto] {
            let list = self.watchlist_mut(kind);
            let before = list.len();
            list.retain(|existing| !existing.eq_ignore_ascii_case(&symbol));
            removed |= list.len() != before;
        }
        if !removed {
            return Err(format!("{symbol} is not on the active watchlist"));
        }

        self.cleanup_watchlist_aliases();
        self.clamp_watchlist_selection();
        self.retain_configured_quotes();
        self.request_market_refresh();
        let _ = self.config.save();
        Ok(format!("removed {symbol} from the active watchlist"))
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

    fn watchlist_mut(&mut self, kind: WatchlistKind) -> &mut Vec<String> {
        match kind {
            WatchlistKind::Stock => &mut self.active_watchlist_mut().stock_symbols,
            WatchlistKind::Crypto => &mut self.active_watchlist_mut().crypto_symbols,
        }
    }

    fn active_watchlist(&self) -> &NamedWatchlist {
        &self.config.watchlist.lists[self.config.watchlist.active]
    }

    fn active_watchlist_mut(&mut self) -> &mut NamedWatchlist {
        let active = self.config.watchlist.active;
        &mut self.config.watchlist.lists[active]
    }

    fn retain_configured_quotes(&mut self) {
        let stock_symbols = self.stock_watchlist().to_vec();
        let crypto_symbols = self.crypto_watchlist().to_vec();
        self.stock_quotes
            .retain(|quote| stock_symbols.contains(&quote.symbol));
        self.crypto_quotes
            .retain(|quote| crypto_symbols.contains(&quote.symbol));
    }

    fn refresh_watchlist_suggestions(&mut self) {
        let Some(mode) = self
            .watchlist_editor
            .as_ref()
            .and_then(|editor| editor.mode.as_ref())
        else {
            self.watchlist_suggestions.clear();
            self.watchlist_suggestion_selection = 0;
            return;
        };

        let (kind, input) = match mode {
            WatchlistEditMode::Add { kind, input }
            | WatchlistEditMode::ChangeTicker { kind, input, .. } => (*kind, input.as_str()),
            WatchlistEditMode::EditAlias { .. } | WatchlistEditMode::CreateWatchlist { .. } => {
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
                return;
            }
        };

        if kind != WatchlistKind::Stock {
            self.watchlist_suggestions.clear();
            self.watchlist_suggestion_selection = 0;
            return;
        }

        match crate::db::open(&self.ticker_db_path)
            .and_then(|connection| search::search_assets(&connection, input, &["stock", "etf"], 6))
        {
            Ok(results) => {
                self.watchlist_suggestions = results;
                self.watchlist_suggestion_selection = self
                    .watchlist_suggestion_selection
                    .min(self.watchlist_suggestions.len().saturating_sub(1));
            }
            Err(_) => {
                self.watchlist_suggestions.clear();
                self.watchlist_suggestion_selection = 0;
            }
        }
    }

    fn migrate_watchlist_alias(&mut self, old_symbol: &str, new_symbol: &str) {
        if old_symbol == new_symbol {
            return;
        }

        if let Some(alias) = self.active_watchlist_mut().display_names.remove(old_symbol) {
            self.active_watchlist_mut()
                .display_names
                .entry(new_symbol.to_string())
                .or_insert(alias);
        }
    }

    fn cleanup_watchlist_aliases(&mut self) {
        let stock_symbols = self.stock_watchlist().to_vec();
        let crypto_symbols = self.crypto_watchlist().to_vec();
        self.active_watchlist_mut()
            .display_names
            .retain(|symbol, _| stock_symbols.contains(symbol) || crypto_symbols.contains(symbol));
    }

    fn request_market_refresh(&mut self) {
        self.market_refresh_requested = true;
    }

    fn clamp_watchlist_selection(&mut self) {
        let count = self.watchlist_rows().len();
        if let Some(editor) = &mut self.watchlist_editor {
            editor.selection = editor.selection.min(count.saturating_sub(1));
        }
    }

    fn toggle_onboarding_preference(&mut self) {
        self.onboarding_complete = !self.onboarding_complete;
        self.config.onboarding.completed = self.onboarding_complete;
        let _ = self.config.save();
    }

    fn reset_settings_to_defaults(&mut self) {
        let config = AppConfig::default().unwrap_or_else(|_| <AppConfig as Default>::default());
        self.config = config.clone();
        let locale = config.locale.clone();
        self.locale = locale.clone();
        self.i18n.set_active(locale);
        self.theme_name = config.theme;
        self.onboarding_complete = config.onboarding.completed;
        self.onboarding_step = OnboardingStep::Welcome;
        self.page = Page::Onboarding;
        self.settings_selection = 0;
        self.reset_confirmation = None;
        self.watchlist_editor = None;
        self.news_items.clear();
        self.news_selection = 0;
        self.news_scroll = 0;
        self.news_loading = false;
        self.news_status = None;
        self.selected_news = None;
        self.news_filter_tab = NewsFilterTab::All;
        self.news_source_label = "news feed".to_string();
        self.news_connection_status = "connecting...".to_string();
        self.news_source_counts.clear();
        self.collapsed_watchlist_news.clear();
        self.known_watchlist_news_symbols.clear();
        self.last_news_refresh = None;
        self.financial_juice_cooldown_until = None;
        self.news_receiver = None;
        self.agent = AgentController::new(&self.config.llm);
        self.notes_tab = NotesFilterTab::All;
        self.notes_selection = 0;
        self.notes_scroll = 0;
        self.notes_search_query.clear();
        self.notes_ticker_filter = None;
        self.notes_insert_mode = false;
        self.notes_draft = None;
        self.notes_suggestions.clear();
        self.notes_suggestion_selection = 0;
        self.pending_note_delete = None;
        self.reset_dashboard();
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

    fn sync_collapsed_watchlist_news(&mut self) {
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

    fn build_watchlist_matchers(&self, kind: WatchlistKind) -> Vec<WatchlistMatcher> {
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

    fn financial_juice_in_cooldown(&self) -> bool {
        self.financial_juice_cooldown_until
            .is_some_and(|until| Instant::now() < until)
    }

    fn update_financial_juice_backoff(&mut self) {
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

impl PanelId {
    pub const ALL: [Self; 4] = [Self::News, Self::Watchlist, Self::Calendar, Self::Notes];
}

impl SettingsItem {
    pub const ALL: [Self; 4] = [Self::Language, Self::Theme, Self::Onboarding, Self::Reset];
}

impl WindowKind {
    pub const CHOICES: [Self; 5] = [
        Self::News,
        Self::Watchlist,
        Self::Calendar,
        Self::Notes,
        Self::Sec,
    ];

    pub fn label_key(self) -> Key {
        match self {
            Self::News => Key::PanelTitleNews,
            Self::Watchlist => Key::PanelTitleWatchlist,
            Self::Calendar => Key::PanelTitleCalendar,
            Self::Notes => Key::PanelTitleNotes,
            Self::Sec => Key::PanelTitleSec,
            Self::Picker => Key::PanelTitlePicker,
        }
    }
}

impl Default for PanelContents {
    fn default() -> Self {
        Self {
            news: WindowKind::News,
            watchlist: WindowKind::Watchlist,
            calendar: WindowKind::Sec,
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

fn extract_note_tags(body: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for token in body.split_whitespace() {
        let trimmed = token.trim_matches(|character: char| {
            character.is_ascii_punctuation() && character != '#'
        });
        if trimmed.len() > 1 && trimmed.starts_with('#') && !tags.contains(&trimmed.to_string()) {
            tags.push(trimmed.to_string());
        }
    }
    tags
}

fn normalize_symbol(input: &str) -> Option<String> {
    let symbol = input.trim().to_ascii_uppercase();
    if symbol.is_empty() {
        None
    } else {
        Some(symbol)
    }
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

impl ThemeName {
    fn move_to(self, direction: SelectionDirection) -> Self {
        match direction {
            SelectionDirection::Previous => self.previous(),
            SelectionDirection::Next => self.next(),
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Dark => Self::Bloomberg,
            Self::Light => Self::Dark,
            Self::Transparent => Self::Light,
            Self::Bloomberg => Self::Transparent,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Transparent,
            Self::Transparent => Self::Bloomberg,
            Self::Bloomberg => Self::Dark,
        }
    }

    pub fn label_key(self) -> Key {
        match self {
            Self::Dark => Key::AppThemeDark,
            Self::Light => Key::AppThemeLight,
            Self::Transparent => Key::AppThemeTransparent,
            Self::Bloomberg => Key::AppThemeBloomberg,
        }
    }
}
