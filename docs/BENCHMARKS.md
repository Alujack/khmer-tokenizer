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
| 2026-07-01 | BiMaxMatch      | chamkho khmerdict.txt (59,526 words)      | 0.7072 | 0.7449 | 0.7255 | 0.8184 | 0.3493 | 0.3650   | khPOS OPEN-TEST |
| 2026-07-01 | UnigramDp       | chamkho khmerdict.txt + khPOS-train freqs | 0.7410 | 0.7929 | 0.7661 | 0.8752 | 0.3499 | 0.3770   | khPOS OPEN-TEST |
| 2026-07-02 | ForwardMaxMatch + HMM | chamkho khmerdict.txt + khPOS-train BMES counts | 0.7224 | 0.7498 | 0.7358 | 0.8144 | 0.4020 | 0.3770   | khPOS OPEN-TEST |
| 2026-07-02 | UnigramDp + HMM | chamkho khmerdict.txt + khPOS-train freqs + BMES counts | 0.7611 | 0.8010 | 0.7805 | 0.8752 | 0.4014 | 0.3900   | khPOS OPEN-TEST |
| 2026-07-02 | ForwardMaxMatch + Normalization | chamkho khmerdict.txt (59,526 words) | 0.7026 | 0.7417 | 0.7216 | 0.8144 | 0.3505 | 0.3650   | khPOS OPEN-TEST |
| 2026-07-02 | UnigramDp + HMM + Normalization | chamkho khmerdict.txt + khPOS-train freqs + BMES counts | 0.7611 | 0.8010 | 0.7805 | 0.8752 | 0.4014 | 0.3900   | khPOS OPEN-TEST |

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

## Reading Phase 3's first result (BiMaxMatch vs. ForwardMaxMatch)

A small, real win, as expected for the "cheap intermediate" step:

- **F1 0.7216 → 0.7255**, **precision 0.7026 → 0.7072**, **R-iv 0.8144 →
  0.8184** — BiMM recovers a slice of the ambiguity FMM introduces, by
  picking the backward-max-match result on disagreement when it has fewer
  tokens (or fewer single-cluster tokens on a tie).
- **Word accuracy unchanged (0.3650 both ways):** BiMM only changes a small
  number of ambiguous spans per sentence; not enough sentences flipped from
  wrong to fully-correct (or vice versa) to move this coarser metric.
- **R-oov dipped very slightly (0.3505 → 0.3493):** BiMM's tie-break applies
  to the whole Khmer run, including runs that mix in-vocabulary and
  out-of-vocabulary clusters, so it can occasionally trade an OOV cluster's
  lucky fallback match for a better overall boundary.

## Reading Phase 3's second result (UnigramDp) — the bigger lever, confirmed

Frequency source: word counts from khPOS's `before-replace/train6.word` split
(12,000 sentences, CC BY-NC-SA 4.0). **Computed and used locally by
`cargo xtask eval` only — never bundled, committed, or shipped** (see
`docs/ROADMAP.md` Phase 3 and `core/ATTRIBUTION.md`). Before trusting this as a
fair comparison, confirmed by exact-line overlap that this training split is
effectively disjoint from `OPEN-TEST` (11/1000 incidental matches — the eval
set this table scores against) — it is, however, **100% contained in
`CLOSE-TEST`**, so `CLOSE-TEST` must never be used as an eval set alongside
these frequencies.

- **F1 0.7216/0.7255 → 0.7661**, beating both FMM and BiMM by a wide margin —
  confirms the roadmap's prediction that `UnigramDp` would be the bigger
  lever, not `BiMaxMatch`.
- **R-iv 0.8144/0.8184 → 0.8752**: real frequency data resolves a lot of the
  longest-match ambiguity that a 59,526-word dictionary introduces — far more
  than BiMM's structural tie-break could reach (BiMM can only choose between
  the forward and backward greedy paths; `UnigramDp`'s DAG can represent and
  score paths neither greedy walk can even produce).
