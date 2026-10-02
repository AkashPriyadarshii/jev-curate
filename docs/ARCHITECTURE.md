# Architecture Specification — jev-curate

## Data Flow Pipeline

```
Raw Parquet / JSONL File
         │
         ▼
 ┌─────────────────────────────────────────┐
 │ Batch Reader (RecordBatchReader / BufReader)│
 └───────────────────┬─────────────────────┘
                     │ incrementally, batch-decoded
                     ▼
 ┌─────────────────────────────────────────┐
 │ Host Sanity Filter (Rust stdlib)        │
 │ - Strip blanks & repetitive ASCII       │
 │ - Enforce 8k-token ceiling              │
 └───────────────────┬─────────────────────┘
                     │ one record per request
                     ▼
 ┌─────────────────────────────────────────┐
 │ Multi-Question Fan-Out Pool (Tokio)     │
 │ - Request Limiter: 20 req/sec + 429 backoff
 │ - POST https://api.typesafe.ai/v1/systemone
 │ - Evaluates Noul, Score, Choice         │
 └───────────────────┬─────────────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
 ┌───────────────┐       ┌────────────────────────┐
 │ clean.jsonl   │       │ rejected.jsonl         │
 │ (Passed Rows) │       │ (Rows + reasons)       │
 └───────────────┘       └────────────────────────┘
```

## Security & Privacy Boundary
- All inputs streamed over TLS 1.3 with Bearer token authentication.
- Zero local disk caching of unencrypted API keys.
- Secret-scanning check runs on all input rows before transmission.
