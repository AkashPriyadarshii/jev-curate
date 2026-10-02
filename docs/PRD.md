# Product Requirements Document (PRD) — jev-curate

## 1. Problem Statement
Fine-tuning and post-training teams (evaluating synthetic reasoning traces, instruction pairs, and mathematical derivations) spend upwards of $15k–$50k running general-purpose LLMs (Claude 3.5 Sonnet, GPT-4o) to filter bad data. These models are slow (30–50 rows/sec) and prone to hallucinations. Traditional regex heuristics check basic syntax but cannot evaluate logical validity or derivation correctness.

## 2. Target Audience
- AI researchers and open-source fine-tuners (Llama, Qwen, Gemma post-trainers).
- RAG pipeline engineers curating knowledge bases.
- Synthetic data generation pipelines requiring high-throughput, low-cost verification.

## 3. Core Value Proposition
- **High Throughput:** 20 records/sec per single worker node (TypeSafe 1,200 req/min ceiling); 1,500+ records/sec cluster target.
- **Ultra-Low Cost:** $0.042/Mtok input tokens with zero output token fees.
- **Calibrated Mathematical Signals:** Receives probabilistic Noul and Score signals directly from TypeSafe AI's Jev model.
- **Dual Distribution:** Single-binary Rust CLI (`cargo install jev-curate`) + Python library (`pip install jev-curate`).

## 4. MVP Requirements
1. Read `.parquet` and `.jsonl` incrementally without loading the full dataset into memory; `examples/bench_mock.rs` measures **24.0 rows/sec** single-node on local mock (rate-limiter bound).
2. One record per Jev request; all questions for that record run together via fan-out; model via `JEV_MODEL` / `--model` (default `jev-latest`).
3. Rate limiter at 20 req/sec (1,200 req/min) with backoff handling.
4. 3 presets (`reasoning-math`, `anti-sycophancy`, `code-correctness`) plus external rubric files (JSON/YAML).
5. Outputs: `clean.jsonl` + `rejected.jsonl` + `errors.jsonl` + `audit.jsonl` + `manifest.json`; JSONL default, `--format parquet` opt-in.
6. Offline tests (WireMock, no API credits). Secrets are scanned before any call.
