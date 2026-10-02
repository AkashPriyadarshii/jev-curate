# Engineering Handoff — jev-curate

## Quick Setup
```bash
# Clone and enter repo
cd jev-curate

# Check compilation
cargo check

# Run offline mock test suite (zero API cost)
cargo test
```

## Environment Variables
- `TYPESAFE_API_KEY`: Required for live runs.
- `JEV_MODEL` / `--model`: Default `jev-latest`.
- `TYPESAFE_ENDPOINT` / `--endpoint`: Custom endpoint for testing.

## Key Invariants
1. **Never generate text with Jev:** It is a decision engine.
2. **Never change row content:** Outputs are verbatim copies.
3. **Fan-out:** One request per row, all questions together.
4. **Fail-closed:** Missing answers → reject.
5. **Outputs:** `clean.jsonl` + `rejected.jsonl` + `errors.jsonl` + `audit.jsonl` + `manifest.json`.
