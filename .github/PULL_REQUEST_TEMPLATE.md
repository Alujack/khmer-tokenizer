<!-- Thanks for contributing! Keep this short. -->

## What & why

<!-- What does this change, and what problem does it solve? -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass
- [ ] New behavior has a test; bug fixes add a regression test
- [ ] Added a line to the `## [Unreleased]` section of `CHANGELOG.md`
- [ ] No new third-party dependency in `core/`; no corpus text committed
- [ ] If the default strategy / dictionary / normalization changed: re-ran `cargo xtask eval` and updated the **Current results** table in `docs/BENCHMARKS.md`

## Accuracy impact (if any)

<!-- Paste before/after F1 from `cargo xtask eval`, or "n/a". -->
