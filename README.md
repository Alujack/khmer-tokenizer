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

Segmentation runs in two passes:

1. **Cluster pass** — the text is grouped into *Khmer Character Clusters* (KCC):
   a base consonant or independent vowel together with any stacked subscripts
   (introduced by COENG, `U+17D2`) and dependent vowels/signs. Working on
   clusters instead of raw Unicode scalars is what guarantees the segmenter never
   splits *inside* an orthographic syllable — the classic bug in naive Khmer
   tokenizers.
2. **Longest-match pass** — a trie keyed on whole clusters is walked with a
   maximum-matching (longest-match) strategy: at each position the engine
   consumes the longest run of clusters that forms a dictionary word. When no
   word matches, it falls back to a single cluster, so output is always
   well-formed. Runs of non-Khmer text (Latin, digits, punctuation) become their
   own tokens; whitespace separates tokens.

The engine is `std`-only and deterministic. No model, no training step, no
network.

## Project layout

```text
khmerTokenizer/
├── Cargo.toml          # workspace manifest
├── core/               # khmer-tokenizer-core — the library
│   ├── src/lib.rs      #   public API + dictionary helpers
│   ├── src/kcc.rs      #   Khmer Character Cluster splitting
│   ├── src/trie.rs     #   cluster trie + longest-match segmentation
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

Covers KCC splitting (subscripts and vowels stay attached), longest-match
segmentation, mixed Khmer/Latin/number input, the out-of-vocabulary fallback,
and dictionary loading.

## Roadmap

Designed so these slot in without restructuring the workspace:

- **WASM bindings** — a `wasm/` crate using `wasm-bindgen` + `wasm-pack` to run
  the engine in browsers and Node, publishable to npm.
- **Python bindings** — a `py/` crate using PyO3 so it drops into existing
  `khnlp`-style pipelines.
- **Benchmarks** — a Criterion suite to track throughput.
- **Scored segmentation** — optional word-frequency weighting (Viterbi) for
  better disambiguation on hard cases.

## License

Dual-licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
