//! Orthographic normalization: canonicalizes well-documented, real-world
//! Khmer Unicode encoding errors before segmentation, so messy input matches
//! the same dictionary entries as canonically-encoded text. On by default
//! (see [`KhmerTokenizer::without_normalization`](crate::KhmerTokenizer::without_normalization)).
//!
//! Per the Unicode Khmer syllable structure, a base is followed by, in
//! order: an optional Robat, the subscript stack (one or more
//! `COENG`+consonant pairs), an optional shifter, a dependent vowel, then
//! other signs. Two corruptions of that order are repaired here, both by
//! rightward rotation (never insertion or deletion):
//!
//! 1. **Mark typed before the subscript stack** — e.g. "សិទិ្ធ" for the
//!    correct "សិទ្ធិ" (confirmed present in khPOS's own gold corpus:
//!    `docs/BENCHMARKS.md` Phase 5). A mark immediately followed by a
//!    `COENG`+consonant pair is moved to follow the pair.
//! 2. **Mark stranded *between* `COENG` and its consonant** — the damage
//!    Unicode normalization itself inflicts. Khmer's canonical combining
//!    classes are erroneous and frozen (Unicode TN61 §"Normalization"):
//!    `COENG` has ccc=9 while `U+17DD` ATTHACAN has ccc=230, so NFC/NFD
//!    canonically reorders a typed `<ATTHACAN, COENG>` into
//!    `<COENG, ATTHACAN>` — splitting the `COENG` from the consonant it
//!    subscripts, which would make [`crate::split_kcc`] mis-cluster. Any
//!    NFC-processing pipeline (very common in web scraping) can produce
//!    this. The stranded mark is moved past the consonant, converging on
//!    the same canonical form rule 1 produces.
//!
//! Robat (`U+17CC`) is excluded from both rules: it's the one mark that's
//! *supposed* to precede the subscript stack. The zero-width joiners
//! `U+200C`/`U+200D` are also excluded — their meaning is tied to their
//! exact position (e.g. requesting an alternate shifter or subscript
//! rendering form), so reordering them would change rendering semantics.
//! Stripping stray joiners entirely was considered and rejected: deleting
//! characters changes byte length, which would break the eval harness's
//! span-based scoring (and any caller relying on byte-accurate boundaries)
//! without an offset map back to the original text — and khPOS's corpus
//! has zero such occurrences to measure against anyway (see
//! `docs/ROADMAP.md` Phase 5).

use crate::kcc::{is_khmer_base, is_khmer_combining, COENG, ROBAT};

/// A mark the reorder rules are allowed to move: a Khmer combining mark
/// that is not `COENG` itself, not Robat (legitimately precedes the
/// subscript stack), and not a zero-width joiner (position-sensitive
/// rendering semantics).
fn is_reorderable_mark(c: char) -> bool {
    is_khmer_combining(c) && c != COENG && c != ROBAT && !matches!(c as u32, 0x200C..=0x200D)
}

/// Canonicalize `text`'s Khmer encoding before segmentation. Idempotent:
/// normalizing already-canonical (or already-normalized) text is a no-op.
/// Byte-length-preserving: only reorders characters, never adds or removes
/// any, so span offsets over the original text stay valid.
pub fn normalize(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();

    // Fixed point: one rotation can expose a new match (e.g. a mark ahead
    // of two stacked subscripts cascades through both), so repeat until
    // nothing changes. Both rules only ever move marks rightward, so this
    // terminates; combining runs are short, so it converges fast.
    loop {
        let mut changed = false;
        let mut i = 0;
        while i + 2 < chars.len() {
            // Rule 1: mark typed before a COENG+consonant pair.
            if is_reorderable_mark(chars[i]) && chars[i + 1] == COENG {
                chars[i..i + 3].rotate_left(1);
                changed = true;
            }
            // Rule 2: mark stranded between COENG and the base it
            // subscripts (NFC canonical-reordering damage). Only fires
            // when a base actually follows — a trailing or doubly-stranded
            // mark has no provably-correct repair and is left alone.
            else if chars[i] == COENG
                && is_reorderable_mark(chars[i + 1])
                && is_khmer_base(chars[i + 2])
            {
                chars[i + 1..i + 3].rotate_left(1);
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

    #[test]
    fn repairs_nfc_stranded_sign_between_coeng_and_consonant() {
        // Khmer's frozen-erroneous combining classes (ccc(COENG)=9,
        // ccc(ATTHACAN)=230) mean NFC turns a typed <base, ATTHACAN,
        // COENG, base> into <base, COENG, ATTHACAN, base>, splitting the
        // COENG from its consonant (Unicode TN61). Rule 2 moves the
        // stranded sign past the consonant.
        let nfc_damaged = "ក\u{17D2}\u{17DD}ក";
        let canonical = "ក\u{17D2}ក\u{17DD}";
        assert_eq!(normalize(nfc_damaged), canonical);
    }

    #[test]
    fn both_corruption_orders_converge_on_the_same_canonical_form() {
        // The same underlying word corrupted two different ways — the raw
        // typed error (mark before COENG, rule 1) and its NFC-processed
        // form (mark after COENG, rule 2) — must normalize identically.
        let typed_error = "ក\u{17DD}\u{17D2}ក"; // what the typist produced
        let nfc_damaged = "ក\u{17D2}\u{17DD}ក"; // same text after NFC
        assert_eq!(normalize(typed_error), normalize(nfc_damaged));
        assert_eq!(normalize(typed_error), "ក\u{17D2}ក\u{17DD}");
    }

    #[test]
    fn leaves_zero_width_joiners_alone_in_both_positions() {
        // ZWNJ/ZWJ meaning is tied to exact position (they request
        // alternate rendering forms), so neither rule may move them.
        let zwnj_before_coeng = "ក\u{200C}\u{17D2}ក";
        assert_eq!(normalize(zwnj_before_coeng), zwnj_before_coeng);
        let zwj_after_coeng = "ក\u{17D2}\u{200D}ក";
        assert_eq!(normalize(zwj_after_coeng), zwj_after_coeng);
    }

    #[test]
    fn leaves_a_trailing_stranded_mark_alone() {
        // COENG + mark at end of text: no following base to pair the
        // COENG with, so there's no provably-correct repair — don't guess.
        let dangling = "ក\u{17D2}\u{17DD}";
        assert_eq!(normalize(dangling), dangling);
    }

    #[test]
    fn nfc_repair_is_idempotent_and_length_preserving() {
        let nfc_damaged = "ក\u{17D2}\u{17DD}ក";
        let once = normalize(nfc_damaged);
        assert_eq!(normalize(&once), once);
        assert_eq!(once.len(), nfc_damaged.len());
    }
}
