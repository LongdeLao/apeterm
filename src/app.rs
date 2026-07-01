use crate::market::MarketEvent;
use crate::quotes::{CryptoQuote, update_crypto_quotes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Onboarding,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    Language,
    Theme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    German,
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

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub page: Page,
    pub onboarding_step: OnboardingStep,
    pub onboarding_complete: bool,
    pub logged_in: bool,
    pub language: Language,
    pub theme_name: ThemeName,
    pub dashboard_layout: DashboardLayout,
    pub focused_panel: PanelId,
    pub closed_panels: Vec<PanelId>,
    pub show_help: bool,
    pub pending_split: bool,
    pub panel_contents: PanelContents,
    pub window_picker_index: usize,
    pub crypto_quotes: Vec<CryptoQuote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardLayout {
    pub top_left_width_percent: u16,
    pub bottom_left_width_percent: u16,
    pub top_height_percent: u16,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            page: Page::Onboarding,
            onboarding_step: OnboardingStep::Welcome,
            onboarding_complete: false,
            logged_in: false,
            language: Language::English,
            theme_name: ThemeName::Dark,
            dashboard_layout: DashboardLayout::default(),
            focused_panel: PanelId::News,
            closed_panels: Vec::new(),
            show_help: false,
            pending_split: false,
            panel_contents: PanelContents::default(),
            window_picker_index: 0,
            crypto_quotes: Vec::new(),
        }
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
            OnboardingStep::Language => self.language = self.language.move_to(direction),
            OnboardingStep::Theme => self.theme_name = self.theme_name.move_to(direction),
        }
    }

    pub fn handle_market_event(&mut self, event: MarketEvent) {
        update_crypto_quotes(&mut self.crypto_quotes, event);
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

    pub fn close_help(&mut self) {
        self.show_help = false;
        self.pending_split = false;
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

    pub fn label(self) -> &'static str {
        match self {
            Self::News => "news",
            Self::Watchlist => "watchlist",
            Self::Calendar => "macro calendar",
            Self::Notes => "notes",
            Self::Picker => "select window",
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

impl Language {
    fn move_to(self, direction: SelectionDirection) -> Self {
        match self {
            Self::English => match direction {
                SelectionDirection::Previous => Self::German,
                SelectionDirection::Next => Self::German,
            },
            Self::German => match direction {
                SelectionDirection::Previous => Self::English,
                SelectionDirection::Next => Self::English,
            },
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
        }
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Transparent => "Transparent",
        }
    }
}
