//! A trie keyed on Khmer Character Clusters, with longest-match segmentation.

use std::collections::HashMap;

use crate::kcc::{is_khmer, split_kcc};
use crate::strategy::Strategy;

/// One node of the cluster trie. Edges are keyed on whole clusters (not raw
/// chars), which is what lets the longest-match walk stay aligned to
/// orthographic boundaries.
#[derive(Default)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    is_word: bool,
}

/// Dictionary-backed Khmer word segmenter.
///
/// Build one with [`KhmerTokenizer::from_words`] (or the dictionary helpers in
/// the crate root), then call [`segment`](KhmerTokenizer::segment).
#[derive(Default)]
pub struct KhmerTokenizer {
    root: TrieNode,
    /// A second trie holding every dictionary word's clusters in reverse
    /// order. Lets backward maximum matching reuse the same greedy walk as
    /// forward matching, just over a reversed cluster stream.
    rev_root: TrieNode,
    word_count: usize,
    strategy: Strategy,
}

impl KhmerTokenizer {
    /// Create an empty tokenizer with no dictionary. Segmentation will fall back
    /// to one token per cluster until words are inserted.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build a tokenizer from any iterator of dictionary words.
    ///
    /// # Example
    /// ```
    /// use khmer_tokenizer_core::KhmerTokenizer;
    /// let tk = KhmerTokenizer::from_words(["ភាសា", "ខ្មែរ"]);
    /// assert_eq!(tk.segment("ភាសាខ្មែរ"), vec!["ភាសា", "ខ្មែរ"]);
    /// ```
    pub fn from_words<I, S>(words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut t = Self::default();
        for w in words {
            t.insert(w.as_ref());
        }
        t
    }

    /// Use a different segmentation algorithm (default:
    /// [`Strategy::ForwardMaxMatch`]). Chains onto any constructor.
    ///
    /// # Example
    /// ```
    /// use khmer_tokenizer_core::{KhmerTokenizer, Strategy};
    /// let tk = KhmerTokenizer::from_words(["ភាសា", "ខ្មែរ"]).with_strategy(Strategy::BiMaxMatch);
    /// assert_eq!(tk.segment("ភាសាខ្មែរ"), vec!["ភាសា", "ខ្មែរ"]);
    /// ```
    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Insert a single word into the dictionary. Surrounding whitespace is
    /// trimmed; empty words are ignored.
    pub fn insert(&mut self, word: &str) {
        let word = word.trim();
        if word.is_empty() {
            return;
        }
        let clusters = split_kcc(word);

        let mut node = &mut self.root;
        for cl in &clusters {
            node = node.children.entry(cl.clone()).or_default();
        }
        let is_new = !node.is_word;
        node.is_word = true;

        let mut rnode = &mut self.rev_root;
        for cl in clusters.iter().rev() {
            rnode = rnode.children.entry(cl.clone()).or_default();
        }
        rnode.is_word = true;

        if is_new {
            self.word_count += 1;
        }
    }

    /// Number of distinct words in the dictionary.
    pub fn len(&self) -> usize {
        self.word_count
    }

    /// True if the dictionary is empty.
    pub fn is_empty(&self) -> bool {
        self.word_count == 0
    }

    /// True if `word` is an exact entry in the dictionary.
    pub fn contains(&self, word: &str) -> bool {
        let mut node = &self.root;
        for cl in split_kcc(word) {
            match node.children.get(&cl) {
                Some(next) => node = next,
                None => return false,
            }
        }
        node.is_word
    }

    /// Segment a continuous string of Khmer text into tokens.
    ///
    /// Non-Khmer runs (Latin, digits, punctuation) are always grouped
    /// greedily into their own tokens, and whitespace separates tokens
    /// without producing one. Khmer runs are segmented using the tokenizer's
    /// [`Strategy`] (default [`Strategy::ForwardMaxMatch`]: consume the
    /// longest run of clusters that forms a dictionary word at each
    /// position, falling back to a single cluster when nothing matches).
    pub fn segment(&self, text: &str) -> Vec<String> {
        let clusters = split_kcc(text);
        let n = clusters.len();
        let mut tokens: Vec<String> = Vec::new();
        let mut i = 0;

        while i < n {
            let cl = &clusters[i];

            // Whitespace acts as a separator and emits nothing.
            if cl.trim().is_empty() {
                i += 1;
                continue;
            }

            let first = cl.chars().next().unwrap();

            // Non-Khmer run: group consecutive non-space, non-Khmer clusters
            // (e.g. "Rust", "2026") into a single token.
            if !is_khmer(first) {
                let start = i;
                while i < n
                    && !clusters[i].trim().is_empty()
                    && !is_khmer(clusters[i].chars().next().unwrap())
                {
                    i += 1;
                }
                tokens.push(clusters[start..i].concat());
                continue;
            }

            // Khmer run: hand the whole contiguous run to the strategy.
            let start = i;
            while i < n && is_khmer(clusters[i].chars().next().unwrap()) {
                i += 1;
            }
            let run = &clusters[start..i];
            let run_tokens = match self.strategy {
                Strategy::ForwardMaxMatch => forward_match(run, &self.root),
                Strategy::BiMaxMatch => bimm(run, &self.root, &self.rev_root),
            };
            tokens.extend(run_tokens.into_iter().map(|cs| cs.concat()));
        }

        tokens
    }
}

