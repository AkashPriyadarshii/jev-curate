# Design Document — jev-curate

## 1. System Components

### A. Host-Side Sanity Filter (`src/filter.rs`)
- Fast pre-flight check in Rust stdlib.
- Drops blank strings and rows with repetitive whitespace padding.
- Hard byte limit per record (`max_tokens_per_row = 8,000`).
- Prevents context rot per TypeSafe AI skill Rule #3.

### B. TypeSafe AI Multi-Question Fan-Out Client (`src/client.rs`)
- Endpoint: `POST https://api.typesafe.ai/v1/systemone`
- Model: `jev-1.13.0`
- One record per `state`; all configured Noul/Score/Choice questions are evaluated together in one request.
- Ingests `state` once, runs multiple questions simultaneously via speculative fan-out.

### C. Adaptive Request Limiter (`src/rate_limiter.rs`)
- Token-bucket algorithm capping requests at 20 requests/sec (1,200 req/min).
- Backs off on HTTP 429 and respects `retry-after` header. Input-token-per-second quotas are not locally enforced.

### D. Parquet/Arrow Reader + Buffered JSONL Writer (`src/parquet_io.rs`)
- Uses `arrow` and `parquet` crates.
- Decodes `RecordBatchReader` batches and reads JSONL line-buffered; records must be yielded incrementally without materializing the full dataset into a `Vec`.
- Copies string values from columns into owned `String`s for evaluation; writes buffered `clean.jsonl` and `rejected.jsonl` via `BufWriter`.

### E. PyO3 Python Layer (`src/lib.rs`)
- Exposes `PyJevCurator` class to Python. Constructor-only for now; row-level sifting uses the CLI. Intended to accept Polars/PyArrow objects via PyO3 once the batch pipeline is finalized.
