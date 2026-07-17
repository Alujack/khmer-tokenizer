//! A CRF-class statistical BMES tagger: an **averaged structured
//! perceptron** (Collins 2002) over Khmer Character Clusters, decoded with
//! Viterbi. This is the tier between this crate's dictionary strategies and
//! neural segmenters — the same tier as the popular CRF-based Khmer tools
//! (see `docs/RESEARCH-3.md` §4) — implemented dependency-free.
//!
//! Where [`HmmModel`](crate::HmmModel) scores each cluster only by its own
//! identity, this tagger scores it by a feature window (the cluster, its
//! neighbors two out in each direction, adjacent bigrams, and its length),
//! which is what lets it generalize to unseen words instead of falling back
//! on transition structure alone.
//!
//! Like the HMM and the `UnigramDp` frequencies, **no trained model ships
//! with this crate** — training needs a segmented corpus, and no bundleable,
//! commercially-clean one has been found (`docs/ROADMAP.md`). Train your own
//! with [`TaggerModel::train`] from a corpus you're licensed to use, persist
//! it with [`TaggerModel::to_text`]/[`TaggerModel::from_text`], and attach
//! it with [`KhmerTokenizer::with_tagger`](crate::KhmerTokenizer::with_tagger)
//! (OOV fallback) or [`Strategy::Tagger`](crate::Strategy::Tagger) (full
//! tagger segmentation, ignoring the dictionary).

use std::collections::HashMap;

use crate::kcc::{is_khmer, is_khmer_base, is_khmer_digit, split_kcc, COENG};
use crate::viterbi::{self, BEGIN, END, MIDDLE, NUM_STATES, SINGLE};

/// Sentinel "cluster" for positions before the start / past the end of a
/// run when building context features. Contains ASCII only, so it can never
/// collide with a real Khmer cluster.
const BOUNDARY: &str = "<s>";

/// A trained averaged-perceptron BMES tagger. See the module docs.
#[derive(Clone, Default)]
pub struct TaggerModel {
    /// Score for each tag at the first position of a run.
    start: [f64; NUM_STATES],
    /// Score for tag `j` directly following tag `i`.
    trans: [[f64; NUM_STATES]; NUM_STATES],
    /// Per-tag score for each context feature.
    weights: HashMap<String, [f64; NUM_STATES]>,
}

/// Helper function to classify KCC clusters for features.
fn cluster_class(cluster: &str) -> &'static str {
    let first = match cluster.chars().next() {
        Some(c) => c,
        None => return "EMPTY",
    };
    if is_khmer(first) {
        if is_khmer_digit(first) {
            "KhmerDigit"
        } else if is_khmer_base(first) {
            if cluster.contains(COENG) {
                "KhmerSub"
            } else {
                "KhmerBase"
            }
        } else {
            "KhmerPunc"
        }
    } else if first.is_ascii_digit() {
        "Digit"
    } else if first.is_whitespace() {
        "Space"
    } else {
        "NonKhmer"
    }
}

/// Context feature strings for position `i` of `clusters`. Templates:
/// current cluster, neighbors at ±1 and ±2, adjacent bigrams, trigrams,
/// cluster length, and cluster type-class features.
fn features(clusters: &[String], i: usize) -> Vec<String> {
    let at = |k: isize| -> &str {
        if k < 0 || k as usize >= clusters.len() {
            BOUNDARY
        } else {
            &clusters[k as usize]
        }
    };

    let class_at = |k: isize| -> &str {
        if k < 0 || k as usize >= clusters.len() {
            "<s>"
        } else {
            cluster_class(&clusters[k as usize])
        }
    };

    let i = i as isize;
    vec![
        // Character Cluster Unigrams
        format!("U0={}", at(i)),
        format!("U-1={}", at(i - 1)),
        format!("U+1={}", at(i + 1)),
        format!("U-2={}", at(i - 2)),
        format!("U+2={}", at(i + 2)),
        // Cluster Bigrams
        format!("B-1={}|{}", at(i - 1), at(i)),
        format!("B+1={}|{}", at(i), at(i + 1)),
        format!("B-2={}|{}", at(i - 2), at(i - 1)),
        format!("B+2={}|{}", at(i + 1), at(i + 2)),
        // Cluster Trigrams
        format!("T0={}|{}|{}", at(i - 1), at(i), at(i + 1)),
        // Cluster Length
        format!("L0={}", at(i).chars().count()),
        // Type Class Unigrams
        format!("C0={}", class_at(i)),
        format!("C-1={}", class_at(i - 1)),
        format!("C+1={}", class_at(i + 1)),
        // Type Class Bigrams
        format!("CB-1={}|{}", class_at(i - 1), class_at(i)),
        format!("CB+1={}|{}", class_at(i), class_at(i + 1)),
    ]
}

