# Khmer Word Segmentation — Research Notes, Supplement 2

Compiled June 2026 as a fact-checked extension of [RESEARCH.md](./RESEARCH.md).
Sources were adversarially verified (3-vote majority needed to survive; 110 agent
calls, 27 primary sources, 125 claims extracted, 25 verified, 24 confirmed, 1
refuted). Where a claim in the original notes was wrong, this document corrects it
in-line and marks it **⚠ CORRECTION**.

---

## 1. Corrections to existing claims

### 1a. Author attribution — "Fu et al. 2017" ⚠ CORRECTION

RESEARCH.md cites "Fu et al., IJCNLP 2017" for *Recall is the Proper Evaluation
Metric for Word Segmentation*. **The paper is real, but the authors are wrong.**

**Correct citation:**
Yan Shao, Christian Hardmeier, and Joakim Nivre.
"Recall is the Proper Evaluation Metric for Word Segmentation."
*Proceedings of the Eighth International Joint Conference on Natural Language
Processing (IJCNLP 2017)*, Volume 2: Short Papers, pp. 86–90.
Taipei, Taiwan, November 2017. Asian Federation of NLP.
ACL Anthology: https://aclanthology.org/I17-2015/

Verified 3-0 against the primary ACL Anthology record.

### 1b. ~98% BiMM accuracy — confirmed but with essential nuance

RESEARCH.md says "(one source cites ~98% for a BiMM variant — treat as a ceiling,
dataset-dependent)". The figure is real and traceable, but the citation path
matters:

**Primary source:** Narin Bi and Nguonly Taing. "Khmer word segmentation based on
Bi-directional Maximal Matching for Plaintext and Microsoft Word document."
*APSIPA 2014*, DOI 10.1109/APSIPA.2014.7041822. IEEE document 7041822. Pages 1–9.
→ Reports ~98.13% accuracy on a Khmer text corpus. Verified 3-0.

**Follow-on paper (different result):** Makara Mao, Sony Peng, Yixuan Yang,
Doo-Soon Park. "Bi-directional Maximal Matching Algorithm to Segment Khmer Words
in Sentence." *Journal of Information Processing Systems*, Vol. 18 No. 4,
pp. 549–561, August 2022. DOI 10.3745/JIPS.04.0250.
→ This JIPS 2022 paper does **not** report 98.13% itself; it cites the Bi & Taing
2014 figure as prior work. Its own result is **8.57% error reduction** over FMM
and BMM independently, evaluated on 94,807 Khmer words. Verified 3-0.

So the 98% figure is from a 2014 APSIPA paper with a specific dataset; the 2022
JIPS paper demonstrates a relative error reduction (not an absolute accuracy). Both
numbers are real; only the 2014 paper is the source of the 98% claim.

### 1c. silnrsi/khmerlbdict license — ⚠ requires due diligence

The `LICENSE` file in the GitHub repo is MIT (verified 3-0). However, the
adversarial pass (vote 2-1) raised a material concern: **the MIT license covers
the tooling/scripts**, but the generated wordlist data is derived from SEALang
Khmer and other upstream sources whose own commercial terms were not fully
confirmed. The conclusion "safe to bundle in a commercial binary" cannot be
asserted with confidence solely from the repo's MIT file.

**Recommended action:** Before bundling any khmerlbdict-derived wordlist data,
check SEALang's current terms of service directly. The code is MIT; the data may
not be. Keep as download-only until confirmed.

### 1d. ALT corpus — two artifacts, two licenses

The NICT ALT project hosts two distinct artifacts with **different** licenses:

- **ALT Parallel Corpus** (multilingual, non-segmented): **CC BY 4.0** — bundleable.
- **Khmer ALT Treebank** (tokenized + POS-tagged): **CC BY-NC-SA 4.0** — non-commercial, download-only.

