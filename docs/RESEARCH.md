# Khmer Word Segmentation — Research Notes

Background research to guide the **accuracy & evaluation** phase of
`khmer-tokenizer`. Compiled June 2026. Treat reported accuracy figures and
licenses as *indicative* — verify each source's `LICENSE` file before depending
on or redistributing it.

## 1. The problem, restated for evaluation

Khmer is written without spaces between words; spaces fall between phrases with
no firm rule. Segmentation is therefore a prerequisite for almost every
downstream task, and it is genuinely ambiguous — the same character run can have
multiple valid splits, and compound words can be read as one token or several.
That ambiguity is exactly why we need (a) a labeled gold corpus and (b) a metric,
rather than eyeballing output.

## 2. Prior art (and why this project still has a niche)

This space is not empty. The most relevant existing tools:

| Tool | Language | Approach | Notes |
|------|----------|----------|-------|
| [khmercut](https://github.com/seanghay/khmercut) | Python (PyPI) | trained model | "fast" toolkit, widely used |
| [khmercut-rs](https://github.com/seanghay/khmercut-rs) | **Rust** | model-based | "blazingly fast" — direct prior art in our language |
| [khmersegment](https://github.com/seanghay/khmersegment) | Python | CRF | wraps the NIPTICT/CADT CRF model |
| [khnlp / Khmer-NLP-Tools (CADT)](https://github.com/seanghay/awesome-khmer-language) | Python | CRF + rules | the academic toolkit referenced in the original plan |
| [Joint WS+POS (Bouy et al. 2021)](https://arxiv.org/pdf/2103.16801) | Python | BiLSTM | [impl](https://github.com/Socret360/joint-khmer-word-segmentation-and-pos-tagging) |
| [UnifiedCut](https://www.researchgate.net/publication/386588060) | — | neural | Thai/Burmese/Khmer joint model |

**Implication.** A *blazingly fast Rust* segmenter already exists, so raw speed
alone is not a differentiator. The defensible niche for this project is:
**zero-dependency, permissively licensed (MIT/Apache), dictionary-driven, and
transparent** — no model file, no training step, easy to embed (incl. WASM), and
easy to read/audit. To make that credible we must *measure* accuracy and be
honest about the gap to CRF/neural tools. That is what this phase is for.

The best living index of the ecosystem is
[seanghay/awesome-khmer-language](https://github.com/seanghay/awesome-khmer-language) —
check it before building anything new.

## 3. Gold-standard corpora (for evaluation)

| Corpus | Size | License | Bundle? | Use |
|--------|------|---------|---------|-----|
| [khPOS](https://github.com/ye-kyaw-thu/khPOS) ([HF mirror](https://huggingface.co/datasets/seanghay/khPOS)) | ~12,000 sentences, manually word-segmented + POS | CC BY-NC-SA 4.0 | ❌ NonCommercial | primary eval set; ships `OPEN-TEST.word` / closed-test splits |
| [ALT (NICT Asian Language Treebank)](https://www2.nict.go.jp/astrec-att/member/mutiyama/ALT/) | parallel, word-segmented + POS + syntax | CC BY-NC-SA | ❌ NonCommercial | second eval set; cross-checks domain generalization |

Both are **CC BY-NC-SA** — fine to *evaluate against*, but they **cannot be
committed into an MIT/Apache repo** and cannot ship inside a commercial binary.
The harness must download them on demand and `.gitignore` the data directory.

khPOS stores one sentence per line with words separated by spaces, so deriving an
evaluation pair is easy: the *gold* token list is the line split on spaces; the
*input* is the same line with spaces removed. (Mind that khPOS uses an explicit
`|`-style or space convention per file — confirm the exact delimiter when wiring
the adapter; see its `data/` folder.) Reference discussion of the data and
tokenization choices: [Towards Tokenization and POS Tagging for Khmer](https://dl.acm.org/doi/fullHtml/10.1145/3464378)
and [Pretrained Models and Evaluation Data for the Khmer Language](https://www.sciopen.com/article/10.26599/TST.2021.9010060).

## 4. Dictionaries & frequency lists (bundleable, for the engine)

To improve accuracy we need a real lexicon — ideally with frequencies, which the
scored algorithm (§6) requires. Permissively-licensed candidates:

| Source | Content | License | Notes |
|--------|---------|---------|-------|
| [khopilot/khmer-lexicon](https://huggingface.co/datasets/khopilot/khmer-lexicon) | broad lexicon incl. formal/royal terms | **CC BY 4.0** | attribution-only → **bundleable** with credit |
| [silnrsi/khmerlbdict](https://github.com/silnrsi/khmerlbdict) | frequency-based wordlist for line/word breaking (built on SEALang) | SIL (verify) | frequencies → enables max-probability path |
| [SEAlang Khmer](http://sealang.net/khmer/corpus.htm) | corpus + frequency wordlist | check terms | upstream of several lists |
| [Wiktionary Khmer frequency list](https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists/Khmer) | frequency list | CC BY-SA | ShareAlike — keep separate from code |

**Plan:** bundle a CC BY 4.0 lexicon (with an `ATTRIBUTION` note) as the default
dictionary, and pair it with a frequency table from `khmerlbdict`/SEALang to feed
the scored segmenter. Keep ShareAlike-licensed lists out of the source tree.

## 5. Evaluation metrics

The field-standard (from Chinese segmentation / the SIGHAN bakeoffs) is
**token-level Precision, Recall, and F1 over correctly segmented word spans**: a
predicted word counts as correct only if both its boundaries match the gold word.
Report alongside it:

- **OOV recall (R-oov)** and **in-vocabulary recall (R-iv)** — segmentation
  quality is dominated by out-of-vocabulary handling, so these separate the
  dictionary's coverage from the algorithm's cleverness.
- **Word-level accuracy** for an at-a-glance number.

One caveat worth knowing: there's a published argument that **recall is the more
faithful single metric** for word segmentation because precision can flatter an
under-splitting system ([Fu et al., IJCNLP 2017](https://aclanthology.org/I17-2015/)).
We'll report P/R/F1 but watch recall (esp. R-oov) as the headline.

## 6. Algorithmic options (in increasing accuracy/cost)

The current engine is **forward maximum matching** (greedy longest-match) over a
cluster trie. Upgrade path, cheapest first:

1. **Bidirectional maximum matching (BiMM).** Run forward and backward
   longest-match; on disagreement pick the segmentation with fewer tokens (or
   fewer single-cluster tokens). Cheap, no training, and the literature reports
   high accuracy for MM-family methods on Khmer (one source cites ~98% for a BiMM
   variant — treat as a ceiling, dataset-dependent). Good intermediate baseline.

2. **Unigram max-probability path (jieba-style).** Build a DAG of all dictionary
   matches over the text, then dynamic-programming for the maximum-probability
   path using word frequencies (work in log-space). This is how
   [jieba](https://github.com/fxsjy/jieba) ([architecture](https://deepwiki.com/fxsjy/jieba))
   resolves ambiguity and is the recommended target: it needs only a frequency
   list (§4), stays dependency-free, and handles ambiguous splits far better than
   greedy MM.

3. **HMM/Viterbi for unknown words.** jieba layers an HMM (BMES tag states) with
   Viterbi decoding to segment runs that the dictionary misses. A lightweight
   character/cluster HMM is a natural OOV upgrade over our current
   one-cluster-per-token fallback.

4. **CRF (stretch).** The CADT/NIPTICT model and `khmersegment` use CRFs; this is
   the accuracy frontier for non-neural methods but introduces a model file and
   training pipeline, breaking the zero-dependency promise. Keep as an optional
   feature-gated crate, not the default.

A clean way to ship this: a `Strategy` enum (`ForwardMaxMatch`, `BiMaxMatch`,
`UnigramDp`) selectable at construction, so all three can be benchmarked on the
same harness and users choose their speed/accuracy trade-off.

## 7. Orthographic normalization (accuracy multiplier)

Khmer Unicode allows multiple byte sequences that render identically (e.g.
subscript/vowel ordering). Without normalization, the same word can miss the
dictionary depending on how it was typed, hurting both accuracy and eval
consistency. A normalization pass (canonical COENG ordering, vowel/sign
reordering) before segmentation raises dictionary hit-rate and should be measured
as its own contribution. The `khmerlbdict`/SIL line-breaking work and Unicode's
Khmer notes are the references here.

## 8. Takeaways for the roadmap

- Build the **evaluation harness first** — without a number, every later change
  is a guess. (khPOS as primary set, downloaded not bundled.)
- Biggest accuracy levers, in order: **a real dictionary** → **scored
  (unigram-DP) segmentation** → **OOV model** → **normalization**.
- Keep the zero-dependency, permissive-license identity; that's the project's
  reason to exist next to `khmercut-rs`.
- Be honest in the README about where we sit versus CRF/neural tools, backed by
  harness numbers.

## Sources

- Asian Language Treebank (ALT): https://www2.nict.go.jp/astrec-att/member/mutiyama/ALT/ — paper: http://www.lrec-conf.org/proceedings/lrec2016/pdf/435_Paper.pdf
- khPOS corpus: https://github.com/ye-kyaw-thu/khPOS — mirror: https://huggingface.co/datasets/seanghay/khPOS
- Towards Tokenization and POS Tagging for Khmer: https://dl.acm.org/doi/fullHtml/10.1145/3464378
- Pretrained Models and Evaluation Data for the Khmer Language: https://www.sciopen.com/article/10.26599/TST.2021.9010060
- Joint Khmer WS + POS (Bouy et al., 2021): https://arxiv.org/pdf/2103.16801 — impl: https://github.com/Socret360/joint-khmer-word-segmentation-and-pos-tagging
- UnifiedCut: https://www.researchgate.net/publication/386588060
- khmercut: https://github.com/seanghay/khmercut — Rust: https://github.com/seanghay/khmercut-rs — CRF wrapper: https://github.com/seanghay/khmersegment
- awesome-khmer-language: https://github.com/seanghay/awesome-khmer-language
- Recall is the Proper Evaluation Metric for Word Segmentation (Fu et al., 2017): https://aclanthology.org/I17-2015/
- jieba: https://github.com/fxsjy/jieba — architecture: https://deepwiki.com/fxsjy/jieba
- khopilot/khmer-lexicon (CC BY 4.0): https://huggingface.co/datasets/khopilot/khmer-lexicon
- SIL khmerlbdict: https://github.com/silnrsi/khmerlbdict
- SEAlang Khmer: http://sealang.net/khmer/corpus.htm
- Wiktionary Khmer frequency list: https://en.wiktionary.org/wiki/Wiktionary:Frequency_lists/Khmer