/// BMES tags for one gold word's clusters (same convention as the HMM
/// trainer): a single cluster is Single; two or more are Begin, Middle*, End.
fn tag_word(clusters: &[String]) -> Vec<usize> {
    match clusters.len() {
        0 => Vec::new(),
        1 => vec![SINGLE],
        n => {
            let mut tags = vec![MIDDLE; n];
            tags[0] = BEGIN;
            tags[n - 1] = END;
            tags
        }
    }
}

/// Split gold-segmented sentences into maximal runs of consecutive Khmer
/// *letter* words, tagging every word's clusters BMES. A run breaks at any
/// word that doesn't start with a Khmer letter base — non-Khmer words, but
/// also Khmer digits and punctuation (។, ១២៣) — the same boundary
/// `KhmerTokenizer::segment` uses when handing a contiguous letter run to
/// the tagger.
fn extract_runs(sentences: &[Vec<String>]) -> Vec<(Vec<String>, Vec<usize>)> {
    let mut runs = Vec::new();
    for sentence in sentences {
        let mut clusters: Vec<String> = Vec::new();
        let mut tags: Vec<usize> = Vec::new();
        for word in sentence {
            if !word.chars().next().is_some_and(is_khmer_base) {
                if !clusters.is_empty() {
                    runs.push((std::mem::take(&mut clusters), std::mem::take(&mut tags)));
                }
                continue;
            }
            let word_clusters = split_kcc(word);
            tags.extend(tag_word(&word_clusters));
            clusters.extend(word_clusters);
        }
        if !clusters.is_empty() {
            runs.push((clusters, tags));
        }
    }
    runs
}

/// Weight averaging bookkeeping for one weight cell (the standard lazy
/// "accumulate on touch" trick): `total` carries weight × steps-held, and
/// `stamp` is the step at which `weight` last changed.
#[derive(Clone, Copy, Default)]
struct Cell {
    weight: f64,
    total: f64,
    stamp: u64,
}

impl Cell {
    fn update(&mut self, delta: f64, step: u64) {
        self.total += (step - self.stamp) as f64 * self.weight;
        self.stamp = step;
        self.weight += delta;
    }

    fn average(&self, final_step: u64) -> f64 {
        let total = self.total + (final_step - self.stamp) as f64 * self.weight;
        total / final_step as f64
    }
}

/// Deterministic xorshift64* PRNG for epoch shuffling — training must be
/// reproducible run-to-run, so no OS entropy.
struct XorShift64(u64);

