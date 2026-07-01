//! Token-span Precision / Recall / F1, following the SIGHAN convention: a
//! predicted word counts as correct only if both its start and end offsets
//! match a gold word exactly.

use std::collections::HashSet;

use khmer_tokenizer_core::KhmerTokenizer;

use crate::corpus::Example;

pub struct Metrics {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    /// Recall restricted to gold words present in the tokenizer's
    /// dictionary. `0.0` if the sample has no in-vocabulary gold words.
    pub r_iv: f64,
    /// Recall restricted to gold words absent from the tokenizer's
    /// dictionary. `0.0` if the sample has no out-of-vocabulary gold words.
    pub r_oov: f64,
    /// Fraction of sentences segmented with a fully correct token sequence.
    pub word_accuracy: f64,
    pub sentences: usize,
}

/// Cumulative-length `(start, end)` spans for a token sequence, in the order
/// the tokens appear. Two token sequences over the same underlying text are
/// comparable by this span set regardless of repeated words.
fn spans(tokens: &[String]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut pos = 0;
    for t in tokens {
        let end = pos + t.len();
        out.push((pos, end));
        pos = end;
    }
    out
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub fn evaluate(examples: &[Example], tokenizer: &KhmerTokenizer) -> Metrics {
    let mut total_pred = 0usize;
    let mut total_gold = 0usize;
    let mut total_correct = 0usize;
    let mut gold_iv = 0usize;
    let mut correct_iv = 0usize;
    let mut gold_oov = 0usize;
    let mut correct_oov = 0usize;
    let mut exact_sentences = 0usize;

    for example in examples {
        let predicted = tokenizer.segment(&example.input);
        let pred_spans: HashSet<(usize, usize)> = spans(&predicted).into_iter().collect();
        let gold_spans = spans(&example.gold_tokens);

        total_pred += predicted.len();
        total_gold += example.gold_tokens.len();

        let mut sentence_exact = predicted.len() == example.gold_tokens.len();

        for (word, span) in example.gold_tokens.iter().zip(&gold_spans) {
            let is_correct = pred_spans.contains(span);
            if is_correct {
                total_correct += 1;
            } else {
                sentence_exact = false;
            }
            if tokenizer.contains(word) {
                gold_iv += 1;
                correct_iv += is_correct as usize;
            } else {
                gold_oov += 1;
                correct_oov += is_correct as usize;
            }
        }

        exact_sentences += sentence_exact as usize;
    }

    let precision = ratio(total_correct, total_pred);
    let recall = ratio(total_correct, total_gold);
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    Metrics {
        precision,
        recall,
        f1,
        r_iv: ratio(correct_iv, gold_iv),
        r_oov: ratio(correct_oov, gold_oov),
        word_accuracy: ratio(exact_sentences, examples.len()),
        sentences: examples.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(input: &str, gold: &[&str]) -> Example {
        Example {
            input: input.to_string(),
            gold_tokens: gold.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn perfect_match_scores_one() {
        let tk = KhmerTokenizer::from_words(["សួស្តី", "អ្នក"]);
        let examples = vec![example("សួស្តីអ្នក", &["សួស្តី", "អ្នក"])];
        let m = evaluate(&examples, &tk);
        assert_eq!(m.precision, 1.0);
        assert_eq!(m.recall, 1.0);
        assert_eq!(m.f1, 1.0);
        assert_eq!(m.r_iv, 1.0);
        assert_eq!(m.r_oov, 0.0); // no OOV gold words in this sample
        assert_eq!(m.word_accuracy, 1.0);
    }

    #[test]
    fn oov_word_fragments_and_hurts_precision_and_recall() {
        // "ខ្មែរ" is missing from the dictionary and splits into 2 KCC
        // clusters, so it falls back to 2 single-cluster tokens instead of
        // the 1 gold token — an over-segmentation error.
        let tk = KhmerTokenizer::from_words(["សួស្តី"]);
        let examples = vec![example("សួស្តីខ្មែរ", &["សួស្តី", "ខ្មែរ"])];
        let m = evaluate(&examples, &tk);
        assert_eq!(m.precision, 1.0 / 3.0);
        assert_eq!(m.recall, 1.0 / 2.0);
        assert_eq!(m.r_iv, 1.0); // "សួស្តី" is in-vocabulary and matched
        assert_eq!(m.r_oov, 0.0); // "ខ្មែរ" is OOV and was not matched whole
        assert_eq!(m.word_accuracy, 0.0);
    }

    #[test]
    fn wrong_boundary_penalizes_despite_matching_token_count() {
        // Dictionary has a wrong merged entry, so the predicted split has
        // the same token count as gold but the boundary lands one cluster
        // off — both tokens are wrong even though counts match.
        let tk = KhmerTokenizer::from_words(["អ្នកទាំង", "អស់"]);
        let examples = vec![example("អ្នកទាំងអស់", &["អ្នក", "ទាំងអស់"])];
        let m = evaluate(&examples, &tk);
        assert_eq!(m.precision, 0.0);
        assert_eq!(m.recall, 0.0);
        assert_eq!(m.word_accuracy, 0.0);
    }

    #[test]
    fn word_accuracy_averages_across_sentences() {
        let tk = KhmerTokenizer::from_words(["សួស្តី", "អ្នក"]);
        let examples = vec![
            example("សួស្តីអ្នក", &["សួស្តី", "អ្នក"]), // exact
            example("សួស្តីខ្មែរ", &["សួស្តី", "ខ្មែរ"]),   // not exact (ខ្មែរ is OOV)
        ];
        let m = evaluate(&examples, &tk);
        assert_eq!(m.sentences, 2);
        assert_eq!(m.word_accuracy, 0.5);
    }
}
