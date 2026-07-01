//! khPOS corpus loading.
//!
//! khPOS (<https://github.com/ye-kyaw-thu/khPOS>, CC BY-NC-SA 4.0 — download
//! only, never bundled) ships `*.word` files: one sentence per line, gold
//! words separated by single spaces. Multi-syllable compound words are
//! written with an internal `_` or `~` joining their parts (e.g.
//! `លោក~ស្រី`, `កង_ទ័ព`) — confirmed against the real corpus that neither
//! character ever appears as a standalone token, so both are stripped to
//! recover the word's plain text before it's used as a gold span or as part
//! of the reconstructed (space-free) input.

use std::fs;
use std::io;
use std::path::Path;

/// One evaluation example: the raw, space-free input text the tokenizer
/// sees, paired with the gold word segmentation for that same text.
pub struct Example {
    pub input: String,
    pub gold_tokens: Vec<String>,
}

/// Which khPOS split to load.
#[derive(Clone, Copy)]
pub enum Split {
    OpenTest,
    CloseTest,
    /// The full 12,000-sentence training corpus. Confirmed (by exact-line
    /// overlap) disjoint from `OpenTest` (11/1000 incidental matches) but
    /// **fully contained in `CloseTest`** (1000/1000) — never evaluate
    /// against `CloseTest` using anything derived from this split.
    Train,
}

impl Split {
    fn relative_path(self) -> &'static str {
        match self {
            Split::OpenTest => "OPEN-TEST.word",
            Split::CloseTest => "CLOSE-TEST.word",
            Split::Train => "before-replace/train6.word",
        }
    }
}

/// Parse khPOS `.word` file contents into evaluation examples.
///
/// Pure string → data; no I/O, so it's directly unit-testable without a
/// network fetch.
pub fn parse_khpos(raw: &str) -> Vec<Example> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let gold_tokens: Vec<String> = line
                .split_whitespace()
                .map(|tok| tok.replace(['_', '~'], ""))
                .collect();
            let input = gold_tokens.concat();
            Example { input, gold_tokens }
        })
        .collect()
}

/// Load a khPOS split from a cloned `ye-kyaw-thu/khPOS` repository checkout.
pub fn load_khpos_dir(repo_dir: &Path, split: Split) -> io::Result<Vec<Example>> {
    let path = repo_dir
        .join("corpus-draft-ver-1.0/data")
        .join(split.relative_path());
    let raw = fs::read_to_string(path)?;
    Ok(parse_khpos(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_gold_tokens_and_rebuilds_input() {
        let examples = parse_khpos("សួស្តី អ្នក\nគាត់ ឈ្មោះ មឿន\n");
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].gold_tokens, vec!["សួស្តី", "អ្នក"]);
        assert_eq!(examples[0].input, "សួស្តីអ្នក");
        assert_eq!(examples[1].gold_tokens, vec!["គាត់", "ឈ្មោះ", "មឿន"]);
    }

    #[test]
    fn strips_compound_word_joiners() {
        let examples = parse_khpos("លោក~ស្រី ឃុន កង_ទ័ព");
        assert_eq!(examples[0].gold_tokens, vec!["លោកស្រី", "ឃុន", "កងទ័ព"]);
        assert_eq!(examples[0].input, "លោកស្រីឃុនកងទ័ព");
    }

    #[test]
    fn skips_blank_lines() {
        let examples = parse_khpos("សួស្តី\n\nអ្នក\n");
        assert_eq!(examples.len(), 2);
    }
}
