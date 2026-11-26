# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0-beta.1] - 2026-11-26
### Added
- `--format parquet` output support (opt-in, preserves input Arrow schema).
- `--resume` support for resuming interrupted curation runs.
- `secret_scan` pre-filtering to catch and handle sensitive tokens.
- Bounded streaming pipeline improvements for controlled memory usage.

### Fixed
- `examples/bench_mock.rs --json` now outputs pure valid JSON to stdout without human-readable prefix lines.
- `site/*.html` CLI snippets updated to `jev-curate filter <INPUT>` (removed nonexistent `-i`/`--input` flags).
- `site/index.html` throughput wording updated to `63-node cluster target (24/s single node)`.
- `README.md` Product Hunt launch date updated to Dec 1, 12:01am PT and version bumped to `0.3.0-beta.1`.
- `site/sitemap.xml` `lastmod` dates updated to `2026-11-26`.

## [0.2.0] - 2026-10-02 (beta)
- Streaming pipeline: bounded channel + backpressure, no Vec materialization, separates errors.jsonl.
- Timeouts 10s/90s, exponential backoff with jitter, retry-after-ms, 429/5xx handling.
- Confidence fail-closed fix (Noul has no confidence), model configurable via --model/JEV_MODEL (default jev-latest).
- Python: PyJevCurator.filter_text/filter_file, per-OS wheels, docs honesty (JSONL outputs, fan-out per record).

## [0.1.1] - 2026-09-24 (beta)
> Beta: this release is experimental. Expect rough edges. Please contribute by opening an issue or PR.
### Fixed
- Score `criteria` sent as an ordered list for `reasoning_depth` and `code_quality` (API returned HTTP 422 for objects).
- Floors moved `3.0` to `2.0` to keep the third level on the 0 to 4 scale returned by the API.
- Mock test updated to `1.5` vs `2.0` floor.
- Fail-closed filter: missing answers and evaluation errors reject the record, never pass it.
- Per-preset min_confidence floor. Strict response parsing with missing-key errors surfaced.
- Char-boundary truncation with blank-line stripping. Key required outside dry-run.

## [0.1.0] - Unreleased
### Added
- Initial project architecture and design specification.
- MD documentation skeleton (PRD, Design, Architecture).
- Multi-threaded Parquet streaming pipeline design.
- Speculative fan-out integration for TypeSafe AI Jev System One model.
- 3 launch presets: `reasoning-math`, `anti-sycophancy`, `code-correctness`.
