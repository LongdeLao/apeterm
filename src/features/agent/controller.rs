//! Owns the agent conversation, panel state, and tool-call orchestration.
//!
//! Flow per user turn:
//!   submit(prompt, context)
//!     -> background request against the LLM client
//!   poll()
//!     -> Message: shown, turn ends
//!     -> ToolCall: returned to the app, which executes it and calls
//!        push_tool_result(), which feeds the result back to the model
//!        and loops (bounded by MAX_TOOL_ROUNDS).

use std::{
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
};

use crate::{
    config::LlmConfig,
    features::agent::{
        llm_client::{LlmClient, LlmError},
        messages::{AgentMessage, AgentRole, Badge, LlmMessage, LlmRequest, LlmResponse, LlmRole},
        openrouter::OpenRouterClient,
        prompts,
        tool_call::{AssistantAction, ToolCall, ToolResult, parse_assistant_action},
        tools,
    },
    preferences::UserPreferences,
};

/// Upper bound on consecutive tool rounds within one user turn, so a
/// misbehaving model cannot loop forever.
const MAX_TOOL_ROUNDS: u8 = 4;

#[derive(Debug)]
pub struct AgentController {
    pub panel_open: bool,
    pub input: String,
    pub messages: Vec<AgentMessage>,
    pub busy: bool,
    pub status: Option<String>,
    pub scroll: u16,
    pub auto_scroll: bool,
    client: Option<Arc<dyn LlmClient>>,
    client_error: Option<String>,
    model_label: Option<String>,
    history: Vec<LlmMessage>,
    receiver: Option<Receiver<Result<LlmResponse, LlmError>>>,
    tool_rounds: u8,
    /// Label shown next to the spinner while busy (the model's tool "note",
    /// or a generic fallback).
    work_label: String,
    /// Whether any tool this turn succeeded (`Some(true)`) or failed
    /// (`Some(false)`); folded into the final assistant message as a badge.
    turn_status: Option<bool>,
}

impl std::fmt::Debug for dyn LlmClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "LlmClient({})", self.label())
    }
}

impl AgentController {
    pub fn new(config: &LlmConfig) -> Self {
        let (client, client_error, model_label) = match OpenRouterClient::from_config(config) {
            Ok(client) => {
                let label = client.label();
                (
                    Some(Arc::new(client) as Arc<dyn LlmClient>),
                    None,
                    Some(label),
                )
            }
            Err(error) => (None, Some(error.to_string()), None),
        };

        Self {
            panel_open: false,
            input: String::new(),
            messages: Vec::new(),
            busy: false,
            status: client_error.clone(),
            scroll: 0,
            auto_scroll: true,
            client,
            client_error,
            model_label,
            history: Vec::new(),
            receiver: None,
            tool_rounds: 0,
            work_label: "thinking".to_string(),
            turn_status: None,
        }
    }

    pub fn model_label(&self) -> Option<&str> {
        self.model_label.as_deref()
    }

    /// Sends the pending input, refreshing the system prompt with the given
    /// app context. No-op while a request is in flight or input is empty.
    pub fn submit(&mut self, context: &str, prefs: &UserPreferences, loading_label: String) {
        if self.busy {
            return;
        }
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            return;
        }

        self.input.clear();
        self.messages.push(AgentMessage {
            role: AgentRole::User,
            content: prompt.clone(),
            badge: None,
        });
        self.auto_scroll = true;
        self.work_label = loading_label;
        self.turn_status = None;

        if self.client.is_none() {
            self.status = Some(
                self.client_error
                    .clone()
                    .unwrap_or_else(|| "no LLM provider configured".to_string()),
            );
            return;
        }

        let system = prompts::build_system_prompt(prefs)
            .replace("{tools}", tools::catalog())
            .replace("{context}", context);
        match self.history.first_mut() {
            Some(message) if message.role == LlmRole::System => message.content = system,
            _ => self
                .history
                .insert(0, LlmMessage::new(LlmRole::System, system)),
        }
        self.history.push(LlmMessage::new(LlmRole::User, prompt));

