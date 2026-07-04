//! OpenRouter implementation of [`LlmClient`]. All OpenRouter-specific wire
//! formatting lives here and nowhere else.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    config::LlmConfig,
    features::agent::{
        llm_client::{LlmClient, LlmError},
        messages::{LlmRequest, LlmResponse, LlmRole},
    },
};

pub struct OpenRouterClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    pub fn from_config(config: &LlmConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or(LlmError::MissingApiKey)?
            .to_string();

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| LlmError::Request(error.to_string()))?;

        Ok(Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            model: config.model.clone(),
        })
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let wire_request = WireRequest {
            model: self.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|message| WireMessage {
                    role: wire_role(message.role),
                    content: message.content.clone(),
                })
                .collect(),
            stream: false,
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .header("X-Title", "apeterm")
            .json(&wire_request)
            .send()
            .await
            .map_err(|error| LlmError::Request(error.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| LlmError::Request(error.to_string()))?;

        if !status.is_success() {
            return Err(LlmError::Provider(compact_provider_error(&status, &body)));
        }

        let parsed: WireResponse = serde_json::from_str(&body)
            .map_err(|error| LlmError::Provider(format!("unexpected response: {error}")))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .unwrap_or_default();

        Ok(LlmResponse { content })
    }

    fn label(&self) -> String {
        self.model.clone()
    }
}

/// The JSON-in-text tool protocol means tool results are just another user
/// turn as far as the wire format is concerned; OpenRouter's `tool` role
/// requires native tool calling, which we do not use.
fn wire_role(role: LlmRole) -> &'static str {
    match role {
        LlmRole::System => "system",
        LlmRole::User | LlmRole::Tool => "user",
        LlmRole::Assistant => "assistant",
    }
}

fn compact_provider_error(status: &reqwest::StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<WireErrorEnvelope>(body)
        .map(|envelope| envelope.error.message)
        .unwrap_or_default();
    if detail.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status}: {detail}")
    }
}

#[derive(Debug, Serialize)]
struct WireRequest {
    model: String,
    messages: Vec<WireMessage>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct WireMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    choices: Vec<WireChoice>,
}

#[derive(Debug, Deserialize)]
struct WireChoice {
    message: WireResponseMessage,
}

#[derive(Debug, Deserialize)]
struct WireResponseMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct WireErrorEnvelope {
    error: WireError,
}

#[derive(Debug, Deserialize)]
struct WireError {
    #[serde(default)]
    message: String,
}
