//! Selects which boundary-choosing algorithm [`segment`](crate::KhmerTokenizer::segment) uses.

/// Which segmentation algorithm [`KhmerTokenizer::segment`](crate::KhmerTokenizer::segment) uses.
///
/// Set at construction with [`KhmerTokenizer::with_strategy`](crate::KhmerTokenizer::with_strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    /// Greedy longest-match, left to right. Fast, deterministic — the
    /// original default (through v0.2.x).
    ForwardMaxMatch,
    /// Fewest-words dynamic programming (the *maximal matching* of the
    /// Thai/Khmer segmentation literature, as distinct from the greedy
    /// walks above): builds the same match DAG as [`Strategy::UnigramDp`]
    /// but scores paths with no frequency data at all — fewest tokens
    /// first, ties broken by the most characters covered by dictionary
    /// words, then by the longest word at the current position. Unlike
    /// greedy forward max-match it can backtrack, so it never commits to
    /// a long first word that forces the rest of the run to shatter
    /// (e.g. ខែកក្កដា: FMM takes the dictionary word ខែក and strands
    /// ក្កដា; this strategy finds ខែ + កក្កដា). Measurably more accurate
    /// than both `ForwardMaxMatch` and `BiMaxMatch` on khPOS with zero
    /// bundled data — see `docs/BENCHMARKS.md` — which is why it is the
    /// default since v0.3.0.
    #[default]
    MinWordsDp,
    /// Bidirectional maximum matching: runs forward and backward max-match
    /// and picks between them on disagreement — fewer tokens wins; ties
    /// broken by fewer single-cluster tokens; a remaining tie favors the
    /// forward result. The canonical tie-break from Bi & Taing (APSIPA
    /// 2014); see `docs/RESEARCH-2.md` §3b.
    BiMaxMatch,
    /// Unigram max-probability path (jieba-style): builds a DAG of every
    /// dictionary match over the cluster run, then dynamic-programs
    /// right-to-left for the path with the highest cumulative
    /// log-probability, using word frequencies set with
    /// [`KhmerTokenizer::with_frequencies`](crate::KhmerTokenizer::with_frequencies).
    /// Falls back to [`Strategy::MinWordsDp`] if no frequencies were
    /// set — there's nothing to score without them.
    UnigramDp,
    /// Full statistical BMES tagging: every Khmer run is segmented by the
    /// [`TaggerModel`](crate::TaggerModel) attached with
    /// [`KhmerTokenizer::with_tagger`](crate::KhmerTokenizer::with_tagger)'s
    /// Viterbi decode, ignoring the dictionary entirely — the CRF-class
    /// tier (see `docs/RESEARCH-3.md` §4). Falls back to
    /// [`Strategy::MinWordsDp`] if no tagger model was set.
    Tagger,
}
