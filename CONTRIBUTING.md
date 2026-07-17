# Contributing to khmer-tokenizer

Thanks for your interest! This project aims to be a fast, dependency-free,
honestly-benchmarked Khmer word segmenter. Contributions of all sizes are
welcome — bug reports, dictionary words, tests, docs, and code.

## Ground rules

- **The core crate stays dependency-free.** `core/` must not gain any
  third-party runtime dependency (dev-dependencies for benches/tests are fine
  but keep them out of the default `cargo build`/`cargo test` path where
  possible). `pyo3`/`wasm-bindgen` live only in the detached `py/` and `wasm/`
  crates.
- **No corpus text is committed.** Evaluation corpora (khPOS, ALT, …) are
  gitignored and often carry non-commercial licenses. Never add them, or text
  derived from them, to the repo. See [core/ATTRIBUTION.md](core/ATTRIBUTION.md).
- **Determinism.** Segmentation and training must be reproducible run-to-run.

## Development

```bash
cargo build --workspace
cargo test  --workspace          # includes the CI regression guard
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all                  # CI enforces `cargo fmt --all -- --check`
```

Bindings (each has its own CI job):

```bash
cd py && maturin develop && pytest tests/ -v      # Python
wasm-pack test --node wasm                        # WASM (Node)
```

Optional tooling:

```bash
cargo bench --manifest-path bench/Cargo.toml      # Criterion throughput
cargo +nightly fuzz run normalize                 # requires cargo-fuzz + nightly
```

## Before you open a PR

1. `cargo fmt --all`, `cargo clippy ... -D warnings`, and `cargo test` all pass.
2. New behavior has a test. Bug fixes add a regression test.
3. If you changed the default strategy, dictionary, or normalization, re-run
   `cargo xtask eval` (with khPOS under `data/khpos`) and update the
   **Current results** table in [docs/BENCHMARKS.md](docs/BENCHMARKS.md).
4. Add a line to the `## [Unreleased]` section of [CHANGELOG.md](CHANGELOG.md).

## Adding dictionary words

The easiest high-value contribution: add a missing modern word to
[core/src/dict.supplement.txt](core/src/dict.supplement.txt) (one word per
line). Include, in the PR description, a link or citation showing real-world
usage. Do **not** edit `core/src/dict.txt` by hand — it is regenerated from
its upstream source by `cargo xtask prepare-dict`.

## Releasing (maintainers)

1. Bump the workspace version and the `py`/`wasm` versions; update CHANGELOG.
2. `git tag v<version>` and push the tag — the `Release` workflow builds and
   publishes to PyPI (OIDC), crates.io (`CARGO_REGISTRY_TOKEN`), and npm
   (`NPM_TOKEN`).

## License

By contributing, you agree that your contributions are dual-licensed under
MIT OR Apache-2.0, matching the project.
