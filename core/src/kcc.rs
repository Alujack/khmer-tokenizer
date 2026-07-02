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

/// A Khmer *base*: consonants (`U+1780..=U+17A2`) and independent vowels
/// (`U+17A3..=U+17B3`). A cluster always begins with one of these.
pub(crate) fn is_khmer_base(c: char) -> bool {
    matches!(c as u32, 0x1780..=0x17B3)
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
                if d == COENG && i + 1 < n {
                    // COENG plus the consonant it subscripts.
                    i += 2;
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
    }
}
