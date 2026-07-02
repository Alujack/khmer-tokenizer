//! Training wrapper for `khmer_tokenizer_core::TaggerModel` — the
//! CRF-class averaged-perceptron tier (see `docs/RESEARCH-3.md` §4).
//!
//! Same licensing posture as `hmm.rs`/`freq.rs`: models are trained from
//! khPOS's CC BY-NC-SA training split for local evaluation only, never
//! bundled, committed, or shipped.

use khmer_tokenizer_core::TaggerModel;

use crate::corpus::Example;

/// Train an averaged-perceptron BMES tagger from gold-segmented examples.
pub fn train_tagger(examples: &[Example], epochs: usize) -> TaggerModel {
    let sentences: Vec<Vec<String>> = examples.iter().map(|e| e.gold_tokens.clone()).collect();
    TaggerModel::train(&sentences, epochs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trains_a_usable_model_from_examples() {
        // "ខគ" is consistently one two-cluster word across contexts.
        let example = |gold: &[&str]| Example {
            input: gold.concat(),
            gold_tokens: gold.iter().map(|s| s.to_string()).collect(),
        };
        let examples = vec![
            example(&["ខគ", "ក"]),
            example(&["ក", "ខគ"]),
            example(&["ខគ"]),
        ];
        let model = train_tagger(&examples, 5);
        assert!(model.feature_count() > 0);

        // The trained model drives segmentation end-to-end.
        let tk = khmer_tokenizer_core::KhmerTokenizer::empty()
            .with_strategy(khmer_tokenizer_core::Strategy::Tagger)
            .with_tagger(model);
        assert_eq!(tk.segment("ខគក"), vec!["ខគ", "ក"]);
    }
}
