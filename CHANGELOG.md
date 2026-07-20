# Changelog

All notable changes to this project are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The workspace crates (`khmer-tokenizer-core`, `khmer-tokenizer-cli`) and the
`py`/`wasm` bindings share a single version number.

> Historical entries below 0.3.0 were reconstructed from git history — the
> 0.2.x releases were published to the registries without matching `v*` tags.
> Going forward, every release is tagged (`v<version>`) so the release
> workflow publishes it and this log stays authoritative.

## [Unreleased]

### Added
- **Optional high-accuracy model tier**, documented and made first-class
  without contaminating the permissive crate: a README "High-accuracy model"
  section, `models/README.md`, and a manual `model.yml` workflow that trains
  the tagger and publishes it as a **separate CC BY-NC-SA 4.0 release asset**
  (the code stays MIT/Apache; the model — derived from the only available,
  non-commercial Khmer corpora — is distributed separately with attribution).
  `.model` files are now git-ignored to prevent accidentally committing one.
- `docs/` comparison table and an **Accuracy** section in the README stating,
  with reproduced numbers, where the tokenizer sits versus CRF/neural tools.
- `CHANGELOG.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and
  GitHub issue/PR templates.
- Criterion throughput benchmark (detached `bench/` crate) —
  `cargo bench --manifest-path bench/Cargo.toml`.
- `cargo-fuzz` targets for `normalize`, `split_kcc`, and `segment` (`fuzz/`).
- Shared `core/src/viterbi.rs` module (BMES lattice + tags→tokens) used by
  both the HMM and the perceptron tagger.

### Changed
- CI now enforces `cargo fmt --all -- --check`.
- The release workflow now publishes to **crates.io** and **npm** in addition
  to PyPI, gated on a real tag push.
- `docs/ARCHITECTURE.md` and `docs/BENCHMARKS.md` refreshed to v0.3 (MinWordsDp
  default, five strategies, OOV grouping, the Tagger tier); BENCHMARKS now
  leads with a current-results table and keeps the running log as history.

## [0.3.0] - 2026-07-17

### Changed
- **`MinWordsDp` (fewest-words dynamic programming) is now the default
  strategy** (was `ForwardMaxMatch`). It backtracks over a DAG of dictionary
  matches, so it no longer strands the tail of a run behind a greedy long
  first word (`ខែកក្កដា` → `ខែ` + `កក្កដា`). khPOS OPEN-TEST F1 0.7467 → 0.7510
  with the additions below.
- **Orthographic normalization runs by default** before every `segment` call
  (opt out with `.without_normalization()`).

### Added
- **OOV run grouping** (on by default; `.without_oov_grouping()` to opt out):
  a maximal run matching nothing in the dictionary is emitted as one
  unknown-word token instead of one-token-per-cluster.
- `normalize()` rules 3 (subscript-RO reordering) and 4 (within-cluster mark
  order), plus `normalize_full()` (orthographic/spelling/spacing corrections).
- `dict.supplement.txt`: a project-authored modern-vocabulary supplement
  (provinces, countries, loanwords, tech terms) loaded by `with_default_dict()`.

## [0.2.2] - 2026-07-03

### Added
- CRF-class **averaged-perceptron BMES tagger** (`TaggerModel`,
  `Strategy::Tagger`, `with_tagger`) with trigram, cluster-length, and
  type-class features; text (de)serialization with escaping and non-finite
  weight rejection.
- Cross-corpus honesty rows in `docs/BENCHMARKS.md`.

### Changed
- WASM npm package inlines the WASM binary as Base64 for zero-config use in
  React/Vite/Webpack without bundler plugins.

### Fixed
- Hardened segmentation against real-world dirty text (dangling COENG, BOM,
  stray marks).

## [0.2.0] - 2026-07-02

### Added
- Python (PyO3/maturin, abi3) bindings — published to PyPI as
  `khmer-tokenizer`.
- WASM (wasm-bindgen/wasm-pack) bindings — published to npm as `kh-tokenizer`.
- HMM/Viterbi BMES OOV fallback (`HmmModel`, `with_hmm`).
- CLI tiers via `--dict`, `--freq`, `--tagger`.
- ALT dataset evaluation utilities.

## [0.1.0] - 2026-07-01

### Added
- Initial release: KCC-aware dictionary word segmenter with `ForwardMaxMatch`,
  `BiMaxMatch`, and `UnigramDp` strategies over a cluster-keyed trie.
- Embedded 59,526-word dictionary (chamkho / SIL NRSI, MIT).
- Evaluation harness (`eval`, `xtask`) with SIGHAN span P/R/F1, R-iv/R-oov,
  and word accuracy; CI regression guard.
- Dual MIT/Apache-2.0 license.

[Unreleased]: https://github.com/Alujack/khmer-tokenizer/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Alujack/khmer-tokenizer/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/Alujack/khmer-tokenizer/compare/v0.2.0...v0.2.2
[0.2.0]: https://github.com/Alujack/khmer-tokenizer/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Alujack/khmer-tokenizer/releases/tag/v0.1.0
