//! Provider abstraction. The rest of the app only ever talks to
//! `dyn LlmClient`; swapping OpenRouter for Groq (or anything else) means
//! adding one module that implements this trait.

use std::fmt;

use async_trait::async_trait;

use crate::agent::messages::{LlmRequest, LlmResponse};

#[derive(Debug, Clone)]
pub enum LlmError {
    MissingApiKey,
    Request(String),
    Provider(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::MissingApiKey => {
                write!(
                    formatter,
                    "missing API key: set OPENROUTER_API_KEY or LLM_API_KEY"
                )
            }
            LlmError::Request(message) => write!(formatter, "request failed: {message}"),
            LlmError::Provider(message) => write!(formatter, "provider error: {message}"),
        }
    }
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Short label shown in the panel header, e.g. the model name.
    fn label(&self) -> String;
}
