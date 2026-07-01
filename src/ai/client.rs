use std::io::{BufRead, BufReader};

use reqwest::blocking::Client as BlockingClient;

use crate::ai::models::{ChatCompletionRequest, ChatMessage, StreamingChatCompletionResponse};

#[derive(Debug, Clone)]
pub struct LlmClient {
    http: BlockingClient,
    base_url: String,
    api_key: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            http: BlockingClient::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            model,
        }
    }

    pub fn chat_stream<F, S>(
        &self,
        prompt: &str,
        mut on_chunk: F,
        mut on_status: S,
    ) -> Result<(), String>
    where
        F: FnMut(String),
        S: FnMut(String),
    {
        on_status("debug: request sent".to_string());
        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&ChatCompletionRequest {
                model: self.model.clone(),
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }],
                stream: true,
            })
            .send()
            .map_err(|error| error.to_string())?;

        let status = response.status();
        if !status.is_success() {
            return Err(format!("request failed: {status}"));
        }
        on_status(format!("debug: response {status}"));

        let reader = BufReader::new(response);
        for line in reader.lines() {
            let line = line.map_err(|error| error.to_string())?;
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }

            let payload = line.trim_start_matches("data: ").trim();
            if payload == "[DONE]" {
                on_status("debug: received [DONE]".to_string());
                break;
            }

            let event: StreamingChatCompletionResponse =
                serde_json::from_str(payload).map_err(|error| error.to_string())?;
            for choice in event.choices {
                if let Some(content) = choice.delta.content.filter(|content| !content.is_empty()) {
                    on_chunk(content);
                }
            }
        }

        Ok(())
    }
}
