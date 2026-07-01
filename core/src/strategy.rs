//! Selects which boundary-choosing algorithm [`segment`](crate::KhmerTokenizer::segment) uses.

/// Which segmentation algorithm [`KhmerTokenizer::segment`](crate::KhmerTokenizer::segment) uses.
///
/// Set at construction with [`KhmerTokenizer::with_strategy`](crate::KhmerTokenizer::with_strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    /// Greedy longest-match, left to right. Fast, deterministic — the
    /// long-standing default.
    #[default]
    ForwardMaxMatch,
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
    /// Falls back to [`Strategy::ForwardMaxMatch`] if no frequencies were
    /// set — there's nothing to score without them.
    UnigramDp,
}
