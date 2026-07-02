//! CI regression guard (`docs/ROADMAP.md` Phase 6): fails if the default
//! tokenizer's accuracy on a small, hand-authored, license-safe sample
//! drops below a floor. Runs as part of plain `cargo test`, so it needs no
//! network access — unlike `cargo xtask eval`, which depends on cloning the
//! (gitignored, CC BY-NC-SA) khPOS corpus.
//!
//! `fixtures/regression.word` is original content written for this project
//! (not derived from khPOS or any other corpus), in the same one-sentence-
//! per-line, space-separated-gold-words format `corpus::parse_khpos`
//! already parses. One line (`សិទិ្ធមនុស្ស`) is a real-world malformed
//! Khmer spelling on purpose, guarding Phase 5's normalization pass against
//! silently regressing.

use khmer_tokenizer_core::KhmerTokenizer;
use khmer_tokenizer_eval::{evaluate, parse_khpos};

/// Measured F1 on `fixtures/regression.word` as of Phase 6 is 1.0 (every
/// sample sentence uses vocabulary already confirmed present in
/// `core/src/dict.txt`). The floor is set well below that so legitimate
/// future changes (a different default strategy, a regenerated dictionary
/// with different coverage) have room to move without tripping this guard
/// on noise — while still catching a real break (e.g. a corrupted
/// `dict.txt`, a broken trie walk, or a regressed normalizer).
const F1_FLOOR: f64 = 0.9;

#[test]
fn default_tokenizer_meets_the_regression_floor() {
    let examples = parse_khpos(include_str!("fixtures/regression.word"));
    assert!(!examples.is_empty(), "regression fixture must not be empty");

    let tokenizer = KhmerTokenizer::with_default_dict();
    let metrics = evaluate(&examples, &tokenizer);

    assert!(
        metrics.f1 >= F1_FLOOR,
        "F1 {:.4} on the regression fixture dropped below the floor {F1_FLOOR:.4} \
         (precision {:.4}, recall {:.4}, word_accuracy {:.4}) — see \
         eval/tests/fixtures/regression.word",
        metrics.f1,
        metrics.precision,
        metrics.recall,
        metrics.word_accuracy,
    );
}
