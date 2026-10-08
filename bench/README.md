# Benchmarks

## Measured (local mock, reproducible)

Run `bash bench/run_mock.sh`. It boots an in-process wiremock server, runs the
real `JevClient` + `CurateFilter` pipeline, and writes `bench/results/mock.json`.
No network, no TypeSafe credits.

Result schema (`bench/results/mock.json`):

| Field | Meaning |
|---|---|
| `rows` | rows evaluated per run |
| `rows_per_sec` | throughput (1 request per row, so equals requests/sec) |
| `requests_per_sec` | request rate, same as rows/sec by construction |
| `latency_p50_ms` / `latency_p95_ms` | per-row latency percentiles, measured |
| `errors` | row evaluations that errored |
| `rate_limits` | 429s observed (0 on mock; only meaningful on live API) |
| `mode` | `local-mock` |
| hardware / commit | recorded in the results file, not guessed |

## Not measured

- **1,500+ rows/sec** is a horizontal-scaling target, not a benchmark result.
  At ~24 rows/sec per node it needs ~63 workers. The table in
  `site/benchmarks.html` is arithmetic on the measured number, not a promise.
- **Quality (precision/recall/F1)** needs a labeled reference set. `bench/datasets/tiny.jsonl`
  is a 5-row smoke fixture for the dry-run path, not a reference set. No quality
  numbers are claimed until that set exists.
- **Cost** is arithmetic on the $0.042/Mtok input price, not a measured bill.
