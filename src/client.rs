use crate::presets::JevQuestionConfig;
use crate::rate_limiter::RateLimiter;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result for an individual question returned by Jev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JevQuestionResult {
    #[serde(default)]
    pub noul: Option<f64>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub choice: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub probabilities: Option<HashMap<String, f64>>,
}

/// Request payload sent to TypeSafe AI System One endpoint.
#[derive(Debug, Serialize)]
struct JevRequest<'a> {
    model: &'a str,
    state: serde_json::Value,
    questions: &'a HashMap<String, JevQuestionConfig>,
}

/// Response payload received from TypeSafe AI.
#[derive(Debug, Deserialize)]
struct JevResponse {
    #[serde(default)]
    answers: HashMap<String, JevQuestionResult>,
    #[serde(default)]
    #[allow(dead_code)]
    error: Option<String>,
}

/// TypeSafe AI System One Client with speculative fan-out batching.
#[derive(Clone)]
pub struct JevClient {
    http: Client,
    api_key: String,
    endpoint: String,
    rate_limiter: RateLimiter,
}

impl JevClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::builder().build().expect("Failed to build HTTP client"),
            api_key,
            endpoint: "https://api.typesafe.ai/v1/systemone".to_string(),
            rate_limiter: RateLimiter::new(20.0), // 1,200 req/min
        }
    }

    /// Overrides the endpoint (used for in-process offline mock tests).
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// Evaluates a state payload against multiple questions in a single speculative fan-out request.
    pub async fn evaluate(
        &self,
        state: serde_json::Value,
        questions: &HashMap<String, JevQuestionConfig>,
    ) -> Result<HashMap<String, JevQuestionResult>> {
        let max_retries = 3;
        let mut attempts = 0;

        loop {
            self.rate_limiter.acquire().await;
            attempts += 1;

            let req_body = JevRequest {
                model: "jev-1.13.0",
                state: state.clone(),
                questions,
            };

            let response = self
                .http
                .post(&self.endpoint)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&req_body)
                .send()
                .await;

            match response {
                Ok(res) if res.status().is_success() => {
                    let jev_res: JevResponse = res
                        .json()
                        .await
                        .context("Failed to parse Jev response JSON")?;
                    return Ok(jev_res.answers);
                }
                Ok(res) if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    let retry_after = res
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok());

                    self.rate_limiter.trigger_backoff(retry_after).await;

                    if attempts >= max_retries {
                        anyhow::bail!("TypeSafe AI rate limit exceeded (HTTP 429) after {} retries", max_retries);
                    }
                    continue;
                }
                Ok(res) => {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    anyhow::bail!("TypeSafe AI API error: HTTP {} - {}", status, text);
                }
                Err(e) => {
                    if attempts >= max_retries {
                        return Err(e).context("Failed to connect to TypeSafe AI API");
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
                }
            }
        }
    }
}
