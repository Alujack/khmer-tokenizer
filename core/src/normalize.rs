//! Orthographic normalization: canonicalizes well-documented, real-world
//! Khmer Unicode encoding errors before segmentation, so messy input matches
//! the same dictionary entries as canonically-encoded text. On by default
//! (see [`KhmerTokenizer::without_normalization`](crate::KhmerTokenizer::without_normalization)).
//!
//! Per the Unicode Khmer syllable structure, a base is followed by, in
//! order: an optional Robat, the subscript stack (one or more
//! `COENG`+consonant pairs), an optional shifter, a dependent vowel, then
//! other signs. Four corruptions of that order are repaired here, all by
//! character reordering (never insertion or deletion):
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
//! 3. **Subscript RO typed before another subscript** — Khmer orthography
//!    puts `COENG`+RO last in a subscript stack, but fonts render both
//!    orders identically, so real text is full of the swap: `ស្រ្តី` for
//!    `ស្ត្រី`, `រដ្ឋមន្រ្តី` for `រដ្ឋមន្ត្រី` (338 and 136 occurrences
//!    respectively in a 4,000-article web-news sample). A
//!    `COENG RO COENG C` sequence is rotated to `COENG C COENG RO` — the
//!    same repair SIL's `khnormal` performs.
//! 4. **Sign typed before a vowel, or vowel before a register shifter** —
//!    within one cluster the canonical mark order is shifter, then vowel,
//!    then signs, but e.g. `ាំ` is frequently typed `ំា` (NIKAHIT first)
//!    because both render identically. Adjacent mark pairs that violate
//!    the class order (shifter < vowel < sign) are swapped back. Robat and
//!    the zero-width joiners are never touched, and marks of the same
//!    class are never reordered relative to each other.
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
use crate::trie::KhmerTokenizer;
use std::sync::OnceLock;

static DEFAULT_TOKENIZER: OnceLock<KhmerTokenizer> = OnceLock::new();

fn get_tokenizer() -> &'static KhmerTokenizer {
    DEFAULT_TOKENIZER.get_or_init(KhmerTokenizer::with_default_dict)
}

/// A mark the reorder rules are allowed to move: a Khmer combining mark
/// that is not `COENG` itself, not Robat (legitimately precedes the
/// subscript stack), and not a zero-width joiner (position-sensitive
/// rendering semantics).
fn is_reorderable_mark(c: char) -> bool {
    is_khmer_combining(c) && c != COENG && c != ROBAT && !matches!(c as u32, 0x200C..=0x200D)
}

/// `U+179A` KHMER LETTER RO — its subscript form is written last in a
/// subscript stack (rule 3).
const RO: char = '\u{179A}';

/// Canonical within-cluster ordering class for rule 4: register shifters
/// come before dependent vowels, which come before the final signs.
/// `None` for anything rule 4 must not reorder (bases, `COENG`, Robat, the
/// deprecated invisible vowels `U+17B4`/`U+17B5`, joiners, non-Khmer).
fn mark_class(c: char) -> Option<u8> {
    match c as u32 {
        0x17C9 | 0x17CA => Some(0),          // register shifters ៉ ៊
        0x17B6..=0x17C5 => Some(1),          // dependent vowels
        0x17C6..=0x17C8 => Some(2),          // ំ ះ ៈ
        0x17CB | 0x17CD..=0x17D1 => Some(2), // ់ ៍ ៎ ៏ ័ ៑ (Robat 17CC excluded)
        0x17D3 | 0x17DD => Some(2),          // ៓ ៝
        _ => None,
    }
}

