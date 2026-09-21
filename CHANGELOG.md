# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Fixed
- Fail-closed filter: missing answers and evaluation errors reject the record, never pass it.
- Per-preset min_confidence floor. Strict response parsing with missing-key errors surfaced.
- Char-boundary truncation with blank-line stripping. Key required outside dry-run.

## [0.1.0] - Unreleased
### Added
- Initial project architecture and design specification.
- MD documentation skeleton (PRD, Design, Architecture, Handoff).
- Multi-threaded Parquet streaming pipeline design.
- Speculative fan-out integration for TypeSafe AI Jev System One model.
- 3 launch presets: `reasoning-math`, `anti-sycophancy`, `code-correctness`.
