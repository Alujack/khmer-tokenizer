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
below), so `ForwardMaxMatch` stays the enum's `#[default]`. `KhmerTokenizer::with_hmm(...)`
(Phase 4, done) composes with any strategy and lifts R-oov by ~0.05 absolute
with zero R-iv cost (best measured: UnigramDp + HMM, F1 0.7805) — same
"needs a caller-supplied model" posture as `UnigramDp`'s frequencies.
Orthographic normalization (Phase 5, done) is on by default via
`core/src/normalize.rs`; measured effect on the bundled dictionary is exactly
zero (root cause understood and documented — see `docs/BENCHMARKS.md`), but
it's kept on as defense in depth for dictionaries/words that dictionary
doesn't already special-case. All 6 phases below are now done: `cargo test`
(and CI, `.github/workflows/ci.yml`) enforces a regression floor on a small
committed sample (Phase 6), so none of the above can silently rot.

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

## Phase 4 — Unknown-word handling ✅

**Goal:** stop emitting one-cluster-per-token on out-of-vocabulary runs.

- [x] Measure current **R-oov** as the baseline: ~0.35 and essentially flat
      across FMM/BiMM/UnigramDp (0.3505 / 0.3493 / 0.3499 — see
      `BENCHMARKS.md`). Confirms none of the Phase 3 strategies touch OOV
      handling; this phase is the first one that will.
- [x] Added a lightweight cluster-level **HMM + Viterbi** (BMES states,
      `core/src/hmm.rs`: `HmmModel::from_counts` + `segment_oov`), mirroring
      jieba's OOV layer. It's a post-process, not a new `Strategy` variant:
      `KhmerTokenizer::with_hmm(...)` composes with any strategy, and only
      re-segments maximal runs of clusters that strategy matched *nothing*
      in the dictionary for — every genuine dictionary hit (including real
      single-cluster words) passes through untouched (`is_dict_word` check
      in `trie.rs`'s `apply_hmm_fallback`). Counts trained from a segmented
      corpus via `eval::train_hmm` (`eval/src/hmm.rs`): BMES tags per gold
      word (Single for 1 cluster, Begin/Middle*/End for 2+), counted across
      khPOS's `before-replace/train6.word` split — same CC BY-NC-SA,
      local-eval-only constraint already established for `UnigramDp`'s
      frequencies (see `ATTRIBUTION.md`). No derived model ships with the
      crate; `with_hmm` requires the caller to supply one, exactly like
      `with_frequencies`.
- [x] Re-measured R-oov (`docs/BENCHMARKS.md`): **0.3505 → 0.4020** on top of
      `ForwardMaxMatch`, **0.3499 → 0.4014** on top of `UnigramDp` — and
      **R-iv is exactly unchanged** in both cases (0.8144 → 0.8144, 0.8752 →
      0.8752), confirming the strategy-agnostic, dictionary-hits-untouched
      design costs zero IV accuracy in practice. No additional gating beyond
      the existing opt-in (`with_hmm` is `None` unless attached) was needed.

*Exit criteria:* R-oov improves without regressing overall F1. **Met** —
R-oov +0.05 absolute, F1 also improves (0.7216 → 0.7358 FMM+HMM; 0.7661 →
0.7805 UnigramDp+HMM, the best configuration measured to date).

## Phase 5 — Orthographic normalization ✅

**Goal:** stop missing dictionary hits because of Unicode ordering variants.

- [x] Added a normalization pass (`core/src/normalize.rs`): reorders a mark
      (shifter, vowel, or other sign) typed directly before a
      `COENG`+consonant subscript pair to instead follow it, per the Unicode
      Khmer syllable structure (base, [Robat], subscript stack, [Shifter],
      [vowel], [signs]) — Robat is excluded since it's the one mark that's
      *supposed* to precede the stack. This is the single most common
      real-world Khmer encoding error; confirmed by directly grepping
      khPOS's own `OPEN-TEST.word` for the pattern (21 genuine occurrences,
      e.g. `សិទិ្ធ` for `សិទ្ធិ`). **Descoped:** stripping stray ZWJ/ZWNJ
      (`U+200C`/`U+200D`) was in the original plan but is left out — deleting
      characters changes byte length, which would break the eval harness's
      span-based scoring (and any caller relying on byte-accurate
      boundaries) without also building an offset map back to the original
      text, and khPOS's corpus has **zero** ZWJ/ZWNJ occurrences to measure
      it against anyway. On by default; `KhmerTokenizer::without_normalization()`
      opts out (e.g. for exact byte-for-byte comparison against pre-Phase-5
      behavior in the eval harness).
