//! BMES tag-count training for `khmer_tokenizer_core::HmmModel`, feeding
//! Phase 4's OOV fallback (see `docs/ROADMAP.md`).
//!
//! Same licensing posture as `freq.rs`: counts are derived from khPOS's
//! CC BY-NC-SA training split for local evaluation only, never bundled.

use std::collections::HashMap;

use khmer_tokenizer_core::{is_khmer, split_kcc};

use crate::corpus::Example;

const NUM_STATES: usize = 4;
const BEGIN: usize = 0;
const MIDDLE: usize = 1;
const END: usize = 2;
const SINGLE: usize = 3;

/// Raw BMES tag counts gathered from a segmented corpus, ready to hand to
/// `khmer_tokenizer_core::HmmModel::from_counts`.
pub struct HmmCounts {
    pub start: [u64; NUM_STATES],
    pub trans: [[u64; NUM_STATES]; NUM_STATES],
    pub emit: HashMap<String, [u64; NUM_STATES]>,
}

fn is_khmer_word(word: &str) -> bool {
    word.chars().next().is_some_and(is_khmer)
}

/// BMES tags for one gold word's clusters: a single cluster is tagged
/// Single; two or more are tagged Begin, Middle*, End.
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

/// Split each example's gold words into maximal runs of consecutive Khmer
/// words, tagging every word's clusters BMES. A run breaks whenever a gold
/// word isn't Khmer (Latin, digits, punctuation) — the same boundary
/// `KhmerTokenizer::segment` uses when handing a contiguous Khmer run to the
/// HMM fallback.
fn extract_khmer_runs(examples: &[Example]) -> Vec<(Vec<String>, Vec<usize>)> {
    let mut runs = Vec::new();

    for example in examples {
        let mut clusters: Vec<String> = Vec::new();
        let mut tags: Vec<usize> = Vec::new();

        for word in &example.gold_tokens {
            if !is_khmer_word(word) {
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

/// Count BMES start/transition/emission statistics over `examples`.
pub fn train_hmm(examples: &[Example]) -> HmmCounts {
    let runs = extract_khmer_runs(examples);

    let mut start = [0u64; NUM_STATES];
    let mut trans = [[0u64; NUM_STATES]; NUM_STATES];
    let mut emit: HashMap<String, [u64; NUM_STATES]> = HashMap::new();

    for (clusters, tags) in &runs {
        if let Some(&first) = tags.first() {
            start[first] += 1;
        }
        for i in 1..tags.len() {
            trans[tags[i - 1]][tags[i]] += 1;
        }
        for (cluster, &tag) in clusters.iter().zip(tags) {
            emit.entry(cluster.clone()).or_insert([0u64; NUM_STATES])[tag] += 1;
        }
    }

    HmmCounts { start, trans, emit }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(gold: &[&str]) -> Example {
        Example {
            input: gold.concat(),
            gold_tokens: gold.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn tags_single_cluster_word_as_single() {
        assert_eq!(tag_word(&["a".to_string()]), vec![SINGLE]);
    }

    #[test]
    fn tags_multi_cluster_word_as_begin_middle_end() {
        let clusters: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();
        assert_eq!(tag_word(&clusters), vec![BEGIN, MIDDLE, MIDDLE, END]);
    }

    #[test]
    fn breaks_a_run_on_a_non_khmer_word() {
        // "ក" (1 cluster, a bare base consonant) then a Latin word (breaks
        // the run) then "ខ្មែរ" (2 clusters) — two separate training runs.
        let examples = vec![example(&["ក", "Rust", "ខ្មែរ"])];
        let runs = extract_khmer_runs(&examples);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].0, vec!["ក".to_string()]);
        assert_eq!(runs[0].1, vec![SINGLE]);
        assert_eq!(runs[1].0, vec!["ខ្មែ".to_string(), "រ".to_string()]);
        assert_eq!(runs[1].1, vec![BEGIN, END]);
    }

    #[test]
    fn counts_start_transition_and_emission_from_one_run() {
        // "ខ្មែរ" splits into clusters ["ខ្មែ", "រ"] -> tags [Begin, End].
        let examples = vec![example(&["ខ្មែរ"])];
        let counts = train_hmm(&examples);

        assert_eq!(counts.start[BEGIN], 1);
        assert_eq!(counts.start.iter().sum::<u64>(), 1);
        assert_eq!(counts.trans[BEGIN][END], 1);
        assert_eq!(counts.emit["ខ្មែ"][BEGIN], 1);
        assert_eq!(counts.emit["រ"][END], 1);
    }
}
