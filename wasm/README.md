# khmer-tokenizer

Fast, dependency-free **Khmer word segmentation** for JavaScript — the
[khmer-tokenizer-core](https://crates.io/crates/khmer-tokenizer-core) Rust
engine compiled to WebAssembly. Runs in Node and browsers; ships its own
TypeScript definitions.

Written Khmer has no spaces between words, so before you can search, index,
or feed Khmer text to a model, you have to split it into words. General-purpose
tokenizers either ignore Khmer or shatter it into meaningless character
fragments; this package segments it correctly.

```js
import { KhmerTokenizer, splitKcc, normalize, isKhmer } from "khmer-tokenizer";

const tk = new KhmerTokenizer(); // embedded 59k-word dictionary, forward max-match
tk.segment("សួស្តីអ្នកទាំងអស់គ្នា");
// ["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
```

## Options

Everything is configured at construction time:

```js
new KhmerTokenizer({ strategy: "bimm" });          // bidirectional max-match
new KhmerTokenizer({ words: ["ភាសា", "ខ្មែរ"] }); // your own word list
new KhmerTokenizer({                                // frequency-scored DP
  strategy: "unigram",
  frequencies: { "ភាសា": 500, "ខ្មែរ": 800 },
});
new KhmerTokenizer({ normalization: false });       // skip orthographic repair
```

- `segment(text)` → `string[]` — segment Khmer text into words. Runs of
  non-Khmer text (Latin, digits) become their own tokens; whitespace and
  `U+200B` ZERO WIDTH SPACE act as hard word boundaries.
- `contains(word)` → `boolean`, `size` → `number` — dictionary lookups.
- `splitKcc(text)` → `string[]` — split into Khmer Character Clusters
  (orthographic syllables; the segmenter never splits inside one).
- `normalize(text)` — repair common mark-ordering corruptions, including the
  damage Unicode NFC itself inflicts on Khmer.
- `isKhmer(char)` → `boolean`.

Normalization is **on by default** because any NFC step upstream of you
silently corrupts Khmer subscript ordering (see the project's
[RESEARCH-3.md](https://github.com/Alujack/khmer-tokenizer/blob/master/docs/RESEARCH-3.md)).

## Use as an LLM pre-tokenizer

Segment your corpus, join with spaces (or ZWSP), then train
BPE/SentencePiece on the result so learned subwords respect real Khmer word
structure.

## Links

- Source, benchmarks, and research notes:
  [github.com/Alujack/khmer-tokenizer](https://github.com/Alujack/khmer-tokenizer)
- Rust crate: [khmer-tokenizer-core](https://crates.io/crates/khmer-tokenizer-core) ·
  CLI: `cargo install khmer-tokenizer-cli` ·
  Python: [`pip install khmer-tokenizer`](https://pypi.org/project/khmer-tokenizer/)

## License

MIT OR Apache-2.0. Bundled dictionary derived from
[chamkho](https://github.com/veer66/chamkho)'s `khmerdict.txt` (MIT, © SIL
NRSI — see
[ATTRIBUTION.md](https://github.com/Alujack/khmer-tokenizer/blob/master/core/ATTRIBUTION.md)).
