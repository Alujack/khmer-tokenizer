# Learning Guide

A path to understanding *everything* this project does — from the Rust you write
today to the model training you'll do later. It's organized as **tracks**. Each
track lists what to learn, *why it matters here*, the core idea in plain words,
and where to learn it. Learn **just enough** of each track to do the current
phase; come back as the project grows.

How the tracks map to the build phases ([ROADMAP.md](./ROADMAP.md)):

| Phase | What you build | Tracks you need |
|-------|----------------|-----------------|
| Done | dictionary segmenter + CLI | A (Rust basics), B (Khmer script) |
| 1 | evaluation harness | A, D (evaluation) |
| 2 | real dictionary | B, E1 (data wrangling) |
| 3 | scored segmentation | C (algorithms), E2 (probability) |
| 4 | unknown-word model | C, E2 |
| 5 | normalization | B |
| future | bindings, training, LLM | A (wasm/PyO3), E3 (ML) |

---

## Track A — Rust, enough to build this

You already chose Rust; here's the subset that this project actually exercises,
roughly in the order you'll meet it.

1. **Ownership, borrowing, lifetimes.** Rust's defining idea: every value has one
   owner; you either *move* it or *borrow* it (`&` shared, `&mut` exclusive). Why
   here: the trie walk borrows nodes as it descends — understanding borrows is
   what makes that code click instead of fighting the compiler.
2. **Structs, enums, pattern matching.** `enum` + `match` is how Rust models
   "one of several cases" — exactly our `Strategy` (forward / bidirectional /
   scored). Learn `Option<T>` and `match`/`if let`.
3. **Traits & generics.** A trait is a shared interface (like `Segmenter`).
   Generics (`fn from_words<I, S>`) let one function accept many types. Why here:
   the `Strategy` seam and `from_words` already use these.
4. **Collections & iterators.** `Vec`, `HashMap`, and iterator chains
   (`.map().filter().collect()`). Why here: clusters are a `Vec`, the trie is
   `HashMap`s, dictionary loading is an iterator chain.
5. **Modules & workspaces.** How `core`, `cli`, `eval` are separate crates in one
   workspace. You've seen this in `Cargo.toml`.
6. **Testing.** `#[test]`, `assert_eq!`, `cargo test`, and doctests. Why here:
   Phase 1 lives or dies on tests and measurement.
7. **Error handling.** `Result<T, E>`, the `?` operator. Needed once you read
   files (corpora, dictionaries).
8. **(Later) Performance & FFI.** `criterion` benchmarks; `wasm-bindgen` for the
   browser; `PyO3` for Python. Only when you reach the bindings phase.

Learn from: *The Rust Book* (doc.rust-lang.org/book) — chapters 4 (ownership), 6
(enums), 10 (traits/generics), 11 (testing), 13 (iterators). *Rust by Example* for
quick snippets. Don't try to master all of Rust first; learn each chapter the
week you need it.

## Track B — Khmer script & Unicode

The linguistic knowledge that makes correct segmentation possible.

- **Abugida / the writing system.** Khmer isn't an alphabet; a base consonant
  carries vowels and stacked subscript consonants. The visual "syllable" is
  several Unicode characters.
- **Khmer Character Cluster (KCC).** The rule for grouping those characters into
  one orthographic unit: a base, plus `COENG (U+17D2)`+consonant subscripts, plus
  dependent vowels/signs. *This is the heart of `kcc.rs`.* Understand it and you
  understand why we never segment on raw `char`s.
- **The Unicode block `U+1780–U+17FF`.** Which code points are bases, which are
  combining marks. (See the ranges in `kcc.rs`.)
- **Orthographic variance / normalization.** The same word can be typed as
  different byte sequences that look identical. Canonicalizing them (Phase 5) is
  what stops the dictionary from missing real words.

Learn from: the Unicode Standard chapter on Southeast Asian scripts; SIL's Khmer
script & line-breaking notes (`silnrsi/khmerlbdict`); read `kcc.rs` line by line
against real Khmer words.

## Track C — Segmentation algorithms

The core computer-science of "where do words begin and end?", easiest to hardest.
You don't need all of it at once — each maps to a phase.

1. **Dictionary + trie (prefix tree).** A trie stores words so you can ask "does a
   word start here, and how long is the longest one?" in near-constant time per
   step. *Mental model:* a branching map of letters; following branches spells
   words. (Built — `trie.rs`.)
2. **Maximum matching (greedy longest-match).** At each position, take the longest
   dictionary word that fits, then continue. Simple and fast; its weakness is
   *ambiguity* — greedy isn't always globally right. (Built.)
3. **Bidirectional max-match (BiMM).** Do it left-to-right *and* right-to-left;
   when they disagree, pick the cleaner result (fewer tokens). Cheap accuracy
   bump. (Phase 3.)
4. **DAG + max-probability path (the big idea).** Treat every possible word match
   as an edge in a graph (a DAG). Each edge has a cost from how *frequent* that
   word is. Find the cheapest path through the whole sentence with **dynamic
   programming**. This chooses the globally best split, not a greedy local one.
   This is how `jieba` (Chinese) works and is your recommended default. (Phase 3.)
