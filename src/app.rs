use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::{
    ai::client::LlmClient,
    config::AppConfig,
    config::LlmConfig,
    i18n::{I18n, Key, Locale},
    market::{MarketEvent, MarketSession},
    news::{self, FeedSource, NewsItem},
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
    Settings,
    Agent,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: AgentRole,
    pub content: String,
}

#[derive(Debug)]
enum AgentEvent {
    Status(String),
    Chunk(String),
    Done,
    Error(String),
}

#[derive(Debug)]
enum NewsEvent {
    Loaded(Vec<NewsItem>),
    Error(String),
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
    pub agent_input: String,
    pub agent_messages: Vec<AgentMessage>,
    pub agent_loading: bool,
    pub agent_status: Option<String>,
    pub agent_scroll: u16,
    pub agent_auto_scroll: bool,
    agent_response_receiver: Option<Receiver<AgentEvent>>,
    news_receiver: Option<Receiver<NewsEvent>>,
    agent_previous_page: Page,
    llm_config: LlmConfig,
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
            page: if onboarding_complete {
                Page::Dashboard
            } else {
                Page::Onboarding
            },
            onboarding_step: OnboardingStep::Welcome,
            onboarding_complete,
            logged_in: false,
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
            agent_input: String::new(),
            agent_messages: Vec::new(),
            agent_loading: false,
            agent_status: None,
            agent_scroll: 0,
            agent_auto_scroll: true,
            agent_response_receiver: None,
            news_receiver: None,
            agent_previous_page: Page::Dashboard,
            llm_config: config.llm.clone(),
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
            self.page = Page::Search;
            self.selected_details = None;
            self.selected_live_details = None;
            self.live_details_loading = false;
            self.live_details_receiver = None;
        } else if self.page == Page::Search {
            self.page = Page::Dashboard;
        } else if self.page == Page::Settings {
            if self.reset_confirmation.is_some() {
                self.reset_confirmation = None;
            } else {
                self.page = Page::Dashboard;
            }
        } else if self.page == Page::Agent {
            self.close_agent();
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
        self.page = Page::Search;
        self.show_help = false;
        self.pending_split = false;
        self.selected_news = None;
        self.refresh_search();
    }

    pub fn open_settings(&mut self) {
        self.page = Page::Settings;
        self.show_help = false;
        self.pending_split = false;
        self.reset_confirmation = None;
        self.selected_news = None;
    }

    pub fn open_agent(&mut self) {
        if self.page != Page::Agent {
            self.agent_previous_page = self.page;
        }
        self.page = Page::Agent;
        self.show_help = false;
        self.pending_split = false;
        self.watchlist_editor = None;
        self.agent_auto_scroll = true;
    }

    pub fn close_agent(&mut self) {
        self.page = self.agent_previous_page;
        self.agent_loading = false;
        self.agent_response_receiver = None;
        self.agent_status = None;
    }

    pub fn agent_background_page(&self) -> Page {
        self.agent_previous_page
    }

    pub fn push_agent_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        self.agent_input.push(character);
    }

    pub fn pop_agent_char(&mut self) {
        self.agent_input.pop();
    }

    pub fn send_agent_message(&mut self) {
        if self.agent_loading {
            return;
        }

        let prompt = self.agent_input.trim().to_string();
        if prompt.is_empty() {
            return;
        }

        self.agent_messages.push(AgentMessage {
            role: AgentRole::User,
            content: prompt.clone(),
        });
        self.agent_messages.push(AgentMessage {
            role: AgentRole::Assistant,
            content: String::new(),
        });
        self.agent_input.clear();
        self.agent_loading = true;
        self.agent_status = Some("debug: preparing request".to_string());
        self.agent_auto_scroll = true;

        let llm_config = self.llm_config.clone();
        let (sender, receiver) = mpsc::channel();
        self.agent_response_receiver = Some(receiver);
        thread::spawn(move || match llm_config.api_key {
            Some(api_key) if !api_key.trim().is_empty() => {
                let _ = sender.send(AgentEvent::Status("debug: connecting".to_string()));
                let result = LlmClient::new(llm_config.base_url, api_key, llm_config.model)
                    .chat_stream(
                        &prompt,
                        |chunk| {
                            let _ = sender.send(AgentEvent::Chunk(chunk));
                        },
                        |status| {
                            let _ = sender.send(AgentEvent::Status(status));
                        },
                    );
                match result {
                    Ok(()) => {
                        let _ = sender.send(AgentEvent::Done);
                    }
                    Err(error) => {
                        let _ = sender.send(AgentEvent::Error(error));
                    }
                }
            }
            _ => {
                let _ = sender.send(AgentEvent::Error(
                    "missing LLM_API_KEY / OPENROUTER_API_KEY".to_string(),
                ));
            }
        });
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

    pub fn poll_agent_response(&mut self) {
        let Some(receiver) = &self.agent_response_receiver else {
            return;
        };

        loop {
            match receiver.try_recv() {
                Ok(AgentEvent::Status(status)) => {
                    self.agent_status = Some(status);
                }
                Ok(AgentEvent::Chunk(content)) => {
                    if let Some(message) = self
                        .agent_messages
                        .iter_mut()
                        .rev()
                        .find(|message| message.role == AgentRole::Assistant)
                    {
                        message.content.push_str(&content);
                    }
                    self.agent_status = Some("debug: receiving chunks".to_string());
                    if self.agent_auto_scroll {
                        self.agent_scroll = u16::MAX;
                    }
                }
                Ok(AgentEvent::Done) => {
                    self.agent_loading = false;
                    self.agent_status = Some("debug: stream completed".to_string());
                    self.agent_response_receiver = None;
                    break;
                }
                Ok(AgentEvent::Error(error)) => {
                    if self.agent_messages.last().is_some_and(|message| {
                        message.role == AgentRole::Assistant && message.content.is_empty()
                    }) {
                        self.agent_messages.pop();
                    }
                    self.agent_loading = false;
                    self.agent_status = Some(error);
                    self.agent_response_receiver = None;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.agent_loading = false;
                    self.agent_status = Some("request interrupted".to_string());
                    self.agent_response_receiver = None;
                    break;
                }
            }
        }
    }

    pub fn move_agent_scroll(&mut self, direction: SelectionDirection) {
        self.agent_auto_scroll = false;
        self.agent_scroll = match direction {
            SelectionDirection::Previous => self.agent_scroll.saturating_sub(1),
            SelectionDirection::Next => self.agent_scroll.saturating_add(1),
        };
    }

    pub fn page_agent_scroll(&mut self, direction: SelectionDirection) {
        self.agent_auto_scroll = false;
        self.agent_scroll = match direction {
            SelectionDirection::Previous => self.agent_scroll.saturating_sub(6),
            SelectionDirection::Next => self.agent_scroll.saturating_add(6),
        };
    }

    pub fn stick_agent_scroll_to_bottom(&mut self) {
        self.agent_auto_scroll = true;
        self.agent_scroll = u16::MAX;
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
        &self.config.watchlist.stock_symbols
    }

    pub fn crypto_watchlist(&self) -> &[String] {
        &self.config.watchlist.crypto_symbols
    }

    pub fn news_fetch_on_startup(&self) -> bool {
        self.config.news.fetch_on_startup
    }

    pub fn refresh_news(&mut self) {
        if self.news_loading {
            return;
        }

        let feed_urls = self.config.news.feeds.clone();
        self.news_loading = true;
        self.news_status = Some(self.t(Key::NewsStatusLoading).to_string());

        let (sender, receiver) = mpsc::channel();
        self.news_receiver = Some(receiver);
        thread::spawn(move || {
            let labels = [
                "Top Stories",
                "Real-time Headlines",
                "Breaking News Bulletins",
                "Market Pulse",
            ];
            let owned_feeds = feed_urls
                .iter()
                .enumerate()
                .map(|(index, url)| FeedSource {
                    label: labels.get(index).copied().unwrap_or("MarketWatch"),
                    url: url.as_str(),
                })
                .collect::<Vec<_>>();

            match news::fetch_news(&owned_feeds) {
                Ok(items) => {
                    let _ = sender.send(NewsEvent::Loaded(items));
                }
                Err(error) => {
                    let _ = sender.send(NewsEvent::Error(error));
                }
            }
        });
    }

    pub fn poll_news(&mut self) {
        let Some(receiver) = &self.news_receiver else {
            return;
        };

        match receiver.try_recv() {
            Ok(NewsEvent::Loaded(items)) => {
                self.news_items = items;
                self.news_selection = self
                    .news_selection
                    .min(self.news_items.len().saturating_sub(1));
                self.sync_news_scroll(12);
                self.news_loading = false;
                self.news_receiver = None;
                self.news_status = if self.news_items.is_empty() {
                    Some(self.t(Key::NewsEmpty).to_string())
                } else {
                    None
                };
            }
            Ok(NewsEvent::Error(error)) => {
                self.news_loading = false;
                self.news_receiver = None;
                self.news_status = Some(self.t(Key::NewsStatusError).replace("{error}", &error));
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.news_loading = false;
                self.news_receiver = None;
                self.news_status = Some(self.t(Key::NewsStatusInterrupted).to_string());
            }
        }
    }

    pub fn move_news_selection(&mut self, direction: SelectionDirection) {
        if self.news_items.is_empty() {
            self.news_selection = 0;
            self.news_scroll = 0;
            return;
        }

        self.news_selection = match direction {
            SelectionDirection::Previous => self.news_selection.saturating_sub(1),
            SelectionDirection::Next => {
                (self.news_selection + 1).min(self.news_items.len().saturating_sub(1))
            }
        };
        self.sync_news_scroll(12);
    }

    pub fn open_selected_news(&mut self) {
        self.selected_news = self.news_items.get(self.news_selection).cloned();
    }

    pub fn open_selected_news_in_browser(&mut self) {
        let Some(item) = self.news_items.get(self.news_selection) else {
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
        timestamp
            .map(|value| {
                value
                    .with_timezone(&Local)
                    .format("%b %d %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| self.t(Key::NewsStatusUndated).to_string())
    }

    pub fn watchlist_display_name(&self, symbol: &str) -> Option<&str> {
        self.config
            .watchlist
            .display_names
            .get(symbol)
            .map(String::as_str)
            .filter(|name| !name.trim().is_empty())
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

    pub fn push_reset_confirmation_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }

        if let Some(input) = &mut self.reset_confirmation {
            input.input.push(character);
        }
    }

    pub fn pop_reset_confirmation_char(&mut self) {
        if let Some(input) = &mut self.reset_confirmation {
            input.input.pop();
        }
    }

    pub fn is_editing_watchlist(&self) -> bool {
        self.watchlist_editor.is_some()
    }

    pub fn open_watchlist_editor(&mut self) {
        self.watchlist_editor = Some(WatchlistEditor {
            selection: 0,
            mode: None,
        });
    }

    pub fn close_watchlist_editor(&mut self) {
        if let Some(editor) = &mut self.watchlist_editor {
            if editor.mode.is_some() {
                editor.mode = None;
                return;
            }
        }

        self.watchlist_editor = None;
    }

    pub fn watchlist_rows(&self) -> Vec<WatchlistEditRow> {
        let mut rows = Vec::new();
        rows.push(WatchlistEditRow::AddStock);
        rows.extend((0..self.config.watchlist.stock_symbols.len()).map(WatchlistEditRow::Stock));
        rows.push(WatchlistEditRow::AddCrypto);
        rows.extend((0..self.config.watchlist.crypto_symbols.len()).map(WatchlistEditRow::Crypto));
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
                if index < self.config.watchlist.stock_symbols.len() {
                    self.config.watchlist.stock_symbols.remove(index);
                }
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                if index < self.config.watchlist.crypto_symbols.len() {
                    self.config.watchlist.crypto_symbols.remove(index);
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
        self.refresh_watchlist_suggestions();
    }

    pub fn push_watchlist_input_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }

        match self
            .watchlist_editor
            .as_mut()
            .and_then(|editor| editor.mode.as_mut())
        {
            Some(WatchlistEditMode::Add { input, .. })
            | Some(WatchlistEditMode::EditAlias { input, .. })
            | Some(WatchlistEditMode::ChangeTicker { input, .. }) => input.push(character),
            None => {}
        }
        self.refresh_watchlist_suggestions();
    }

    pub fn pop_watchlist_input_char(&mut self) {
        match self
            .watchlist_editor
            .as_mut()
            .and_then(|editor| editor.mode.as_mut())
        {
            Some(WatchlistEditMode::Add { input, .. })
            | Some(WatchlistEditMode::EditAlias { input, .. })
            | Some(WatchlistEditMode::ChangeTicker { input, .. }) => {
                input.pop();
            }
            None => {}
        }
        self.refresh_watchlist_suggestions();
    }

    fn begin_watchlist_alias_edit(&mut self, kind: WatchlistKind, index: usize) {
        let symbol = match kind {
            WatchlistKind::Stock => self.config.watchlist.stock_symbols.get(index),
            WatchlistKind::Crypto => self.config.watchlist.crypto_symbols.get(index),
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
            WatchlistKind::Stock => self.config.watchlist.stock_symbols.get(index),
            WatchlistKind::Crypto => self.config.watchlist.crypto_symbols.get(index),
        }
        .cloned()
        .unwrap_or_default();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::ChangeTicker { kind, index, input });
        }
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
                    self.config.watchlist.display_names.remove(&symbol);
                } else {
                    self.config
                        .watchlist
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
        }

        self.config.watchlist.stock_symbols.sort();
        self.config.watchlist.stock_symbols.dedup();
        self.config.watchlist.crypto_symbols.sort();
        self.config.watchlist.crypto_symbols.dedup();
        self.cleanup_watchlist_aliases();
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
        self.watchlist_suggestions.clear();
        self.watchlist_suggestion_selection = 0;
        self.request_market_refresh();
        let _ = self.config.save();
    }

    fn watchlist_mut(&mut self, kind: WatchlistKind) -> &mut Vec<String> {
        match kind {
            WatchlistKind::Stock => &mut self.config.watchlist.stock_symbols,
            WatchlistKind::Crypto => &mut self.config.watchlist.crypto_symbols,
        }
    }

    fn retain_configured_quotes(&mut self) {
        self.stock_quotes
            .retain(|quote| self.config.watchlist.stock_symbols.contains(&quote.symbol));
        self.crypto_quotes
            .retain(|quote| self.config.watchlist.crypto_symbols.contains(&quote.symbol));
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
            WatchlistEditMode::EditAlias { .. } => {
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

        if let Some(alias) = self.config.watchlist.display_names.remove(old_symbol) {
            self.config
                .watchlist
                .display_names
                .entry(new_symbol.to_string())
                .or_insert(alias);
        }
    }

    fn cleanup_watchlist_aliases(&mut self) {
        let stock_symbols = &self.config.watchlist.stock_symbols;
        let crypto_symbols = &self.config.watchlist.crypto_symbols;
        self.config
            .watchlist
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
        self.llm_config = config.llm.clone();
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
        self.news_receiver = None;
        self.agent_input.clear();
        self.agent_messages.clear();
        self.agent_loading = false;
        self.agent_status = None;
        self.agent_response_receiver = None;
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

impl SettingsItem {
    pub const ALL: [Self; 4] = [Self::Language, Self::Theme, Self::Onboarding, Self::Reset];
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
