//! Orthographic normalization: canonicalizes a well-documented, real-world
//! Khmer Unicode encoding error before segmentation, so messy input matches
//! the same dictionary entries as canonically-encoded text. On by default
//! (see [`KhmerTokenizer::without_normalization`](crate::KhmerTokenizer::without_normalization)).
//!
//! Per the Unicode Khmer syllable structure, a base is followed by, in
//! order: an optional Robat, the subscript stack (one or more
//! `COENG`+consonant pairs), an optional shifter, a dependent vowel, then
//! other signs. In practice a shifter, vowel, or other sign sometimes gets
//! typed or encoded *before* the subscript stack instead of after it — e.g.
//! "សិទិ្ធ" for the correct "សិទ្ធិ" (confirmed present in khPOS's own gold
//! corpus: `docs/BENCHMARKS.md` Phase 5). This pass detects a mark
//! immediately followed by a `COENG`+consonant pair and moves the mark to
//! follow the pair instead. Robat is excluded from the rule since it's the
//! one mark that's *supposed* to precede the subscript stack.
//!
//! Stripping stray zero-width joiners (`U+200C`/`U+200D`) was considered for
//! this phase too, but deleting characters changes byte length, which would
//! break the eval harness's span-based scoring (and any caller relying on
//! byte-accurate boundaries) without also building an offset map back to the
//! original text — and khPOS's corpus has zero such occurrences to measure
//! against anyway (see `docs/ROADMAP.md` Phase 5). Deferred.

use crate::kcc::{is_khmer_combining, COENG, ROBAT};

/// Canonicalize `text`'s Khmer encoding before segmentation. Idempotent:
/// normalizing already-canonical (or already-normalized) text is a no-op.
/// Byte-length-preserving: only reorders characters, never adds or removes
/// any, so span offsets over the original text stay valid.
pub fn normalize(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();

    // Fixed point: one rotation can expose a new mark-before-COENG pattern
    // (e.g. a mark ahead of two stacked subscripts), so repeat until nothing
    // changes. Combining runs are always short, so this converges fast.
    loop {
        let mut changed = false;
        let mut i = 0;
        while i + 2 < chars.len() {
            if chars[i] != COENG
                && chars[i] != ROBAT
                && is_khmer_combining(chars[i])
                && chars[i + 1] == COENG
            {
                chars[i..i + 3].rotate_left(1);
                changed = true;
            }
            i += 1;
        }
        if !changed {
            break;
        }
    }

    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorders_a_vowel_typed_before_a_subscript() {
        // "សិទិ្ធ" (a common real-world typo for "សិទ្ធិ", "rights"):
        // the vowel ិ attached to ទ was typed before ្ធ instead of after.
        assert_eq!(normalize("សិទិ្ធ"), "សិទ្ធិ");
    }

    #[test]
    fn reorders_a_shifter_typed_before_a_subscript() {
        // Base ប + shifter ៊ + COENG-ល + vowel ិ, in that (wrong) order,
        // should become base + COENG-ល + shifter + vowel.
        let malformed = "ប\u{17CA}\u{17D2}\u{179B}\u{17B7}";
        let canonical = "ប\u{17D2}\u{179B}\u{17CA}\u{17B7}";
        assert_eq!(normalize(malformed), canonical);
    }

    #[test]
    fn leaves_a_legitimate_robat_before_a_subscript_alone() {
        // Robat is the one mark that's supposed to precede the subscript
        // stack — this must not be touched.
        let already_canonical = "ក\u{17CC}\u{17D2}\u{1780}";
        assert_eq!(normalize(already_canonical), already_canonical);
    }

    #[test]
    fn is_a_no_op_on_already_canonical_text() {
        assert_eq!(normalize("កម្ពុជា"), "កម្ពុជា");
        assert_eq!(normalize("សិទ្ធិ"), "សិទ្ធិ");
    }

    #[test]
    fn is_idempotent() {
        let malformed = "សិទិ្ធ";
        let once = normalize(malformed);
        let twice = normalize(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn preserves_byte_length() {
        let malformed = "សិទិ្ធ";
        assert_eq!(normalize(malformed).len(), malformed.len());
    }

    #[test]
    fn cascades_through_a_mark_before_a_doubly_stacked_subscript() {
        // A vowel typed before two stacked subscripts should end up after
        // both, not just the first.
        let malformed = "ស\u{17B7}\u{17D2}\u{178F}\u{17D2}\u{179A}";
        let canonical = "ស\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B7}";
        assert_eq!(normalize(malformed), canonical);
    }
}
