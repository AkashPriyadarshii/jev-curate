"""Launch-day self-check for jev-curate docs vs code. Run: python3 checks/launch_check.py."""

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
fails = []


def check(name, cond, detail=""):
    print(
        ("PASS " if cond else "FAIL ")
        + name
        + (f" ({detail})" if detail and not cond else "")
    )
    if not cond:
        fails.append(name)


readme = (ROOT / "README.md").read_text()
lib = (ROOT / "src" / "lib.rs").read_text()
main = (ROOT / "src" / "main.rs").read_text()
presets = (ROOT / "src" / "presets.rs").read_text()
llms = (ROOT / "site" / "llms.txt").read_text()

# Python filter_file must not promise manifest/audit it does not write
check("readme-no-py-manifest-claim", "errors.jsonl + manifest" not in readme)
check("lib-no-manifest-write", "manifest.json" not in lib)
check("lib-no-audit-write", "audit.jsonl" not in lib)
# CLI writes all five outputs
for out in [
    "clean.jsonl",
    "rejected.jsonl",
    "errors.jsonl",
    "audit.jsonl",
    "manifest.json",
]:
    check(f"cli-writes-{out}", out in main, "src/main.rs")
# Preset floors match README table (2.0 floors)
check(
    "reasoning-floor-2.0",
    'min_scores.insert("reasoning_depth".to_string(), 2.0)' in presets,
)
check("code-floor-2.0", 'min_scores.insert("code_quality".to_string(), 2.0)' in presets)
# PH Upcoming link present in README
check("readme-ph-link", "producthunt.com/products/jev-curate" in readme)
# llms.txt has no duplicated quickstart phrase
check("llms-no-dup-phrase", "six-step quickstart + six-step" not in llms)
# No beta word leaks into PH copy file
ph_copy = pathlib.Path.home() / "Downloads" / "jev-curate-launch" / "PH-COPY.md"
if ph_copy.exists():
    lines = ph_copy.read_text().splitlines()
    body = "\n".join(lines[1:]).lower()
    check("ph-copy-no-beta", "beta" not in body)
else:
    check("ph-copy-exists", False, "PH-COPY.md missing")

print(f"\n{len(fails)} failures: {fails}" if fails else "\nall checks pass")
sys.exit(1 if fails else 0)
