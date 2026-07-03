//! Provider-neutral message types shared by every LLM client, plus the
//! transcript types rendered in the agent panel.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    /// Result of a tool the app executed on behalf of the assistant.
    Tool,
}

#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

impl LlmMessage {
    pub fn new(role: LlmRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
}

/// Role of an entry in the user-facing transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: AgentRole,
    pub content: String,
}