impl XorShift64 {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

/// Viterbi-decode with the *current* (non-averaged) training weights. Shares
/// the lattice with [`crate::viterbi`]; only the emission scoring — summing
/// the live `Cell` weights of each active feature — is specific to training.
fn viterbi_decode(
    clusters: &[String],
    start: &[f64; NUM_STATES],
    trans: &[[f64; NUM_STATES]; NUM_STATES],
    weights: &HashMap<String, [Cell; NUM_STATES]>,
) -> Vec<usize> {
    viterbi::viterbi(clusters.len(), start, trans, |i| {
        let mut scores = [0.0; NUM_STATES];
        for feat in features(clusters, i) {
            if let Some(cells) = weights.get(&feat) {
                for (score, cell) in scores.iter_mut().zip(cells) {
                    *score += cell.weight;
                }
            }
        }
        scores
    })
}

impl TaggerModel {
    /// Train from gold-segmented sentences (each a `Vec` of words in
    /// sentence order; non-Khmer words act as run boundaries and are
    /// otherwise ignored). `epochs` full passes are made over the data,
    /// shuffled deterministically between passes, and the returned model
    /// holds the **averaged** weights — training is fully reproducible.
    ///
    /// Typical usage: 5 epochs over a few thousand sentences trains in
    /// seconds.
    // Several loops here index two parallel `[_; NUM_STATES]` structures at
    // once (the live `Cell` matrices and the averaged `model.*` arrays), which
    // a range loop expresses more clearly than an iterator chain.
    #[allow(clippy::needless_range_loop)]
    pub fn train(sentences: &[Vec<String>], epochs: usize) -> Self {
        let runs = extract_runs(sentences);

        let mut start = [Cell::default(); NUM_STATES];
        let mut trans = [[Cell::default(); NUM_STATES]; NUM_STATES];
        let mut weights: HashMap<String, [Cell; NUM_STATES]> = HashMap::new();

        let mut order: Vec<usize> = (0..runs.len()).collect();
        let mut rng = XorShift64(0x9E37_79B9_7F4A_7C15);
        let mut step: u64 = 0;

        for _ in 0..epochs {
            rng.shuffle(&mut order);
            for &idx in &order {
                let (clusters, gold) = &runs[idx];
                step += 1;

                // Decode with the *current* (non-averaged) weights.
                let mut current_start = [0.0; NUM_STATES];
                for s in 0..NUM_STATES {
                    current_start[s] = start[s].weight;
                }
                let mut current_trans = [[0.0; NUM_STATES]; NUM_STATES];
                for s in 0..NUM_STATES {
                    for j in 0..NUM_STATES {
                        current_trans[s][j] = trans[s][j].weight;
                    }
                }
                let pred = viterbi_decode(clusters, &current_start, &current_trans, &weights);
                if &pred == gold {
                    continue;
                }

                if gold[0] != pred[0] {
                    start[gold[0]].update(1.0, step);
                    start[pred[0]].update(-1.0, step);
                }
                for t in 1..gold.len() {
                    if (gold[t - 1], gold[t]) != (pred[t - 1], pred[t]) {
                        trans[gold[t - 1]][gold[t]].update(1.0, step);
                        trans[pred[t - 1]][pred[t]].update(-1.0, step);
                    }
                }
                for t in 0..gold.len() {
                    if gold[t] == pred[t] {
                        continue;
                    }
                    for feat in features(clusters, t) {
                        let cells = weights.entry(feat).or_default();
                        cells[gold[t]].update(1.0, step);
                        cells[pred[t]].update(-1.0, step);
                    }
                }
            }
        }

        // Final model = averaged weights, dropping exact zeros (features
        // whose updates cancelled out) to keep the model compact.
        let final_step = step.max(1);
        let mut model = TaggerModel::default();
        for s in 0..NUM_STATES {
            model.start[s] = start[s].average(final_step);
            for j in 0..NUM_STATES {
                model.trans[s][j] = trans[s][j].average(final_step);
            }
        }
        for (feat, cells) in weights {
            let averaged: Vec<f64> = cells.iter().map(|c| c.average(final_step)).collect();
            if averaged.iter().any(|&w| w != 0.0) {
                model
                    .weights
                    .insert(feat, [averaged[0], averaged[1], averaged[2], averaged[3]]);
            }
        }
        model
    }

    /// Summed feature score for each tag at position `i`.
    fn emit_scores(&self, clusters: &[String], i: usize) -> [f64; NUM_STATES] {
        let mut scores = [0.0; NUM_STATES];
        for feat in features(clusters, i) {
            if let Some(ws) = self.weights.get(&feat) {
                for s in 0..NUM_STATES {
                    scores[s] += ws[s];
                }
            }
        }
        scores
    }

    /// Viterbi-decode the highest-scoring BMES tag sequence for `clusters`.
    /// `clusters` must be non-empty. Ties break toward the lower tag index,
    /// so decoding is deterministic. Emissions are the summed feature scores;
    /// the lattice is the shared [`crate::viterbi`] decoder.
    fn viterbi_tags(&self, clusters: &[String]) -> Vec<usize> {
        viterbi::viterbi(clusters.len(), &self.start, &self.trans, |i| {
            self.emit_scores(clusters, i)
        })
    }

