//! Word-frequency counting for [`crate::corpus::Split::Train`], feeding
//! [`khmer_tokenizer_core::Strategy::UnigramDp`].
//!
//! khPOS is CC BY-NC-SA 4.0, so frequencies counted from it are only for
//! local, non-bundled use (evaluated against `OpenTest`, never committed or
//! shipped) — same constraint noted for Phase 4's HMM in `docs/ROADMAP.md`.

use std::collections::HashMap;

use crate::corpus::Example;

/// Count how many times each gold word occurs across `examples`.
pub fn count_frequencies(examples: &[Example]) -> HashMap<String, u64> {
    let mut counts = HashMap::new();
    for example in examples {
        for word in &example.gold_tokens {
            *counts.entry(word.clone()).or_insert(0) += 1;
        }
    }
    counts
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
    fn counts_repeated_words_across_examples() {
        let examples = vec![
            example(&["សួស្តី", "អ្នក"]),
            example(&["សួស្តី", "ខ្មែរ"]),
        ];
        let counts = count_frequencies(&examples);
        assert_eq!(counts.get("សួស្តី"), Some(&2));
        assert_eq!(counts.get("អ្នក"), Some(&1));
        assert_eq!(counts.get("ខ្មែរ"), Some(&1));
        assert_eq!(counts.get("ភាសា"), None);
    }
}