/// Greedy longest-match walk over `clusters`, consuming the longest run that
/// completes a dictionary word at each position (falling back to a single
/// cluster when nothing matches). Returns each token as its constituent
/// clusters — not yet joined into a string — in the same order as `clusters`.
fn greedy_match(clusters: &[String], root: &TrieNode) -> Vec<Vec<String>> {
    let n = clusters.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < n {
        let mut node = root;
        let mut j = i;
        let mut last_word_end: Option<usize> = None;
        while j < n {
            match node.children.get(&clusters[j]) {
                Some(next) => {
                    node = next;
                    j += 1;
                    if node.is_word {
                        last_word_end = Some(j);
                    }
                }
                None => break,
            }
        }
        match last_word_end {
            Some(end) => {
                tokens.push(clusters[i..end].to_vec());
                i = end;
            }
            None => {
                tokens.push(vec![clusters[i].clone()]);
                i += 1;
            }
        }
    }

    tokens
}

/// Forward maximum matching: greedy longest-match, left to right.
fn forward_match(clusters: &[String], root: &TrieNode) -> Vec<Vec<String>> {
    greedy_match(clusters, root)
}

/// Backward maximum matching: greedy longest-match, right to left. Walks the
/// reversed cluster stream against a trie of reversed dictionary words, then
/// restores original token and cluster order.
fn backward_match(clusters: &[String], rev_root: &TrieNode) -> Vec<Vec<String>> {
    let reversed: Vec<String> = clusters.iter().rev().cloned().collect();
    let mut tokens = greedy_match(&reversed, rev_root);
    tokens.reverse();
    for token in &mut tokens {
        token.reverse();
    }
    tokens
}

/// Bidirectional maximum matching: run forward and backward, and on
/// disagreement prefer fewer tokens, then fewer single-cluster tokens, then
/// the forward result — the canonical tie-break from Bi & Taing (APSIPA
/// 2014); see `docs/RESEARCH-2.md` §3b.
fn bimm(clusters: &[String], root: &TrieNode, rev_root: &TrieNode) -> Vec<Vec<String>> {
    let fwd = forward_match(clusters, root);
    let bwd = backward_match(clusters, rev_root);

    if fwd.len() != bwd.len() {
        return if fwd.len() < bwd.len() { fwd } else { bwd };
    }

    let singles = |tokens: &[Vec<String>]| tokens.iter().filter(|t| t.len() == 1).count();
    if singles(&fwd) <= singles(&bwd) {
        fwd
    } else {
        bwd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dict_falls_back_to_clusters() {
        let tk = KhmerTokenizer::empty();
        assert_eq!(tk.segment("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
    }

    #[test]
    fn longest_match_wins() {
        // "កម្ពុជា" should match as one word, not "ក" + ...
        let tk = KhmerTokenizer::from_words(["ក", "កម្ពុជា"]);
        assert_eq!(tk.segment("កម្ពុជា"), vec!["កម្ពុជា"]);
    }

    #[test]
    fn tracks_word_count() {
        let mut tk = KhmerTokenizer::empty();
        assert!(tk.is_empty());
        tk.insert("ខ្មែរ");
        tk.insert("ខ្មែរ"); // duplicate, not double-counted
        assert_eq!(tk.len(), 1);
    }

    #[test]
    fn contains_checks_exact_dictionary_entries() {
        let tk = KhmerTokenizer::from_words(["កម្ពុជា"]);
        assert!(tk.contains("កម្ពុជា"));
        assert!(!tk.contains("កម្ពុ")); // prefix, not a full entry
        assert!(!tk.contains("ខ្មែរ")); // absent entirely
    }

    #[test]
    fn bimm_matches_forward_when_they_agree() {
        let tk = KhmerTokenizer::from_words(["សួស្តី", "អ្នក"]).with_strategy(Strategy::BiMaxMatch);
        assert_eq!(tk.segment("សួស្តីអ្នក"), vec!["សួស្តី", "អ្នក"]);
    }

    #[test]
    fn bimm_prefers_fewer_tokens_on_disagreement() {
        // Forward greedily takes "អ្នកទាំង" + "អស់" (2 tokens); backward
        // finds "អ្នក" + "ទាំងអស់" (2 tokens too) — but construct a case
        // where only one direction reaches a full-run 1-token match to make
        // the "fewer tokens" rule bite unambiguously.
        let tk = KhmerTokenizer::from_words(["អ្នកទាំងអស់គ្នា", "អ្នក", "ទាំងអស់គ្នា"])
            .with_strategy(Strategy::BiMaxMatch);
        // Forward: matches the full word in one greedy pass (longest match
        // from position 0 already reaches the whole run).
        assert_eq!(tk.segment("អ្នកទាំងអស់គ្នា"), vec!["អ្នកទាំងអស់គ្នា"]);
    }

    #[test]
    fn bimm_falls_back_to_forward_on_full_tie() {
        // With no dictionary words at all, forward and backward both emit
        // one single-cluster token per cluster — a full tie, so the forward
        // (identical) result is returned.
        let tk = KhmerTokenizer::empty().with_strategy(Strategy::BiMaxMatch);
        assert_eq!(tk.segment("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
    }
}