/// Canonicalize `text`'s Khmer encoding before segmentation. Idempotent:
/// normalizing already-canonical (or already-normalized) text is a no-op.
/// Byte-length-preserving: only reorders characters, never adds or removes
/// any, so span offsets over the original text stay valid.
pub fn normalize(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();

    // Fixed point: one rotation can expose a new match (e.g. a mark ahead
    // of two stacked subscripts cascades through both), so repeat until
    // nothing changes. Every rule only ever moves characters toward their
    // canonical position (marks rightward, subscript RO rightward, mark
    // classes into sorted order), so this terminates; combining runs are
    // short, so it converges fast.
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
            // Rule 3: subscript RO typed before another subscript
            // (COENG RO COENG C -> COENG C COENG RO). Khmer orthography
            // writes COENG+RO last in a stack; both orders render the
            // same, so the swap is a pure encoding repair.
            else if chars[i] == COENG
                && chars[i + 1] == RO
                && chars[i + 2] == COENG
                && i + 3 < chars.len()
                && is_khmer_base(chars[i + 3])
            {
                chars[i..i + 4].rotate_left(2);
                changed = true;
            }
            i += 1;
        }
        // Rule 4: adjacent marks out of canonical class order
        // (shifter < vowel < sign), e.g. NIKAHIT typed before its vowel
        // (ំា for ាំ). Same-class pairs are left alone, so each swap
        // strictly reduces the number of out-of-order pairs and the outer
        // fixed-point loop terminates.
        let mut i = 1;
        while i < chars.len() {
            if let (Some(a), Some(b)) = (mark_class(chars[i - 1]), mark_class(chars[i])) {
                if a > b {
                    chars.swap(i - 1, i);
                    changed = true;
                }
            }
            i += 1;
        }
        if !changed {
            break;
        }
    }

    chars.into_iter().collect()
}

