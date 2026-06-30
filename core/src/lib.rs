//! # khmer-tokenizer-core
//!
//! A fast, dependency-free Khmer word segmenter.
//!
//! Written Khmer does not put spaces between words, so segmentation is the first
//! step of nearly every Khmer NLP pipeline. This crate does it in two passes:
//!
//! 1. **Cluster pass** — [`split_kcc`] groups the text into Khmer Character
//!    Clusters (a base character plus its subscripts and vowels). This keeps the
//!    segmenter from ever splitting *inside* an orthographic syllable.
//! 2. **Longest-match pass** — [`KhmerTokenizer`] walks a cluster-keyed trie and
//!    consumes the longest dictionary word at each position, falling back to a
//!    single cluster when nothing matches.
//!
//! The engine is `std`-only (no external dependencies) and deterministic.
//!
//! ## Quick start
//! ```
//! use khmer_tokenizer_core::KhmerTokenizer;
//!
//! let tk = KhmerTokenizer::with_default_dict();
//! let tokens = tk.segment("សួស្តីអ្នកទាំងអស់គ្នា");
//! assert_eq!(tokens, vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]);
//! ```

mod kcc;
mod trie;

pub use kcc::{is_khmer, split_kcc};
pub use trie::KhmerTokenizer;

/// The embedded seed dictionary: one word per line; blank lines and lines
/// starting with `#` are ignored. Replace or extend it for production use — see
/// the dictionary notes in the project README.
pub const DEFAULT_DICT: &str = include_str!("dict.txt");

impl KhmerTokenizer {
    /// Build a tokenizer pre-loaded with the embedded seed dictionary
    /// ([`DEFAULT_DICT`]).
    pub fn with_default_dict() -> Self {
        Self::from_dict_str(DEFAULT_DICT)
    }

    /// Build a tokenizer from a newline-separated word list. Blank lines and
    /// lines beginning with `#` are skipped, so dictionary files may carry
    /// comments.
    pub fn from_dict_str(dict: &str) -> Self {
        let words = dict
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'));
        Self::from_words(words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tk() -> KhmerTokenizer {
        KhmerTokenizer::from_words([
            "សួស្តី",
            "អ្នក",
            "ទាំងអស់គ្នា",
            "កម្ពុជា",
            "ភាសា",
            "ខ្មែរ",
            "ខ្ញុំ",
            "ស្រឡាញ់",
        ])
    }

    #[test]
    fn segments_known_words() {
        assert_eq!(
            tk().segment("សួស្តីអ្នកទាំងអស់គ្នា"),
            vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
        );
        assert_eq!(
            tk().segment("ខ្ញុំស្រឡាញ់កម្ពុជា"),
            vec!["ខ្ញុំ", "ស្រឡាញ់", "កម្ពុជា"]
        );
        assert_eq!(tk().segment("ភាសាខ្មែរ"), vec!["ភាសា", "ខ្មែរ"]);
    }

    #[test]
    fn handles_mixed_scripts() {
        assert_eq!(
            tk().segment("ខ្ញុំស្រឡាញ់ Rust 2026 កម្ពុជា"),
            vec!["ខ្ញុំ", "ស្រឡាញ់", "Rust", "2026", "កម្ពុជា"]
        );
    }

    #[test]
    fn oov_falls_back_to_clusters() {
        // ឆ្នាំ and ថ្មី are absent from this dictionary -> single clusters.
        assert_eq!(tk().segment("ឆ្នាំថ្មី"), vec!["ឆ្នាំ", "ថ្មី"]);
    }

    #[test]
    fn default_dict_loads_and_segments() {
        let tk = KhmerTokenizer::with_default_dict();
        assert!(!tk.is_empty());
        assert_eq!(
            tk.segment("សួស្តីអ្នកទាំងអស់គ្នា"),
            vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
        );
    }

    #[test]
    fn dict_str_skips_comments_and_blanks() {
        let tk = KhmerTokenizer::from_dict_str("# comment\n\nខ្មែរ\n  ភាសា  \n");
        assert_eq!(tk.len(), 2);
    }
}
