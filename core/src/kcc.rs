//! Khmer Character Cluster (KCC) splitting.
//!
//! Written Khmer has no spaces between words and uses an abugida script: a base
//! consonant can carry stacked subscript consonants (each introduced by COENG,
//! `U+17D2`) plus dependent vowels and signs. The smallest meaningful unit for
//! word segmentation is therefore the *orthographic cluster*, not the Unicode
//! scalar value.
//!
//! Splitting into clusters first is what keeps the segmenter from ever cutting a
//! base character away from its subscripts or vowels — the core correctness bug
//! in naive char-by-char tokenizers.

/// `U+17D2` KHMER SIGN COENG — joins a following consonant as a subscript.
pub(crate) const COENG: char = '\u{17D2}';

/// `U+17CC` KHMER SIGN ROBAT — the one combining mark that legitimately
/// precedes a `COENG`+consonant subscript pair (see `crate::normalize`).
pub(crate) const ROBAT: char = '\u{17CC}';

/// A Khmer *letter base*: consonants (`U+1780..=U+17A2`) and independent
/// vowels (`U+17A3..=U+17B3`). A word cluster always begins with one of
/// these — Khmer digits, punctuation, and the currency sign are in the
/// Khmer block (so [`is_khmer`] is true for them) but are **not** bases and
/// never start a word cluster.
pub fn is_khmer_base(c: char) -> bool {
    matches!(c as u32, 0x1780..=0x17B3)
}

/// A Khmer digit (`០..=៩`, `U+17E0..=U+17E9`) or divination-lore numeral
/// (`U+17F0..=U+17F9`). Runs of these group into one token, like runs of
/// ASCII digits.
pub(crate) fn is_khmer_digit(c: char) -> bool {
    matches!(c as u32, 0x17E0..=0x17E9 | 0x17F0..=0x17F9)
}

/// Dependent vowels, signs and diacritics that attach to a base, plus the
/// zero-width joiners that may appear inside a cluster.
pub(crate) fn is_khmer_combining(c: char) -> bool {
    let o = c as u32;
    matches!(o, 0x17B4..=0x17D1)   // dependent vowels & most signs
        || o == 0x17D3             // bathamasat
        || o == 0x17DD             // atthacan
        || matches!(o, 0x200C..=0x200D) // ZWNJ / ZWJ
}

/// True if `c` falls anywhere in the Khmer Unicode block (`U+1780..=U+17FF`).
pub fn is_khmer(c: char) -> bool {
    matches!(c as u32, 0x1780..=0x17FF)
}

/// Split `text` into Khmer Character Clusters.
///
/// Each Khmer cluster is a base character followed by any number of
/// COENG+consonant subscripts and combining marks. Non-Khmer scalars (spaces,
/// Latin letters, digits, punctuation) are returned as individual single-char
/// clusters, so downstream code can group or separate them as needed.
///
/// # Example
/// ```
/// use khmer_tokenizer_core::split_kcc;
/// assert_eq!(split_kcc("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
/// assert_eq!(split_kcc("ស្ត្រី"), vec!["ស្ត្រី"]); // stacked subscripts stay whole
/// ```
pub fn split_kcc(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut clusters: Vec<String> = Vec::new();
    let mut i = 0;

    while i < n {
        let c = chars[i];
        if is_khmer_base(c) {
            let start = i;
            i += 1;
            while i < n {
                let d = chars[i];
                if d == COENG {
                    // COENG subscripts the *next base* only. Real-world text
                    // contains dangling COENGs (truncation, typos) followed
                    // by a space, ZWSP, Latin, or nothing — blindly
                    // consuming whatever follows would swallow a word
                    // boundary into the middle of a cluster. A dangling
                    // COENG stays attached to its base; the next char is
                    // left for the outer loop.
                    if i + 1 < n && is_khmer_base(chars[i + 1]) {
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else if is_khmer_combining(d) {
                    i += 1;
                } else {
                    break;
                }
            }
            clusters.push(chars[start..i].iter().collect());
        } else {
            // Non-base scalar (space, Latin, digit, punctuation, stray mark).
            clusters.push(c.to_string());
            i += 1;
        }
    }

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_base_with_subscripts_and_vowels() {
        assert_eq!(split_kcc("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
        assert_eq!(split_kcc("ស្ត្រី"), vec!["ស្ត្រី"]);
    }

    #[test]
    fn separates_non_khmer_scalars() {
        assert_eq!(split_kcc("ab 1"), vec!["a", "b", " ", "1"]);
    }

    #[test]
    fn classifies_khmer_block() {
        assert!(is_khmer('ខ'));
        assert!(!is_khmer('a'));
        assert!(!is_khmer('1'));
        // Digits, punctuation, and currency are in the block...
        assert!(is_khmer('១') && is_khmer('។') && is_khmer('៛'));
        // ...but are not letter bases.
        assert!(!is_khmer_base('១') && !is_khmer_base('។') && !is_khmer_base('៛'));
        assert!(is_khmer_base('ខ'));
    }

    #[test]
    fn dangling_coeng_never_swallows_what_follows() {
        // COENG followed by a space, ZWSP, or Latin letter is malformed
        // (truncated/typo'd) text — the COENG stays with its base and the
        // next char is NOT absorbed into the cluster.
        assert_eq!(split_kcc("ក្ ក"), vec!["ក\u{17D2}", " ", "ក"]);
        assert_eq!(split_kcc("ក្\u{200B}ខ"), vec!["ក\u{17D2}", "\u{200B}", "ខ"]);
        assert_eq!(split_kcc("ក្a"), vec!["ក\u{17D2}", "a"]);
        // At end of text the dangling COENG also stays with its base.
        assert_eq!(split_kcc("ក្"), vec!["ក\u{17D2}"]);
        // And a well-formed subscript pair is untouched.
        assert_eq!(split_kcc("ក្ក"), vec!["ក\u{17D2}ក"]);
    }
}
