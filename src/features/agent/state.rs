use crate::app::*;
use crate::i18n::Key;

impl App {
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
    pub fn send_agent_message(&mut self) {
        let context = crate::features::agent::context::build_context(self);
        let loading_label = self.t(Key::AgentStatusLoading).to_string();
        self.agent
            .submit(&context, &self.preferences, loading_label);
    }
    /// Drives the agent turn: when the model requested a tool, execute it
    /// against the app and feed the result back into the conversation.
    pub fn poll_agent_response(&mut self) {
        while let Some(call) = self.agent.poll() {
            let result = crate::features::agent::tools::execute(self, call);
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
}
