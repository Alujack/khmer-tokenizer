# Roadmap — Accuracy & Evaluation

The next phase makes `khmer-tokenizer` *measurable* and then *better*. Guiding
rule: **no accuracy change lands without a before/after number from the harness.**
Background and citations: [RESEARCH.md](./RESEARCH.md).

Current default: forward maximum-matching over a Khmer-Character-Cluster trie,
with the 59,526-word dictionary from Phase 2. `Strategy::BiMaxMatch` and
`Strategy::UnigramDp` also exist (Phase 3, done) — see
[BENCHMARKS.md](./BENCHMARKS.md) (FMM: F1 0.7216; BiMM: F1 0.7255; UnigramDp:
F1 0.7661, all against khPOS OPEN-TEST). `UnigramDp` is the clear winner when
frequencies are available, but the crate ships none by default (see Phase 3
below), so `ForwardMaxMatch` stays the enum's `#[default]`.

---

## Phase 1 — Evaluation harness (do this first) ✅

**Goal:** a repeatable command that prints token-level Precision / Recall / F1
(plus OOV recall) for any strategy against a gold corpus, and records a baseline.

- [x] Add an `eval/` crate (or a `cargo xtask eval` task) that depends on `core`.
- [x] Corpus loader for **khPOS** (`OPEN-TEST` + closed-test). Derive each
      example as `(input_without_spaces, gold_tokens)`. Confirmed the file's
      word delimiter (single space; compound words joined with an internal
      `_`/`~`, stripped by the parser) against the real `data/khpos/` clone.