    /// Segment a run of clusters by Viterbi-decoded BMES tags. Used both as
    /// the OOV fallback (via
    /// [`with_tagger`](crate::KhmerTokenizer::with_tagger)) and as the full
    /// segmenter (via [`Strategy::Tagger`](crate::Strategy::Tagger)).
    pub(crate) fn segment_clusters(&self, clusters: &[String]) -> Vec<Vec<String>> {
        if clusters.is_empty() {
            return Vec::new();
        }
        let tags = self.viterbi_tags(clusters);
        viterbi::bmes_to_tokens(clusters, &tags)
    }

    /// Serialize to a plain-text format (`khmer-tokenizer-tagger v1`):
    /// tab-separated lines, one per weight vector, loadable with
    /// [`from_text`](TaggerModel::from_text). Feature lines are sorted so
    /// the output is byte-identical run-to-run. Feature keys have `\`,
    /// tab, newline, and carriage return escaped, so a model trained on
    /// words containing them still round-trips.
    pub fn to_text(&self) -> String {
        let fmt = |ws: &[f64; NUM_STATES]| {
            ws.iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join("\t")
        };
        let mut out = String::from("khmer-tokenizer-tagger v1\n");
        out.push_str(&format!("S\t{}\n", fmt(&self.start)));
        for (i, row) in self.trans.iter().enumerate() {
            out.push_str(&format!("T\t{i}\t{}\n", fmt(row)));
        }
        let mut feats: Vec<&String> = self.weights.keys().collect();
        feats.sort();
        for feat in feats {
            out.push_str(&format!(
                "F\t{}\t{}\n",
                escape_key(feat),
                fmt(&self.weights[feat])
            ));
        }
        out
    }

    /// Load a model serialized with [`to_text`](TaggerModel::to_text).
    /// Returns a description of the first malformed line on failure.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        match lines.next() {
            Some("khmer-tokenizer-tagger v1") => {}
            other => {
                return Err(format!(
                    "expected header \"khmer-tokenizer-tagger v1\", found {other:?}"
                ))
            }
        }

        let parse4 = |fields: &[&str], line_no: usize| -> Result<[f64; NUM_STATES], String> {
            if fields.len() != NUM_STATES {
                return Err(format!(
                    "line {line_no}: expected {NUM_STATES} weights, found {}",
                    fields.len()
                ));
            }
            let mut ws = [0.0; NUM_STATES];
            for (i, f) in fields.iter().enumerate() {
                let w = f
                    .parse::<f64>()
                    .map_err(|e| format!("line {line_no}: bad weight {f:?}: {e}"))?;
                // "NaN"/"inf" parse successfully but poison every Viterbi
                // comparison — a corrupted model file must fail here, at
                // load, not decode garbage later.
                if !w.is_finite() {
                    return Err(format!("line {line_no}: non-finite weight {f:?}"));
                }
                ws[i] = w;
            }
            Ok(ws)
        };

        let mut model = TaggerModel::default();
        for (line_no, line) in lines.enumerate() {
            let line_no = line_no + 2; // 1-based, after the header
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[0] {
                "S" => model.start = parse4(&fields[1..], line_no)?,
                "T" => {
                    let i: usize = fields
                        .get(1)
                        .and_then(|f| f.parse().ok())
                        .filter(|&i| i < NUM_STATES)
                        .ok_or_else(|| format!("line {line_no}: bad transition row index"))?;
                    model.trans[i] = parse4(&fields[2..], line_no)?;
                }
                "F" => {
                    if fields.len() != 2 + NUM_STATES {
                        return Err(format!("line {line_no}: malformed feature line"));
                    }
                    let ws = parse4(&fields[2..], line_no)?;
                    model.weights.insert(unescape_key(fields[1]), ws);
                }
                other => return Err(format!("line {line_no}: unknown record type {other:?}")),
            }
        }
        Ok(model)
    }

    /// Number of context features with non-zero weight.
    pub fn feature_count(&self) -> usize {
        self.weights.len()
    }
}

/// Escape a feature key for the tab-separated, line-based `to_text` format.
/// Keys embed raw cluster text, and `TaggerModel::train` accepts arbitrary
/// words — a tab or newline inside one must not corrupt the model file.
fn escape_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

