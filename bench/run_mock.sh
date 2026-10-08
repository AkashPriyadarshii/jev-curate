#!/usr/bin/env bash
# Reproducible local-mock benchmark. No network, no API credits.
# Usage: bash bench/run_mock.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
mkdir -p bench/results
cargo run --release --example bench_mock -- --json | tee bench/results/mock.json
