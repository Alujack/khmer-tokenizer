//! A lightweight Hidden Markov Model over Khmer Character Clusters, used only
//! as a fallback for clusters no dictionary strategy could match at all (see
//! [`KhmerTokenizer::with_hmm`](crate::KhmerTokenizer::with_hmm)). States are
//! the classic **BMES** tag set (Begin/Middle/End/Single), decoded with
//! Viterbi — the same jieba-style design as `Strategy::UnigramDp`'s DAG, but
//! for clusters that have zero dictionary matches to score in the first
//! place. See `docs/ROADMAP.md` Phase 4.

use std::collections::HashMap;

use crate::viterbi::{self, NUM_STATES};

/// A trained BMES Hidden Markov Model for segmenting clusters the dictionary
/// has no match for at all.
///
/// Ships with no built-in parameters: like
/// [`with_frequencies`](crate::KhmerTokenizer::with_frequencies), training
/// needs a segmented corpus, and no bundleable, commercially-clean one has
/// been found (see `docs/ROADMAP.md` Phase 4). Build one with
/// [`HmmModel::from_counts`] from BMES tag counts gathered over a corpus
/// you're licensed to use, then attach it with
/// [`KhmerTokenizer::with_hmm`](crate::KhmerTokenizer::with_hmm).
#[derive(Clone)]
pub struct HmmModel {
    start: [f64; NUM_STATES],
    trans: [[f64; NUM_STATES]; NUM_STATES],
    emit: HashMap<String, [f64; NUM_STATES]>,
}

/// Add-one (Laplace) smoothed log-probabilities from raw counts.
fn smoothed_log_probs(counts: &[u64; NUM_STATES]) -> [f64; NUM_STATES] {
    let total: f64 = counts.iter().map(|&c| c as f64 + 1.0).sum();
    let mut out = [0.0; NUM_STATES];
    for (i, &c) in counts.iter().enumerate() {
        out[i] = ((c as f64 + 1.0) / total).ln();
    }
    out
}

impl HmmModel {
    /// Build a model from raw BMES tag counts: `start_counts[state]` is how
    /// often a Khmer run began tagged `state`; `trans_counts[i][j]` is how
    /// often a cluster tagged `i` was immediately followed by one tagged
    /// `j`; `emit_counts[cluster][state]` is how often `cluster` was tagged
    /// `state`. All three are add-one smoothed, so an unseen transition or
    /// start state is merely unlikely, never impossible.
    pub fn from_counts(
        start_counts: [u64; NUM_STATES],
        trans_counts: [[u64; NUM_STATES]; NUM_STATES],
        emit_counts: HashMap<String, [u64; NUM_STATES]>,
    ) -> Self {
        let start = smoothed_log_probs(&start_counts);
        let mut trans = [[0.0; NUM_STATES]; NUM_STATES];
        for (i, row) in trans_counts.iter().enumerate() {
            trans[i] = smoothed_log_probs(row);
        }
        let emit = emit_counts
            .into_iter()
            .map(|(cluster, counts)| (cluster, smoothed_log_probs(&counts)))
            .collect();
        Self { start, trans, emit }
    }

    /// Log-emission-probability of `cluster` under `state`. A cluster never
    /// seen during training gets a uniform, uninformative floor, so decoding
    /// falls back on transition structure alone rather than treating it as
    /// impossible.
    fn emit_log_prob(&self, cluster: &str, state: usize) -> f64 {
        match self.emit.get(cluster) {
            Some(probs) => probs[state],
            None => (1.0 / NUM_STATES as f64).ln(),
        }
    }

    /// Viterbi-decode the most likely BMES tag sequence for `clusters`.
    /// `clusters` must be non-empty. Emissions come from `emit_log_prob`; the
    /// lattice itself is the shared [`crate::viterbi`] decoder.
    fn viterbi_tags(&self, clusters: &[String]) -> Vec<usize> {
        viterbi::viterbi(clusters.len(), &self.start, &self.trans, |t| {
            std::array::from_fn(|s| self.emit_log_prob(&clusters[t], s))
        })
    }

    /// Segment a run of clusters that a dictionary strategy matched nothing
    /// in at all, placing boundaries from the Viterbi-decoded BMES tags.
    pub(crate) fn segment_oov(&self, clusters: &[String]) -> Vec<Vec<String>> {
        if clusters.is_empty() {
            return Vec::new();
        }
        let tags = self.viterbi_tags(clusters);
        viterbi::bmes_to_tokens(clusters, &tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viterbi::{BEGIN, END, SINGLE};

    /// Build a model where "a" strongly emits Begin, "b" strongly emits End,
    /// "c" strongly emits Single, Begin strongly transitions to End, and End
    /// strongly transitions to Single — decisively decoding ["a","b","c"]
    /// into the two words ["a","b"] and ["c"].
    fn two_word_model() -> HmmModel {
        let start = [50, 0, 0, 0]; // sequences overwhelmingly start Begin
        let mut trans = [[0u64; NUM_STATES]; NUM_STATES];
        trans[BEGIN][END] = 50;
        trans[END][SINGLE] = 50;

        let mut emit = HashMap::new();
        emit.insert("a".to_string(), [50, 0, 0, 0]);
        emit.insert("b".to_string(), [0, 0, 50, 0]);
        emit.insert("c".to_string(), [0, 0, 0, 50]);

        HmmModel::from_counts(start, trans, emit)
    }

    #[test]
    fn decodes_a_two_word_run_via_bmes_tags() {
        let model = two_word_model();
        let clusters = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(
            model.segment_oov(&clusters),
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string()]
            ]
        );
    }

    #[test]
    fn single_cluster_run_is_its_own_token() {
        let model = two_word_model();
        let clusters = vec!["c".to_string()];
        assert_eq!(model.segment_oov(&clusters), vec![vec!["c".to_string()]]);
    }

    #[test]
    fn empty_run_yields_no_tokens() {
        let model = two_word_model();
        assert!(model.segment_oov(&[]).is_empty());
    }

    #[test]
    fn unseen_cluster_falls_back_to_uniform_emission_without_panicking() {
        let model = two_word_model();
        let clusters = vec!["z".to_string()];
        // No assertion on the exact tag — just confirm decoding an unseen
        // cluster degrades gracefully instead of crashing.
        assert_eq!(model.segment_oov(&clusters).concat(), vec!["z".to_string()]);
    }
}
