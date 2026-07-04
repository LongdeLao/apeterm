//! High-level app state and feature coordination.
//!
//! `App` owns all runtime state. Per-feature state and behavior live in the
//! `features/*/state.rs` modules as focused `impl App` blocks; rendering belongs
//! in `features/*/view.rs`, input routing in `event.rs`. `plugins::registry` maps each
//! feature area to its modules.

use std::{path::PathBuf, sync::mpsc::Receiver};

use serde::{Deserialize, Serialize};

use crate::features::agent::AgentController;
use crate::{
    backend::BackendInsight,
    config::AppConfig,
    features::news::feed::FetchResult,
    features::search::engine::{InstrumentDetails, LiveInstrumentDetails, SearchResult},
    features::watchlist::market::{MarketEvent, MarketSession},
    features::watchlist::quotes::{Quote, update_market_quotes},
    i18n::{I18n, Key, Locale},
    preferences::{AgentStyle, Experience, ExplanationLevel, Language, Tone, UserPreferences},
};

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
pub enum DetailTimeframe {
    OneDay,
    OneWeek,
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    FiveYears,
    Max,
}

impl DetailTimeframe {
    pub const ALL: [Self; 8] = [
        Self::OneDay,
        Self::OneWeek,
        Self::OneMonth,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::OneYear,
        Self::FiveYears,
        Self::Max,
    ];

    pub const fn day_window(self) -> Option<i64> {
        match self {
            Self::OneDay => Some(1),
            Self::OneWeek => Some(7),
            Self::OneMonth => Some(31),
            Self::ThreeMonths => Some(93),
            Self::SixMonths => Some(186),
            Self::OneYear => Some(366),
            Self::FiveYears => Some(1_826),
            Self::Max => None,
        }
    }

