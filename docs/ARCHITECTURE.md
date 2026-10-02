# Architecture Specification — jev-curate

## Data Flow

```
Parquet / JSONL file
  → Batch reader (incremental)
  → Host sanity filter (blanks, padding, 32k-char ceiling)
  → Fan-out pool (one request per row, all questions together)
    rate-limited at 20 req/sec + retry handling
    POST https://api.typesafe.ai/v1/systemone
  → clean.jsonl / rejected.jsonl / errors.jsonl / audit.jsonl / manifest.json
```

## Security & Privacy
- Inputs sent over TLS with bearer token; secrets are scanned and never transmitted.
- No API key stored on disk.
- `--dry-run` runs without a key or network.
