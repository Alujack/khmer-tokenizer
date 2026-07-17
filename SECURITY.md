# Security Policy

## Supported versions

Only the latest released `0.3.x` line receives fixes. This is a pre-1.0
library; pin a version you have tested.

## Threat model

`khmer-tokenizer` is a text-processing library with **no `unsafe` code** and,
in its core, **no third-party dependencies**. It performs no I/O and no network
access on its own. The realistic risks are:

- **Denial of service** from a pathological input (excessive time or memory on
  crafted/malformed text).
- **Panics** on malformed UTF-8/Khmer input (a panic in a library is a bug).
- **Model-file parsing** (`TaggerModel::from_text`) on untrusted input.

Malformed-Unicode handling is covered by unit tests and by `cargo-fuzz`
targets (`fuzz/`) over `normalize`, `split_kcc`, and `segment`. If you find an
input that panics or blows up time/memory, that is a security-relevant bug.

## Reporting a vulnerability

Please **do not** open a public issue for a suspected vulnerability. Instead
use GitHub's private reporting:

- Go to the repository's **Security → Report a vulnerability** ("Private
  vulnerability reporting"), or
- email the maintainer listed in `Cargo.toml` (`authors`).

Include a minimal reproducing input and the observed behavior. We aim to
acknowledge within a few days and to credit reporters who wish it.
