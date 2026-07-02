```

```

# Architecture

How `khmer-tokenizer` is put together — the pieces, how data flows through them,
and how today's simple dictionary engine grows into a data-and-training platform
without rewrites. Pair this with [LEARNING.md](./LEARNING.md) (what to learn) and
[ROADMAP.md](./ROADMAP.md) (what to build, in order).

## The one idea to hold onto

Everything is a **pipeline of small, replaceable stages**:

```
raw text → normalize → split into clusters → choose word boundaries → tokens
```

Each stage does one job and hands a clean result to the next. Because the stages
are separate, you can improve or swap any one of them — a better normalizer, a
smarter boundary chooser, a bigger dictionary — without touching the others. That
separation is the whole reason the project can start tiny and still grow into
model training later.

## Components

| Component        | Crate / file               | Job                                          | Status     |
| ---------------- | -------------------------- | -------------------------------------------- | ---------- |
| KCC splitter     | `core/src/kcc.rs`        | group bytes into Khmer Character Clusters    | ✅ built   |
| Trie             | `core/src/trie.rs`       | store the dictionary, find word matches fast | ✅ built   |
| Tokenizer API    | `core/src/lib.rs`        | public entry point, dictionary loading       | ✅ built   |
| CLI              | `cli/src/main.rs`        | run it from the terminal / pipes             | ✅ built   |
| Normalizer       | `core/src/normalize.rs`  | canonicalize Unicode ordering variants       | ✅ built (Phase 5 — see below) |
| Strategy         | `core/src/strategy.rs`   | pick the boundary algorithm                  | ✅ built (FMM, BiMM, UnigramDp — see below) |
| HMM OOV fallback | `core/src/hmm.rs`      | guess boundaries where the dictionary matched nothing at all | ✅ built (Phase 4 — see below) |
| Eval harness     | `eval/` + `xtask`      | measure P/R/F1 on a gold corpus              | ✅ built   |
| Regression guard | `eval/tests/regression.rs` + CI | fail the build if accuracy silently rots | ✅ built (Phase 6 — see below) |
| Statistical tagger | `core/src/tagger.rs` | CRF-class averaged-perceptron BMES tagger: OOV fallback (`with_tagger`) or full segmenter (`Strategy::Tagger`); no model ships — train + persist your own | ✅ built (F1 0.93 full-mode on khPOS — see BENCHMARKS.md) |
| Bindings         | `py/` (PyO3), `wasm/`  | run from Python and JS/browser               | ✅ Python on PyPI (`pip install khmer-tokenizer`; release workflow builds all-platform wheels); ✅ WASM on npm (`npm install kh-tokenizer`) |

## Today's default pipeline (`Strategy::ForwardMaxMatch`)

```mermaid
flowchart LR
    A[Raw Khmer text] --> N[Normalizer<br/>Phase 5]
    N --> B[KCC splitter]
    B --> C[Cluster trie<br/>longest match]
    C --> D[Tokens]
    E[(dict.txt<br/>embedded)] --> C
```

What happens, in words: the text is first passed through the normalizer
(`core/src/normalize.rs`, on by default — see below), then split into
clusters so a base letter never gets cut away from its subscripts/vowels;
then the segmenter walks a trie built from the dictionary and, at each
position, takes the longest run of clusters that spells a real word. No
match → it emits one cluster and moves on. Whitespace and `U+200B` ZERO
WIDTH SPACE (the Unicode-recommended Khmer word-boundary marker, common as
an invisible hint in real Khmer web text) act as trusted separators —
consumed, never emitted, never merged across. Deterministic, no model,
microsecond-fast.

## The normalizer (built, Phase 5)

Written Khmer's encoding has a well-documented failure mode: per the Unicode
Khmer syllable structure (base, optional Robat, subscript stack, optional
shifter, dependent vowel, other signs), a shifter/vowel/sign is sometimes
typed or encoded *before* the subscript stack instead of after it — e.g.
`សិទិ្ធ` for the correct `សិទ្ធិ` ("rights"). Because clustering is purely
structural (it just consumes whatever combining marks and `COENG` pairs
follow a base, in whatever order they appear), a malformed and a canonical
spelling of the same word produce two *different* cluster strings — so the
dictionary trie, keyed on exact cluster sequences, needs both spellings
listed separately to match either one.

`normalize()` fixes this before clustering ever runs, with two mirror-image
rules run to a fixed point: a mark immediately *before* a `COENG`+consonant
pair moves after it (the typed-error form), and a mark stranded *between*
`COENG` and its consonant moves past the consonant — that second form is
what Unicode NFC itself produces on Khmer text, because Khmer's canonical
combining classes are erroneous and frozen (ccc(COENG)=9 vs
ccc(ATTHACAN)=230 — see `docs/RESEARCH-3.md` §2a), making every
NFC-processing pipeline a standing corruption vector this repairs. Both
rules are pure character reordering — never adding or removing a character
— so normalization is byte-length-preserving and never shifts token
boundaries relative to the raw input's byte offsets, which matters because
the eval harness's span-based scoring depends on that alignment. Robat and
the zero-width joiners are exempt from both rules (Robat legitimately
precedes the subscript stack; ZWNJ/ZWJ meaning is position-sensitive).

