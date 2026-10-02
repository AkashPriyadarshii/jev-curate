# Project State: jev-curate

## Current Status
- **Phase:** v0.2.0 (beta) — streaming pipeline complete. Contributions welcome.

## Progress
- [x] Market research & architecture (awesome-jev audit, 9/9 decisions).
- [x] Rust core + PyO3 bindings + CLI (`client.rs`, `parquet_io.rs`, `presets.rs`, `filter.rs`, `main.rs`).
- [x] Streaming pipeline with bounded concurrency; outputs `clean`/`rejected`/`errors`/`audit`/`manifest`.
- [x] Offline tests (WireMock, no API credits).
- [x] Distribution: `cargo install`, prebuilt archives, `pip install` wheels + sdist.
