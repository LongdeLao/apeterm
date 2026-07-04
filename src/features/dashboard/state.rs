use crate::app::*;
use crate::i18n::Key;

/// Layout + panel state owned by the dashboard.
#[derive(Debug)]
pub struct DashboardFeature {
    pub layout: DashboardLayout,
    pub focused_panel: PanelId,
    pub closed_panels: Vec<PanelId>,
    pub panel_contents: PanelContents,
    pub window_picker_index: usize,
    pub pending_split: bool,
}

impl Default for DashboardFeature {
    fn default() -> Self {
        Self {
            layout: DashboardLayout::default(),
            focused_panel: PanelId::News,
            closed_panels: Vec::new(),
            panel_contents: PanelContents::default(),
            window_picker_index: 0,
            pending_split: false,
        }
    }
}

impl App {
    pub fn focus_panel(&mut self, panel_id: PanelId) {
        if self.is_panel_open(panel_id) {
            self.dashboard.focused_panel = panel_id;
        }
    }
    pub fn focus_next_panel(&mut self) {
        self.focus_by_offset(1);
    }
    pub fn focus_previous_panel(&mut self) {
        self.focus_by_offset(PanelId::ALL.len() - 1);
    }
    pub fn focus_panel_in_direction(&mut self, direction: MoveDirection) {
        let next = match (self.dashboard.focused_panel, direction) {
            (PanelId::News, MoveDirection::Right) => PanelId::Watchlist,
            (PanelId::News, MoveDirection::Down) => PanelId::Calendar,
            (PanelId::Watchlist, MoveDirection::Left) => PanelId::News,
            (PanelId::Watchlist, MoveDirection::Down) => PanelId::Notes,
            (PanelId::Calendar, MoveDirection::Up) => PanelId::News,
            (PanelId::Calendar, MoveDirection::Right) => PanelId::Notes,
            (PanelId::Notes, MoveDirection::Up) => PanelId::Watchlist,
            (PanelId::Notes, MoveDirection::Left) => PanelId::Calendar,
            _ => self.dashboard.focused_panel,
        };

        self.focus_panel(next);
    }
    pub fn close_focused_panel(&mut self) {
        if self.open_panel_count() <= 1 || !self.is_panel_open(self.dashboard.focused_panel) {
            return;
        }

        self.dashboard.closed_panels.push(self.dashboard.focused_panel);
        self.dashboard.pending_split = false;
        self.focus_next_panel();
    }
    pub fn reset_dashboard(&mut self) {
        self.dashboard.layout = DashboardLayout::default();
        self.dashboard.closed_panels.clear();
        self.dashboard.focused_panel = PanelId::News;
        self.show_help = false;
        self.dashboard.pending_split = false;
        self.dashboard.panel_contents = PanelContents::default();
        self.dashboard.window_picker_index = 0;
    }
    pub fn is_panel_open(&self, panel_id: PanelId) -> bool {
        !self.dashboard.closed_panels.contains(&panel_id)
    }
    pub fn is_panel_focused(&self, panel_id: PanelId) -> bool {
        self.dashboard.focused_panel == panel_id && self.is_panel_open(panel_id)
    }
    pub fn resize_dashboard(&mut self, direction: MoveDirection) {
        if self.page != Page::Dashboard {
            return;
        }

        match (self.dashboard.focused_panel, direction) {
            (PanelId::News | PanelId::Watchlist, MoveDirection::Left) => {
                self.dashboard.layout.resize_top_left_width(-5)
            }
            (PanelId::News | PanelId::Watchlist, MoveDirection::Right) => {
                self.dashboard.layout.resize_top_left_width(5)
            }
            (PanelId::Calendar | PanelId::Notes, MoveDirection::Left) => {
                self.dashboard.layout.resize_bottom_left_width(-5)
            }
            (PanelId::Calendar | PanelId::Notes, MoveDirection::Right) => {
                self.dashboard.layout.resize_bottom_left_width(5)
            }
            (PanelId::News | PanelId::Calendar, MoveDirection::Up) => {
                self.dashboard.layout.resize_top_height(-5)
            }
            (PanelId::News | PanelId::Calendar, MoveDirection::Down) => {
                self.dashboard.layout.resize_top_height(5)
            }
            (PanelId::Watchlist | PanelId::Notes, MoveDirection::Up) => {
                self.dashboard.layout.resize_top_height(-5)
            }
            (PanelId::Watchlist | PanelId::Notes, MoveDirection::Down) => {
                self.dashboard.layout.resize_top_height(5)
            }
        }
    }
    pub fn begin_split_command(&mut self) {
        if self.page == Page::Dashboard {
            self.dashboard.pending_split = true;
        }
    }
    pub fn split_focused_panel(&mut self, direction: SplitDirection) {
        if self.page != Page::Dashboard {
            return;
        }

        self.dashboard.pending_split = false;

        let panel_id = match (self.dashboard.focused_panel, direction) {
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
        self.dashboard.window_picker_index = 0;
    }
    pub fn add_panel(&mut self) {
        self.dashboard.pending_split = false;

        let Some(panel_id) = self.dashboard.closed_panels.pop() else {
            return;
        };

        self.dashboard.focused_panel = panel_id;
        self.set_panel_content(panel_id, WindowKind::Picker);
        self.dashboard.window_picker_index = 0;
    }
    pub fn change_focused_panel_content(&mut self) {
        if self.page != Page::Dashboard || !self.is_panel_open(self.dashboard.focused_panel) {
            return;
        }

        let current = self.panel_content(self.dashboard.focused_panel);
        self.dashboard.window_picker_index = WindowKind::CHOICES
            .iter()
            .position(|window_kind| *window_kind == current)
            .unwrap_or(0);
        self.set_panel_content(self.dashboard.focused_panel, WindowKind::Picker);
    }
    pub fn cancel_pending_command(&mut self) {
        self.dashboard.pending_split = false;
    }
    pub fn panel_content(&self, panel_id: PanelId) -> WindowKind {
        self.dashboard.panel_contents.get(panel_id)
    }
    pub fn is_choosing_window(&self) -> bool {
        self.panel_content(self.dashboard.focused_panel) == WindowKind::Picker
    }
    pub fn move_window_picker(&mut self, direction: SelectionDirection) {
        let choices = WindowKind::CHOICES.len();
        self.dashboard.window_picker_index = match direction {
            SelectionDirection::Previous => {
                if self.dashboard.window_picker_index == 0 {
                    choices - 1
                } else {
                    self.dashboard.window_picker_index - 1
                }
            }
            SelectionDirection::Next => (self.dashboard.window_picker_index + 1) % choices,
        };
    }
    pub fn confirm_window_picker(&mut self) {
        if !self.is_choosing_window() {
            return;
        }

        let window_kind = WindowKind::CHOICES[self.dashboard.window_picker_index];
        self.set_panel_content(self.dashboard.focused_panel, window_kind);
    }
    pub(crate) fn focus_by_offset(&mut self, offset: usize) {
        let current_index = PanelId::ALL
            .iter()
            .position(|panel_id| *panel_id == self.dashboard.focused_panel)
            .unwrap_or(0);

        for step in 1..=PanelId::ALL.len() {
            let index = (current_index + offset * step) % PanelId::ALL.len();
            let panel_id = PanelId::ALL[index];
            if self.is_panel_open(panel_id) {
                self.dashboard.focused_panel = panel_id;
                break;
            }
        }
    }
    pub(crate) fn open_panel_count(&self) -> usize {
        PanelId::ALL
            .iter()
            .filter(|panel_id| self.is_panel_open(**panel_id))
            .count()
    }
    pub(crate) fn open_panel(&mut self, panel_id: PanelId) {
        self.dashboard.closed_panels
            .retain(|closed_panel_id| *closed_panel_id != panel_id);
        self.dashboard.focused_panel = panel_id;
    }
    pub(crate) fn set_panel_content(&mut self, panel_id: PanelId, window_kind: WindowKind) {
        self.dashboard.panel_contents.set(panel_id, window_kind);
    }
}

impl PanelId {
    pub const ALL: [Self; 4] = [Self::News, Self::Watchlist, Self::Calendar, Self::Notes];
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
    pub(crate) fn get(self, panel_id: PanelId) -> WindowKind {
        match panel_id {
            PanelId::News => self.news,
            PanelId::Watchlist => self.watchlist,
            PanelId::Calendar => self.calendar,
            PanelId::Notes => self.notes,
        }
    }

    pub(crate) fn set(&mut self, panel_id: PanelId, window_kind: WindowKind) {
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

    pub(crate) fn resize_top_left_width(&mut self, amount: i16) {
        self.top_left_width_percent = adjust_percent(self.top_left_width_percent, amount);
    }

    pub(crate) fn resize_bottom_left_width(&mut self, amount: i16) {
        self.bottom_left_width_percent = adjust_percent(self.bottom_left_width_percent, amount);
    }

    pub(crate) fn resize_top_height(&mut self, amount: i16) {
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
