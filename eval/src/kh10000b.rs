//! Loader for `kh_data_10000b`: ~10,000 real-world Khmer web articles, each
//! as an `<id>_orig.txt` (raw article) / `<id>_seg_200b.txt` (word-segmented)
//! pair. No README/license/provenance ships with the data, and the `_200b`
//! segmentation looks machine-produced (it inserts far more boundary marks
//! than the original text's own line-wrapping hints) rather than
//! human-verified — so it's treated as a **silver** reference throughout,
//! not gold on the same footing as khPOS. Never committed: `data/` is
//! gitignored, and this corpus's source and rights are unclear.
//!
//! The segmentation marks word boundaries with `U+200B` ZERO WIDTH SPACE
//! (invisible, so the segmented text still *looks* identical to the
//! original when rendered) and, less consistently, plain spaces. Before
//! trusting a document pair, its `_orig.txt` and `_seg_200b.txt` are
//! compared with every boundary marker stripped from both sides — this
//! caught that ~99.4% of the 10,000 pairs are byte-identical underneath,
//! with the small remainder (~0.6%) differing only by a bullet character
//! (`•`) the segmentation step normalized to a hyphen. Misaligned pairs are
//! skipped rather than silently corrupting the eval set.

use std::fs;
use std::io;
use std::path::Path;

use crate::corpus::Example;

const ZWSP: char = '\u{200B}';

fn is_boundary(c: char) -> bool {
    c == ZWSP || c == ' '
}

/// Split one segmented line into gold tokens on runs of ZWSP/space.
fn split_tokens(line: &str) -> Vec<String> {
    line.split(is_boundary)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Strip every boundary marker this corpus uses (plus newlines, so
/// multi-line documents compare as one stream), for alignment checking.
fn strip_boundaries(text: &str) -> String {
    text.chars()
        .filter(|&c| !is_boundary(c) && c != '\n')
        .collect()
}

/// Parse one `_orig.txt` / `_seg_200b.txt` pair into examples, one per
/// non-empty line of the segmented file. Returns `None` if the pair's
/// underlying text doesn't match once boundary markers are stripped from
/// both sides — callers should skip the pair rather than trust it.
pub fn parse_pair(orig: &str, seg: &str) -> Option<Vec<Example>> {
    if strip_boundaries(orig) != strip_boundaries(seg) {
        return None;
    }

    Some(
        seg.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let gold_tokens = split_tokens(line);
                let input = gold_tokens.concat();
                Example { input, gold_tokens }
            })
            .filter(|ex| !ex.gold_tokens.is_empty())
            .collect(),
    )
}

/// Outcome of loading a directory of `<id>_orig.txt`/`<id>_seg_200b.txt`
/// pairs: the valid examples, plus how many pairs were skipped (missing
/// counterpart or failed alignment) out of how many `_orig.txt` files were
/// found — so a caller can report data quality, not just silently proceed.
pub struct LoadResult {
    pub examples: Vec<Example>,
    pub skipped_pairs: usize,
    pub total_pairs: usize,
}

/// Load every `<id>_orig.txt`/`<id>_seg_200b.txt` pair directly inside `dir`.
pub fn load_dir(dir: &Path) -> io::Result<LoadResult> {
    let mut ids: Vec<String> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter_map(|name| name.strip_suffix("_orig.txt").map(str::to_string))
        .collect();
    ids.sort();

    let mut examples = Vec::new();
    let mut skipped_pairs = 0;
    let total_pairs = ids.len();

    for id in &ids {
        let orig = fs::read_to_string(dir.join(format!("{id}_orig.txt")))?;
        let seg = match fs::read_to_string(dir.join(format!("{id}_seg_200b.txt"))) {
            Ok(s) => s,
            Err(_) => {
                skipped_pairs += 1;
                continue;
            }
        };

        match parse_pair(&orig, &seg) {
            Some(exs) => examples.extend(exs),
            None => skipped_pairs += 1,
        }
    }

    Ok(LoadResult {
        examples,
        skipped_pairs,
        total_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_zwsp_and_space_delimited_line_into_gold_tokens() {
        let line = "\u{200B}មក\u{200B}ដឹង\u{200B}ពី ក្រោយ\u{200B}ចាក់\u{200B}!";
        assert_eq!(
            split_tokens(line),
            vec!["មក", "ដឹង", "ពី", "ក្រោយ", "ចាក់", "!"]
        );
    }

    #[test]
    fn parses_an_aligned_pair_into_one_example_per_line() {
        let orig = "សួស្តី\u{200B}អ្នក\nទាំងអស់\u{200B}គ្នា";
        let seg = "សួស្តី\u{200B} អ្នក\nទាំងអស់ គ្នា";
        let examples = parse_pair(orig, seg).expect("aligned pair should parse");
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].gold_tokens, vec!["សួស្តី", "អ្នក"]);
        assert_eq!(examples[0].input, "សួស្តីអ្នក");
        assert_eq!(examples[1].gold_tokens, vec!["ទាំងអស់", "គ្នា"]);
    }

    #[test]
    fn rejects_a_pair_whose_underlying_text_diverges() {
        let orig = "សួស្តីអ្នក";
        let seg = "សួស្តី ខ្មែរ"; // different word entirely, not just re-spaced
        assert!(parse_pair(orig, seg).is_none());
    }

    #[test]
    fn tolerates_boundary_markers_already_present_in_the_original() {
        // The original CMS text can already carry its own ZWSP hints;
        // those must not cause a false alignment mismatch.
        let orig = "សួស្តី\u{200B}អ្នក";
        let seg = "សួស្តី អ្នក";
        assert!(parse_pair(orig, seg).is_some());
    }

    #[test]
    fn skips_blank_lines() {
        let orig = "សួស្តី\n\nអ្នក";
        let seg = "សួស្តី\n\nអ្នក";
        let examples = parse_pair(orig, seg).unwrap();
        assert_eq!(examples.len(), 2);
    }
}
