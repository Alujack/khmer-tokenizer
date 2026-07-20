# Trained models (optional, non-commercial)

This directory documents the **optional** high-accuracy tagger model. **No
model file is committed here** — model files are `.gitignore`d, because every
model this project can currently train is derived from a **non-commercial**
corpus and must not be relicensed under the crate's MIT/Apache license.

## What the model is

An averaged-perceptron BMES tagger (`khmer_tokenizer_core::TaggerModel`) — the
CRF-class tier. Loaded with `--tagger` (CLI), `with_tagger(...)` (Rust),
`tagger=...` (Python), or `{ tagger: ... }` (JS). Under `Strategy::Tagger` it
segments every Khmer run itself; under a dictionary strategy it acts as the
out-of-vocabulary fallback.

## Accuracy (khPOS OPEN-TEST, 1,000 sentences)

| Model | F1 | Notes |
| --- | --- | --- |
| Trained on khPOS train (12k) | **0.94** | in-domain (shared annotators/conventions) |
| Same model, on `kh_data_10000b` (silver) | **≈ 0.87** | cross-corpus — the honest real-world number |

Compare the shipped, model-free dictionary default: **F1 0.75**.

## License — CC BY-NC-SA 4.0 (non-commercial)

Any model produced by `cargo xtask train-tagger` is a derivative of its training
corpus:

- **khPOS** — © Ye Kyaw Thu et al., CC BY-NC-SA 4.0
  (<https://github.com/ye-kyaw-thu/khPOS>)
- **Khmer ALT Treebank** (with `--alt`) — © NICT/NIPTICT, CC BY-NC-SA 4.0
  (<https://www2.nict.go.jp/astrec-att/member/mutiyama/ALT/>)

Both carry a **NonCommercial + ShareAlike** clause, so a trained model:

- may be used and redistributed **for non-commercial purposes only**,
- must keep the **CC BY-NC-SA 4.0** license and the attribution above, and
- **cannot be bundled** into this MIT/Apache-licensed crate.

This is why the crate ships no model. It is the same reason spaCy, Stanza, and
similar libraries keep permissive code and separately-licensed models.

## How to get a model

```bash
# Train locally (seconds). Downloads khPOS for your own non-commercial use.
cargo xtask train-tagger kh-tagger.model
cargo xtask train-tagger --alt kh-tagger.model    # or from ALT

# A pre-trained model may also be attached to a GitHub Release (CC BY-NC-SA).
```

## Commercial use

Train your own model with `TaggerModel::train` / `cargo xtask train-tagger`
pointed at a corpus **you are licensed to use commercially**. The engine is
permissive; only these specific *models* are restricted, because their *training
data* is.
