use super::*;

impl App {
    pub fn open_spotlight(&mut self) {
        self.spotlight.open = true;
        self.spotlight.query.clear();
        self.spotlight.selection = 0;
        crate::spotlight::refresh(self);
    }
    pub fn close_spotlight(&mut self) {
        self.spotlight.open = false;
        self.spotlight.query.clear();
        self.spotlight.selection = 0;
        self.spotlight.results.clear();
    }
    pub fn spotlight_push_char(&mut self, character: char) {
        if character.is_control() {
            return;
        }
        self.spotlight.query.push(character);
        crate::spotlight::refresh(self);
    }
    pub fn spotlight_pop_char(&mut self) {
        self.spotlight.query.pop();
        crate::spotlight::refresh(self);
    }
    pub fn spotlight_move_selection(&mut self, direction: SelectionDirection) {
        let count = self.spotlight.results.len();
        if count == 0 {
            return;
        }
        self.spotlight.selection = match direction {
            SelectionDirection::Previous => (self.spotlight.selection + count - 1) % count,
            SelectionDirection::Next => (self.spotlight.selection + 1) % count,
        };
    }
    pub fn execute_spotlight_selection(&mut self) {
        let Some(result) = self
            .spotlight
            .results
            .get(self.spotlight.selection)
            .cloned()
        else {
            self.close_spotlight();
            return;
        };
        self.close_spotlight();

        match result.entry {
            crate::spotlight::SpotlightEntry::Symbol(symbol) => {
                let _ = self.agent_open_symbol(&symbol);
            }
            crate::spotlight::SpotlightEntry::Panel(panel) => panel.apply(self),
            crate::spotlight::SpotlightEntry::Action(index) => {
                if let Some(action) = crate::spotlight::actions().get(index) {
                    (action.run)(self);
                }
            }
        }
    }
    /// Focuses (re-opening if closed) the given panel slot and switches it
    /// to the requested content, as used by Spotlight panel jumps.
    pub fn spotlight_focus_panel(&mut self, panel_id: PanelId, window_kind: WindowKind) {
        self.page = Page::Dashboard;
        self.open_panel(panel_id);
        self.set_panel_content(panel_id, window_kind);
    }
}