- [x] Implement metrics: token-span **P / R / F1**, **R-oov** and **R-iv**
      (needs the dictionary's vocab), and word-level accuracy. Match the
      SIGHAN convention (a token is correct only if both boundaries match).
- [x] `cargo xtask eval` downloads the corpus to `data/` (which is
      `.gitignore`d — corpora are **CC BY-NC-SA**, never committed) and prints a
      table.
- [x] **Record the baseline** for forward-MM + seed dict in
      `docs/BENCHMARKS.md`.

*Exit criteria:* `cargo xtask eval` prints P/R/F1 for the current engine. **Met.**

## Phase 2 — Real dictionary ✅

**Goal:** replace the seed list; measure the lift from coverage alone.

- [x] Integrate a permissive lexicon. **Changed from the original plan:**
      [khopilot/khmer-lexicon](https://huggingface.co/datasets/khopilot/khmer-lexicon)
      (CC BY 4.0) turned out to be **gated** on HuggingFace — it needs an
      authenticated, terms-accepted account and a personal access token, which
      no automated `xtask` can obtain, and it ships only as Parquet. Used
      [chamkho](https://github.com/veer66/chamkho)'s `khmerdict.txt` instead:
      59,526 words, ungated, plain text, its own standalone MIT license file
      (`LICENSE-khmerdict`, copyright SIL NRSI). Documented in
      `ATTRIBUTION.md`, including why khmer-lexicon was passed over.
- [ ] ~~Add a frequency table from silnrsi/khmerlbdict / SEALang~~ — **deferred
      to Phase 3.** Inspected `khmerlbdict` directly: its `LICENSE` (MIT) only
      covers the tooling: the wordlist data itself is compiled from SEALang,
      CLDR, Khmer Bible translations, and excerpts of specific published
      books, none of which have a stated permissive license in that repo. Not
      safe to bundle as-is. `khmerdict.txt` has no frequency field either, so
      Phase 3 needs to either source frequencies separately (e.g. count them
      from khPOS's *training* split, which is fine for local, non-bundled use)
      or revisit `khopilot/khmer-lexicon` if an HF token becomes available.
- [x] Data-prep script (`cargo xtask prepare-dict`, `xtask/src/dict.rs` +
      `download.rs`): clones chamkho, trims/dedupes/drops blank or comment
      lines, emits `core/src/dict.txt`. (No bad rows existed in the source —
      verified empirically: 0 blank lines, 0 duplicates, 0 non-Khmer/ASCII
      contamination.)
- [x] Re-ran the harness; logged the delta in `BENCHMARKS.md`.

*Exit criteria:* F1 improvement from dictionary coverage is quantified. **Met**
— F1 0.2174 → 0.7216.

## Phase 3 — Scored segmentation (the algorithmic upgrade) ✅

**Goal:** beat greedy longest-match on ambiguous input.

- [x] Introduce a `Strategy` enum (`core/src/strategy.rs`): `ForwardMaxMatch`
      (default, stays deterministic), `BiMaxMatch`, `UnigramDp`. Selected via
      `KhmerTokenizer::with_strategy(...)`; FMM/BiMM also exposed as `cli
      --strategy fmm|bimm` (`UnigramDp` isn't, since the CLI has no mechanism
      yet to load an external frequency file — see the note below).
- [x] Implement **bidirectional max-match** (`core/src/trie.rs`: `rev_root` +
      `backward_match` + `bimm`) as a cheap intermediate — forward + backward
      over the same cluster run; on disagreement, fewer tokens wins, then
      fewer single-cluster tokens, then forward (Bi & Taing, APSIPA 2014).
- [x] Implement **unigram max-probability path** (jieba-style, `core/src/trie.rs`
      `unigram_dp`): a DAG of every dictionary match starting at each
      position (not just the longest — this is what lets it represent paths
      neither greedy walk can reach), then right-to-left DP for the highest
      cumulative log-probability path. OOV words get a floor count of 1
      (penalized, not impossible). Frequencies are supplied via
      `KhmerTokenizer::with_frequencies(...)` — **no table ships with the
      crate**: no bundleable, commercially-clean corpus-frequency source was
      found (see the Phase 2 licensing notes). Falls back to
      `ForwardMaxMatch` if no frequencies are set.
      **Frequency source used for evaluation:** word counts from khPOS's
      `before-replace/train6.word` split (12,000 sentences, CC BY-NC-SA),
      computed by `cargo xtask eval` **locally only** — never bundled,
      committed, or shipped. Confirmed by exact-line overlap that this split
      is effectively disjoint from `OPEN-TEST` (the eval set) but **100%
      contained in `CLOSE-TEST`** — so `CLOSE-TEST` must never be used as an
      eval set alongside these frequencies.
- [x] Benchmarked all three on the harness (`docs/BENCHMARKS.md`): `UnigramDp`
      wins decisively (F1 0.7216 → 0.7255 (BiMM) → **0.7661** (UnigramDp);
      R-iv 0.8144 → 0.8184 → **0.8752**), confirming it as the expected bigger
      lever over BiMM. **Default stays `ForwardMaxMatch`** — resolves the
      "Decisions to confirm" item below: since no frequency table is bundled,
      `UnigramDp` silently degrades to FMM for anyone who doesn't supply
      their own, so changing the enum's nominal default wouldn't change any
      out-of-the-box behavior. The actionable takeaway instead: use
      `UnigramDp` with your own frequencies whenever you have them.

*Exit criteria:* `UnigramDp` (or BiMM) shows a measured F1 gain over
forward-MM. **Met** — both do; `UnigramDp`'s gain is the larger one, as
predicted.

## Phase 4 — Unknown-word handling

**Goal:** stop emitting one-cluster-per-token on out-of-vocabulary runs.

- [x] Measure current **R-oov** as the baseline: ~0.35 and essentially flat
      across FMM/BiMM/UnigramDp (0.3505 / 0.3493 / 0.3499 — see
      `BENCHMARKS.md`). Confirms none of the Phase 3 strategies touch OOV
      handling; this phase is the first one that will.
- [ ] Add a lightweight cluster-level **HMM + Viterbi** (BMES states) for runs the
      dictionary misses, mirroring jieba's OOV layer. Train counts from a
      segmented corpus (document the NC-license constraint on shipping any
      derived model).
- [ ] Re-measure R-oov; gate behind a strategy flag if it costs IV accuracy.

*Exit criteria:* R-oov improves without regressing overall F1.

## Phase 5 — Orthographic normalization

**Goal:** stop missing dictionary hits because of Unicode ordering variants.

- [ ] Add a normalization pass (canonical COENG ordering, vowel/sign reorder,
      strip zero-width joiners where spurious) applied before segmentation.
- [ ] Add round-trip and idempotency tests on the normalizer.
- [ ] Measure its isolated contribution to F1 / R-oov.

*Exit criteria:* normalization shows a measured, non-negative effect and is
documented as on-by-default (with an opt-out).

## Phase 6 — Regression guard

**Goal:** accuracy can't silently rot.

- [ ] CI job runs `cargo test` + the eval harness on a small committed *synthetic*
      sample (license-safe) and fails if F1 drops below a threshold.
- [ ] Keep `docs/BENCHMARKS.md` as the running record of metric numbers per
      change.

---

## Proposed layout after this phase

```text
khmerTokenizer/
├── core/                 # engine (strategy.rs + scoring built-in; normalize.rs remaining)
├── cli/                  # gains a --strategy flag
├── eval/                 # corpus loaders + P/R/F1 harness   (new)
├── xtask/                # download/prepare-dict/eval automation (new)
├── data/                 # downloaded corpora — .gitignored   (new)
└── docs/
    ├── RESEARCH.md
    ├── ROADMAP.md
    └── BENCHMARKS.md      # running metric log               (new)
```

## Decisions to confirm before starting

1. **Harness home:** a dedicated `eval/` crate vs. a `cargo xtask` (lighter, no
   published crate). Recommendation: `xtask` + a thin `eval` lib module.
2. **Primary corpus:** khPOS only to start, or khPOS + ALT for cross-domain?
   Recommendation: khPOS first, add ALT in Phase 3 once the harness is proven.
3. **Default strategy after Phase 3:** keep `ForwardMaxMatch` as default for
   determinism/speed, or promote `UnigramDp` once it wins? Decide on the
   numbers. **Still open:** BiMM's numbers are in (a small win — see
   `BENCHMARKS.md`) but not enough to justify switching the default off FMM by
   itself; revisit once `UnigramDp` exists and all three can be compared.
4. **License posture:** confirm we will *only* bundle CC BY / MIT-class data and
   keep all NC / ShareAlike corpora download-only. (Recommended — protects the
   MIT/Apache licensing of the project.)

## Sequencing

Phase 1 is the unlock and should ship on its own. 2 → 3 are the main accuracy
gains and depend on the harness. 4 → 5 are refinements measured against it. 6
locks the gains in. Each phase is independently shippable and individually
measurable.