Measured contribution on khPOS + the bundled dictionary is **exactly
zero** (see `docs/BENCHMARKS.md` Phase 5) — not because the fix doesn't
work (its unit tests confirm it does, on real examples pulled from the
corpus), but because chamkho's dictionary already lists many common
malformed spellings as separate, duplicate entries right next to the
canonical one, so plain trie matching already succeeds without any
normalization. It's kept on by default anyway (`without_normalization()`
opts out) as defense in depth: that duplicate-entry workaround only covers
the specific words chamkho's maintainers happened to special-case, not any
custom dictionary a caller supplies via `from_words`/`from_dict_str`.

## The Strategy seam (built)

The boundary-choosing logic sits behind one interface, so the *how* can
change while the *what* stays stable:

```mermaid
flowchart TB
    IN[clusters] --> S{Strategy}
    S -->|default| FM[Forward max-match]
    S -->|BiMaxMatch| BM[Bidirectional max-match]
    S -->|UnigramDp| DP[Unigram max-probability path]
    S -->|future| ML[Trained CRF / neural model]
    FM --> OUT[tokens]
    BM --> OUT
    DP --> OUT
    ML --> OUT
```

```rust
// The seam: callers never change, the engine behind it can.
pub enum Strategy {
    ForwardMaxMatch,   // default (determinism/speed) — always available
    BiMaxMatch,        // cheap accuracy bump, no extra data needed
    UnigramDp,         // best of the three, but needs KhmerTokenizer::with_frequencies(...) —
                        // no frequency table ships with the crate (see BENCHMARKS.md Phase 3)
    // Model(Box<dyn Segmenter>) // future — a trained model, same API
}
```

A user who writes `tokenizer.segment(text)` doesn't need to change anything
when switching strategies — `KhmerTokenizer::with_strategy(Strategy::BiMaxMatch)`
chains onto any constructor, and the CLI exposes all four strategies the
same way via `--strategy fmm|bimm|unigram|tagger`, with `--dict`, `--freq`,
and `--tagger` flags to load the data the stronger tiers require (a model
file is produced locally by `cargo xtask train-tagger`).

`UnigramDp`'s DAG-plus-DP approach is a real algorithmic step up from the
other two, not just a variant: `greedy_match` (used by both FMM and BiMM)
only ever records the *longest* dictionary match at each trie-walk position,
so neither can represent — let alone choose — a competing shorter match that
leads to a better global path. `unigram_dp` records *every* match ending
position as a DAG edge, then a right-to-left dynamic-programming pass over
that DAG picks the path with the highest cumulative log-probability. That
structural difference is why it measurably outperforms BiMM in
`docs/BENCHMARKS.md`, not just a better tie-break rule.

A user who writes `tokenizer.segment(text)` today keeps working when you later
drop in a trained model. That stability is what lets you experiment freely.

## The HMM OOV fallback (built, Phase 4)

Every strategy above shares one blind spot: when a run of clusters matches
*nothing* in the dictionary, they all fall back to one token per cluster —
`Strategy` only ever chooses between different ways of walking the trie, and
a trie has nothing to say about clusters it has no entry for at all. That's
what `R-oov` in `docs/BENCHMARKS.md` measures, and why it stayed flat
(~0.35) across all three Phase 3 strategies.

`KhmerTokenizer::with_hmm(...)` is an orthogonal knob, not a fourth
`Strategy` variant, because it operates on the *output* of whichever
strategy ran, not on the trie walk itself:

```mermaid
flowchart LR
    S[Strategy output<br/>tokens] --> C{token is a single<br/>cluster AND not a<br/>real dictionary word?}
    C -->|no| KEEP[keep token as-is]
    C -->|yes| BUF[buffer cluster]
    BUF --> RUN{next token breaks<br/>the buffer?}
    RUN -->|yes| HMM[HMM Viterbi<br/>BMES decode]
    HMM --> OUT[replacement tokens]
    KEEP --> OUT2[tokens]
    OUT --> OUT2
```

