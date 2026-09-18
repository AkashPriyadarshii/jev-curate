# Design Document — jev-curate

## 1. System Components

### A. Host-Side Sanity Filter (`src/filter.rs`)
- Fast pre-flight check in Rust stdlib.
- Drops blank strings and rows with repetitive whitespace padding.
- Hard byte limit per record (`max_tokens_per_row = 8,000`).
- Prevents context rot per TypeSafe AI skill Rule #3.

### B. TypeSafe AI Speculative Fan-Out Client (`src/client.rs`)
- Endpoint: `POST https://api.typesafe.ai/v1/systemone`
- Model: `jev-1.13.0`
- Micro-batches records into a single multi-question JSON payload.
- Ingests `state` once, runs multiple Noul and Score questions simultaneously.

### C. Adaptive Rate Limiter (`src/rate_limiter.rs`)
- Token-bucket algorithm capping requests at 1,200 req/min.
- Automatically captures HTTP 429 and parses `retry-after` header to pause worker threads gracefully.

### D. Streaming Parquet & Arrow Engine (`src/parquet_io.rs`)
- Uses `arrow` and `parquet` crates.
- Iterates over `RecordBatchReader` in chunks of 500 rows.
- Zero-copy extraction of text columns.
- Writes partitioned Arrow batches to `clean.parquet` and `rejected.parquet`.

### E. PyO3 Python Layer (`src/lib.rs`)
- Exposes `JevCurator` class to Python.
- Accepts and returns Polars DataFrames or PyArrow Tables with zero serialization overhead.
