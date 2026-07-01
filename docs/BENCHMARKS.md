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

| Date       | Strategy        | Dictionary                              | P      | R      | F1     | R-iv   | R-oov  | Word Acc | Corpus          |
| ---------- | --------------- | ---------------------------------------- | ------ | ------ | ------ | ------ | ------ | -------- | ---------------- |
| 2026-07-01 | ForwardMaxMatch | seed dict (~100 words)                    | 0.1576 | 0.3501 | 0.2174 | 0.9990 | 0.2047 | 0.0050   | khPOS OPEN-TEST |
| 2026-07-01 | ForwardMaxMatch | chamkho khmerdict.txt (59,526 words)      | 0.7026 | 0.7417 | 0.7216 | 0.8144 | 0.3505 | 0.3650   | khPOS OPEN-TEST |

## Reading Phase 1's baseline (~100-word seed dict)

- **R-iv ≈ 1.0**: the trie/longest-match mechanism itself is correct — when
  a word is actually in the dictionary, it's recovered almost perfectly.
- **Low P/F1**: the seed dictionary covers a small fraction of the corpus's
  vocabulary, so most words are out-of-vocabulary and fall back to
  single-cluster tokens, badly over-segmenting the output (many more
  predicted tokens than gold tokens, which drives precision down even
  though the words that *do* match are counted correctly).
- **R-oov ≈ 0.20**: about a fifth of OOV gold words happen to be a single
  KCC cluster already, so the naive fallback gets them right by luck.

## Reading Phase 2's lift (59,526-word dictionary)

Confirms `ROADMAP.md`'s framing — dictionary coverage was the biggest lever:

- **F1 0.2174 → 0.7216** and **word accuracy 0.5% → 36.5%** from swapping the
  dictionary alone; no algorithm change.
- **R-oov 0.2047 → 0.3505**: fewer gold words are truly out-of-vocabulary now,
  and the ones that remain are still sometimes matched right by luck.
- **R-iv 0.9990 → 0.8144 (a regression, expected):** with only ~100 words,
  every in-vocabulary hit was an easy, unambiguous word. With 59,526 words
  there's real longest-match ambiguity — greedy forward-max-match now
  sometimes swallows a dictionary word into a longer neighboring dictionary
  entry, picking the wrong boundary even though *a* dictionary word matched.
  This is exactly the greedy-FMM failure mode Phase 3 (BiMM / UnigramDp) is
  designed to fix, and it's now visible and measurable rather than masked by
  a tiny dictionary.

This sets up Phase 3 to be evaluated meaningfully: BiMM/UnigramDp's job is to
recover the R-iv/precision this larger dictionary gave up to ambiguity.
