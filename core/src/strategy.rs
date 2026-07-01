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
}
