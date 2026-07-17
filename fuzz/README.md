# Fuzzing

[`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) targets for
`khmer-tokenizer-core`. This is a **detached crate** (its own `[workspace]`),
because `libfuzzer-sys` needs a nightly toolchain and a sanitizer and must not
touch the stable core build or CI.

## Setup

```bash
cargo install cargo-fuzz
rustup toolchain install nightly
```

## Run

```bash
cargo +nightly fuzz run normalize     # byte-length + idempotence invariants
cargo +nightly fuzz run split_kcc     # clusters must rejoin to the input exactly
cargo +nightly fuzz run segment       # full pipeline must never panic/hang
```

Each target decodes the raw bytes as UTF-8 (ignoring non-UTF-8 inputs) and
feeds the string to the function under test. The `normalize` and `split_kcc`
targets additionally assert the documented invariants, so a violation — not
just a panic — is reported as a crash.

Found a crashing input? `cargo-fuzz` writes it to `fuzz/artifacts/`; add a
minimized version as a unit test in `core/` before fixing.
