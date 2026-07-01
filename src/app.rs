use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

use serde::{Deserialize, Serialize};

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
    Settings,
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
    Rename {
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
        } else if self.page == Page::Settings {
            if self.reset_confirmation.is_some() {
                self.reset_confirmation = None;
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
        self.page = Page::Search;
        self.show_help = false;
        self.pending_split = false;
        self.refresh_search();
    }

    pub fn open_settings(&mut self) {
        self.page = Page::Settings;
        self.show_help = false;
        self.pending_split = false;
        self.reset_confirmation = None;
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
                self.begin_watchlist_rename(WatchlistKind::Stock, index)
            }
            Some(WatchlistEditRow::Crypto(index)) => {
                self.begin_watchlist_rename(WatchlistKind::Crypto, index)
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

        self.clamp_watchlist_selection();
        self.retain_configured_quotes();
        let _ = self.config.save();
    }

    pub fn begin_watchlist_add(&mut self, kind: WatchlistKind) {
        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::Add {
                kind,
                input: String::new(),
            });
        }
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
            | Some(WatchlistEditMode::Rename { input, .. }) => input.push(character),
            None => {}
        }
    }

    pub fn pop_watchlist_input_char(&mut self) {
        match self
            .watchlist_editor
            .as_mut()
            .and_then(|editor| editor.mode.as_mut())
        {
            Some(WatchlistEditMode::Add { input, .. })
            | Some(WatchlistEditMode::Rename { input, .. }) => {
                input.pop();
            }
            None => {}
        }
    }

    fn begin_watchlist_rename(&mut self, kind: WatchlistKind, index: usize) {
        let input = match kind {
            WatchlistKind::Stock => self.config.watchlist.stock_symbols.get(index),
            WatchlistKind::Crypto => self.config.watchlist.crypto_symbols.get(index),
        }
        .cloned()
        .unwrap_or_default();

        if let Some(editor) = &mut self.watchlist_editor {
            editor.mode = Some(WatchlistEditMode::Rename { kind, index, input });
        }
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
                if let Some(symbol) = normalize_symbol(&input) {
                    let list = self.watchlist_mut(kind);
                    if !list.contains(&symbol) {
                        list.push(symbol);
                    }
                }
            }
            WatchlistEditMode::Rename { kind, index, input } => {
                if let Some(symbol) = normalize_symbol(&input) {
                    let list = self.watchlist_mut(kind);
                    if index < list.len() {
                        list[index] = symbol;
                    }
                }
            }
        }

        self.config.watchlist.stock_symbols.sort();
        self.config.watchlist.stock_symbols.dedup();
        self.config.watchlist.crypto_symbols.sort();
        self.config.watchlist.crypto_symbols.dedup();
        self.retain_configured_quotes();
        self.clamp_watchlist_selection();
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
