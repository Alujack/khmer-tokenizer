//! A trie keyed on Khmer Character Clusters, with longest-match segmentation.

use std::collections::HashMap;

use crate::kcc::{is_khmer, split_kcc};

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
    word_count: usize,
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

    /// Insert a single word into the dictionary. Surrounding whitespace is
    /// trimmed; empty words are ignored.
    pub fn insert(&mut self, word: &str) {
        let word = word.trim();
        if word.is_empty() {
            return;
        }
        let mut node = &mut self.root;
        for cl in split_kcc(word) {
            node = node.children.entry(cl).or_default();
        }
        if !node.is_word {
            node.is_word = true;
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
    /// The algorithm runs a maximum-matching (longest-match) walk over the
    /// cluster stream: at each position it consumes the longest run of clusters
    /// that forms a dictionary word. Unmatched Khmer falls back to a single
    /// cluster so output is always well-formed. Runs of non-Khmer text (Latin,
    /// digits, punctuation) are grouped into their own tokens, and whitespace
    /// separates tokens without producing one.
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

            // Khmer: walk the trie, remembering the last position that completed
            // a dictionary word.
            let mut node = &self.root;
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
                    tokens.push(clusters[i..end].concat());
                    i = end;
                }
                None => {
                    // No dictionary match: emit a single cluster and advance.
                    tokens.push(clusters[i].clone());
                    i += 1;
                }
            }
        }

        tokens
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
}
