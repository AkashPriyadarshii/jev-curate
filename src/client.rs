use crate::presets::JevQuestionConfig;
use crate::rate_limiter::RateLimiter;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

fn rand_jitter() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0)
}

fn parse_retry_delay_secs(s: &str) -> Option<Duration> {
    if let Ok(v) = s.parse::<f64>() {
        let ms = (v * 1000.0) as u64;
        return Some(Duration::from_millis(ms.min(60_000)));
    }
    // HTTP-date not supported, fall back
    None
}

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

#[derive(Debug, Deserialize)]
struct JevUsage {
    #[serde(default)]
    input_tokens: u64,
}

/// Response payload received from TypeSafe AI.
#[derive(Debug, Deserialize)]
struct JevResponse {
    answers: HashMap<String, JevQuestionResult>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    usage: Option<JevUsage>,
}

/// TypeSafe AI System One Client with speculative fan-out batching.
#[derive(Clone)]
pub struct JevClient {
    http: Client,
    api_key: String,
    endpoint: String,
    rate_limiter: RateLimiter,
    model: String,
}

impl JevClient {
    pub fn new(api_key: String) -> Self {
        Self::new_with_model(api_key, "jev-latest".to_string())
    }

    pub fn new_with_model(api_key: String, model: String) -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(90))
                .build()
                .expect("Failed to build HTTP client"),
            api_key,
            endpoint: "https://api.typesafe.ai/v1/systemone".to_string(),
            rate_limiter: RateLimiter::new(20.0), // 1,200 req/min
            model,
        }
    }

    /// Overrides the endpoint (used for in-process offline mock tests).
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = endpoint;
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    fn parse_retry_delay(res: &reqwest::Response) -> Option<Duration> {
        for key in [
            "retry-after-ms",
            "retry-after",
            "Retry-After",
            "Retry-After-Ms",
        ] {
            if let Some(v) = res.headers().get(key).and_then(|h| h.to_str().ok()) {
                if key.to_lowercase().contains("ms") {
                    if let Ok(ms) = v.parse::<u64>() {
                        return Some(Duration::from_millis(ms.min(60_000)));
                    }
                } else if let Some(d) = parse_retry_delay_secs(v) {
                    return Some(d);
                } else if let Ok(secs) = v.parse::<u64>() {
                    return Some(Duration::from_secs(secs.min(60)));
                }
            }
        }
        None
    }

    /// Evaluates a state payload against multiple questions in a single speculative fan-out request.
    /// Returns (answers, input_tokens) for cost tracking.
    pub async fn evaluate(
        &self,
        state: serde_json::Value,
        questions: &HashMap<String, JevQuestionConfig>,
    ) -> Result<(HashMap<String, JevQuestionResult>, u64)> {
        let max_retries = 3;
        let mut attempts = 0;

        loop {
            self.rate_limiter.acquire().await;
            attempts += 1;

            let req_body = JevRequest {
                model: &self.model,
                state: state.clone(),
                questions,
            };

            let response = tokio::time::timeout(
                Duration::from_secs(90),
                self.http
                    .post(&self.endpoint)
                    .header("Authorization", format!("Bearer {}", self.api_key))
                    .header("Content-Type", "application/json")
                    .json(&req_body)
                    .send(),
            )
            .await;
            let response = match response {
                Ok(r) => r,
                Err(_) => {
                    if attempts >= max_retries {
                        anyhow::bail!("TypeSafe AI request timed out after 90s");
                    }
                    let base = 500u64 * (1u64 << (attempts - 1));
                    let jitter = rand_jitter() % 250;
                    tokio::time::sleep(Duration::from_millis((base + jitter).min(5000))).await;
                    continue;
                }
            };

            match response {
                Ok(res) if res.status().is_success() => {
                    let jev_res: JevResponse = res
                        .json()
                        .await
                        .context("Failed to parse Jev response JSON")?;
                    if let Some(err) = jev_res.error {
                        anyhow::bail!("Jev returned error: {}", err);
                    }
                    for key in questions.keys() {
                        if !jev_res.answers.contains_key(key) {
                            anyhow::bail!("Jev response missing answer for '{}'", key);
                        }
                    }
                    let tok = jev_res.usage.map(|u| u.input_tokens).unwrap_or(0);
                    return Ok((jev_res.answers, tok));
                }
                Ok(res) if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    let delay = Self::parse_retry_delay(&res);
                    self.rate_limiter.trigger_backoff(delay).await;
                    if attempts >= max_retries {
                        anyhow::bail!(
                            "TypeSafe AI rate limit exceeded (HTTP 429) after {} retries",
                            max_retries
                        );
                    }
                    // exponential backoff with jitter when no header
                    if delay.is_none() {
                        let base = 500u64 * (1u64 << (attempts - 1));
                        let jitter = rand_jitter() % 250;
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            (base + jitter).min(5000),
                        ))
                        .await;
                    }
                    continue;
                }
                Ok(res)
                    if res.status().is_server_error()
                        || res.status() == reqwest::StatusCode::REQUEST_TIMEOUT =>
                {
                    if attempts >= max_retries {
                        let status = res.status();
                        let text = res.text().await.unwrap_or_default();
                        anyhow::bail!("TypeSafe AI API error: HTTP {} - {}", status, text);
                    }
                    let base = 500u64 * (1u64 << (attempts - 1));
                    let jitter = rand_jitter() % 250;
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        (base + jitter).min(5000),
                    ))
                    .await;
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
                    let base = 500u64 * (1u64 << (attempts - 1));
                    let jitter = rand_jitter() % 250;
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        (base + jitter).min(5000),
                    ))
                    .await;
                }
            }
        }
    }
}
