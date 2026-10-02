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
1. Read `.parquet` (batch-decoded via Arrow `RecordBatchReader`) and `.jsonl` (line-buffered) incrementally; pipeline must not materialize the full dataset into a `Vec` before evaluation.
2. One record per Jev request; all configured questions for that record are evaluated together via speculative fan-out (multi-question, not record-level micro-batching).
3. Built-in adaptive request-rate limiter targeting 20 requests/sec with HTTP 429 backoff. Provider input-token-per-second quotas are not locally enforced.
4. Support 3 default launch presets (`reasoning-math`, `anti-sycophancy`, `code-correctness`).
5. Output split: `clean.jsonl` (passed) and `rejected.jsonl` (with rejection reasons). Parquet output is not implemented.
6. Offline tests: local unit tests plus WireMock integration coverage (zero live API credits).