- [x] Added round-trip and idempotency tests (`core/src/normalize.rs`):
      reorders a vowel-before-subscript case and a shifter-before-subscript
      case (both drawn from real corpus occurrences), leaves a legitimate
      Robat-before-subscript alone, is a no-op on already-canonical text,
      is idempotent when applied twice, preserves byte length, and cascades
      correctly through a mark ahead of two stacked subscripts.
- [x] Measured its isolated contribution to F1/R-oov on top of both the
      weakest (`ForwardMaxMatch`) and strongest (`UnigramDp + HMM`)
      configurations (`docs/BENCHMARKS.md`): **exactly zero measured
      effect**, root-caused by cross-referencing all 21 real corpus
      occurrences against `core/src/dict.txt` — chamkho's dictionary already
      bundles the malformed spelling as a duplicate entry for 12 of the 21
      (so plain trie matching already succeeds without normalization), and
      the other 9 are personal names absent from the dictionary in *either*
      spelling. Not one of the 21 flips from wrong to right.

*Exit criteria:* normalization shows a measured, non-negative effect and is
documented as on-by-default (with an opt-out). **Met, precisely**: the
measured effect is zero (non-negative, and zero regression risk confirmed on
the shipped corpus/dictionary), documented on-by-default with
`without_normalization()` as the opt-out. Kept on anyway as defense in
depth — the dictionary's duplicate-entry workaround only covers the specific
words chamkho's maintainers happened to special-case, not custom
dictionaries (`from_words`/`from_dict_str`) or words chamkho missed.

## Phase 6 — Regression guard ✅

**Goal:** accuracy can't silently rot.

- [x] Added `eval/tests/regression.rs`, a `cargo test` integration test —
      not a separate `cargo xtask` command — specifically so the guard needs
      no network access. It runs `KhmerTokenizer::with_default_dict()`
      against `eval/tests/fixtures/regression.word`: 15 hand-authored,
      original sentences (not derived from khPOS or any other corpus, so
      it's freely committable, unlike the gitignored CC BY-NC-SA khPOS
      data). Every word was confirmed present in `core/src/dict.txt` before
      being included, and one line (`សិទិ្ធមនុស្ស`) is a real-world
      malformed spelling on purpose, guarding Phase 5's normalization pass
      specifically. Measured F1 today is exactly 1.0; the assertion floor is
      set at 0.9 — enough headroom that a legitimate future change (a
      different default strategy, a regenerated `dict.txt` with different
      coverage) won't trip it on noise, while a real break (corrupted
      dictionary, broken trie walk, regressed normalizer) still would.
- [x] Added `.github/workflows/ci.yml`: runs `cargo build --workspace`,
      `cargo test --workspace` (which includes the regression guard above),
      and `cargo clippy --workspace --all-targets -- -D warnings` on every
      push/PR to `master`. Deliberately does **not** run `cargo xtask eval`
      in CI — that needs network access to clone khPOS, and its real-corpus
      numbers are already the ones tracked by hand in `BENCHMARKS.md` per
      phase (this is exactly why Phase 6 asked for a *committed synthetic*
      sample rather than gating on khPOS directly). `cargo fmt --check` was
      considered but left out of the gate: this session found it already
      disagrees with itself on pre-existing Khmer-string-heavy code across
      rustfmt versions (line-width calculation for wide Khmer text differs
      across versions), which would make CI flaky for reasons unrelated to
      actual code health.
- [x] `docs/BENCHMARKS.md` has been the running record of every phase's
      metrics since Phase 1 and continues to be — no change needed here
      beyond continuing the existing practice.

*Exit criteria:* Met — `cargo test` (and therefore CI) now fails if the
default tokenizer's F1 on the committed sample drops below 0.9, with no
dependency on network access or an ungated third-party corpus.

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
