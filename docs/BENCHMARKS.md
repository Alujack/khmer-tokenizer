# Benchmarks

Running log of evaluation-harness results. Produced by `cargo xtask eval`
(see [ROADMAP.md](./ROADMAP.md) Phase 1). Corpus is khPOS `OPEN-TEST`
(1,000 sentences, CC BY-NC-SA 4.0 — downloaded to `data/khpos`, never
committed; see [RESEARCH.md](./RESEARCH.md) §3).

Metric definitions: token-span **P/R/F1** (SIGHAN convention — a predicted
token counts as correct only if both boundaries match gold), **R-iv** /
**R-oov** (recall restricted to gold words that are / aren't in the
tokenizer's dictionary), **word accuracy** (fraction of sentences segmented
with a fully correct token sequence).

| Date       | Strategy         | Dictionary               | P      | R      | F1     | R-iv   | R-oov  | Word Acc | Corpus            |
| ---------- | ---------------- | ------------------------- | ------ | ------ | ------ | ------ | ------ | -------- | ------------------ |
| 2026-07-01 | ForwardMaxMatch  | seed dict (~100 words)    | 0.1576 | 0.3501 | 0.2174 | 0.9990 | 0.2047 | 0.0050   | khPOS OPEN-TEST     |

## Reading the baseline

- **R-iv ≈ 1.0**: the trie/longest-match mechanism itself is correct — when
  a word is actually in the dictionary, it's recovered almost perfectly.
- **Low P/F1**: the seed dictionary covers a small fraction of the corpus's
  vocabulary, so most words are out-of-vocabulary and fall back to
  single-cluster tokens, badly over-segmenting the output (many more
  predicted tokens than gold tokens, which drives precision down even
  though the words that *do* match are counted correctly).
- **R-oov ≈ 0.20**: about a fifth of OOV gold words happen to be a single
  KCC cluster already, so the naive fallback gets them right by luck.

This confirms `ROADMAP.md`'s framing: the biggest lever is Phase 2 (a real
lexicon), not the algorithm — a bigger dictionary should move precision and
F1 the most before Phase 3 (BiMM/UnigramDp) or Phase 4 (HMM/OOV) can be
meaningfully evaluated against it.
