//! End-to-end throughput benchmark against an in-process mock server (₹0).
//!
//! Measures the real pipeline: rate limiter + HTTP client + Jev evaluation,
//! against wiremock. No network, no TypeSafe credits.
//!
//! Run: cargo run --release --example bench_mock
use std::sync::Arc;
use std::time::{Duration, Instant};

use jev_curate::client::JevClient;
use jev_curate::filter::CurateFilter;
use jev_curate::presets::PresetConfig;
use serde_json::json;
use tokio::sync::Semaphore;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ROWS: usize = 120;
const CONCURRENCY: usize = 32;

fn canned_response() -> serde_json::Value {
    json!({
        "answers": {
            "has_circular_reasoning": { "noul": 0.05 },
            "reasoning_depth": { "score": 4.8, "confidence": 0.95 }
        }
    })
}

fn row_payload() -> String {
    "Let f(x) = x^2. By taking the derivative with respect to x, f'(x) = 2x. \
     The chain rule gives d/dx x^2 = 2x, so the antiderivative is x^3/3."
        .to_string()
}

async fn run_batch(client: JevClient, concurrent: usize) -> Duration {
    let filter = Arc::new(CurateFilter::new(client, PresetConfig::reasoning_math()));
    let sem = Arc::new(Semaphore::new(concurrent));
    let start = Instant::now();
    let mut handles = Vec::with_capacity(ROWS);

    for _ in 0..ROWS {
        let filter = filter.clone();
        let sem = sem.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let row = row_payload();
            filter
                .evaluate_record(&row)
                .await
                .expect("mock evaluation should succeed");
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
    start.elapsed()
}

#[tokio::main]
async fn main() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_response()))
        .mount(&mock_server)
        .await;

    let make_client = || {
        JevClient::new("bench-key".to_string())
            .with_endpoint(format!("{}/v1/systemone", mock_server.uri()))
    };

    let seq = run_batch(make_client(), 1).await;
    let par = run_batch(make_client(), CONCURRENCY).await;

    let rate = |d: Duration| ROWS as f64 / d.as_secs_f64();

    println!("jev-curate benchmark (mock server, no network)");
    println!("machine cores: {}", std::thread::available_parallelism().unwrap());
    println!("rows: {ROWS}, concurrency 1 and {CONCURRENCY}");
    println!("sequential: {:.1} rows/sec ({:.0} ms/row)", rate(seq), seq.as_millis() as f64 / ROWS as f64);
    println!("concurrent {CONCURRENCY}: {:.1} rows/sec ({:.0} ms/row)", rate(par), par.as_millis() as f64 / ROWS as f64);
    println!("note: client rate limiter defaults to 20 req/sec (TypeSafe 1,200 req/min policy)");
}