5. **HMM + Viterbi (for unknown words).** Model each cluster as having a hidden
   tag — **B**egin/**M**iddle/**E**nd/**S**ingle — and find the most likely tag
   sequence with the Viterbi algorithm. This segments words that aren't in the
   dictionary at all. (Phase 4.)
6. **CRF / neural (the frontier).** Conditional Random Fields and BiLSTM models
   *learn* boundaries from labeled data and use context. Highest accuracy, needs
   training data and a model file. Understand conceptually now; build later.

Two concepts power 4 and 5 — learn these well:

- **Dynamic programming:** break a big problem into overlapping small ones and
  reuse answers. (Classic warm-ups: Fibonacci with memoization, then "word
  break" on LeetCode — *that problem is literally simplified word segmentation*.)
- **Viterbi:** dynamic programming over hidden states; the standard decoder for
  HMMs and CRFs.

Learn from: Jurafsky & Martin, *Speech and Language Processing* (free online) —
chapters on n-gram language models, HMMs/Viterbi, and sequence labeling/CRF;
the `jieba` source + its DeepWiki write-up for a concrete DAG+DP example.

## Track D — Evaluation (Phase 1, do early)

You can't improve what you can't measure. This track is small but essential.

- **Gold corpus / train-test split.** A human-segmented dataset (khPOS, ALT) is
  "ground truth." You compare your output against it. Never tune on your test set.
- **Precision, Recall, F1.** *Precision* = of the words you produced, how many
  were right. *Recall* = of the real words, how many you found. *F1* = their
  harmonic mean. A word counts only if **both** its boundaries match the gold.
- **OOV recall (R-oov).** Recall measured only on out-of-vocabulary words —
  reveals how well you handle words not in the dictionary. This is usually the
  weakest spot and the most honest number.
- **Error analysis.** Look at *what* you get wrong (over-splitting compounds?
  missing names?) — that tells you which phase to do next.

Learn from: any "precision/recall/F1" tutorial; the SIGHAN Chinese-segmentation
bakeoff scoring convention; Fu et al., *"Recall is the Proper Evaluation Metric
for Word Segmentation"* (aclanthology.org/I17-2015) for the nuance.

## Track E — The data & ML future

For when the project becomes a training platform. Three sub-tracks.

**E1 — Data wrangling.** Cleaning and shaping text data: Unicode normalization,
deduplication, building `word<TAB>count` frequency tables, file formats. Mostly
Rust iterators + small scripts (`xtask`). Needed from Phase 2.

**E2 — Probability for language.** *n-gram language models* (how likely is this
word / this sequence?), turning counts into probabilities, why we add logs
instead of multiplying tiny numbers (numerical stability), smoothing for unseen
words. This is the math under the `UnigramDp` scorer and the HMM. Light but
important. (Jurafsky & Martin, the language-model chapters.)

**E3 — Machine learning for sequences.** The path to a trained model:
- **Weak supervision / self-training:** use your rule-based segmenter to make
  *silver* labels, train a model on them, let the model relabel, repeat.
- **Sequence labeling:** framing segmentation as tagging each cluster B/M/E/S;
  this is the bridge from dictionary methods to CRF/neural ones.
- **Subword tokenization & pre-tokenization:** BPE / SentencePiece for LLMs, and
  why word-segmenting *first* produces a cleaner Khmer subword vocabulary.
- **New-word discovery:** statistics (mutual information, branching entropy) that
  find unknown words in raw text to grow the dictionary automatically.

Learn from: Jurafsky & Martin (sequence labeling, CRF); the HuggingFace
`tokenizers` docs (BPE, pre-tokenizers); papers in [RESEARCH.md](./RESEARCH.md)
(joint WS+POS, UnifiedCut) for how others trained Khmer models.

---

## A sane order to actually learn in

1. **Now:** Track A chapters 4/6/10/11 + Track B (read `kcc.rs` and `trie.rs`
   until every line makes sense). You'll fully understand the code you already
   have.
2. **Phase 1:** Track D. Build the harness, get your first F1 number.
3. **Phase 2–3:** Track C items 1–4 + Track E1/E2 (dynamic programming is the key
   skill — do the "word break" exercise). Ship scored segmentation.
4. **Phase 4–5:** Track C item 5 (HMM/Viterbi) + finish Track B (normalization).
5. **Future:** Track A FFI (wasm/PyO3) + Track E3 (the ML platform).

## How to study so it sticks

- **Read the code against real input.** Take `សួស្តីអ្នកទាំងអស់គ្នា`, trace it
  through `kcc.rs` then `trie.rs` by hand, and check against the Python prototype
  in your head. Understanding beats memorizing.
- **Learn each concept the week you build it**, not all upfront — you'll forget
  unused theory.
- **Make the number go up.** Once Phase 1 exists, every concept you learn should
  move F1 on the harness. That feedback loop is the best teacher.
- **Write down what you change in `BENCHMARKS.md`.** Teaching your future self is
  how you really learn.