/// Fully normalize text: performs combining character ordering, orthographic
/// replacements (e.g. ឲ -> ឱ្យ), common spelling corrections, and punctuation/whitespace
/// cleanup. Note that this changes string length, so byte offsets will not align
/// with the original text.
pub fn normalize_full(text: &str) -> String {
    // 1. Combining character reordering (existing rule 1 & rule 2)
    let reordered = normalize(text);

    // 2. Replacements: spelling and orthographic corrections.
    //
    // Order matters: "ឲ្យ" (the extremely common variant spelling of
    // "ឱ្យ") must be rewritten *before* bare "ឲ", or the bare-ឲ rule
    // would turn it into "ឱ្យ្យ" — a double subscript, i.e. corrupted
    // text, not a correction.
    //
    // "េា"/"េី" are two-part-vowel typing errors: a base takes one
    // dependent vowel, so េ directly followed by ា or ី is never valid —
    // the typist built ោ (or ើ) out of two keystrokes. Rewriting to the
    // single canonical code point is the standard repair.
    let normalized = reordered
        .replace("ឲ្យ", "ឱ្យ")
        .replace("ឲ", "ឱ្យ")
        .replace("\u{17C1}\u{17B6}", "\u{17C4}") // េ + ា -> ោ
        .replace("\u{17C1}\u{17B8}", "\u{17BE}") // េ + ី -> ើ
        .replace("\u{17A3}", "\u{17A2}") // deprecated ឣ -> អ
        // NB: សាស្រ្ត -> សាស្ត្រ no longer needs a replacement here —
        // normalize()'s rule 3 (subscript-RO reordering) repairs it, and
        // every other word with the same swap, before this point.
        .replace("យូលង់", "យូរលង់")
        .replace("ចរិក", "ចរិត")
        .replace("ប្រភទ", "ប្រភេទ");

    // 3. Segment the text to identify word boundaries.
    let tokenizer = get_tokenizer();
    let tokens = tokenizer.segment(&normalized);

    // 4. Reconstruct the text with normalized spaces.
    let mut result = String::with_capacity(normalized.len());
    
    // Spaced words: conjunctions and clause-starting markers
    let spaced_before = [
        "ឬក៏ជា", "ប៉ុន្តែ", "តែ", "ហើយ", "ព្រោះ", "ពីព្រោះ", "ព្រមទាំង", 
        "កាលណា", "ខណៈពេល", "ដូចជា", "មាន", "មិនមាន", "មិនមែន", "និង", "ឬ"
    ];
    let spaced_after = [
        "ឬក៏ជា", "ប៉ុន្តែ", "តែ", "ហើយ", "ព្រោះ", "ពីព្រោះ", "ព្រមទាំង", 
        "កាលណា", "ខណៈពេល", "ដូចជា"
    ];

    // Suffixes/modifiers that should NEVER have a space before them (forces merging with the left token)
    let force_merge_left = [
        "ខ្មែរ", "ជាតិ", "ធំ", "តូច", "ថ្មី", "ចាស់", "ល្អ", "អាក្រក់", "ច្រើន", "តិច", 
        "ខ្លាំង", "ខ្សោយ", "គ្រប់", "ទាំងអស់", "នីមួយៗ", "ផ្ទាល់ខ្លួន", "ផ្ទាល់", 
        "ផ្ទាល់មាត់", "ក្នុងស្រុក", "ក្រៅប្រទេស", "ទៅ", "មក", "ឡើង", "ចុះ", "ចេញ", 
        "ចូល", "ទៀត", "ដែរ", "ណាស់", "ពេក", "ជាង", "បំផុត", "ពិតប្រាកដ", "ពិត", 
        "ប្រាកដ", "របស់ខ្លួន", "ខ្លួន", "ឃើញថា", "យល់ឃើញ", "អារាម", "សិល្ប៍", "សាស្ត្រ", "សង្គម"
    ];

    // Prefixes/particles that should NEVER have a space after them (forces merging with the right token)
    let force_merge_right = [
        "មិន", "កុំ", "គ្មាន", "ឥត", "និង", "នៃ", "ក្នុង", "ចំពោះ", "លើ", "ក្រោម", 
        "ពី", "ទៅ", "ដល់", "តាម", "ដោយ", "នៅ", "ជាមួយ", "ជាមួយនឹង", "ការ", 
        "សេចក្តី", "ភាព", "រឿង", "វត្ត", "ព្រះ", "អក្សរ", "ត្រូវ", "អាច", "បាន", 
        "កំពុង", "តែង", "ធ្លាប់", "ឱ្យ", "ជា", "គឺ", "គឺជា"
    ];

    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 {
            let prev = &tokens[i - 1];
            
            // Check if we should insert a space
            let should_space = (prev == "។" || prev == "៕" || prev == "៖")
                || ((spaced_before.contains(&tok.as_str()) || spaced_after.contains(&prev.as_str()))
                    && !force_merge_right.contains(&prev.as_str())
                    && !force_merge_left.contains(&tok.as_str()));

            if should_space {
                result.push(' ');
            }
        }
        result.push_str(tok);
    }

    result.trim().to_string()
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

    #[test]
    fn reorders_subscript_ro_behind_a_following_subscript() {
        // Rule 3: COENG+RO must be last in a subscript stack.
        assert_eq!(normalize("ស្រ្តី"), "ស្ត្រី");
        assert_eq!(normalize("រដ្ឋមន្រ្តី"), "រដ្ឋមន្ត្រី");
        assert_eq!(normalize("សាស្រ្ត"), "សាស្ត្រ");
        // Already-canonical RO-last stacks are untouched.
        assert_eq!(normalize("ស្ត្រី"), "ស្ត្រី");
        assert_eq!(normalize("ហ្វ្រង្ក"), "ហ្វ្រង្ក");
    }

    #[test]
    fn reorders_a_sign_typed_before_its_vowel() {
        // Rule 4: ាំ typed as ំា (both render identically).
        let swapped = "ណ\u{17C6}\u{17B6}"; // ណ ំ ា
        assert_eq!(normalize(swapped), "ណ\u{17B6}\u{17C6}"); // ណាំ
        // ុំ typed as ំុ.
        let swapped = "ខ\u{17D2}\u{1789}\u{17C6}\u{17BB}"; // ខ ្ញ ំ ុ
        assert_eq!(normalize(swapped), "ខ្ញុំ");
        // A vowel typed before a register shifter is also repaired.
        let swapped = "ប\u{17B7}\u{17CA}"; // ប ិ ៊
        assert_eq!(normalize(swapped), "ប\u{17CA}\u{17B7}");
        // Canonical order (shifter, vowel, sign) is a no-op.
        assert_eq!(normalize("ប៊ិច"), "ប៊ិច");
        assert_eq!(normalize("នាំ"), "នាំ");
    }

    #[test]
    fn ro_swap_and_sign_order_repairs_preserve_byte_length_and_idempotence() {
        for s in ["ស្រ្តី", "រដ្ឋមន្រ្តី", "ណ\u{17C6}\u{17B6}"] {
            let once = normalize(s);
            assert_eq!(once.len(), s.len(), "byte length changed for {s:?}");
            assert_eq!(normalize(&once), once, "not idempotent for {s:?}");
        }
    }

    #[test]
    fn normalize_full_does_not_corrupt_the_common_oy_spelling() {
        // Regression: replace("ឲ", "ឱ្យ") used to turn ឲ្យ into ឱ្យ្យ —
        // a double subscript, i.e. corrupted text.
        assert_eq!(normalize_full("ឲ្យ"), "ឱ្យ");
        assert_eq!(normalize_full("គាត់ឲ្យលុយខ្ញុំ"), "គាត់ឱ្យលុយខ្ញុំ");
    }

    #[test]
    fn normalize_full_repairs_two_part_vowel_typing() {
        // េ + ា typed as two keystrokes for ោ, and េ + ី for ើ.
        assert_eq!(normalize_full("ក\u{17C1}\u{17B6}ះ"), "កោះ");
        assert_eq!(normalize_full("គ\u{17C1}\u{17B8}"), "គើ");
    }

    #[test]
    fn normalize_full_reorders_and_corrects_spelling_and_spacing() {
        // Unicode combining order correction:
        assert_eq!(normalize_full("សិទិ្ធ"), "សិទ្ធិ");

        // Orthographic mapping:
        assert_eq!(normalize_full("ឲ"), "ឱ្យ");
        assert_eq!(normalize_full("ខ្ញុំឲនំបុ័ងទៅសត្វ"), "ខ្ញុំឱ្យនំបុ័ងទៅសត្វ");

        // Spelling correction:
        assert_eq!(normalize_full("យូលង់"), "យូរលង់");
        assert_eq!(normalize_full("ចរិក"), "ចរិត");
        assert_eq!(normalize_full("ប្រភទ"), "ប្រភេទ");
        assert_eq!(normalize_full("អក្សរសាស្រ្ត"), "អក្សរសាស្ត្រ");

        // Spacing and word joining:
        assert_eq!(normalize_full("អក្សរសិល្ប៍ ខ្មែរ"), "អក្សរសិល្ប៍ខ្មែរ");
        assert_eq!(normalize_full("អក្សរ ត្រូវ"), "អក្សរត្រូវ");
        assert_eq!(normalize_full("ត្រូវ ប្រែប្រួល"), "ត្រូវប្រែប្រួល");
        assert_eq!(normalize_full("ការវិវត្តន៍ សង្គម"), "ការវិវត្តន៍សង្គម");
        assert_eq!(normalize_full("នៃមនុស្ស ជាដាច់ខាត"), "នៃមនុស្សជាដាច់ខាត");
        assert_eq!(normalize_full("ប្រជាជន ឲ"), "ប្រជាជនឱ្យ");
        assert_eq!(normalize_full("និង អរិយធម៌"), "និងអរិយធម៌");
        assert_eq!(normalize_full("ភាសា ជា សម្ភារៈ"), "ភាសាជាសម្ភារៈ");
        assert_eq!(normalize_full("គេ អាច"), "គេអាច");
        assert_eq!(normalize_full("សម្លឹងមើល ឃើញថា"), "សម្លឹងមើលឃើញថា");
        assert_eq!(normalize_full("ជីវិត មនុស្ស"), "ជីវិតមនុស្ស");
        assert_eq!(normalize_full("នៃ មនុស្ស"), "នៃ...មនុស្ស".replace("...", "")); // avoiding raw combining mark issues
        assert_eq!(normalize_full("ចែកចេញជា ពីរ ប្រភេទ"), "ចែកចេញជាពីរប្រភេទ");
        assert_eq!(normalize_full("រឿង រាមកេរ្តិ៍"), "រឿងរាមកេរ្តិ៍");
        assert_eq!(normalize_full("វត្ត អារាម"), "វត្តអារាម");
        assert_eq!(normalize_full("រឿង ធនញ្ជ័យ"), "រឿងធនញ្ជ័យ");

        // Punctuation spacing:
        assert_eq!(normalize_full("កម្ពុជា    ។"), "កម្ពុជា។");
        assert_eq!(normalize_full("កម្ពុជា   និង   សៀម   ៖"), "កម្ពុជា និងសៀម៖");
        assert_eq!(normalize_full("   ភាសាខ្មែរ  "), "ភាសាខ្មែរ");
    }
}
