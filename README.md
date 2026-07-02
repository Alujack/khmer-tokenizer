# khmer-tokenizer

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

## How it works

Segmentation runs in three passes:

0. **Normalization pass (on by default)** — [`normalize`](core/src/normalize.rs)
   reorders a shifter, vowel, or sign that was typed directly before a
   `COENG`+consonant subscript pair to instead follow it, per the Unicode
   Khmer syllable structure — the single most common real-world Khmer
   encoding error (e.g. `សិទិ្ធ` for the correct `សិទ្ធិ`). Pure character
   reordering, so it's byte-length-preserving. Opt out with
   `.without_normalization()` — see [BENCHMARKS.md](docs/BENCHMARKS.md) for
   why it's kept on by default even though its measured effect on the
   bundled dictionary is zero.
1. **Cluster pass** — the text is grouped into *Khmer Character Clusters* (KCC):
   a base consonant or independent vowel together with any stacked subscripts
   (introduced by COENG, `U+17D2`) and dependent vowels/signs. Working on
   clusters instead of raw Unicode scalars is what guarantees the segmenter never
   splits *inside* an orthographic syllable — the classic bug in naive Khmer
   tokenizers.
2. **Boundary pass** — a trie keyed on whole clusters is walked to place word
   boundaries, using one of three [`Strategy`](core/src/strategy.rs) algorithms:
   - `ForwardMaxMatch` (default) — greedy longest-match, left to right: at
     each position, consume the longest run of clusters that forms a
     dictionary word. Falls back to a single cluster when nothing matches.
   - `BiMaxMatch` — also runs backward max-match and picks between them on
     disagreement (fewer tokens wins, then fewer single-cluster tokens);
     measurably more accurate than the default.
   - `UnigramDp` — builds a DAG of every dictionary match (not just the
     longest) and dynamic-programs the highest-probability path using word
     frequencies you supply via `with_frequencies(...)`. The most accurate of
     the three by a clear margin — see [BENCHMARKS.md](docs/BENCHMARKS.md) —
     but needs a frequency table; **none ships with this crate** (see
     "Dictionary" below for why). Falls back to `ForwardMaxMatch` if none is
     set.

   Either way, runs of non-Khmer text (Latin, digits, punctuation) become
   their own tokens, and whitespace separates tokens without producing one.
3. **OOV fallback (optional)** — every strategy above still falls back to one
   token per cluster when a run matches *nothing* in the dictionary at all.
   Attaching an [`HmmModel`](core/src/hmm.rs) via `with_hmm(...)` replaces
   just those unmatched runs with a Viterbi-decoded BMES guess instead,
   leaving every dictionary hit (including real single-cluster words)
   untouched — lifts out-of-vocabulary recall by ~0.05 absolute with no
   measured cost to in-vocabulary accuracy (see
   [BENCHMARKS.md](docs/BENCHMARKS.md)). Needs a model you train yourself;
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
└── cli/                # khmer-tokenizer-cli — the command-line tool
    └── src/main.rs
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

// Orthographic normalization runs by default; opt out if you need exact
// byte-for-byte parity with pre-Phase-5 behavior.
let tk = KhmerTokenizer::with_default_dict().without_normalization();

// Need just the orthographic clusters?
use khmer_tokenizer_core::split_kcc;
assert_eq!(split_kcc("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
```

## CLI usage

```bash
# Build
cargo build --release

# Segment an argument (space-separated output)
./target/release/khmer-tokenizer "សួស្តីអ្នកទាំងអស់គ្នា"
# -> សួស្តី អ្នក ទាំងអស់គ្នា

# JSON array output
./target/release/khmer-tokenizer --json "ភាសាខ្មែរ"
# -> ["ភាសា","ខ្មែរ"]

# Bidirectional max-match instead of the default forward max-match
./target/release/khmer-tokenizer --strategy bimm "សួស្តីអ្នកទាំងអស់គ្នា"

# Read from stdin, one line at a time
echo "ខ្ញុំស្រឡាញ់កម្ពុជា" | ./target/release/khmer-tokenizer
```

## Dictionary

Segmentation quality is bounded by the dictionary. The bundled
`core/src/dict.txt` has **59,526 words**, sourced from
[chamkho](https://github.com/veer66/chamkho)'s `khmerdict.txt`
(MIT license, copyright SIL NRSI — see [ATTRIBUTION.md](./ATTRIBUTION.md)).
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

Covers orthographic normalization (reordering marks typed before a subscript,
idempotency, byte-length preservation — see `core/src/normalize.rs`), KCC
splitting (subscripts and vowels stay attached), all three segmentation
strategies (forward max-match, bidirectional max-match, and unigram DP —
including a hand-built case where only DP-based scoring can reach the
correct segmentation), the HMM OOV fallback (a hand-built BMES model that
resegments an unmatched cluster run while leaving a real dictionary hit
alone), mixed Khmer/Latin/number input, the out-of-vocabulary fallback, and
dictionary loading.

## Roadmap

Designed so these slot in without restructuring the workspace:

- **WASM bindings** — a `wasm/` crate using `wasm-bindgen` + `wasm-pack` to run
  the engine in browsers and Node, publishable to npm.
- **Python bindings** — a `py/` crate using PyO3 so it drops into existing
  `khnlp`-style pipelines.
- **Benchmarks** — a Criterion suite to track throughput.
- **A bundleable frequency table** for `UnigramDp` — no commercially-clean,
  bundleable corpus-frequency source has been found yet (see
  [docs/ROADMAP.md](docs/ROADMAP.md) Phase 3); until then, callers supply
  their own via `with_frequencies(...)`.
- **CLI support for `UnigramDp` and `with_hmm`** — the CLI has no mechanism
  yet to load an external frequency table or HMM model file, so
  `--strategy` only exposes `fmm`/`bimm`.
- **A CI regression guard** (Phase 6) — run `cargo test` plus the eval
  harness against a small, license-safe synthetic sample on every change,
  and fail if F1 drops below a threshold.

## License

Dual-licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
