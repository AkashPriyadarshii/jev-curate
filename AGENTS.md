# AGENTS.md — jev-curate

## Architecture & Project Directives
`jev-curate` is a high-throughput synthetic and pretraining dataset sifter powered by TypeSafe AI's Jev model (`jev-1.13.0` / `jev-latest` at `https://api.typesafe.ai/v1/systemone`).

- **Target Speed:** 20 records/sec per worker node (TypeSafe 1,200 req/min limit); 1,500+ records/sec cluster target.
- **Cost:** ~$0.042 per million input tokens; output unmetered.
- **Core Engine:** Pure Rust multi-threaded streaming core with Arrow/Parquet integration + PyO3 Python bindings.
- **I/O Formats:** Parquet (`.parquet`) and JSON Lines (`.jsonl`).

## Build & Test Pipeline
```bash
# Rust core
cargo build --release
cargo test

# Python bindings (via maturin)
maturin develop
pytest
```

## Key Files
| File | Purpose |
|---|---|
| `src/lib.rs` | PyO3 module interface and public Rust crate entrypoint |
| `src/main.rs` | Standalone CLI binary (`jev-curate`) |
| `src/client.rs` | TypeSafe AI HTTP client with speculative fan-out batching |
| `src/filter.rs` | Pre-filtering and Jev evaluation pipeline |
| `src/parquet_io.rs` | Streaming Parquet reader and writer |
| `src/rate_limiter.rs` | Token bucket rate limiter with auto 429 backoff |
| `src/presets.rs` | Pre-built evaluation rubrics (math-reasoning, anti-sycophancy, code-correctness) |

## Constraints & Rules
- **Zero Token Waste:** Ingest state once per micro-batch, run all questions via speculative fan-out.
- **Context Rot Guard:** Enforce 8k-token maximum per row in host code; drop blank/padding lines in Rust before sending to API.
- **₹0 Testing Budget:** All unit and integration tests must run against in-process mock server (`typesafe-rs-mock`); never burn live API credits in CI.
