# khmer-tokenizer

Fast, dependency-free **Khmer word segmentation** — a Rust core with Python
bindings.

Written Khmer has no spaces between words; this package splits it into
words correctly and quickly, with an embedded 59,526-word dictionary and no
model files, network calls, or Python dependencies.

```python
from khmer_tokenizer import KhmerTokenizer, split_kcc, normalize

tk = KhmerTokenizer()
tk.segment("សួស្តីអ្នកទាំងអស់គ្នា")
# ['សួស្តី', 'អ្នក', 'ទាំងអស់គ្នា']

# Bring your own dictionary and/or strategy:
tk = KhmerTokenizer(strategy="bimm")                      # bidirectional max-match
tk = KhmerTokenizer(words=["ភាសា", "ខ្មែរ"])              # custom word list
tk = KhmerTokenizer(strategy="unigram",
                    frequencies={"ភាសា": 500, "ខ្មែរ": 800})  # frequency-scored DP

# Lower-level pieces:
split_kcc("ខ្មែរ")      # ['ខ្មែ', 'រ'] — orthographic clusters
normalize("សិទិ្ធ")     # 'សិទ្ធិ' — repair Khmer Unicode encoding errors
```

A typical LLM-pretokenization pipeline: run your corpus through
`KhmerTokenizer.segment()` and join tokens with a space (or U+200B ZERO
WIDTH SPACE, the Unicode-recommended Khmer boundary marker) before training
a BPE/SentencePiece tokenizer — the learned subword vocabulary will respect
real Khmer word structure instead of merging across word boundaries.

Full documentation, benchmarks (F1 against khPOS and a 80k-sentence web
corpus), architecture notes, and the Rust crates live at
[github.com/Alujack/khmer-tokenizer](https://github.com/Alujack/khmer-tokenizer).

## License

MIT OR Apache-2.0. The embedded dictionary is MIT (chamkho / SIL NRSI) —
see the repository's `core/ATTRIBUTION.md`.
