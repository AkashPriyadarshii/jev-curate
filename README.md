<!--
Title: jev-curate: High-Throughput Synthetic & Pretraining Dataset Sifter Powered by TypeSafe AI (Jev System One)
Description: High-throughput synthetic & pretraining dataset curation pipeline in Rust & Python powered by TypeSafe AI's Jev model (api.typesafe.ai). Stream, filter, and score millions of Parquet and JSONL rows using Jev System One typed decisions (Choice, Score, Noul), speculative question fan-out, and calibrated post-training reasoning rubrics.
Keywords: typesafe ai, type safe ai, jev, api.typesafe.ai, jev-1.13.0, jev-latest, system one, choice, score, noul, dataset curation, synthetic data filtering, pretraining datasets, post-training, fine-tuning, rlcd, reasoning models, parquet, arrow, polars, pyarrow, pyo3, rust, jsonl, llm evaluation, speculative fan-out
-->

<div align="center">

# `jev-curate`

**Support:** fuel the next build — [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://buymeacoffee.com/AkashPriyadarshi)

**High-Throughput Synthetic & Pretraining Dataset Sifter Powered by TypeSafe AI (Jev)**

> Beta: v0.1.1 is experimental. Expect rough edges. Please contribute by opening an issue or PR.

**Live:** [jev-curate.vercel.app](https://jev-curate.vercel.app) (measured **24.0 rows/sec** single-node on local mock bench `examples/bench_mock.rs`, 1,500+ cluster target)

[![PyPI](https://img.shields.io/pypi/v/jev-curate?style=flat-square)](https://pypi.org/project/jev-curate/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![TypeSafe AI](https://img.shields.io/badge/Model-Jev--1.13.0-indigo.svg?style=flat-square)](https://typesafe.ai)

By **[Akash Priyadarshi](https://github.com/AkashPriyadarshii)**

[Why jev-curate](#why-jev-curate) &bull; [Quickstart](#quickstart) &bull; [CLI Reference](#cli-reference) &bull; [Python API](#python-api) &bull; [Architecture](#architecture) &bull; [Non-Goals](#non-goals) &bull; [Ecosystem](#ecosystem)

</div>

[![stars](https://img.shields.io/github/stars/AkashPriyadarshii/jev-curate?style=flat-square&label=stars)](https://github.com/AkashPriyadarshii/jev-curate/stargazers) [![crates.io](https://img.shields.io/crates/v/jev-curate?style=flat-square)](https://crates.io/crates/jev-curate) [![downloads](https://img.shields.io/crates/d/jev-curate?style=flat-square)](https://crates.io/crates/jev-curate) [![release](https://img.shields.io/github/v/release/AkashPriyadarshii/jev-curate?style=flat-square&label=release)](https://github.com/AkashPriyadarshii/jev-curate/releases)

---

## Why `jev-curate`?

Cleaning 10M to 1B rows of synthetic reasoning data, instruction tuning pairs, or web-scraped corpora is an economic and technical nightmare:
* **Generative LLMs are too slow and expensive:** Running Claude 3.5 Sonnet or GPT-4o to judge synthetic rows costs **$15,000–$50,000** per billion tokens and crawls at a painful 30–50 rows/sec.
* **Regex heuristics are blind to reasoning flaws:** Keyword and regex filters can check syntax, but fail to detect circular reasoning, hallucinated derivation steps, or robotic sycophancy.
* **Context rot from uncompressed inputs:** Naively feeding raw data into LLMs causes decision accuracy to crater while burning money on boilerplate text.

`jev-curate` solves this by piping Apache Arrow and Parquet streams through **TypeSafe AI's Jev model** (`jev-1.13.0`):
* **Single-round-trip rubrics per row:** Evaluates all rubric questions concurrently in one HTTP request per row via Jev's speculative parallel fan-out with zero per-question round-trips. Single-node throughput is bounded by TypeSafe's 1,200 req/min (20 rows/sec) limit; horizontal scaling across worker nodes targets 1,500+ rows/sec cluster throughput.
* **~$4.20 per 100M tokens:** Jev bills $0.042/Mtok for input, zero for output. TypeSafe benchmarks System One workflows **444.6x cheaper and 193.6x faster** than generative LLMs ([source](https://typesafe.ai)).
* **Mathematical calibration:** Receives calibrated probabilities (`Noul`), ordinal rubrics (`Score` on a 0 to 4 scale, sent as an ordered list), and categorical choices (`Choice`), eliminating generative text slop.
* **Zero Rewriting:** Emits clean records verbatim without rewriting or altering mathematical formulas.

---

## Quickstart

### CLI (Rust Single Binary).
```bash
# Install via Cargo
cargo install jev-curate

# Set your TypeSafe AI key
export TYPESAFE_API_KEY="your-api-key"

# Filter a Parquet dataset using the math reasoning preset:
jev-curate filter train.parquet \
  --preset reasoning-math \
  --out ./output/ \
  --concurrency 32
```

### Python API
PyO3 bindings (build from source with the `python` cargo feature):
```bash
git clone https://github.com/AkashPriyadarshii/jev-curate
cd jev-curate
maturin develop   # python feature auto-enabled via pyproject.toml
```

```python
from jev_curate import PyJevCurator

curator = PyJevCurator(api_key="your-api-key", preset="reasoning-math")
```

`PyJevCurator` currently wraps the same filter pipeline as the CLI (see `src/filter.rs`); constructor-only for now. Use the CLI for row-level sifting.

---

## CLI Reference

`jev-curate filter [OPTIONS] <INPUT_PATH>`

| Flag | Default | Description |
|---|:---:|---|
| `<INPUT_PATH>` | *Required* | Path to input `.parquet` or `.jsonl` file. |
| `-p, --preset` | `reasoning-math` | Pre-built rubric (`reasoning-math`, `anti-sycophancy`, `code-correctness`). |
| `-o, --out` | `./curated/` | Destination folder for `clean.jsonl` and `rejected.jsonl`. |
| `-c, --concurrency` | `32` | Worker concurrency (adaptive token bucket prevents 429 rate limits). |
| `--dry-run` | `false` | Offline evaluation simulation with host pre-filtering and zero API calls (no `TYPESAFE_API_KEY` needed). |
| `--endpoint` | *None* | Custom API endpoint URL for offline mock testing (or set `TYPESAFE_ENDPOINT`). |

---

## Presets

| Preset | Primitives Evaluated | Target Problem Solved |
|---|---|---|
| **`reasoning-math`** | `has_circular_reasoning` (`Noul`)<br>`reasoning_depth` (`Score` 0-4, keep 2.0 or higher) | Drops ungrounded math derivations and repetitive circular proofs. |
| **`anti-sycophancy`** | `is_sycophantic` (`Noul`)<br>`has_ai_disclaimer` (`Noul`) | Eliminates "As an AI...", ungrounded flattery, and conversational filler. |
| **`code-correctness`** | `has_stub_placeholders` (`Noul`)<br>`code_quality` (`Score` 0-4, keep 2.0 or higher) | Drops incomplete code blocks and unrunnable pseudo-code mocks. |

---

## Build & Test

```bash
# Rust core
cargo build --release
cargo test

# Python bindings (via maturin)
maturin develop
pytest
```

All tests run against an in-process mock server with zero live API credits in CI.

---

## Architecture

```
jev-curate/
├── Cargo.toml                 # Rust core manifest (arrow, parquet, tokio, pyo3, clap, reqwest, serde)
├── pyproject.toml             # Maturin Python package manifest
├── src/
│   ├── lib.rs                 # PyO3 module bindings & crate entry
│   ├── main.rs                # Standalone CLI binary entrypoint
│   ├── client.rs              # TypeSafe AI HTTP client (speculative fan-out)
│   ├── filter.rs              # Host-side sanity pruning & Jev pipeline
│   ├── parquet_io.rs          # Streaming Parquet/Arrow reader and writer
│   ├── rate_limiter.rs        # Adaptive token-bucket with auto 429 backoff
│   └── presets.rs             # Pre-built post-training evaluation rubrics
└── tests/
    └── mock_test.rs           # In-process mock tests via typesafe-rs-mock (100% offline)
```

---

## Non-Goals

1. **Not a Generative Re-writer:** `jev-curate` never paraphrases or re-generates text. Data is kept 100% verbatim.
2. **Not a Heavy Local Vector DB:** No embeddings, no vector indices, zero PyTorch/CUDA runtime requirements.
3. **Not a Generic Web Scraper:** Tailored strictly for structured datasets (Parquet, Arrow, JSONL).

---

<p align="center">
  <img src="https://api.star-history.com/svg?repos=AkashPriyadarshii/jev-curate&type=Date" width="600" alt="star history" />
</p>

## Mandatory Ecosystem, Author & Social Directory

### Ecosystem
* [`jev-seo`](https://github.com/AkashPriyadarshii/jev-seo)
* [`jev-superpowers`](https://github.com/AkashPriyadarshii/jev-superpowers)
* [`jev-curate`](https://github.com/AkashPriyadarshii/jev-curate)
* [`jev-git`](https://github.com/AkashPriyadarshii/jev-git)
* [`tdlib-android`](https://github.com/AkashPriyadarshii/tdlib-android)
* [`kharcha`](https://github.com/AkashPriyadarshii/kharcha)

### Author
* **Akash Priyadarshi** (Patna, Bihar, India)
* [GitHub](https://github.com/AkashPriyadarshii) &bull; [Portfolio](https://akashpriyadarshi.vercel.app) &bull; [LinkedIn](https://linkedin.com/in/akashpriyadarshii) &bull; [Resume](https://akashpriyadarshii.github.io/Resume/)

### Social
* [X / Twitter](https://x.com/Akash__ydv001) &bull; [Threads](https://www.threads.net/@akash.priyadarshii) &bull; [Instagram](https://www.instagram.com/akash.priyadarshii/) &bull; [Reddit](https://reddit.com/user/akashpriyadarshi)

---

*Built with high-performance Rust for the TypeSafe AI System One (Jev) ecosystem.*

*Keywords: TypeSafe AI, Jev, api.typesafe.ai, System One, Choice, Score, Noul, dataset curation, synthetic data filtering, pretraining datasets, Parquet streaming, arrow, rust.*
