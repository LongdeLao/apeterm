use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::config::BackendConfig;

#[derive(Debug, Clone, Default)]
pub struct BackendInsight {
    pub ticker: String,
    pub context: Option<InsightContextResponse>,
    pub explanation: Option<InsightExplanationResponse>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct InsightContextResponse {
    pub ticker: String,
    pub stale_context: bool,
    pub article_count: usize,
    pub articles: Vec<InsightArticle>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct InsightArticle {
    pub title: String,
    pub url: String,
    pub source: String,
    pub published_at: Option<String>,
    pub age_hours: Option<f64>,
    pub ticker: String,
    pub relevance_score: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct InsightExplanationResponse {
    pub ticker: String,
    pub model: String,
    pub cache_hit: bool,
    pub stale_context: bool,
    pub summary: String,
    pub key_drivers: Vec<String>,
    pub sources_used: Vec<String>,
    pub confidence: String,
}

#[derive(Debug)]
pub struct BackendClient {
    client: Client,
    base_url: String,
    enabled: bool,
}

impl BackendClient {
    pub fn new(config: &BackendConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            enabled: config.enabled,
        })
    }

    pub fn fetch_insight(&self, ticker: &str) -> Result<Option<BackendInsight>, String> {
        if !self.enabled {
            return Ok(None);
        }

        let normalized = normalize_ticker(ticker);
        if normalized.is_empty() {
            return Ok(None);
        }

        let context: InsightContextResponse = self.get_json(&context_url(&self.base_url, &normalized))?;
        let explanation: InsightExplanationResponse =
            self.get_json(&explanation_url(&self.base_url, &normalized))?;

        Ok(Some(BackendInsight {
            ticker: normalized,
            context: Some(context),
            explanation: Some(explanation),
        }))
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, String> {
        let response = self.client.get(url).send().map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("backend request failed: {} {}", response.status(), url));
        }
        response.json::<T>().map_err(|error| error.to_string())
    }
}

fn normalize_ticker(ticker: &str) -> String {
    ticker.trim().to_ascii_uppercase()
}

fn context_url(base_url: &str, ticker: &str) -> String {
    format!("{base_url}/api/v1/insights/{ticker}/context")
}

fn explanation_url(base_url: &str, ticker: &str) -> String {
    format!("{base_url}/api/v1/insights/{ticker}/explanation")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_backend_urls_from_local_base() {
        assert_eq!(
            context_url("http://localhost:8080", "NVDA"),
            "http://localhost:8080/api/v1/insights/NVDA/context"
        );
        assert_eq!(
            explanation_url("http://localhost:8080", "NVDA"),
            "http://localhost:8080/api/v1/insights/NVDA/explanation"
        );
    }

    #[test]
    fn normalizes_ticker_for_requests() {
        assert_eq!(normalize_ticker(" nvda "), "NVDA");
    }
}