/// Inverse of [`escape_key`]. Unknown escapes pass through as-is.
fn unescape_key(escaped: &str) -> String {
    let mut out = String::with_capacity(escaped.len());
    let mut chars = escaped.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny gold corpus where "ខគ" (clusters ["ខ","គ"]) is always one
    /// word and "ក" is always its own word, in varied contexts.
    fn corpus() -> Vec<Vec<String>> {
        let s = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        vec![
            s(&["ខគ", "ក"]),
            s(&["ក", "ខគ"]),
            s(&["ខគ"]),
            s(&["ក", "ខគ", "ក"]),
            s(&["ខគ", "ខគ"]),
        ]
    }

    #[test]
    fn learns_word_shapes_from_a_synthetic_corpus() {
        let model = TaggerModel::train(&corpus(), 5);
        let clusters: Vec<String> = ["ខ", "គ", "ក"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            model.segment_clusters(&clusters),
            vec![
                vec!["ខ".to_string(), "គ".to_string()],
                vec!["ក".to_string()]
            ]
        );
    }

    #[test]
    fn training_is_deterministic() {
        let a = TaggerModel::train(&corpus(), 5).to_text();
        let b = TaggerModel::train(&corpus(), 5).to_text();
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_round_trips() {
        let model = TaggerModel::train(&corpus(), 5);
        let restored = TaggerModel::from_text(&model.to_text()).unwrap();
        assert_eq!(model.to_text(), restored.to_text());

        // And the restored model decodes identically.
        let clusters: Vec<String> = ["ខ", "គ", "ក"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            model.segment_clusters(&clusters),
            restored.segment_clusters(&clusters)
        );
    }

    #[test]
    fn from_text_rejects_garbage() {
        assert!(TaggerModel::from_text("not a model").is_err());
        assert!(TaggerModel::from_text("khmer-tokenizer-tagger v1\nX\t1\t2\t3\t4").is_err());
        assert!(TaggerModel::from_text("khmer-tokenizer-tagger v1\nS\t1\t2").is_err());
    }

    #[test]
    fn from_text_rejects_non_finite_weights() {
        // "NaN" and "inf" parse as f64 — but NaN poisons every Viterbi
        // comparison, so a corrupted model must fail at load, loudly.
        for bad in ["NaN", "inf", "-inf"] {
            let text = format!("khmer-tokenizer-tagger v1\nS\t{bad}\t0\t0\t0");
            assert!(TaggerModel::from_text(&text).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn keys_with_tabs_and_newlines_still_round_trip() {
        // train() accepts arbitrary words; a tab or newline inside one ends
        // up inside feature keys and must not corrupt the line-based format.
        let sentences = vec![vec![
            "ក\tx".to_string(),
            "ក\ny".to_string(),
            "ខគ".to_string(),
        ]];
        let model = TaggerModel::train(&sentences, 3);
        let restored = TaggerModel::from_text(&model.to_text()).unwrap();
        assert_eq!(model.to_text(), restored.to_text());
    }

    #[test]
    fn zero_epoch_model_degrades_to_one_token_per_cluster() {
        let model = TaggerModel::train(&corpus(), 0);
        let clusters: Vec<String> = ["ខ", "គ"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            model.segment_clusters(&clusters),
            vec![vec!["ខ".to_string()], vec!["គ".to_string()]]
        );
    }

    #[test]
    fn empty_run_yields_no_tokens() {
        let model = TaggerModel::train(&corpus(), 2);
        assert!(model.segment_clusters(&[]).is_empty());
    }

    #[test]
    fn unseen_clusters_decode_without_panicking() {
        let model = TaggerModel::train(&corpus(), 2);
        let clusters: Vec<String> = ["ង", "ច"].iter().map(|s| s.to_string()).collect();
        // No assertion on the exact split — unseen contexts must degrade
        // gracefully (transition/boundary features only), never crash.
        assert_eq!(
            model.segment_clusters(&clusters).concat(),
            vec!["ង".to_string(), "ច".to_string()]
        );
    }

    #[test]
    fn non_khmer_words_break_training_runs() {
        let sentences = vec![vec!["ក".to_string(), "Rust".to_string(), "ខគ".to_string()]];
        let runs = extract_runs(&sentences);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].1, vec![SINGLE]);
        assert_eq!(runs[1].1, vec![BEGIN, END]);
    }
}
