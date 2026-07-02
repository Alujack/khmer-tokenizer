# Research 3 — Post-roadmap landscape survey (2026-07-02)

Where this library sits relative to the current state of the art, and what
that implies for what to build next. Produced by a multi-agent deep-research
run (28 sources fetched, 101 claims extracted, 25 adversarially verified by
3-vote refutation panels: 18 confirmed, 7 refuted), then cross-checked
against this codebase and, where possible, **re-verified empirically here**
(the Unicode findings below were reproduced locally before acting on them).
Follows [RESEARCH.md](./RESEARCH.md) and [RESEARCH-2.md](./RESEARCH-2.md).

## 1. Where this library sits in the Khmer segmentation landscape

Three tiers are in active use, and this project's dictionary-DP approach is
the first one:

| Tier | Representative tools | Verified characterization |
| --- | --- | --- |
| Dictionary-based | [chamkho](https://github.com/veer66/chamkho) (our `dict.txt` source), **this project** | wordcut-engine algorithm; notably fast (chamkho benchmarks ~2× faster than other dictionary segmenters for Thai) |
| Statistical (CRF) | [khmercut](https://github.com/seanghay/khmercut) / [khmercut-rs](https://github.com/seanghay/khmercut-rs) / [khmercut.cpp](https://github.com/seanghay/khmercut.cpp), [khmer-nltk](https://github.com/VietHoang1512/khmer-nltk) | The popular modern Khmer tools are **CRF-based** (khmercut-rs loads a pretrained `crf_ner_10000.crfsuite` model via the `crfs` crate; khmercut.cpp uses CRFSuite), not dictionary or neural |
| Neural | [UnifiedCut](https://www.researchgate.net/publication/386588060) (2024) | Character-level sequence labeling: 1/2/3-gram embeddings → single-layer 8-head transformer over a 21-char sliding window → B/I binary classifier; no language-specific preprocessing |

**The headline verified numbers (UnifiedCut on khPOS, confirmed 3-0):**
P = 0.983, R = 0.987, **F1 = 0.985** — but **OOV recall = 0.613**.

Two implications, both favorable to this project's framing:

- The accuracy gap from our best config (F1 0.7805, UnigramDp + HMM) to
  neural SOTA is real and large (~0.20 F1). Anyone needing maximum accuracy
  and able to carry a model should use a neural or CRF tool.
- **Even the SOTA neural model recalls only 61% of out-of-vocabulary
  words.** OOV is the structurally hard part of Khmer segmentation at every
  tier — which is exactly the weakness `ROADMAP.md` Phase 4 targeted. Our
  architecture attacks the right problem; it just attacks it with a
  lighter-weight tool.

Also verified: a joint Khmer word-segmentation + POS-tagging model using a
character-level BiLSTM exists in the literature
([arXiv:2103.16801](https://arxiv.org/pdf/2103.16801)), and self-attention
networks are competitive for segmentation with *local windowing* — but the
specific claim that windowed n-gram attention improves OOV generalization
was **refuted (0-3)**; architecture tricks don't dissolve the OOV problem.

## 2. Unicode findings — both acted on in this codebase

### 2a. Khmer's canonical combining classes are erroneous and frozen

Verified against [Unicode TN61](https://www.unicode.org/notes/tn61/utn61-Khmer_Encoding_Structure_V2.pdf)
and **reproduced locally with Python's `unicodedata` before implementing**:

- `U+17D2` KHMER SIGN COENG has ccc=9; `U+17DD` KHMER SIGN ATTHACAN has
  ccc=230. All other Khmer marks have ccc=0.
- Therefore NFC/NFD canonical reordering turns a typed
  `<base, ATTHACAN, COENG, base₂>` into `<base, COENG, ATTHACAN, base₂>` —
  stranding the COENG away from the consonant it subscripts. Confirmed
  locally: `unicodedata.normalize("NFC", ...)` performs exactly this swap.
- This cannot be fixed upstream (Unicode Normalization Stability policy
  freezes ccc values forever), so every NFC-processing pipeline — a
  near-universal habit in web scraping and Python NLP — is a standing
  corruption vector for Khmer text.

**Acted on:** `core/src/normalize.rs` rule 2 repairs the stranded-mark form
(`<COENG, mark, base>` → `<COENG, base, mark>`), converging on the same
canonical form as the existing rule 1 (mark typed before the stack). At the
same time, ZWNJ/ZWJ (`U+200C`/`U+200D`) were excluded from *both* rules:
their meaning is tied to exact position (they request alternate rendering
forms), so reordering them was a latent correctness bug in the original
rule 1.

Measured frequency in our corpora: ~0 (khPOS and kh_data_10000b are not
NFC-processed — 0 lines altered by NFC across both), so the measured F1
effect is zero, same epistemic status as Phase 5's rule 1. The value is
defensive: input that *has* been NFC-normalized upstream segments
correctly instead of mis-clustering.

### 2b. U+200B ZWSP is the standard Khmer word-boundary marker

Verified against the [Unicode core spec ch. 16](http://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-16/)
and [W3C SEAlreq](https://www.w3.org/International/sealreq/khmer/): the
Unicode Standard recommends `U+200B` ZERO WIDTH SPACE for marking word
boundaries in Khmer (it's invisible, so marked text renders identically to
unmarked text). Empirically confirmed in our own data: `kh_data_10000b`'s
raw articles carry hundreds of ZWSP boundary hints per document, and its
segmented reference files use ZWSP as the delimiter.

**Acted on, two ways:**

- **Input:** `segment()` now treats ZWSP as a token separator (consumed,
  never emitted, trusted as a hard boundary even when the dictionary
  contains a word spanning it). Previously a ZWSP in input became a
  stray standalone token and glued into non-Khmer runs — a real-world
  correctness bug, since Khmer web text is full of them.
- **Output:** the CLI gained `-z`/`--zwsp`, joining tokens with ZWSP —
  output that renders identically to the input while carrying
  machine-readable boundaries, and round-trips through the tokenizer.

One deliberate non-action: real corpora contain separators (space/ZWSP)
stuck *inside* a COENG+consonant pair (`ស្ រី`; ~10 instances found across
khPOS + 2,000 kh_data_10000b docs). Not auto-repaired: khPOS's own gold
tokenization treats the fragments as separate tokens, so "repairing" would
contradict the gold standard, and a mid-word space may be an intentional
boundary. Documented rather than guessed at.

## 3. Downstream pretokenization — the thesis is now evidence-backed

- **Confirmed (3-0):** applying word segmentation before Khmer NLG
  pretraining measurably helps downstream generation quality
  ([PrahokBART line of work](https://arxiv.org/html/2512.13552v1)). This
  upgrades `README.md`'s pre-tokenizer pitch from plausible to verified.
- **Confirmed:** among BPE, WordPiece, and SentencePiece-Unigram,
  SentencePiece-Unigram was the most effective subword tokenizer for a
  low-resource abugida-script language (Dzongkha), on normalized sequence
  length, fertility, and continued-word proportion
  ([arXiv:2509.15255](https://arxiv.org/pdf/2509.15255)).
- **Refuted (don't cite):** the specific "4.32% downstream gain from
  optimal-vs-greedy BPE" number (1-2), and the "MorphAcc / BPE memorizes
  surface forms" claims (0-3) — treat morphological-tokenizer-eval claims
  skeptically.

Practical consequence: the highest-leverage integration path for this
library in LLM pipelines is ZWSP-marked (or space-marked) pre-segmentation
feeding a SentencePiece-Unigram trainer — both halves of which this
codebase now supports natively (`--zwsp` output; deterministic
segmentation).

## 4. What this implies for the roadmap (not yet built)

1. **A statistical BMES tagger (CRF or averaged perceptron) as a
   `Strategy` tier** — the verified landscape shows CRF is what the
   popular Khmer tools actually ship. Our `Strategy` seam and the Phase 4
   BMES/HMM training infrastructure (`eval/src/hmm.rs`) are the natural
   substrate; a feature-richer tagger (character class, affix, cluster
   n-gram features) trained the same local-only way would close part of
   the gap to khmercut without abandoning the zero-dependency posture.
2. **Python (PyO3) and WASM bindings** — most Khmer NLP happens in Python;
   the pre-tokenizer use case (§3) is only reachable there. Already on the
   README roadmap; this survey raises its priority.
3. **Not worth pursuing here:** chasing UnifiedCut-class neural accuracy
   inside this crate. It would abandon the zero-dependency, no-model-file
   design that is this project's actual niche; users needing 0.985 F1
   should use a neural tool, and our docs should keep saying so plainly.

## 5. Verification notes

7 of 25 claims were killed by the adversarial panels — including two
plausible-sounding OOV-recall numbers and the "windowed attention improves
OOV" mechanism claim. The findings above only include claims that survived
(2-1 or 3-0) or that were re-verified directly in this repo (all of §2).
The full source list: Unicode TN61, Unicode core spec ch. 16, W3C Khmer
layout requirements, r12a's Khmer orthography notes, khPOS, chamkho,
khmercut (×3 implementations), khmer-nltk, PyThaiNLP, UnifiedCut,
arXiv:2103.16801, arXiv:2512.13552, arXiv:2412.06926, arXiv:2509.15255,
nlpprogress.com (Chinese segmentation benchmarks), SIGHAN bakeoff
literature.
