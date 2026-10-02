# Design Document — jev-curate

## 1. System Components

### A. Host-Side Sanity Filter (`src/filter.rs`)
- Drops blank rows and repetitive padding; enforces 32k-char ceiling.

### B. TypeSafe AI Fan-Out Client (`src/client.rs`)
- `POST https://api.typesafe.ai/v1/systemone` (default `jev-latest`, override via `--model`/`JEV_MODEL`).
- One request per row, all questions together.

### C. Request Limiter (`src/rate_limiter.rs`)
- Token bucket at 20 req/sec (1,200 req/min) with retry handling.

### D. Reader & Writer (`src/parquet_io.rs`)
- Incremental Parquet and JSONL reading; buffered JSONL outputs.
- Parquet output is opt-in via `--format parquet`.

### E. Python Layer (`src/lib.rs`)
- `PyJevCurator` exposes `filter_text` / `filter_file`.

### F. CLI Pipeline (`src/main.rs`)
- Streaming pipeline with bounded concurrency; supports `--dry-run` and resume.

### G. Filter Guarantees (`src/filter.rs`)
- Secrets scanned before API; fail-closed on missing answers or confidence.

### H. Presets (`src/presets.rs`)
- 3 built-ins or external YAML/JSON rubric file.