Verified 3-0. The Zenodo record (https://zenodo.org/records/3937914) and the
canonical NICT project page both confirm CC BY-NC-SA 4.0 for the Khmer treebank.
RESEARCH.md correctly marks it non-commercial; this note adds the parallel corpus
distinction.

### 1e. khopilot/khmer-lexicon — richer than noted

RESEARCH.md describes it as "a broad lexicon incl. formal/royal terms, CC BY 4.0."
The full schema, verified 3-0, is richer:

- **12,653 unique terms** (confirmed)
- **30,242 semantic relationships** (antonyms, synonyms, morphological, co-occurrence, classifier-for)
- Per-entry fields: `word`, `pos` (POS tag), `frequency` (integer frequency score),
  `weight` (importance weight 0–100), `domain`, `definition` (Khmer-language)

The `frequency` field is an integer score, not a raw corpus count. For the
UnigramDp scorer it is usable as-is for ranking, but normalizing to log-probability
will need either: (a) treating the score as a proxy count, or (b) supplementing
with a corpus-derived frequency table. The `weight` field (0–100) is a separate
importance signal and not a frequency count.

Confirmed CC BY 4.0 verbatim from the HuggingFace dataset card YAML and README.

---

## 2. New tools and resources not in RESEARCH.md

### 2a. chamkho — Rust dictionary-based word breaker

**chamkho** (https://github.com/veer66/chamkho) is a **Rust** dictionary-based
word breaker supporting Khmer, Lao, Myanmar, and Thai. Verified 3-0.

Key facts:
- Language: Rust; approach: maximum matching, dictionary-driven — same family as
  this project.
- Ships bundled dictionaries for all four languages.
- Underlying segmentation algorithm uses a dictionary and the `wordcut` algorithm
  (bidirectional + heuristics).
- License: unconfirmed in this pass — verify before depending on it.

**Relevance:** The most direct Rust peer to `khmer-tokenizer`. Worth reading its
dictionary format and segmentation loop for implementation reference. Unlike
`khmercut-rs` (model-based CRF), chamkho is dictionary-driven like this project.

### 2b. PrahokBART (COLING 2025) — normalization + segmentation confirmed as accuracy levers

"PrahokBART: Pre-trained seq2seq model for Khmer."
*COLING 2025*, February 2025. https://aclanthology.org/2025.coling-main.87/

Relevant finding (verified 3-0): The paper performs an **ablation analysis** that
explicitly measures the contribution of word segmentation and Unicode normalization
as preprocessing modules. Both steps produce measurable gains in downstream task
performance. This validates the project's Phase 3 + Phase 5 roadmap priorities
from an independent empirical source.

### 2c. khopilot/km-tokenizer-khmer — SentencePiece Unigram subword tokenizer

https://huggingface.co/khopilot/km-tokenizer-khmer

A SentencePiece Unigram subword tokenizer with an 8,000-token vocabulary, released
under **Apache 2.0** (bundleable). Distinct from a word segmenter — it operates at
the subword level — but relevant in two ways:

1. Provides a comparison baseline for LLM pre-tokenization (the data-flywheel
   scenario in ARCHITECTURE.md).
2. The Apache 2.0 license on the vocabulary artifact means it is safe to study or
   use as a reference.

### 2d. SEACrowd khmer_alt_pos — HuggingFace mirror of ALT Khmer data

https://huggingface.co/datasets/SEACrowd/khmer_alt_pos

A HuggingFace-hosted version of the NICT ALT Khmer POS data (SEACrowd version
2024.06.20, source version 1.1.0). License: **CC BY-NC-SA 4.0** — non-commercial,
download-only. This is the same data as the Zenodo ALT treebank, just more
convenient to access programmatically for the eval harness.

### 2e. Khmer frequency list at kheng.info

The `seanghay/awesome-khmer-language` index links a Khmer word frequency list at
`kheng.info/frequencies`. License and suitability for bundling were not confirmed
in this pass. Worth investigating as a supplemental frequency source.

---

## 3. Algorithm implementation — deeper guidance

### 3a. jieba's DAG construction (confirmed against source)

jieba (https://github.com/fxsjy/jieba) builds the word graph as follows, confirmed
3-0 against the Go port (jiebago) which exposes the algorithm more readably:

```
for k = 0 to len(clusters)-1:
    dag[k] = []
    i = k
    fragment = clusters[k:k+1]
    loop:
        if dict.has_frequency(fragment):
            dag[k].append(i)   // valid word ending at i
        i++
        fragment = clusters[k:i+1]
        if i >= n: break
    if dag[k] is empty:
        dag[k] = [k]           // single-cluster fallback
```

The DP then runs **right-to-left** over the DAG:

```
route[n] = (0.0, n)           // sentinel
for k = n-1 downto 0:
    best = max over j in dag[k] of:
        log(freq(clusters[k:j+1]) / total) + route[j+1].score
    route[k] = (best, best_j)
```

Reconstruct left-to-right by following `route`. For OOV clusters (single-cluster
fallback in the DAG), jieba applies the HMM Viterbi pass as a second stage over
runs of unrecognized characters.

**Khmer adaptation:** Replace Chinese characters with Khmer Character Clusters
(KCCs) as the atomic unit. The cluster trie you already have is the prefix
dictionary. The only addition needed is a `word → log-frequency` lookup table
(Phase 2's `freq.tsv`).

### 3b. BiMM tie-breaking — what the literature actually says

From Bi & Taing (APSIPA 2014) and the Mao et al. (JIPS 2022) follow-on, verified
against primary sources:

The canonical BiMM tie-breaking rule when FMM and BMM disagree is:
1. **Fewer tokens** — prefer the split with fewer words (avoids over-segmentation).
2. **Fewer single-cluster tokens** — minimize the number of residual single-cluster
   tokens (they are likely segmentation errors).
3. **FMM as tiebreaker** — if still equal, take the forward result.

The JIPS 2022 paper adds a dictionary-structure improvement that pre-sorts the
dictionary to speed up the backward pass. For the Rust implementation:

```rust
fn bimm(clusters: &[KCC], trie: &Trie) -> Vec<Token> {
    let fwd = forward_max_match(clusters, trie);
    let bwd = backward_max_match(clusters, trie);
    if fwd.len() == bwd.len() {
        let fwd_singles = fwd.iter().filter(|t| t.cluster_count == 1).count();
        let bwd_singles = bwd.iter().filter(|t| t.cluster_count == 1).count();
        if fwd_singles <= bwd_singles { fwd } else { bwd }
    } else if fwd.len() <= bwd.len() {
        fwd
    } else {
        bwd
    }
}
```

### 3c. HMM / Viterbi for OOV — BMES state model

jieba's OOV layer uses a 4-state HMM over character n-grams (BMES: Begin, Middle,
End, Single). For Khmer, the atomic unit is again the KCC, not the raw character.

States: `{B, M, E, S}` — each KCC in an OOV run receives one of these tags.
Transition and emission probabilities are estimated from a segmented corpus by
counting:

```
trans[prev_state][curr_state] = count(prev→curr) / count(prev)
emit[state][cluster] = count(cluster in state) / count(state)
```

Viterbi decoding (standard, O(n × |states|²)) finds the most likely BMES sequence,
then extracts word boundaries (B starts a word, S is a single-cluster word).

**Training data constraint:** The NC clause on khPOS and ALT means any model
trained on those corpora inherits the NC restriction. Document this if you ship a
pre-trained HMM: the model file cannot be bundled in an MIT/Apache commercial
binary if it was trained on CC BY-NC-SA data.

---

## 4. Khmer Unicode normalization — concrete specification

### 4a. Canonical syllable structure (KCC)

From SIL's Khmer Character Specification (https://github.com/sillsdev/khmer-character-specification/blob/master/specification.md)
and Unicode Technical Note 61 (https://www.unicode.org/notes/tn61/), verified
against primary sources:

```
Syllable = Base Robat? Coengs? FinalCoeng? Shifter? Vowels Modifiers? Final?
```

Where:
- `Base` — a Khmer consonant (U+1780–U+17A2) or independent vowel (U+17A3–U+17B3)
- `Robat` — U+17CC (a diacritic placed above the base)
- `Coengs` — zero, one, or two repetitions of `U+17D2` + consonant (max two stacked subscripts)
- `FinalCoeng` — a COENG used at the end of a final cluster
- `Shifter` — U+17C9 (MUUSIKATOAN) or U+17CA (TRIISAP)
- `Vowels` — the dependent vowels (U+17B6–U+17C5 plus U+17B4–U+17B5)
- `Modifiers` — combining signs (U+17C6 NIKAHIT, U+17CB BANTOC, U+17D0 SAMYOK SANNYA, etc.)
- `Final` — U+17D1, U+17CE, U+17CF, U+17D3

**Key rules for a normalizer:**
1. COENG subscripts must immediately follow the base and each other in order.
   Reorder any out-of-sequence COENG pairs to canonical order.
2. Shifter (register shifter) must follow all COENGs.
3. Vowels follow the shifter.
4. Modifier signs follow vowels.
5. At most **two** stacked subscripts (COENGs) are allowed.

### 4b. COENG mechanism

U+17D2 KHMER SIGN COENG is a "coeng generator" — it is not itself the visible
subscript glyph. A subscript consonant is the two-codepoint sequence
`U+17D2, consonant`. This is the foundational rule for normalization: never split
or reorder within a COENG pair.

Verified from Unicode Core Spec chapter 16 (https://www.unicode.org/versions/latest/core-spec/chapter-16/).

### 4c. Reference implementations

SIL provides two reference implementations under permissive licenses:
- **khnormal** — Python, in https://github.com/sillsdev/khmer-character-specification
- **khmer-normalizer** — JavaScript, separate repo

Both normalize Khmer strings to canonical KCC ordering. Reading these before
implementing `core/src/normalize.rs` will save significant time.

The 2021 arxiv paper "Pretrained Models and Evaluation Data for the Khmer Language"
(https://arxiv.org/pdf/2112.08918) also performs normalization as part of its NLP
pipeline and is a useful reference for how normalization interacts with downstream
tasks.

### 4d. Why normalization matters quantitatively

PrahokBART (COLING 2025) measured normalization as an isolated ablation and found a
positive contribution. The khmerlbdict/SIL line-breaking work notes that
normalization is a prerequisite for dictionary matching because the same word can
be encoded with different COENG orders that look visually identical but byte-differ.

---

## 5. License / commercial safety — master table

Updated from RESEARCH.md. The "bundle?" column means: safe to commit into the
MIT/Apache repo and ship inside a commercial binary.

| Resource | License (verified) | Bundle? | Notes |
|---|---|---|---|
| khopilot/khmer-lexicon | **CC BY 4.0** | ✅ Yes | Attribution required; has frequency scores |
| silnrsi/khmerlbdict (code/tools) | **MIT** | ✅ Yes | The LICENSE file is MIT |
| silnrsi/khmerlbdict (wordlist data) | **MIT + upstream SEALang** | ⚠ Unclear | Derived from SEALang; verify SEALang's commercial terms before bundling data |
| khPOS corpus | **CC BY-NC-SA 4.0** | ❌ No | NC clause; download-only for evaluation |
| ALT Parallel Corpus (multilingual) | **CC BY 4.0** | ✅ Yes | Not word-segmented; for NMT, not eval |
| ALT Khmer Treebank (tokenized/POS) | **CC BY-NC-SA 4.0** | ❌ No | NC clause; download-only for evaluation |
| SEACrowd khmer_alt_pos (HF mirror) | **CC BY-NC-SA 4.0** | ❌ No | Same data as ALT Khmer Treebank |
| Wiktionary Khmer frequency list | **CC BY-SA** | ❌ No | ShareAlike; keep out of source tree |
| khmercut-rs (code) | **MIT** | ✅ Yes | CRF model file also MIT; model-based approach |
| chamkho (code) | Verify separately | ❓ | Not confirmed in this pass |
| khopilot/km-tokenizer-khmer | **Apache 2.0** | ✅ Yes | Subword tokenizer, not word segmenter |
| SEAlang Khmer corpus | Check current terms | ❓ | Upstream of khmerlbdict; check directly |

**Bottom line:** The only confirmed-safe-to-bundle dictionary source with frequency
data is `khopilot/khmer-lexicon` (CC BY 4.0). For `khmerlbdict` data specifically,
read SEALang's terms before committing any derived wordlist file.

---

## 6. Updated ecosystem table

Additions and corrections to the prior-art table in RESEARCH.md:

| Tool | Language | Approach | License | Notes |
|---|---|---|---|---|
| [chamkho](https://github.com/veer66/chamkho) | **Rust** | Dictionary / max-match | (verify) | Khmer + Lao + Myanmar + Thai; closest Rust peer |
| [khmercut](https://github.com/seanghay/khmercut) | Python | CRF (pycrfsuite) | MIT | pip install; verified CRF-based, not dict |
| [khmercut-rs](https://github.com/seanghay/khmercut-rs) | **Rust** | CRF (crf_ner_10000.crfsuite) | MIT | Model file ships with the repo |
| [khmersegment](https://github.com/seanghay/khmersegment) | Python | CRF++ (NIPTICT/CADT model) | (verify) | Requires external km-5tag-seg-model |
| [UnifiedCut](https://doi.org/10.3390/app142311435) | — | Neural (Thai/Burmese/Khmer) | — | MDPI Applied Sciences 2024; peer-reviewed |
| [PrahokBART](https://aclanthology.org/2025.coling-main.87/) | Python | seq2seq | (verify) | COLING 2025; ablation confirms seg+norm gains |
| [Joint WS+POS](https://arxiv.org/pdf/2103.16801) | Python | BiLSTM | (verify) | Bouy et al. 2021 |

---

## 7. Updated recommendations for the roadmap

These build on §8 of RESEARCH.md with the new evidence:

1. **Phase 2 dictionary:** Use `khopilot/khmer-lexicon` as the primary bundled
   dictionary. Its `frequency` field can drive UnigramDp. If raw corpus counts are
   needed, supplement with `khmerlbdict` — but verify SEALang's terms first and
   keep the data download-only until confirmed.

2. **Phase 3 BiMM tie-breaking:** Use the canonical rule: fewer tokens → fewer
   single-cluster tokens → FMM wins. The APSIPA 2014 and JIPS 2022 papers both
   support this rule.

3. **Phase 3 UnigramDp implementation:** The jieba DAG construction and DP are
   confirmed against primary source. The Rust implementation should work directly
   over the KCC stream (not raw chars) and use `khmer-lexicon`'s `frequency` field
   in log-space.

4. **Phase 4 HMM training:** Any HMM trained on khPOS or ALT inherits the
   CC BY-NC-SA restriction. The pre-trained model file cannot be bundled
   commercially. Either (a) train on permissive data only, (b) require the user to
   download and train, or (c) gate behind a non-default feature flag with clear
   license documentation.

5. **Phase 5 normalization:** Implement against the SIL specification's canonical
   syllable form. Read `khnormal` (Python reference) before writing `normalize.rs`.
   The COENG pair is sacred — never split `U+17D2 + consonant`.

6. **New competitor to mention in README:** chamkho is the most directly comparable
   Rust dictionary-based segmenter. The `khmer-tokenizer` differentiators vs.
   chamkho are: single-language focus, syllable-aware KCC splitting, frequency
   scored segmentation (Phase 3), and explicit evaluation harness.

---

## 8. Additional sources

- Bi, N. & Taing, N. "Khmer word segmentation based on Bi-directional Maximal Matching…" APSIPA 2014. DOI 10.1109/APSIPA.2014.7041822 — https://ieeexplore.ieee.org/document/7041822
- Mao, M., Peng, S., Yang, Y. & Park, D. "Bi-directional Maximal Matching Algorithm to Segment Khmer Words in Sentence." JIPS Vol.18 No.4, 2022. DOI 10.3745/JIPS.04.0250 — https://www.researchgate.net/publication/380461591
- Shao, Y., Hardmeier, C. & Nivre, J. "Recall is the Proper Evaluation Metric for Word Segmentation." IJCNLP 2017. ACL Anthology I17-2015 — https://aclanthology.org/I17-2015/
- Wen, Y. et al. "UnifiedCut: A Simple and Efficient Neural Model for Thai, Burmese and Khmer Word Segmentation." MDPI Applied Sciences 2024. DOI 10.3390/app142311435 — https://doi.org/10.3390/app142311435
- PrahokBART (COLING 2025) — https://aclanthology.org/2025.coling-main.87/
- SIL Khmer Character Specification — https://github.com/sillsdev/khmer-character-specification/blob/master/specification.md
- Unicode Technical Note 61 (Khmer Encoding Structure) — https://www.unicode.org/notes/tn61/utn61-Khmer_Encoding_Structure_V2.pdf
- Unicode Core Spec Chapter 16 (Khmer) — https://www.unicode.org/versions/latest/core-spec/chapter-16/
- Unicode L2/2022/22290 (Khmer Encoding) — https://www.unicode.org/L2/L2022/22290-khmer-encoding.pdf
- chamkho (Rust dictionary word breaker) — https://github.com/veer66/chamkho
- khopilot/km-tokenizer-khmer (Apache 2.0 SentencePiece tokenizer) — https://huggingface.co/khopilot/km-tokenizer-khmer
- SEACrowd khmer_alt_pos (HF mirror of ALT Khmer) — https://huggingface.co/datasets/SEACrowd/khmer_alt_pos
- Pretrained Models and Evaluation Data for the Khmer Language (arxiv 2112.08918) — https://arxiv.org/pdf/2112.08918
- jieba source (algorithm reference) — https://github.com/fxsjy/jieba
- jiebago (Go port, algorithm more readable) — https://github.com/wangbin/jiebago/blob/master/jieba.go
- Phylypo Tum — NLP: Text Segmentation Using Dictionary Based Algorithms — https://medium.com/@phylypo/nlp-text-segmentation-using-dictionary-based-algorithms-6d0a45a76c08
- khnormal (Python Khmer normalizer) — https://github.com/sillsdev/khmer-character-specification
- SEACrowd dataset Zenodo (ALT Khmer treebank) — https://zenodo.org/records/3937914