- **R-oov flat (~0.35 across all three):** frequency scoring disambiguates
  between *competing dictionary matches* — it doesn't help clusters with zero
  dictionary matches at all, which is what R-oov measures. Expected; that's
  Phase 4's job (HMM/Viterbi fallback for OOV runs).

**Default strategy decision (`ROADMAP.md`'s open item #3):** `Strategy`'s
`#[default]` stays `ForwardMaxMatch`. Without a bundled frequency table (see
`with_frequencies`'s doc comment for why none ships), `UnigramDp` silently
falls back to `ForwardMaxMatch` anyway for anyone who doesn't supply their
own — so changing the nominal default wouldn't change any out-of-the-box
behavior. The real, actionable finding: **use `UnigramDp` with your own
frequency table whenever you have one** — it's the best of the three by a
clear margin.

## Reading Phase 4's result (HMM fallback for OOV runs)

Training data: BMES tag counts gathered from khPOS's `before-replace/train6.word`
split (same split, same disjointness/leakage caveat as `UnigramDp`'s
frequencies above — see `docs/ROADMAP.md` Phase 4 and `core/ATTRIBUTION.md`).
`with_hmm(...)` composes with any `Strategy`; it only re-segments maximal runs
of clusters that strategy matched *nothing* in the dictionary for at all —
every genuine dictionary hit (including real single-cluster words) is passed
through untouched. Measured on top of both the weakest and strongest Phase 3
strategy to see whether that holds:

- **R-oov 0.3505 → 0.4020 (FMM+HMM)** and **0.3499 → 0.4014 (UnigramDp+HMM)** —
  roughly a 5-point absolute (~15% relative) recall gain on exactly the
  words neither dictionary coverage nor frequency scoring could ever touch,
  confirming the roadmap's framing: R-oov was flat across all of Phase 3
  because none of those strategies do anything for clusters with zero
  dictionary matches — this is the first change that does.
- **R-iv exactly unchanged (0.8144 → 0.8144, 0.8752 → 0.8752):** the
  strategy-agnostic design — only ever touching runs of already-established
  dictionary-fallback single clusters — costs zero in-vocabulary accuracy in
  practice, not just in principle. This resolves the roadmap's "gate behind a
  strategy flag if it costs IV accuracy" condition: it doesn't cost anything
  measured here, so the existing opt-in (`with_hmm(...)` is `None` unless a
  caller attaches a model, exactly like `with_frequencies`) is gate enough —
  no additional `Strategy` variant needed.
- **F1 also improves** (0.7216 → 0.7358 FMM; 0.7661 → 0.7805 UnigramDp) and
  **word accuracy ticks up** (0.3650 → 0.3770; 0.3770 → 0.3900) — since R-oov
  recall gains flow straight into fewer over-segmented sentences, on top of
  whatever the base strategy already got right.
- **UnigramDp + HMM is the best configuration measured to date** (F1 0.7805,
  R-iv 0.8752, R-oov 0.4014) — the two Phase 3/4 levers (frequency-scored
  dictionary matches, statistically-guessed OOV runs) are complementary, not
  overlapping: one only ever operates where the dictionary matched something,
  the other only where it matched nothing.

## Reading Phase 5's result (orthographic normalization) — measured zero effect, root-caused

`core/src/normalize.rs` reorders a mark (shifter, vowel, or other sign) that
was typed directly before a `COENG`+consonant subscript pair to instead
follow it — the single most common real-world Khmer encoding error. This
isn't hypothetical: `grep`-ing khPOS's real `OPEN-TEST.word` file for the
pattern found **21 genuine occurrences** (e.g. `សិទិ្ធ`, a well-known typo
for `សិទ្ធិ` "rights"; `ស៊្រុន`, `ប៉្រាត`, `ប៉្រេម` — transliterated
personal names with a shifter typed before its subscript). Zero occurrences
of stray ZWJ/ZWNJ were found, so — since deleting characters would change
byte length and break the eval harness's span-based scoring without also
building an offset map back to the original text — that half of the
original Phase 4 roadmap item was descoped to just the reordering fix; see
`core/src/normalize.rs`'s module doc and `docs/ROADMAP.md` Phase 5 for the
full reasoning.

- **Measured effect: exactly zero** — `ForwardMaxMatch` and
  `UnigramDp + HMM` score byte-for-byte identically with normalization on
  vs. off (both rows above match their `without_normalization()`
  counterparts to 4 decimal places).
- **Root cause, confirmed by directly cross-referencing all 21 corpus
  occurrences against `core/src/dict.txt`:** chamkho's dictionary already
  bundles the malformed spelling as a **separate, duplicate entry** right
  next to the canonical one for 12 of the 21 (e.g. both `កម្មសិទិ្ធ` *and*
  `កម្មសិទ្ធិ` are independent dictionary words) — so the plain trie match
  already succeeds without any normalization. The other 9 are personal
  names absent from the dictionary **in either spelling** (proper names
  aren't general-vocabulary words), so normalizing them doesn't create a
  match where none existed. Not one of the 21 flips from wrong to right —
  confirmed by re-implementing the same reorder rule directly against the
  corpus and dictionary in a standalone script, independent of the Rust
  eval harness.
- **This still meets the roadmap's exit criteria** ("a measured,
  non-negative effect"): zero is non-negative, and — more importantly — it
  confirms the pass carries **zero regression risk** on the corpus and
  dictionary this project ships. Kept on by default anyway, as defense in
  depth: the dictionary's duplicate-entry workaround is specific to the
  handful of words chamkho's maintainers happened to special-case, not a
  general fix. Any custom dictionary a caller supplies (`from_words`,
  `from_dict_str`), any word chamkho didn't special-case, and any future
  deduplication of `dict.txt` would all still benefit from normalizing the
  input rather than relying on that duplication.

## Cross-corpus check: kh_data_10000b (silver reference — treat with caution)

A second, much larger corpus: ~10,000 real-world Khmer web articles as
`<id>_orig.txt` / `<id>_seg_200b.txt` pairs (word boundaries marked with
`U+200B` ZERO WIDTH SPACE). **Not committed** (`data/` is gitignored) and
**not gold**: no license or provenance ships with it, and the `_200b`
segmentation looks machine-produced rather than human-verified — so these
numbers measure *agreement with that unknown system*, not accuracy on the
same footing as khPOS above. Loader: `eval/src/kh10000b.rs`; run with
`cargo xtask eval-kh10000b` (never auto-downloaded). Alignment validation
during loading accepted 9,943/10,000 pairs (the 57 rejects differ only by a
`•`→`-` bullet normalization in the segmented copy), yielding **80,216
sentences** — 80× khPOS's OPEN-TEST.

| Date       | Strategy        | P      | R      | F1     | R-iv   | R-oov  | Word Acc | Corpus (reference type) |
| ---------- | --------------- | ------ | ------ | ------ | ------ | ------ | -------- | ----------------------- |
| 2026-07-02 | ForwardMaxMatch | 0.7041 | 0.7048 | 0.7044 | 0.7836 | 0.3376 | 0.0463   | kh_data_10000b (silver) |
| 2026-07-02 | BiMaxMatch      | 0.7101 | 0.7082 | 0.7092 | 0.7879 | 0.3370 | 0.0466   | kh_data_10000b (silver) |
| 2026-07-02 | ForwardMaxMatch + Normalization | 0.7042 | 0.7048 | 0.7045 | 0.7836 | 0.3378 | 0.0463 | kh_data_10000b (silver) |

Reading it:

- **F1 generalizes** — 0.70–0.71 here vs. 0.72–0.73 on khPOS, on an
  independently-sourced corpus 80× larger, against a different reference
  system. The tokenizer is not overfit to the one academic benchmark.
- **Word accuracy is NOT comparable across corpora** — 4.6% here vs. 36.5%
  on khPOS looks alarming but is a sentence-length artifact: this corpus
  averages **38.9 tokens/line** vs. khPOS's 10.8, and word accuracy demands
  a perfect full-sentence match, so it collapses combinatorially with
  length (≈ p^n) even at a constant per-token error rate. F1/R-iv are the
  cross-corpus-comparable numbers.
- **Phase 5's "normalization ≈ zero effect" replicates** at 80× the sample
  size (F1 0.7044 → 0.7045).

## Post-roadmap changes from RESEARCH-3 (NFC repair rule + ZWSP boundaries)

Two changes landed after Phase 6, driven by the verified findings in
[RESEARCH-3.md](./RESEARCH-3.md) §2:

1. **Normalizer rule 2** (`core/src/normalize.rs`): repairs a mark stranded
   between `COENG` and its consonant — the corruption Unicode NFC itself
   inflicts on Khmer text (erroneous, permanently frozen canonical
   combining classes; reproduced locally with `unicodedata` before
   implementing). ZWNJ/ZWJ were simultaneously exempted from *both*
   reorder rules (their meaning is position-sensitive — moving them was a
   latent bug in the original rule).
2. **ZWSP as a trusted boundary** (`core/src/trie.rs`): `U+200B` — the
   Unicode-recommended Khmer word-boundary marker — is now consumed as a
   separator instead of leaking through as a stray standalone token. The
   CLI gained `--zwsp` to emit it back out.

Measured effect, per the no-silent-changes rule:

- **khPOS OPEN-TEST: every row above reproduces byte-for-byte** (verified
  by a full re-run) — khPOS isn't NFC-processed (0/1,000 lines altered by
  NFC), contains zero ZWJ/ZWNJ, and the eval reconstructs inputs from gold
  tokens (no ZWSP), so nothing there exercises the new paths. Same for
  kh_data_10000b's rows. Zero regression, zero flattering delta to report.
- The ZWSP change is a **raw-input correctness fix, not an eval-metric
  one**: kh_data_10000b's 10,000 raw articles contain **1,303,740 ZWSP
  characters (avg 130/document)**, every one of which the previous
  behavior emitted as a stray invisible token (and glued into adjacent
  non-Khmer tokens). Anyone feeding real Khmer web text through the CLI
  was getting ~130 garbage tokens per article; now each one is consumed as
  the word boundary it's defined to be.
- The NFC rule's value is likewise defensive: it only fires on
  NFC-processed input, which neither eval corpus is — but which most
  Python/scraping pipelines produce. Both rules carry the same
  measured-zero, defense-in-depth status as Phase 5's original rule, now
  with the failure mode documented as *provable* (TN61) rather than
  hypothetical.

## Post-roadmap: the CRF-class tagger tier (averaged structured perceptron)

`core/src/tagger.rs` adds the tier RESEARCH-3 §4 identified between this
project's dictionary strategies and neural SOTA: an **averaged structured
perceptron** BMES tagger over KCC clusters (Collins 2002 — the classic
dependency-free stand-in for a CRF), decoded with Viterbi, deterministic
in training and inference. Features per cluster: identity, neighbors at
±1/±2, adjacent bigrams, cluster length. Like the HMM and the UnigramDp
frequencies, **no trained model ships** (khPOS is CC BY-NC-SA); training
here uses khPOS's train split (disjoint from OPEN-TEST), 5 epochs.

khPOS OPEN-TEST (1,000 sentences), all rows `without_normalization()` for
comparability with the tables above:

| Configuration | P | R | F1 | R-iv | R-oov | WordAcc |
|---|---|---|---|---|---|---|
| FMM + HMM (prior fallback)       | 0.7224 | 0.7498 | 0.7358 | 0.8144 | 0.4020 | 0.3770 |
| FMM + Tagger fallback            | 0.7260 | 0.7518 | 0.7387 | 0.8144 | 0.4150 | 0.3770 |
| UnigramDp + HMM (prior best)     | 0.7611 | 0.8010 | 0.7805 | 0.8752 | 0.4014 | 0.3900 |
| UnigramDp + Tagger fallback      | 0.7647 | 0.8030 | **0.7834** | 0.8752 | 0.4144 | 0.3900 |
| **Tagger full (`Strategy::Tagger`)** | 0.9260 | 0.9341 | **0.9300** | 0.9409 | 0.8976 | 0.7850 |

Reading it:

- **As an OOV fallback**, the tagger is a strict upgrade over the HMM in
  every configuration: R-oov 0.4020 → 0.4150 under FMM, and the best
  hybrid F1 moves 0.7805 → 0.7834. R-iv is untouched (0.8752), by
  construction — the fallback still only ever touches truly-unmatched
  runs. When both are attached the tagger wins; `with_hmm` remains for
  zero-training-cost setups.
- **As a full segmenter** (`Strategy::Tagger`, dictionary ignored), it
  jumps to **F1 0.9300** — a +0.15 absolute leap over the best dictionary
  configuration, landing squarely in the CRF-tool tier RESEARCH-3
  projected (khmercut-class, reported ~0.95 on comparable splits). Word
  accuracy nearly doubles (0.39 → 0.785).
- **Caveat on R-oov 0.8976:** "OOV" in this harness means *absent from
  the 59,526-word chamkho dictionary* — the full tagger doesn't use that
  dictionary, so this is not comparable to neural papers' OOV-vs-training
  numbers (UnifiedCut's 0.613 is measured against its training
  vocabulary). It *is* fair evidence that the tagger generalizes past the
  dictionary's coverage.
- **The trade**: `Strategy::Tagger` needs a trained model (a corpus you're
  licensed to use) and gives up the dictionary strategies' zero-setup
  determinism-by-construction. The dictionary tiers stay the default; the
  tagger is opt-in, persisted via `TaggerModel::to_text`/`from_text`.
- Historical rows were re-run and **reproduce byte-for-byte** — the
  fallback-seam refactor (`apply_hmm_fallback` → `apply_oov_fallback`)
  changed no existing behavior.

### Cross-corpus honesty check + adversarial hardening

The F1 0.9300 above is **in-domain**: khPOS's train and test splits share
annotators and segmentation conventions, which flatters any learned model.
To measure real generalization, the same khPOS-trained model (identical
weights, 5 epochs, deterministic) was evaluated against kh_data_10000b —
80,216 silver-reference sentences from a domain it never saw:

| Configuration (kh10000b, silver) | P | R | F1 | R-iv | R-oov | WordAcc |
|---|---|---|---|---|---|---|
| BiMaxMatch (best dictionary row) | 0.7101 | 0.7082 | 0.7092 | 0.7879 | 0.3370 | 0.0466 |
| FMM + Tagger fallback (CROSS)    | 0.7427 | 0.7199 | 0.7311 | 0.7836 | 0.4235 | 0.0507 |
| Tagger full (CROSS)              | 0.8794 | 0.8531 | **0.8660** | 0.8889 | 0.6863 | 0.1481 |

Reading it honestly:

- The full tagger **loses 6.4 F1 points out of domain** (0.9300 → 0.8660)
  and a quarter of its OOV recall (0.8976 → 0.6863). That's the real
  domain gap, stated up front.
- It still beats the best dictionary configuration on this corpus by
  **+0.157 F1** and triples word accuracy — the advantage survives domain
  transfer; it isn't a split artifact.
- Usual silver caveat: kh10000b's reference is machine-produced, so this
  measures agreement, not verified accuracy — but both tiers are measured
  against the *same* reference, so the comparison between them stands.

The same skepticism pass adversarially probed the implementation and
found (and fixed) two real bugs, now regression-tested:

1. **Serialization round-trip violation** — `TaggerModel::train` accepts
   arbitrary words; a tab or newline inside one ended up inside a feature
   key and produced a model file `from_text` itself rejected. Keys are now
   escaped (`\t`/`\n`/`\r`/`\\`).
2. **NaN-poisoned models loaded silently** — `"NaN"`/`"inf"` parse as
   valid `f64`, and NaN makes every Viterbi comparison false. `from_text`
   now rejects non-finite weights at load.

Also probed and confirmed fine: zero-epoch (all-zero-weight) models
degrade to one-token-per-cluster; a 5,000-cluster single run decodes in
~26 ms.
