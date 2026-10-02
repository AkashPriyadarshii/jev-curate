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
| `src/client.rs` | TypeSafe AI HTTP client (fan-out, retry, timeouts) |
| `src/filter.rs` | Host sanity + secret scan + Jev evaluation pipeline |
| `src/parquet_io.rs` | Parquet/JSONL reader (batch + streaming) + JSONL writer |
| `src/rate_limiter.rs` | Token-bucket rate limiter (20 req/sec) |
| `src/presets.rs` | Evaluation rubrics (3 presets + YAML/JSON file) |
| `src/main.rs` | CLI — streaming pipeline, outputs, `--dry-run`/`--format`/`--model` |

## Constraints & Rules
- **One record, one request:** All questions for a row run together via fan-out.
- **Context guard:** Enforce 32k-char (~8k-token) ceiling; drop blank and padding lines before API.
- **Fail-closed:** Missing answer or confidence → reject. Secrets are scanned before any call and never sent.
- **Bounded streaming:** Records stream incrementally; never materialize the full dataset.
- **Outputs:** `clean.jsonl` + `rejected.jsonl` + `errors.jsonl` + `audit.jsonl` + `manifest.json` in `--out`.
- **Tests are offline:** `cargo test` uses an in-process mock, no API credits needed.