        self.tool_rounds = 0;
        self.spawn_request();
    }

    /// Drains the in-flight request. Returns a tool call when the model
    /// requested one; the caller executes it and passes the result to
    /// [`Self::push_tool_result`].
    pub fn poll(&mut self) -> Option<ToolCall> {
        let receiver = self.receiver.as_ref()?;
        let outcome = match receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                self.finish_turn(Some("request interrupted".to_string()));
                return None;
            }
        };
        self.receiver = None;

        let response = match outcome {
            Ok(response) => response,
            Err(error) => {
                self.finish_turn(Some(error.to_string()));
                return None;
            }
        };

        match parse_assistant_action(&response.content) {
            AssistantAction::Message(text) => {
                self.history
                    .push(LlmMessage::new(LlmRole::Assistant, response.content));
                let badge = self
                    .turn_status
                    .map(|ok| if ok { Badge::Ok } else { Badge::Failed });
                self.push_message(AgentRole::Assistant, text, badge);
                self.finish_turn(None);
                None
            }
            AssistantAction::ToolCall { call, note } => {
                self.history
                    .push(LlmMessage::new(LlmRole::Assistant, response.content));
                // The note is shown transiently beside the spinner, not as its
                // own transcript bubble.
                if let Some(note) = note {
                    self.work_label = note;
                }
                Some(call)
            }
            AssistantAction::Invalid { reason } => {
                self.history
                    .push(LlmMessage::new(LlmRole::Assistant, response.content));
                self.push_tool_result(ToolResult::failure("unknown", reason));
                None
            }
        }
    }

    /// Feeds a tool result back to the model and continues the turn, unless
    /// the tool-round budget is exhausted. The result is not shown directly;
    /// its success is folded into the next assistant message's badge.
    pub fn push_tool_result(&mut self, result: ToolResult) {
        if result.success {
            // A later failure downgrades the turn; don't upgrade back.
            if self.turn_status != Some(false) {
                self.turn_status = Some(true);
            }
        } else {
            self.turn_status = Some(false);
        }
        self.history
            .push(LlmMessage::new(LlmRole::Tool, result.to_json()));

        self.tool_rounds += 1;
        if self.tool_rounds >= MAX_TOOL_ROUNDS {
            self.finish_turn(Some("stopped: tool-call limit reached".to_string()));
            return;
        }
        self.spawn_request();
    }

    fn spawn_request(&mut self) {
        let Some(client) = self.client.clone() else {
            self.finish_turn(Some("no LLM provider configured".to_string()));
            return;
        };

        self.busy = true;
        // Rendered as a spinner label while busy.
        self.status = Some(self.work_label.clone());
        let request = LlmRequest {
            messages: self.history.clone(),
        };

        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        thread::spawn(move || {
            let outcome = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| LlmError::Request(error.to_string()))
                .and_then(|runtime| runtime.block_on(client.complete(request)));
            let _ = sender.send(outcome);
        });
    }

    fn finish_turn(&mut self, status: Option<String>) {
        self.busy = false;
        self.status = status;
    }

    fn push_message(&mut self, role: AgentRole, content: String, badge: Option<Badge>) {
        self.messages.push(AgentMessage {
            role,
            content,
            badge,
        });
        if self.auto_scroll {
            self.scroll = u16::MAX;
        }
    }

    pub fn scroll_by(&mut self, delta: i32) {
        self.auto_scroll = false;
        self.scroll = self.scroll.saturating_add_signed(delta as i16);
    }

    pub fn stick_scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
        self.scroll = u16::MAX;
    }

    /// Conversation-starter chips shown in the empty state.
    pub fn suggestions() -> &'static [&'static str] {
        &[
            "Add or remove from watchlist",
            "Create a new watchlist",
            "Open a stock's details",
            "What's on my watchlists?",
        ]
    }
}
