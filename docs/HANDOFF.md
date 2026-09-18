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
- `TYPESAFE_API_KEY`: API token from TypeSafe AI (`https://typesafe.ai`). Required for live execution; not needed for unit tests.

## Key Invariants
1. **Never generate code/text with Jev:** Jev is purely a decision engine.
2. **Never change row content:** Clean rows must remain 100% byte-exact copies of original records.
3. **Speculative Fan-Out:** Never send sequential questions for the same batch.
