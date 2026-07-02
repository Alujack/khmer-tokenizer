//! Dictionary preparation: clean a raw wordlist and render it as
//! `core/src/dict.txt`. See `core/ATTRIBUTION.md` for the source and license.

/// Clean a raw one-word-per-line wordlist: trim, drop blank/comment lines,
/// and dedupe (first occurrence wins), preserving source order.
pub fn clean_wordlist(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut words = Vec::new();
    for line in raw.lines() {
        let word = line.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }
        if seen.insert(word) {
            words.push(word.to_string());
        }
    }
    words
}

/// Render a cleaned wordlist as the contents of `core/src/dict.txt`, with a
/// header documenting provenance so the file is self-describing.
pub fn render_dict_txt(words: &[String]) -> String {
    let mut out = format!(
        "# Khmer dictionary for khmer-tokenizer-core.\n\
         # One word per line. Blank lines and lines starting with '#' are ignored.\n\
         #\n\
         # {} words, sourced from chamkho's khmerdict.txt (MIT license, copyright\n\
         # SIL NRSI) via `cargo xtask prepare-dict`. See core/ATTRIBUTION.md.\n\n",
        words.len()
    );
    for word in words {
        out.push_str(word);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_skips_blanks_and_comments() {
        let words = clean_wordlist(" ខ្មែរ \n\n# comment\nភាសា\n");
        assert_eq!(words, vec!["ខ្មែរ", "ភាសា"]);
    }

    #[test]
    fn dedupes_preserving_first_occurrence_order() {
        let words = clean_wordlist("ខ្មែរ\nភាសា\nខ្មែរ\n");
        assert_eq!(words, vec!["ខ្មែរ", "ភាសា"]);
    }

    #[test]
    fn renders_header_and_words() {
        let rendered = render_dict_txt(&["ខ្មែរ".to_string(), "ភាសា".to_string()]);
        assert!(rendered.starts_with("# Khmer dictionary"));
        assert!(rendered.contains("2 words"));
        assert!(rendered.ends_with("ខ្មែរ\nភាសា\n"));
    }
}