    pub const fn label_key(self) -> Key {
        match self {
            Self::OneDay => Key::DetailsTimeframeOneDay,
            Self::OneWeek => Key::DetailsTimeframeOneWeek,
            Self::OneMonth => Key::DetailsTimeframeOneMonth,
            Self::ThreeMonths => Key::DetailsTimeframeThreeMonths,
            Self::SixMonths => Key::DetailsTimeframeSixMonths,
            Self::OneYear => Key::DetailsTimeframeOneYear,
            Self::FiveYears => Key::DetailsTimeframeFiveYears,
            Self::Max => Key::DetailsTimeframeMax,
        }
    }
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
    ApePreset,
    ProPreset,
    CustomPreset,
    Experience,
    Tone,
    Explanations,
    AgentStyle,
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
pub(crate) enum NewsEvent {
    Loaded { result: FetchResult, done: bool },
    Error(String),
}

#[derive(Debug)]
pub(crate) enum SecEvent {
    Done(String),
    Error(String),
}

#[derive(Debug)]
pub(crate) enum BackendInsightEvent {
    Loaded {
        symbol: String,
        insight: Option<BackendInsight>,
    },
    Error {
        symbol: String,
        message: String,
    },
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
    pub preferences: UserPreferences,
    pub i18n: I18n,
    pub theme_name: ThemeName,
    pub dashboard_layout: DashboardLayout,
    pub focused_panel: PanelId,
    pub closed_panels: Vec<PanelId>,
    /// Page to return to when backing out of Search or Settings, so opening
    /// either one from Details/Search/Settings doesn't strand the user on
    /// Dashboard when they press Esc.
    pub return_page: Option<Page>,
    pub show_help: bool,
    pub pending_split: bool,
    pub panel_contents: PanelContents,
    pub window_picker_index: usize,
    pub watchlist: crate::features::watchlist::state::WatchlistFeature,
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
    pub(crate) live_details_receiver: Option<Receiver<Option<LiveInstrumentDetails>>>,
    pub detail_timeframe: DetailTimeframe,
    pub detail_sidebar_scroll: usize,
    pub detail_metric_selection: usize,
    pub detail_description_expanded: bool,
    pub detail_context_expanded: bool,
    pub backend_insight: Option<BackendInsight>,
    pub backend_insight_loading: bool,
    pub backend_insight_status: Option<String>,
    pub(crate) backend_insight_receiver: Option<Receiver<BackendInsightEvent>>,
    pub search_message: Option<String>,
    pub settings: crate::features::settings::state::SettingsFeature,
    pub news: crate::features::news::state::NewsFeature,
    pub notes: crate::features::notes::state::NotesFeature,
    pub sec: crate::features::sec::state::SecFeature,
    pub agent: AgentController,
    pub spotlight: crate::features::spotlight::engine::SpotlightState,
    pub(crate) config: AppConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardLayout {
    pub top_left_width_percent: u16,
    pub bottom_left_width_percent: u16,
    pub top_height_percent: u16,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let preferences = config.preferences;
        let locale = preferences.language.locale();
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
            preferences,
            i18n: I18n::new(locale),
            theme_name: config.theme,
            dashboard_layout: DashboardLayout::default(),
            focused_panel: PanelId::News,
            closed_panels: Vec::new(),
            return_page: None,
            show_help: false,
            pending_split: false,
            panel_contents: PanelContents::default(),
            window_picker_index: 0,
            watchlist: Default::default(),
            ticker_db_path: config.ticker_db_path.clone(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selection: 0,
            search_scroll: 0,
            search_limit: crate::features::search::state::SEARCH_PAGE_SIZE,
            search_asset_kind: SearchAssetKind::Stocks,
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
            search_message: None,
            settings: Default::default(),
            news: Default::default(),
            notes: Default::default(),
            sec: Default::default(),
            agent: AgentController::new(&config.llm),
            spotlight: crate::features::spotlight::engine::SpotlightState::default(),
            config,
        }
    }
    pub fn t(&self, key: Key) -> &str {
        self.i18n.t_with_tone(key, self.preferences.tone)
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
            &mut self.watchlist.crypto_quotes,
            &mut self.watchlist.stock_quotes,
            &mut self.watchlist.stock_market_session,
            event,
        );
    }
    pub fn take_market_refresh_request(&mut self) -> bool {
        let requested = self.watchlist.market_refresh_requested;
        self.watchlist.market_refresh_requested = false;
        requested
    }
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.pending_split = false;
    }
    pub fn toggle_locale(&mut self) {
        self.set_language(self.preferences.language.next());
    }
    pub fn cycle_experience(&mut self) {
        self.set_experience(self.preferences.experience.next());
    }
    pub fn close_help(&mut self) {
        if self.news.selected.is_some() {
            self.news.selected = None;
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
            self.detail_sidebar_scroll = 0;
            self.detail_description_expanded = false;
            self.detail_context_expanded = false;
            self.backend_insight = None;
            self.backend_insight_loading = false;
            self.backend_insight_status = None;
            self.backend_insight_receiver = None;
        } else if self.page == Page::Search {
            self.mode = AppMode::Normal;
            self.page = self.return_page.take().unwrap_or(Page::Dashboard);
        } else if self.page == Page::Settings {
            if self.settings.reset_confirmation.is_some() {
                self.settings.reset_confirmation = None;
                self.clear_text_input_mode();
            } else {
                self.page = self.return_page.take().unwrap_or(Page::Dashboard);
            }
        } else if self.is_editing_watchlist() {
            self.close_watchlist_editor();
        }
    }
    pub fn cycle_theme(&mut self) {
        self.set_theme(self.theme_name.next());
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
                self.settings.reset_confirmation = None;
                self.clear_text_input_mode();
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                if let Some(editor) = &mut self.watchlist.editor {
                    editor.mode = None;
                }
                self.watchlist.suggestions.clear();
                self.watchlist.suggestion_selection = 0;
                self.clear_text_input_mode();
            }
            // Notes editing is fully handled in event.rs before this dispatcher runs.
            AppMode::TextInput(InputTarget::Notes) => {}
            AppMode::TextInput(InputTarget::NotesSearch) => {
                self.notes.search_query.clear();
                self.notes.ticker_filter = None;
                self.notes.selection = 0;
                self.notes.scroll = 0;
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
                    .settings.reset_confirmation
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
                if let Some(input) = &mut self.settings.reset_confirmation {
                    input.input.push(character);
                }
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                match self
                    .watchlist.editor
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
                self.notes.search_query.push(character);
                self.notes.ticker_filter = None;
                self.notes.selection = 0;
                self.notes.scroll = 0;
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
                if let Some(input) = &mut self.settings.reset_confirmation {
                    input.input.pop();
                }
            }
            AppMode::TextInput(InputTarget::Watchlist) => {
                match self
                    .watchlist.editor
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
                self.notes.search_query.pop();
                self.notes.ticker_filter = None;
                self.notes.selection = 0;
                self.notes.scroll = 0;
            }
            AppMode::Normal => {}
        }
    }
    pub(crate) fn set_locale(&mut self, locale: Locale) {
        self.set_language(Language::from_locale(&locale));
    }
    pub(crate) fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        self.apply_preferences();
    }
    pub(crate) fn set_experience(&mut self, experience: Experience) {
        self.preferences.experience = experience;
        self.detail_metric_selection = 0;
        self.apply_preferences();
    }
    pub(crate) fn set_tone(&mut self, tone: Tone) {
        self.preferences.tone = tone;
        self.apply_preferences();
    }
    pub(crate) fn set_explanations(&mut self, explanations: ExplanationLevel) {
        self.preferences.explanations = explanations;
        self.apply_preferences();
    }
    pub(crate) fn set_agent_style(&mut self, agent_style: AgentStyle) {
        self.preferences.agent_style = agent_style;
        self.apply_preferences();
    }
    pub(crate) fn set_preferences(&mut self, preferences: UserPreferences) {
        self.preferences = preferences;
        self.detail_metric_selection = 0;
        self.apply_preferences();
    }
    pub(crate) fn apply_preferences(&mut self) {
        let locale = self.preferences.language.locale();
        self.locale = locale.clone();
        self.i18n.set_active(locale.clone());
        self.config.locale = locale;
        self.config.preferences = self.preferences;
        let _ = self.config.save();
    }
    pub fn set_theme(&mut self, theme_name: ThemeName) {
        self.theme_name = theme_name;
        self.config.theme = theme_name;
        let _ = self.config.save();
    }
}

impl ThemeName {
    pub(crate) fn move_to(self, direction: SelectionDirection) -> Self {
        match direction {
            SelectionDirection::Previous => self.previous(),
            SelectionDirection::Next => self.next(),
        }
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::Dark => Self::Bloomberg,
            Self::Light => Self::Dark,
            Self::Transparent => Self::Light,
            Self::Bloomberg => Self::Transparent,
        }
    }

    pub(crate) fn next(self) -> Self {
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
