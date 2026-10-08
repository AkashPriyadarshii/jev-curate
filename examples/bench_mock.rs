//! End-to-end throughput benchmark against an in-process mock server (₹0).
//!
//! Measures the real pipeline: rate limiter + HTTP client + Jev evaluation,
//! against wiremock. No network, no TypeSafe credits.
//!
//! Run: cargo run --release --example bench_mock
//! JSON: cargo run --release --example bench_mock -- --json > bench/results/mock.json
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

/// Per-row latency sample plus pass/fail outcome.
struct RowSample {
    latency: Duration,
    passed: bool,
    errored: bool,
}

async fn run_batch(client: JevClient, concurrent: usize) -> (Duration, Vec<RowSample>) {
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
            let t0 = Instant::now();
            let verdict = filter.evaluate_record(&row).await;
            let latency = t0.elapsed();
            match verdict {
                Ok(v) => RowSample { latency, passed: v.passed, errored: false },
                Err(_) => RowSample { latency, passed: false, errored: true },
            }
        }));
    }

    let mut samples = Vec::with_capacity(ROWS);
    for h in handles {
        samples.push(h.await.unwrap());
    }
    (start.elapsed(), samples)
}

/// Nearest-rank percentile over sorted millis. Empty input -> 0.0 (no data, not zero latency).
fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    if sorted_ms.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0 * sorted_ms.len() as f64).ceil() as usize;
    sorted_ms[rank.clamp(1, sorted_ms.len()) - 1]
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

    let (seq_elapsed, seq_samples) = run_batch(make_client(), 1).await;
    let (par_elapsed, par_samples) = run_batch(make_client(), CONCURRENCY).await;

    let rate = |d: Duration| ROWS as f64 / d.as_secs_f64();

    let summarize = |samples: &[RowSample]| {
        let mut ms: Vec<f64> = samples.iter().map(|s| s.latency.as_secs_f64() * 1000.0).collect();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let errors = samples.iter().filter(|s| s.errored).count();
        let passed = samples.iter().filter(|s| s.passed).count();
        (percentile(&ms, 50.0), percentile(&ms, 95.0), errors, passed)
    };
    let (seq_p50, seq_p95, seq_err, seq_pass) = summarize(&seq_samples);
    let (par_p50, par_p95, par_err, par_pass) = summarize(&par_samples);

    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    println!("jev-curate benchmark (mock server, no network)");
    println!("machine cores: {cores}");
    println!("rows: {ROWS}, concurrency 1 and {CONCURRENCY}");
    println!(
        "sequential: {:.1} rows/sec ({:.0} ms/row) p50={:.0}ms p95={:.0}ms errors={} passed={}/{ROWS}",
        rate(seq_elapsed),
        seq_elapsed.as_millis() as f64 / ROWS as f64,
        seq_p50, seq_p95, seq_err, seq_pass,
    );
    println!(
        "concurrent {CONCURRENCY}: {:.1} rows/sec ({:.0} ms/row) p50={:.0}ms p95={:.0}ms errors={} passed={}/{ROWS}",
        rate(par_elapsed),
        par_elapsed.as_millis() as f64 / ROWS as f64,
        par_p50, par_p95, par_err, par_pass,
    );
    println!("note: client rate limiter defaults to 20 req/sec (TypeSafe 1,200 req/min policy)");

    if std::env::args().any(|a| a == "--json") {
        let out = serde_json::json!({
            "tool": "jev-curate bench_mock",
            "mode": "local-mock",
            "rows": ROWS,
            "concurrency": [1, CONCURRENCY],
            "sequential": {
                "rows_per_sec": rate(seq_elapsed),
                "requests_per_sec": rate(seq_elapsed),
                "latency_p50_ms": seq_p50,
                "latency_p95_ms": seq_p95,
                "errors": seq_err,
                "rate_limits": 0,
                "passed": seq_pass,
            },
            "concurrent": {
                "rows_per_sec": rate(par_elapsed),
                "requests_per_sec": rate(par_elapsed),
                "latency_p50_ms": par_p50,
                "latency_p95_ms": par_p95,
                "errors": par_err,
                "rate_limits": 0,
                "passed": par_pass,
            },
            "note": "local mock: zero-latency judge. 1 req/row so requests_per_sec == rows_per_sec. rate_limits counts 429s from a live API; mock returns none.",
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    }
}
