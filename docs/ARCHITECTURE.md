# Architecture Specification — jev-curate

## Data Flow Pipeline

```
Raw Parquet / JSONL File
         │
         ▼
 ┌─────────────────────────────────────────┐
 │ Stream Reader (arrow::RecordBatchReader)│
 └───────────────────┬─────────────────────┘
                     │ Chunks of 500 rows
                     ▼
 ┌─────────────────────────────────────────┐
 │ Host Sanity Filter (Rust stdlib)        │
 │ - Strip blanks & repetitive ASCII       │
 │ - Enforce 8k-token ceiling              │
 └───────────────────┬─────────────────────┘
                     │ Micro-batches (3-5 rows)
                     ▼
 ┌─────────────────────────────────────────┐
 │ Speculative Fan-Out Worker Pool (Tokio) │
 │ - Rate Limiter: 1,200 req/min           │
 │ - POST https://api.typesafe.ai/v1/systemone
 │ - Evaluates Noul, Score, Choice         │
 └───────────────────┬─────────────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
 ┌───────────────┐       ┌────────────────────────┐
 │ clean.parquet │       │ rejected.parquet       │
 │ (Passed Rows) │       │ (Rows + Jev Breakdown) │
 └───────────────┘       └────────────────────────┘
```

## Security & Privacy Boundary
- All inputs streamed over TLS 1.3 with Bearer token authentication.
- Zero local disk caching of unencrypted API keys.
- Secret-scanning check runs on all input rows before transmission.