Concretely (`trie.rs`'s `apply_hmm_fallback` + `is_dict_word`): scan the
strategy's token output; any *maximal run* of single-cluster tokens that
aren't themselves dictionary entries gets buffered and handed to
`HmmModel::segment_oov`, which Viterbi-decodes the most likely BMES
(Begin/Middle/End/Single) tag sequence and converts it to token boundaries.
Every other token — including genuine single-cluster dictionary words —
passes through untouched. That untouched-ness is why it composed cleanly
with both `ForwardMaxMatch` and `UnigramDp` with **zero measured R-iv
cost** (see `docs/BENCHMARKS.md` Phase 4): it structurally cannot affect a
token the trie walk already matched.

Same posture as `UnigramDp`'s frequencies: no trained `HmmModel` ships with
the crate (see `core/ATTRIBUTION.md`) — callers build one with
`HmmModel::from_counts(...)` from a segmented corpus they're licensed to
use.

## The regression guard (built, Phase 6)

Every prior phase measured its effect against khPOS — but khPOS is
gitignored (CC BY-NC-SA, download-only, never committed — see
`core/ATTRIBUTION.md`) and needs a network clone, so it can't be the thing that
gates every commit in CI. `eval/tests/regression.rs` solves that with a
different, deliberately smaller corpus: `eval/tests/fixtures/regression.word`,
15 sentences written for this project (not derived from khPOS or anywhere
else), so it's freely committable. It runs as an ordinary `cargo test`
integration test — no `cargo xtask`, no network — and fails if
`KhmerTokenizer::with_default_dict()`'s F1 on that fixture drops below 0.9.
`.github/workflows/ci.yml` runs `cargo test` (this guard included) plus
`cargo clippy -- -D warnings` on every push and PR.

This is intentionally a *floor*, not a pin to today's exact numbers: a
legitimate future change (swapping the default strategy, regenerating
`dict.txt` with different coverage) should be free to move the score around
without tripping CI, while an actual regression (a corrupted dictionary, a
broken trie walk, a regressed normalizer) still gets caught immediately
instead of silently shipping. One fixture line is deliberately a real-world
malformed Khmer spelling (`សិទិ្ធមនុស្ស`), so Phase 5's normalization fix
specifically stays covered too.

## The data flywheel (how massive data plugs in)

A dictionary engine looks static, but it's actually the *starter motor* for a
self-improving loop. Two flows feed it, and one flow comes out of it:

```mermaid
flowchart TB
    RAW[(Massive raw Khmer<br/>web, books, chats)]
    SEG[Segmenter<br/>this project]
    SILVER[(Silver-labeled<br/>segmented corpus)]
    MODEL[Trained model<br/>CRF / neural]
    FREQ[(Word frequencies)]
    NEW[New-word discovery<br/>entropy / mutual info]
    DICT[(Dictionary)]

    RAW --> SEG
    SEG --> SILVER
    SILVER --> MODEL
    MODEL -->|relabel, better| SILVER
    RAW --> FREQ
    FREQ --> SEG
    RAW --> NEW
    NEW --> DICT
    DICT --> SEG
```

- **Frequencies in:** counting words over a huge corpus gives the numbers the
  `UnigramDp` scorer needs — more data, better disambiguation.
- **New words in:** statistics over cluster sequences surface words the
  dictionary is missing, so the corpus *grows the dictionary by itself*.
- **Labels out → models in:** the segmenter labels raw text cheaply (silver
  data); you train a CRF/neural model on that; the better model relabels; repeat.
  This is **weak supervision / self-training**, and the dictionary engine is the
  cold start that makes it possible for a low-resource language.

## Feeding a Khmer LLM (pre-tokenization)

LLMs don't train on words; they train on **subword pieces** (BPE/SentencePiece).
On a language with no spaces, BPE makes ugly, meaningless merges. If you segment
into words *first* and feed those boundaries in, the learned subword vocabulary
respects real Khmer structure:

```mermaid
flowchart LR
    R[Raw Khmer] --> P[Segmenter<br/>as pre-tokenizer]
    P --> BPE[BPE / SentencePiece<br/>training]
    BPE --> VOCAB[(Subword vocab)]
    VOCAB --> LLM[Khmer LLM]
```

So the same engine that helps a startup tokenize product reviews today can be the
front-end that makes a future Khmer foundation model's tokenizer clean.

## What keeps it "open" (design rules)

1. **Stages are independent.** Normalize, split, score are separate modules with
   plain inputs/outputs. Improve one, disturb none.
2. **One public API, swappable engines.** The `Strategy` seam means dictionary
   and learned models coexist behind `segment()`.
3. **Offsets and labels are first-class (planned).** `segment()` will be able to
   return byte offsets and **BMES tags** (Begin/Middle/End/Single) — the exact
   format CRF/neural training expects. That turns the tool from a CLI into a
   *label generator*.
4. **Zero required dependencies.** The core stays `std`-only and tiny, so it runs
   anywhere in a data pipeline — parallel batch jobs, edge, browser (WASM).
5. **Permissive license, permissive data.** MIT/Apache code + CC-BY/MIT-class
   bundled data keeps every downstream use (including commercial LLM training)
   legal.

## Future crate layout

```text
khmerTokenizer/
├── core/      # kcc, normalize, trie, strategy, hmm  (std-only, all built)
├── cli/       # terminal tool (+ --strategy, --tags)
├── eval/      # P/R/F1 harness over a gold corpus
├── xtask/     # download corpora, prepare dict, run eval
├── model/     # OPTIONAL trained CRF/ONNX segmenter (feature-gated)
├── wasm/      # wasm-bindgen → npm/browser
├── py/        # PyO3 → pip, for data pipelines (BUILT — outside the Cargo
│              #   workspace so core/cli/eval/xtask stay zero-dependency)
├── data/      # downloaded corpora (gitignored, NC-licensed)
└── docs/      # RESEARCH, ROADMAP, ARCHITECTURE, LEARNING, BENCHMARKS
```

Nothing here forces you to build it all. The point is that each box can be added
later **without reshaping** what already exists — which is the definition of an
architecture that's open to the future.