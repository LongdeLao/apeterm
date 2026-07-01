use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingChatCompletionResponse {
    pub choices: Vec<StreamingChatChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingChatChoice {
    pub delta: StreamingChatDelta,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamingChatDelta {
    pub content: Option<String>,
}
