//! A trie keyed on Khmer Character Clusters, with longest-match segmentation.

use std::collections::HashMap;

use crate::hmm::HmmModel;
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
    /// Word frequencies for [`Strategy::UnigramDp`], set with
    /// [`with_frequencies`](KhmerTokenizer::with_frequencies). Empty by
    /// default — no frequency table ships with this crate (see the
    /// dictionary notes in the project README for why).
    freq_counts: HashMap<String, u64>,
    freq_total: u64,
    /// Optional HMM fallback for clusters no strategy matched at all (Phase
    /// 4), set with [`with_hmm`](KhmerTokenizer::with_hmm). `None` by
    /// default — unmatched clusters are emitted one per token, same as ever.
    hmm: Option<HmmModel>,
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

    /// Supply word frequencies for [`Strategy::UnigramDp`]. Chains onto any
    /// constructor. Without this, `UnigramDp` falls back to
    /// [`Strategy::ForwardMaxMatch`] — there's nothing to score.
    ///
    /// This crate ships no default frequency table: a bundleable,
    /// commercially-clean corpus-frequency source hasn't been found yet (see
    /// `docs/ROADMAP.md` Phase 3). Callers must supply their own counts,
    /// e.g. counted from a corpus they're licensed to use.
    pub fn with_frequencies<I>(mut self, counts: I) -> Self
    where
        I: IntoIterator<Item = (String, u64)>,
    {
        let counts: HashMap<String, u64> = counts.into_iter().collect();
        self.freq_total = counts.values().sum();
        self.freq_counts = counts;
        self
    }

    /// Attach an HMM fallback for clusters that no strategy could match in
    /// the dictionary at all — the case that still degrades to one token per
    /// cluster otherwise. Chains onto any constructor and composes with any
    /// [`Strategy`]: matched dictionary tokens (including genuine
    /// single-cluster words) are left untouched; only maximal runs of
    /// truly-unmatched clusters get re-segmented by the model's
    /// Viterbi-decoded BMES tags.
    ///
    /// Ships with no default model — like
    /// [`with_frequencies`](KhmerTokenizer::with_frequencies), training
    /// needs a segmented corpus and no bundleable, commercially-clean one
    /// has been found (see `docs/ROADMAP.md` Phase 4).
    pub fn with_hmm(mut self, model: HmmModel) -> Self {
        self.hmm = Some(model);
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
                Strategy::UnigramDp if self.freq_total > 0 => {
                    unigram_dp(run, &self.root, &self.freq_counts, self.freq_total)
                }
                Strategy::UnigramDp => forward_match(run, &self.root),
            };
            let run_tokens = match &self.hmm {
                Some(model) => apply_hmm_fallback(run_tokens, &self.root, model),
                None => run_tokens,
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

/// Unigram max-probability path (jieba-style). Builds a DAG where `dag[k]`
/// holds every end position reachable from `k` by a dictionary word starting
/// there (or, if none match, a single-cluster fallback edge), then dynamic
/// programs right-to-left for the path maximizing cumulative
/// log-probability — computed in log-space to avoid floating-point
/// underflow from multiplying many small fractions. OOV words (absent from
/// `freq_counts`) get a floor count of 1 so they're penalized, not
/// impossible. See `docs/RESEARCH-2.md` §3a.
fn unigram_dp(
    clusters: &[String],
    root: &TrieNode,
    freq_counts: &HashMap<String, u64>,
    freq_total: u64,
) -> Vec<Vec<String>> {
    let n = clusters.len();
    if n == 0 {
        return Vec::new();
    }

    let mut dag: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (k, edges) in dag.iter_mut().enumerate() {
        let mut node = root;
        let mut j = k;
        while j < n {
            match node.children.get(&clusters[j]) {
                Some(next) => {
                    node = next;
                    j += 1;
                    if node.is_word {
                        edges.push(j);
                    }
                }
                None => break,
            }
        }
        if edges.is_empty() {
            edges.push(k + 1);
        }
    }

    let log_prob = |word: &str| -> f64 {
        let count = freq_counts.get(word).copied().unwrap_or(0).max(1) as f64;
        (count / freq_total as f64).ln()
    };

    // Right-to-left DP for the highest cumulative log-probability path.
    let mut best_score = vec![f64::NEG_INFINITY; n + 1];
    let mut best_end = vec![0usize; n];
    best_score[n] = 0.0;
    for k in (0..n).rev() {
        for &j in &dag[k] {
            let word = clusters[k..j].concat();
            let score = log_prob(&word) + best_score[j];
            if score > best_score[k] {
                best_score[k] = score;
                best_end[k] = j;
            }
        }
    }

    // Reconstruct left to right.
    let mut tokens = Vec::new();
    let mut k = 0;
    while k < n {
        let j = best_end[k];
        tokens.push(clusters[k..j].to_vec());
        k = j;
    }
    tokens
}

/// True if the exact cluster sequence is a dictionary entry.
fn is_dict_word(root: &TrieNode, clusters: &[String]) -> bool {
    let mut node = root;
    for cl in clusters {
        match node.children.get(cl) {
            Some(next) => node = next,
            None => return false,
        }
    }
    node.is_word
}

/// Replace maximal runs of dictionary-fallback single clusters — spots
/// where a strategy found no dictionary match at all, not genuine
/// single-cluster words — with the HMM's Viterbi-decoded guess. This is
/// strategy-agnostic: it only looks at whether each already-produced token
/// is a real dictionary hit, so it composes with FMM, BiMM, and UnigramDp
/// output alike.
fn apply_hmm_fallback(
    tokens: Vec<Vec<String>>,
    root: &TrieNode,
    hmm: &HmmModel,
) -> Vec<Vec<String>> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut buffer: Vec<String> = Vec::new();

    for token in tokens {
        if token.len() == 1 && !is_dict_word(root, &token) {
            buffer.push(token.into_iter().next().unwrap());
            continue;
        }
        if !buffer.is_empty() {
            out.extend(hmm.segment_oov(&buffer));
            buffer.clear();
        }
        out.push(token);
    }
    if !buffer.is_empty() {
        out.extend(hmm.segment_oov(&buffer));
    }
    out
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

    #[test]
    fn unigramdp_falls_back_to_forward_without_frequencies() {
        let tk = KhmerTokenizer::from_words(["សួស្តី", "អ្នក"]).with_strategy(Strategy::UnigramDp);
        assert_eq!(tk.segment("សួស្តីអ្នក"), vec!["សួស្តី", "អ្នក"]);
    }

    #[test]
    fn unigramdp_prefers_the_higher_probability_path_over_greedy_match() {
        // "ក", "ខ", "គ" are three separate single-cluster base consonants
        // (no vowels/subscripts), so "កខគ" splits into exactly 3 clusters —
        // a clean synthetic ambiguity. Both greedy forward-max-match and
        // BiMM (which ties and defaults to forward) always resolve this to
        // ["កខ", "គ"], because plain longest-match can never backtrack to
        // consider starting fresh at cluster 1. Only a DAG-based scorer can
        // even represent the alternative path ["ក", "ខគ"].
        let tk = KhmerTokenizer::from_words(["ក", "កខ", "ខគ", "គ"]);
        assert_eq!(tk.segment("កខគ"), vec!["កខ", "គ"]); // sanity: FMM's fixed answer

        // Weight "ក" and "ខគ" heavily over "កខ" and "គ" — enough that the
        // alternative path's cumulative probability wins decisively.
        let freqs = [
            ("ក".to_string(), 100),
            ("ខគ".to_string(), 100),
            ("កខ".to_string(), 1),
            ("គ".to_string(), 1),
        ];
        let tk = tk.with_strategy(Strategy::UnigramDp).with_frequencies(freqs);
        assert_eq!(tk.segment("កខគ"), vec!["ក", "ខគ"]);
    }

    #[test]
    fn hmm_fallback_resegments_only_the_truly_oov_run() {
        use crate::hmm::HmmModel;
        use std::collections::HashMap;

        // "ក" is a real dictionary word; "ខ", "គ", "ង" are not, so with no
        // HMM attached forward-max-match would emit ["ក", "ខ", "គ", "ង"] —
        // one cluster per token for the whole unmatched tail.
        let tk = KhmerTokenizer::from_words(["ក"]);
        assert_eq!(tk.segment("កខគង"), vec!["ក", "ខ", "គ", "ង"]);

        // Craft an HMM that decisively resegments an unmatched ["ខ","គ","ង"]
        // run into ["ខគ", "ង"] (Begin, End, Single) instead of 3 loose
        // single-cluster tokens.
        let start = [50, 0, 0, 0]; // Begin
        let mut trans = [[0u64; 4]; 4];
        trans[0][2] = 50; // Begin -> End
        trans[2][3] = 50; // End -> Single
        let mut emit = HashMap::new();
        emit.insert("ខ".to_string(), [50, 0, 0, 0]); // Begin
        emit.insert("គ".to_string(), [0, 0, 50, 0]); // End
        emit.insert("ង".to_string(), [0, 0, 0, 50]); // Single
        let model = HmmModel::from_counts(start, trans, emit);

        let tk = tk.with_hmm(model);
        // The real dictionary hit "ក" is untouched; only the unmatched tail
        // is re-segmented, and by the HMM's tags rather than one-per-cluster.
        assert_eq!(tk.segment("កខគង"), vec!["ក", "ខគ", "ង"]);
    }
}
