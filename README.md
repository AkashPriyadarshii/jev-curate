<!--
Title: jev-curate — High-Throughput Synthetic & Pretraining Dataset Sifter Powered by TypeSafe AI (Jev System One)
Description: High-throughput synthetic & pretraining dataset curation pipeline in Rust & Python powered by TypeSafe AI's Jev model (api.typesafe.ai). Stream, filter, and score millions of Parquet and JSONL rows at 1,500+ records/sec using Jev System One typed decisions (Choice, Score, Noul), speculative fan-out, and calibrated post-training reasoning rubrics.
Keywords: typesafe ai, type safe ai, jev, api.typesafe.ai, jev-1.13.0, jev-latest, system one, choice, score, noul, dataset curation, synthetic data filtering, pretraining datasets, post-training, fine-tuning, rlcd, reasoning models, parquet, arrow, polars, pyarrow, pyo3, rust, jsonl, llm evaluation, speculative fan-out
-->

<div align="center">

# `jev-curate`

**High-Throughput Synthetic & Pretraining Dataset Sifter Powered by TypeSafe AI (Jev)**

[![Crates.io](https://img.shields.io/crates/v/jev-curate.svg?style=flat-square)](https://crates.io/crates/jev-curate)
[![PyPI](https://img.shields.io/pypi/v/jev-curate.svg?style=flat-square)](https://pypi.org/project/jev-curate/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![TypeSafe AI](https://img.shields.io/badge/Model-Jev--1.13.0-indigo.svg?style=flat-square)](https://typesafe.ai)

By **[Akash Priyadarshi](https://github.com/AkashPriyadarshii)**

[Why jev-curate](#why-jev-curate) &bull; [Quickstart](#quickstart) &bull; [CLI Reference](#cli-reference) &bull; [Python API](#python-api) &bull; [Architecture](#architecture) &bull; [Non-Goals](#non-goals) &bull; [Ecosystem](#ecosystem)

</div>

---

## Why `jev-curate`?

Cleaning 10M to 1B rows of synthetic reasoning data, instruction tuning pairs, or web-scraped corpora is an economic and technical nightmare:
* **Generative LLMs are too slow and expensive:** Running Claude 3.5 Sonnet or GPT-4o to judge synthetic rows costs **$15,000–$50,000** per billion tokens and crawls at a painful 30–50 rows/sec.
* **Regex heuristics are blind to reasoning flaws:** Keyword and regex filters can check syntax, but fail to detect circular reasoning, hallucinated derivation steps, or robotic sycophancy.
* **Context rot from uncompressed inputs:** Naively feeding raw data into LLMs causes decision accuracy to crater while burning money on boilerplate text.

`jev-curate` solves this by piping Apache Arrow and Parquet streams through **TypeSafe AI's Jev model** (`jev-1.13.0`):
* **1,500+ rows/sec throughput:** Evaluates rows in multi-threaded batches using Jev's speculative parallel fan-out.
* **~$4.20 per 100M tokens:** Jev charges $0.042/Mtok for input with zero output token fees—over 100x cheaper than GPT-4o-mini and 700x cheaper than Claude 3.5 Sonnet.
* **Mathematical calibration:** Receives calibrated probabilities (`Noul`), ordinal rubrics (`Score` 1–5), and categorical choices (`Choice`), eliminating generative text slop.
* **Zero Rewriting:** Emits clean records verbatim without rewriting or altering mathematical formulas.

---

## Quickstart

### CLI (Rust Single Binary)
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

### Python (PyO3 + Polars / PyArrow)
```bash
pip install jev-curate
```

```python
import polars as pl
from jev_curate import JevCurator

df = pl.read_parquet("synthetic_data.parquet")

curator = JevCurator(
    preset="reasoning-math",
    concurrency=32,
)

clean_df, rejected_df = curator.sift(df)
clean_df.write_parquet("clean.parquet")
rejected_df.write_parquet("rejected.parquet")
```

---

## CLI Reference

`jev-curate filter [OPTIONS] <INPUT_PATH>`

| Flag | Default | Description |
|---|:---:|---|
| `<INPUT_PATH>` | *Required* | Path to input `.parquet` or `.jsonl` file. |
| `--preset` | `reasoning-math` | Pre-built rubric (`reasoning-math`, `anti-sycophancy`, `code-correctness`). |
| `--out` | `./curated/` | Destination folder for `clean.jsonl` and `rejected.jsonl`. |
| `--concurrency` | `32` | Worker concurrency (adaptive token bucket prevents 429 rate limits). |
| `--dry-run` | `false` | Offline evaluation simulation with host pre-filtering and zero API calls. |

---

## Presets

| Preset | Primitives Evaluated | Target Problem Solved |
|---|---|---|
| **`reasoning-math`** | `has_circular_logic` (`Noul`)<br>`is_step_valid` (`Noul`)<br>`reasoning_depth` (`Score` 1–5) | Drops ungrounded math derivations and repetitive circular proofs. |
| **`anti-sycophancy`** | `is_sycophantic` (`Noul`)<br>`has_robotic_filler` (`Noul`) | Eliminates "As an AI...", ungrounded flattery, and conversational filler. |
| **`code-correctness`** | `has_unclosed_fence` (`Noul`)<br>`has_stub_placeholders` (`Noul`) | Drops incomplete code blocks and unrunnable pseudo-code mocks. |

---

## Architecture

```
jev-curate/
├── Cargo.toml                 # Rust core manifest (arrow, parquet, pyo3, tokio)
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

## Mandatory Ecosystem, Author & Social Directory

### Ecosystem
* [`design-genius`](https://github.com/AkashPriyadarshii/design-genius)
* [`akash-design-engineering`](https://github.com/AkashPriyadarshii/akash-design-engineering)
* [`tdlib-android`](https://github.com/AkashPriyadarshii/tdlib-android)
* [`kharcha`](https://github.com/AkashPriyadarshii/kharcha)

### Author
* **Akash Priyadarshi** (Patna, Bihar, India)
* [GitHub](https://github.com/AkashPriyadarshii) &bull; [Portfolio](https://akashpriyadarshi.vercel.app) &bull; [LinkedIn](https://linkedin.com/in/akash-priyadarshi-1aa51b37a) &bull; [Resume](https://akashpriyadarshii.github.io/Resume/)

### Social
* [X / Twitter](https://x.com/Akash__ydv001) &bull; [Threads](https://www.threads.net/@akash.priyadarshii) &bull; [Instagram](https://www.instagram.com/akash.priyadarshii/) &bull; [Reddit](https://reddit.com/user/DragonfruitWeak2801)

---

*Built with high-performance Rust for the TypeSafe AI System One (Jev) ecosystem.*

*Keywords: TypeSafe AI, Jev, api.typesafe.ai, System One, Choice, Score, Noul, dataset curation, synthetic data filtering, pretraining datasets, Parquet streaming, arrow, rust.*
