# khmer-tokenizer

[![CI](https://github.com/Alujack/khmer-tokenizer/actions/workflows/ci.yml/badge.svg)](https://github.com/Alujack/khmer-tokenizer/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/khmer-tokenizer-core.svg)](https://crates.io/crates/khmer-tokenizer-core)
[![docs.rs](https://img.shields.io/docsrs/khmer-tokenizer-core)](https://docs.rs/khmer-tokenizer-core)
[![PyPI](https://img.shields.io/pypi/v/khmer-tokenizer.svg)](https://pypi.org/project/khmer-tokenizer/)
[![npm](https://img.shields.io/npm/v/kh-tokenizer.svg)](https://www.npmjs.com/package/kh-tokenizer)

A fast, dependency-free **Khmer word segmenter** written in Rust.

Written Khmer has no spaces between words, so before you can search, index,
spell-check, translate, or train a model on Khmer text, you first have to split
it into words. Most general-purpose tokenizers either ignore Khmer or shatter it
into meaningless character fragments. This library segments Khmer text correctly
and quickly, with no external dependencies.

```text
input:   សួស្តីអ្នកទាំងអស់គ្នា
output:  ["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
```

## Install

```bash
# The Rust library, in your Rust project
cargo add khmer-tokenizer-core

# The command-line tool
cargo install khmer-tokenizer-cli   # installs the `khmer-tokenizer` binary

# The Python bindings
pip install khmer-tokenizer

# The JavaScript/WASM bindings (named kh-tokenizer on npm)
npm install kh-tokenizer
```

API docs: [docs.rs/khmer-tokenizer-core](https://docs.rs/khmer-tokenizer-core)

## How it works

Segmentation runs in three passes:

0. **Normalization pass (on by default)** — [`normalize`](https://github.com/Alujack/khmer-tokenizer/blob/master/core/src/normalize.rs)
   repairs two real-world corruptions of the Unicode Khmer syllable
   structure: a shifter, vowel, or sign typed directly *before* a
   `COENG`+consonant subscript pair (the most common typing error, e.g.
   `សិទិ្ធ` for the correct `សិទ្ធិ`), and a mark stranded *between*
   `COENG` and its consonant — which is what Unicode NFC itself produces
   on Khmer text, thanks to erroneous-and-frozen canonical combining
   classes (see [RESEARCH-3.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/RESEARCH-3.md) §2a), so any
   NFC-processing pipeline upstream of you silently corrupts Khmer this
   way. Pure character reordering, so it's byte-length-preserving. Opt out
   with `.without_normalization()` — see [BENCHMARKS.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/BENCHMARKS.md)
   for why it's kept on by default even though its measured effect on the
   bundled dictionary is zero.
1. **Cluster pass** — the text is grouped into *Khmer Character Clusters* (KCC):
   a base consonant or independent vowel together with any stacked subscripts
   (introduced by COENG, `U+17D2`) and dependent vowels/signs. Working on
   clusters instead of raw Unicode scalars is what guarantees the segmenter never
   splits *inside* an orthographic syllable — the classic bug in naive Khmer
   tokenizers.
2. **Boundary pass** — a trie keyed on whole clusters is walked to place word
   boundaries, using one of three [`Strategy`](https://github.com/Alujack/khmer-tokenizer/blob/master/core/src/strategy.rs) algorithms:
   - `ForwardMaxMatch` (default) — greedy longest-match, left to right: at
     each position, consume the longest run of clusters that forms a
     dictionary word. Falls back to a single cluster when nothing matches.
   - `BiMaxMatch` — also runs backward max-match and picks between them on
     disagreement (fewer tokens wins, then fewer single-cluster tokens);
     measurably more accurate than the default.
   - `UnigramDp` — builds a DAG of every dictionary match (not just the
     longest) and dynamic-programs the highest-probability path using word
     frequencies you supply via `with_frequencies(...)`. The most accurate
     dictionary strategy by a clear margin — see [BENCHMARKS.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/BENCHMARKS.md) —
     but needs a frequency table; **none ships with this crate** (see
     "Dictionary" below for why). Falls back to `ForwardMaxMatch` if none is
     set.
   - `Tagger` — skips the dictionary entirely: every Khmer run is segmented
     by an averaged-perceptron BMES tagger (`TaggerModel`, the CRF-class
     tier) attached via `with_tagger(...)`. The most accurate mode overall
     — **F1 0.93 vs 0.78 for the best dictionary configuration** on khPOS
     (see [BENCHMARKS.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/BENCHMARKS.md)) — but needs a model you train
     yourself with `TaggerModel::train` on a segmented corpus; **none ships
     with this crate**. Falls back to `ForwardMaxMatch` if none is set.

   Either way, only runs of Khmer *letters* go to the strategy. Runs of
   non-Khmer text (Latin, ASCII digits, punctuation) become their own
   tokens, and so do runs of **Khmer digits** (`១២៣` — dates, prices) and
   individual **Khmer punctuation / symbols** (`។ ៕ ៛` …), which are never
   fed to the dictionary or an OOV model that might shatter or glue them.
   Whitespace separates tokens without producing one. So do `U+200B` ZERO
   WIDTH SPACE — the character the Unicode Standard recommends for marking
   Khmer word boundaries, ubiquitous as an invisible hint in real Khmer web
   text — and `U+FEFF` (the byte-order mark that begins countless files);
   each is trusted as a boundary: consumed, never emitted as a token, never
   merged across. A dangling `COENG` (`U+17D2`) in truncated or mistyped
   text stays attached to its base and never swallows the character after
   it, so a following space or ZWSP still marks the boundary it should.
3. **OOV fallback (optional)** — every dictionary strategy above still falls
   back to one token per cluster when a run matches *nothing* in the
   dictionary at all. Attaching a model replaces just those unmatched runs
   with a Viterbi-decoded BMES guess instead, leaving every dictionary hit
   (including real single-cluster words) untouched. Two model types fit the
   same seam: an [`HmmModel`](https://github.com/Alujack/khmer-tokenizer/blob/master/core/src/hmm.rs) via `with_hmm(...)` (cluster-identity
   emissions; lifts OOV recall ~0.05 absolute), or a
   [`TaggerModel`](https://github.com/Alujack/khmer-tokenizer/blob/master/core/src/tagger.rs) via `with_tagger(...)` (context-feature
   perceptron; a strict upgrade — lifts OOV recall further with no
   in-vocabulary cost, and is preferred when both are attached; see
   [BENCHMARKS.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/BENCHMARKS.md)). Both need a model you train yourself;
   none ships with this crate (same reason as `UnigramDp`'s frequencies).

The engine is `std`-only and deterministic. No model, no training step, no
network.

## Project layout

```text
khmerTokenizer/
├── Cargo.toml          # workspace manifest
├── core/               # khmer-tokenizer-core — the library
│   ├── src/lib.rs      #   public API + dictionary helpers
│   ├── src/kcc.rs      #   Khmer Character Cluster splitting
│   ├── src/normalize.rs #  orthographic normalization (Phase 5)
│   ├── src/trie.rs     #   cluster trie + strategies + HMM fallback
│   ├── src/hmm.rs      #   BMES HMM/Viterbi OOV fallback (Phase 4)
│   └── src/dict.txt    #   embedded default dictionary
├── cli/                # khmer-tokenizer-cli — the command-line tool
│   └── src/main.rs
├── py/                 # khmer-tokenizer on PyPI — PyO3/maturin bindings
│   ├── src/lib.rs      #   (outside the Cargo workspace: needs pyo3)
│   └── tests/          #   pytest suite
└── wasm/               # kh-tokenizer for npm — wasm-bindgen/wasm-pack
    ├── src/lib.rs      #   (outside the Cargo workspace: needs wasm-bindgen)
    └── tests/          #   wasm-bindgen-test suite, run in Node
```

## Library usage

```rust
use khmer_tokenizer_core::KhmerTokenizer;

// Use the embedded default dictionary...
let tk = KhmerTokenizer::with_default_dict();
let tokens = tk.segment("សួស្តីអ្នកទាំងអស់គ្នា");
assert_eq!(tokens, vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]);

// ...or bring your own word list.
let tk = KhmerTokenizer::from_words(["ភាសា", "ខ្មែរ"]);
assert_eq!(tk.segment("ភាសាខ្មែរ"), vec!["ភាសា", "ខ្មែរ"]);

// ...or a different strategy (see "How it works" above).
use khmer_tokenizer_core::Strategy;
let tk = KhmerTokenizer::with_default_dict().with_strategy(Strategy::BiMaxMatch);

// UnigramDp needs your own word frequencies (word -> count).
let freqs = [("ភាសា".to_string(), 500), ("ខ្មែរ".to_string(), 800)];
let tk = KhmerTokenizer::with_default_dict()
    .with_strategy(Strategy::UnigramDp)
    .with_frequencies(freqs);

// Any strategy can add an HMM fallback for clusters the dictionary matches
// nothing in at all — trained yourself (BMES tag counts) from a segmented
// corpus with HmmModel::from_counts(start_counts, trans_counts, emit_counts).
use khmer_tokenizer_core::HmmModel;
let tk = KhmerTokenizer::with_default_dict().with_hmm(my_hmm_model);

// Or the stronger CRF-class option: train an averaged-perceptron BMES
// tagger from gold-segmented sentences. As a fallback it upgrades the HMM
// at the same seam; with Strategy::Tagger it segments everything itself
// (F1 0.93 vs 0.78 on khPOS — see BENCHMARKS.md). Persist with
// to_text()/from_text().
use khmer_tokenizer_core::TaggerModel;
let model = TaggerModel::train(&gold_sentences, 5);
let tk = KhmerTokenizer::with_default_dict().with_tagger(model.clone()); // fallback
let tk = KhmerTokenizer::empty()
    .with_strategy(Strategy::Tagger)
    .with_tagger(model); // full tagger segmentation

// Orthographic normalization runs by default; opt out if you need exact
// byte-for-byte parity with pre-Phase-5 behavior.
let tk = KhmerTokenizer::with_default_dict().without_normalization();

// Need just the orthographic clusters?
use khmer_tokenizer_core::split_kcc;
assert_eq!(split_kcc("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
```

## CLI usage

```bash
# Install from crates.io (or build from source with `cargo build --release`)
cargo install khmer-tokenizer-cli

# Segment an argument (space-separated output)
khmer-tokenizer "សួស្តីអ្នកទាំងអស់គ្នា"
# -> សួស្តី អ្នក ទាំងអស់គ្នា

# JSON array output
khmer-tokenizer --json "ភាសាខ្មែរ"
# -> ["ភាសា","ខ្មែរ"]

# Bidirectional max-match instead of the default forward max-match
khmer-tokenizer --strategy bimm "សួស្តីអ្នកទាំងអស់គ្នា"

# Join tokens with U+200B ZERO WIDTH SPACE — the Unicode-recommended Khmer
# word-boundary marker. Renders identically to the input, round-trips
# through the tokenizer, and is what SentencePiece-style trainers can eat.
khmer-tokenizer --zwsp "សួស្តីអ្នកទាំងអស់គ្នា"

# Read from stdin, one line at a time
echo "ខ្ញុំស្រឡាញ់កម្ពុជា" | khmer-tokenizer
```

## Python usage

[`pip install khmer-tokenizer`](https://pypi.org/project/khmer-tokenizer/) —
the [`py/`](https://github.com/Alujack/khmer-tokenizer/tree/master/py)
crate exposes the same engine to Python via PyO3 (abi3 wheel, works on any
CPython ≥ 3.9, no Python dependencies):

```python
from khmer_tokenizer import KhmerTokenizer, split_kcc, normalize

tk = KhmerTokenizer()  # embedded default dictionary, forward max-match
tk.segment("សួស្តីអ្នកទាំងអស់គ្នា")
# ['សួស្តី', 'អ្នក', 'ទាំងអស់គ្នា']

KhmerTokenizer(strategy="bimm")                    # bidirectional max-match
KhmerTokenizer(words=["ភាសា", "ខ្មែរ"])            # custom word list
KhmerTokenizer(strategy="unigram",
               frequencies={"ភាសា": 500})          # frequency-scored DP

split_kcc("ខ្មែរ")    # ['ខ្មែ', 'រ']
normalize("សិទិ្ធ")   # 'សិទ្ធិ'
```

This is the pre-tokenizer path for LLM pipelines: segment your corpus,
join with spaces (or ZWSP), then train BPE/SentencePiece on the result so
the learned subwords respect real Khmer word structure.

## JavaScript / WASM usage

[`npm install kh-tokenizer`](https://www.npmjs.com/package/kh-tokenizer) —
the [`wasm/`](https://github.com/Alujack/khmer-tokenizer/tree/master/wasm)
crate compiles the same engine to WebAssembly via wasm-bindgen, for Node
and browsers, with TypeScript definitions generated by wasm-pack. (It's
`kh-tokenizer` on npm — not `khmer-tokenizer` like PyPI — because npm's
similar-name rule blocks the full name; same engine, same API.)

```js
import { KhmerTokenizer, splitKcc, normalize, isKhmer } from "kh-tokenizer";

const tk = new KhmerTokenizer(); // embedded default dictionary, forward max-match
tk.segment("សួស្តីអ្នកទាំងអស់គ្នា");
// ["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]

new KhmerTokenizer({ strategy: "bimm" });          // bidirectional max-match
new KhmerTokenizer({ words: ["ភាសា", "ខ្មែរ"] }); // custom word list
new KhmerTokenizer({                                // frequency-scored DP
  strategy: "unigram",
  frequencies: { "ភាសា": 500 },
});

splitKcc("ខ្មែរ"); // ["ខ្មែ", "រ"]
normalize("សិទិ្ធ"); // "សិទ្ធិ"
```

Build the npm package from a checkout with
`wasm-pack build wasm --release --target nodejs --out-dir pkg-node`
(or `--target web`/`--target bundler` for browsers).

## Dictionary

Segmentation quality is bounded by the dictionary. The bundled
`core/src/dict.txt` has **59,526 words**, sourced from
[chamkho](https://github.com/veer66/chamkho)'s `khmerdict.txt`
(MIT license, copyright SIL NRSI — see [ATTRIBUTION.md](https://github.com/Alujack/khmer-tokenizer/blob/master/core/ATTRIBUTION.md)).
It's regenerated with `cargo xtask prepare-dict`, which re-downloads and
re-cleans the source rather than hand-editing the committed file.

To use your own lexicon instead:

- Put one word per line in a text file (`#` comments and blank lines are
  ignored), then load it with `KhmerTokenizer::from_dict_str(std::fs::read_to_string(path)?.as_str())`,
  or replace `core/src/dict.txt` to keep it embedded in the binary via
  `include_str!`.

> **Licensing note:** many published Khmer word lists and corpora carry their own
> licenses. Before bundling a third-party lexicon into this (MIT/Apache-2.0)
> project, check that its license permits redistribution and is compatible —
> see `docs/RESEARCH-2.md` §5 for a survey of common sources.

## Tests

```bash
cargo test
```

Covers orthographic normalization (marks typed before a subscript, the
NFC-stranded-mark repair, joiner exemptions, idempotency, byte-length
preservation — see `core/src/normalize.rs`), ZWSP boundary handling, KCC
splitting (subscripts and vowels stay attached), all three segmentation
strategies (forward max-match, bidirectional max-match, and unigram DP —
including a hand-built case where only DP-based scoring can reach the
correct segmentation), the HMM OOV fallback (a hand-built BMES model that
resegments an unmatched cluster run while leaving a real dictionary hit
alone), mixed Khmer/Latin/number input, the out-of-vocabulary fallback, and
dictionary loading — plus a CI regression guard
(`eval/tests/regression.rs`) that fails the build if the default
tokenizer's accuracy on a small, committed, hand-authored sample drops
below a floor. [CI](https://github.com/Alujack/khmer-tokenizer/blob/master/.github/workflows/ci.yml) runs this on every push/PR.

## Roadmap

Designed so these slot in without restructuring the workspace:

- **Benchmarks** — a Criterion suite to track throughput.
- **A bundleable frequency table** for `UnigramDp` — no commercially-clean,
  bundleable corpus-frequency source has been found yet (see
  [docs/ROADMAP.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/ROADMAP.md) Phase 3); until then, callers supply
  their own via `with_frequencies(...)`.
- **CLI support for `UnigramDp`, `with_hmm`, and `Strategy::Tagger`** — the
  CLI has no mechanism yet to load an external frequency table or model
  file (the tagger's `to_text` format is designed for exactly this), so
  `--strategy` only exposes `fmm`/`bimm`.

## License

Dual-licensed under either of [Apache License, Version 2.0](https://github.com/Alujack/khmer-tokenizer/blob/master/LICENSE-APACHE) or
[MIT license](https://github.com/Alujack/khmer-tokenizer/blob/master/LICENSE-MIT) at your option.
