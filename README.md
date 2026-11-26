# `jev-curate`

High-throughput synthetic & pretraining dataset sifter in Rust and Python, powered by TypeSafe AI's Jev model (`jev-latest`).

**Stream, filter, and score millions of Parquet and JSONL rows without rewriting a single character.** Single-round-trip speculative fan-out per row, calibrated `Noul` / `Score` / `Choice` judgments, bounded streaming with `--resume`, and opt-in `--format parquet` output.

> Beta: `v0.3.0-beta.1` is experimental. Expect rough edges. Please contribute by opening an issue or PR.

[![PyPI](https://img.shields.io/pypi/v/jev-curate?style=flat-square)](https://pypi.org/project/jev-curate/)
[![crates.io](https://img.shields.io/crates/v/jev-curate?style=flat-square)](https://crates.io/crates/jev-curate)
[![downloads](https://img.shields.io/crates/d/jev-curate?style=flat-square)](https://crates.io/crates/jev-curate)
[![total downloads](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/AkashPriyadarshii/jev-curate/main/analytics/badge.json&style=flat-square)](analytics/README.md)
[![GitHub release](https://img.shields.io/github/v/release/AkashPriyadarshii/jev-curate?style=flat-square&label=release)](https://github.com/AkashPriyadarshii/jev-curate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![stars](https://img.shields.io/github/stars/AkashPriyadarshii/jev-curate?style=flat-square&label=stars)](https://github.com/AkashPriyadarshii/jev-curate/stargazers)
[![Live Site](https://img.shields.io/badge/site-jev--curate.vercel.app-black?style=flat-square)](https://jev-curate.vercel.app)

**Product Hunt:** [jev-curate Upcoming](https://www.producthunt.com/products/jev-curate?launch=jev-curate) (launches Dec 1, 12:01am PT)

<a href="https://www.producthunt.com/products/jev-curate?embed=true&utm_source=badge-featured&utm_medium=badge&utm_campaign=badge-jev-curate" target="_blank" rel="noopener noreferrer"><img alt="jev-curate - Sift 1B-row datasets 444x cheaper with Jev | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1272040&theme=light&t=1791323078281"></a>

---

## Quickstart

```bash
# Rust (any OS)
cargo install jev-curate

# Python (no Rust toolchain needed; wheels on PyPI)
pip install jev-curate

# Set your TypeSafe AI key
export TYPESAFE_API_KEY="your-api-key"

# Filter a Parquet dataset using the math reasoning preset:
jev-curate filter train.parquet \
  --preset reasoning-math \
  --model jev-latest \
  --out ./output/ \
  --concurrency 32
```

Use `--dry-run` for host-side pre-filtering (`secret_scan` and sanity pruning) with **zero API calls** and no `TYPESAFE_API_KEY` required.

### Install options

| OS | Rust | Python |
|---|---|---|
| **Linux** | `cargo install jev-curate` or `gh release download` `tar.gz` | `pip install jev-curate` (manylinux wheel) |
| **macOS** | `cargo install jev-curate` or `tar.gz` (`x86_64` + `arm64`) | `pip install jev-curate` (universal2 wheel) |
| **Windows** | `cargo install jev-curate` or `zip` | `pip install jev-curate` (win wheel) |

From source (Python bindings via `maturin`):

```bash
git clone https://github.com/AkashPriyadarshii/jev-curate
cd jev-curate
maturin develop   # python feature auto-enabled via pyproject.toml
```

```python
from jev_curate import PyJevCurator

curator = PyJevCurator(api_key="your-api-key", preset="reasoning-math")
# Single row
verdict = curator.filter_text("Let x=5... therefore...")  # -> {passed, rejection_reasons, scores, nouls}
# Whole file -> ./curated/clean.jsonl + rejected.jsonl + errors.jsonl
stats = curator.filter_file("train.parquet", out="./curated")  # -> {passed, rejected, errors, total}
```

---

## Output in practice

`jev-curate filter` streams input rows and writes three separated outputs in `--out` (default `./curated/`):

- `clean.jsonl` (or `clean.parquet` with `--format parquet`, preserving the input Arrow schema)
- `rejected.jsonl` — rows that failed host sanity checks, `secret_scan`, or Jev rubric thresholds, with rejection reasons
- `errors.jsonl` — rows that hit transport/API errors after retries, kept separate so clean/rejected counts stay exact

---

## Capabilities

- **Single-round-trip rubrics per row**: Evaluates all rubric questions concurrently in one HTTP request per row via Jev's speculative parallel fan-out. Receives calibrated probabilities (`Noul`), ordinal rubrics (`Score` on a `0–4` scale sent as an ordered list), and categorical choices (`Choice`).
- **Pre-built & custom YAML presets**:
  - `reasoning-math`: `has_circular_reasoning` (`Noul`), `reasoning_depth` (`Score` 0–4, keep `>= 2.0`). Drops ungrounded math derivations and circular proofs.
  - `anti-sycophancy`: `is_sycophantic` (`Noul`), `has_ai_disclaimer` (`Noul`). Eliminates "As an AI...", ungrounded flattery, and conversational filler.
  - `code-correctness`: `has_stub_placeholders` (`Noul`), `code_quality` (`Score` 0–4, keep `>= 2.0`). Drops incomplete code blocks and pseudo-code mocks.
  - Custom rubrics: pass `--preset rubric.yaml`.
- **Bounded streaming & `--resume`**: Uses a bounded channel with backpressure (no full-dataset `Vec` materialization) and supports `--resume` to continue interrupted runs.
- **Opt-in Parquet output (`--format parquet`)**: Preserves the input Arrow schema when writing clean records.
- **Host-side `secret_scan` & sanity pruning**: Catches sensitive tokens and malformed/empty rows before any API request is sent.
- **Zero rewriting**: Emits clean records verbatim without paraphrasing or altering mathematical formulas.

---

## Evidence and trust

| Check | Result | Rerun |
|---|---|---|
| Test suite | 16 passing unit & integration tests (in-process mock server, 0 live credits) | `cargo test` |
| Linter | Clean under clippy | `cargo clippy --all-targets` |
| Local mock benchmark | **23.9 rows/sec** single-node local mock (`bench/results/mock.json`) | `bash bench/run_mock.sh` |
| Dry-run gate | Host pre-filtering only, zero API calls | `jev-curate filter <INPUT> --dry-run` |

`jev-curate`'s built-in limiter defaults to 20 requests per second (`v0.3.0-beta.1`); TypeSafe's current org default is 80 req/s and 100,000 tok/s (planned for `v0.3.0`). Evaluation is fail-closed: missing answers or evaluation errors reject the record rather than passing it.

---

## What's new

**v0.3.0-beta.1** (2026-11-26):
- Added `--format parquet` output support (opt-in, preserves input Arrow schema)
- Added `--resume` support for resuming interrupted curation runs
- Added `secret_scan` host-side pre-filtering
- Bounded streaming pipeline improvements for controlled memory usage
- `examples/bench_mock.rs --json` outputs pure valid JSON to stdout; updated `site/*.html` CLI snippets to `jev-curate filter <INPUT>`

**v0.2.0** (2026-10-02): Bounded streaming pipeline with backpressure, `errors.jsonl` separation, 10s/90s timeouts with exponential backoff and 429/5xx handling, fail-closed confidence, `--model`/`JEV_MODEL` support, and `PyJevCurator` Python bindings. Full history in [`CHANGELOG.md`](CHANGELOG.md).

---

## Workflows

### 1. Filter Parquet or JSONL with a preset or custom YAML

```bash
jev-curate filter train.parquet --preset reasoning-math --out ./curated/
jev-curate filter train.jsonl --preset rubric.yaml --out ./curated/
```

### 2. Preserve Parquet schema and resume interrupted runs

```bash
jev-curate filter train.parquet --format parquet --resume --out ./curated/
```

### 3. Host-only dry run (`secret_scan` + sanity pruning)

```bash
jev-curate filter train.parquet --dry-run --out ./curated/
```

### 4. Offline mock testing

```bash
jev-curate filter train.parquet --endpoint http://127.0.0.1:8080 --out ./curated/
```

---

## Command Reference

`jev-curate filter [OPTIONS] <INPUT_PATH>`

| Flag | Default | Description |
|---|:---:|---|
| `<INPUT_PATH>` | *Required* | Path to input `.parquet` or `.jsonl` file. |
| `-p, --preset` | `reasoning-math` | Pre-built rubric (`reasoning-math`, `anti-sycophancy`, `code-correctness`) or path to custom `.yaml`. |
| `-o, --out` | `./curated/` | Destination folder for `clean.jsonl`, `rejected.jsonl`, and `errors.jsonl`. |
| `-c, --concurrency` | `32` | Worker concurrency (adaptive token bucket prevents 429 rate limits). |
| `--dry-run` | `false` | Host pre-filtering only, no API calls (no `TYPESAFE_API_KEY` needed). |
| `--model` | `jev-latest` | Jev model (or `JEV_MODEL` env; pin `jev-1.13.0` for repro). |
| `--format` | `jsonl` | Output format: `jsonl` or `parquet` (opt-in, preserves schema). |
| `--resume` | `false` | Resume an interrupted curation run from existing output state. |
| `--endpoint` | *None* | Custom API endpoint URL for offline mock testing (or set `TYPESAFE_ENDPOINT`). |

---

## What runs where, and what it costs

| Component | Where it runs | Cost |
|---|---|---|
| Parquet/JSONL streaming, `secret_scan`, sanity pruning, `--dry-run` | Local machine | Free |
| Jev `Choice` / `Score` / `Noul` judgments | TypeSafe System One (`api.typesafe.ai`) | ~$4.20 per 100M tokens ($0.042/Mtok input, zero output) |

TypeSafe benchmarks System One workflows **444.6x cheaper and 193.6x faster** than generative LLMs ([source](https://typesafe.ai)).

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
│   ├── filter.rs              # Host-side sanity pruning, secret_scan & Jev pipeline
│   ├── parquet_io.rs          # Batch-decoded Parquet reader + buffered JSONL/Parquet writer
│   ├── rate_limiter.rs        # Request rate limiter with auto 429 backoff
│   └── presets.rs             # Pre-built post-training evaluation rubrics
└── tests/
    ├── cli_test.rs            # CLI integration tests
    └── mock_test.rs           # In-process mock tests via typesafe-rs-mock (100% offline)
```

---

## Development

```bash
git clone https://github.com/AkashPriyadarshii/jev-curate
cd jev-curate
cargo test
cargo clippy --all-targets

# Python bindings (via maturin; no .py tests yet)
maturin develop
```

All tests run against an in-process mock server with zero live API credits in CI.

---

## Limits and non-goals

- **Not a generative rewriter**: `jev-curate` never paraphrases or re-generates text. Data is kept 100% verbatim.
- **Not a local vector DB**: No embeddings, no vector indices, zero PyTorch/CUDA runtime requirements.
- **Not a generic web scraper**: Tailored strictly for structured datasets (Parquet, Arrow, JSONL).

---

## Ecosystem & Author

- [`jev-seo`](https://github.com/AkashPriyadarshii/jev-seo) — Free Rust SEO & GEO CLI with 19-tool MCP server
- [`jev-superpowers`](https://github.com/AkashPriyadarshii/jev-superpowers) — Jev-gated agent engineering workflows
- [`jev-curate`](https://github.com/AkashPriyadarshii/jev-curate) — High-throughput synthetic & pretraining dataset sifter
- [`jev-git`](https://github.com/AkashPriyadarshii/jev-git) — Jev reflex gates for git
- [`tdlib-android`](https://github.com/AkashPriyadarshii/tdlib-android) — Precompiled TDLib binaries for four Android ABIs
- [`kharcha`](https://github.com/AkashPriyadarshii/kharcha) — Offline-first UPI expense tracker for Android

**Author:** **Akash Priyadarshi** (Patna, Bihar, India) · [GitHub](https://github.com/AkashPriyadarshii) · [Portfolio](https://akashpriyadarshi.is-a.dev) · [Project page](https://akashpriyadarshi.is-a.dev/projects/jev-curate/) · [LinkedIn](https://linkedin.com/in/akashpriyadarshii) · [Resume](https://akashpriyadarshii.github.io/Resume/) · [X](https://x.com/Akash__ydv001) · [Threads](https://www.threads.net/@akash.priyadarshii) · [Instagram](https://www.instagram.com/akash.priyadarshii/) · [Reddit](https://reddit.com/user/akashpriyadarshii)

<p>
  <a href="https://buymeacoffee.com/AkashPriyadarshi"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black" alt="Buy Me a Coffee" /></a>
</p>

<img referrerpolicy="no-referrer-when-downgrade" src="https://static.scarf.sh/a.png?x-pxid=ce1c0ae3-34fa-4e5c-9716-6732d0401758" />
