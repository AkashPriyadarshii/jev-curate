# CLAUDE.md — jev-curate

## Commands
```bash
# Build
cargo build
cargo build --release

# Test (100% offline, zero token cost)
cargo test

# Python development
maturin develop
pytest
```

## Code Style
- **Rust:** Idiomatic Rust 2024, zero unsafe unless strictly required for FFI, descriptive error handling with `thiserror`/`anyhow`.
- **Formatting:** `cargo fmt` and `cargo clippy -- -D warnings`.
- **Concurrency:** `tokio` async tasks + `rayon` CPU parallel pre-filtering.
- **Immutability:** Transform data via iterators and streams; return fresh Arrow RecordBatches.
