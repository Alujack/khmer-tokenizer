//! Command-line interface for the Khmer tokenizer.
//!
//! ```text
//! khmer-tokenizer "សួស្តីអ្នកទាំងអស់គ្នា"   # segment argument(s)
//! echo "សួស្តីអ្នក" | khmer-tokenizer        # segment stdin
//! khmer-tokenizer --json "ភាសាខ្មែរ"         # JSON array per line
//! khmer-tokenizer --strategy bimm "..."     # bidirectional max-match
//! khmer-tokenizer --zwsp "..."              # U+200B-delimited output
//! khmer-tokenizer --dict words.txt "..."    # custom dictionary
//! khmer-tokenizer --strategy unigram --freq freqs.txt "..."
//! khmer-tokenizer --strategy tagger --tagger model.txt "..."
//! ```
//!
//! Output is space-separated tokens by default, a JSON array with `--json`,
//! or tokens joined with `U+200B` ZERO WIDTH SPACE with `--zwsp` — the
//! delimiter the Unicode Standard recommends for marking Khmer word
//! boundaries, giving output that renders identically to the input while
//! carrying machine-readable boundaries. Input is read from the
//! command-line arguments, or from stdin when no text argument is given.
//! Each input line is segmented independently.
//!
//! The stronger tiers need data you supply (none ships — see the crate
//! README): `--strategy unigram` wants a `--freq` table (`word<TAB>count`
//! per line), and `--strategy tagger` wants a `--tagger` model file (the
//! `TaggerModel::to_text` format, e.g. produced by `cargo xtask
//! train-tagger`). A `--tagger` model given alongside a dictionary strategy
//! acts as the out-of-vocabulary fallback instead.

use std::io::{self, Read, Write};

use khmer_tokenizer_core::{KhmerTokenizer, Strategy, TaggerModel};

fn main() {
    let mut json = false;
    let mut zwsp = false;
    let mut strategy = Strategy::default();
    let mut dict_path: Option<String> = None;
    let mut freq_path: Option<String> = None;
    let mut tagger_path: Option<String> = None;
    let mut text_args: Vec<String> = Vec::new();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" | "-j" => json = true,
            "--zwsp" | "-z" => zwsp = true,
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--strategy" | "-s" => {
                strategy = match value_for(&args, &mut i, "--strategy").as_str() {
                    "minwords" => Strategy::MinWordsDp,
                    "fmm" => Strategy::ForwardMaxMatch,
                    "bimm" => Strategy::BiMaxMatch,
                    "unigram" => Strategy::UnigramDp,
                    "tagger" => Strategy::Tagger,
                    other => {
                        fail(&format!(
                            "unknown strategy '{other}' (expected minwords, fmm, bimm, unigram, or tagger)"
                        ));
                    }
                };
            }
            "--dict" | "-d" => dict_path = Some(value_for(&args, &mut i, "--dict")),
            "--freq" | "-f" => freq_path = Some(value_for(&args, &mut i, "--freq")),
            "--tagger" | "-t" => tagger_path = Some(value_for(&args, &mut i, "--tagger")),
            arg => text_args.push(arg.to_string()),
        }
        i += 1;
    }

    let tk = build_tokenizer(strategy, dict_path, freq_path, tagger_path);

    // Prefer text from arguments; otherwise read stdin.
    let input = if !text_args.is_empty() {
        text_args.join(" ")
    } else {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            fail(&format!("failed to read stdin: {e}"));
        }
        buf
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in input.lines() {
        let tokens = tk.segment(line);
        let rendered = if json {
            let items: Vec<String> = tokens.iter().map(|t| json_escape(t)).collect();
            format!("[{}]", items.join(","))
        } else if zwsp {
            tokens.join("\u{200B}")
        } else {
            tokens.join(" ")
        };
        let _ = writeln!(out, "{rendered}");
    }
}

/// Assemble the tokenizer from the parsed flags, loading any data files and
/// exiting with a clear message on failure. Warns (without failing) when a
/// strategy is selected but the data it needs is missing — the engine falls
/// back to forward max-match in that case, and a silent fallback would look
/// like a bug.
fn build_tokenizer(
    strategy: Strategy,
    dict_path: Option<String>,
    freq_path: Option<String>,
    tagger_path: Option<String>,
) -> KhmerTokenizer {
    let mut tk = match &dict_path {
        Some(path) => KhmerTokenizer::from_dict_str(&read_file(path)).with_strategy(strategy),
        None => KhmerTokenizer::with_default_dict().with_strategy(strategy),
    };

    if let Some(path) = &freq_path {
        tk = tk.with_frequencies(parse_freqs(&read_file(path), path));
    } else if strategy == Strategy::UnigramDp {
        eprintln!(
            "warning: --strategy unigram needs a --freq table to score paths; \
             falling back to minwords"
        );
    }

    if let Some(path) = &tagger_path {
        let model = TaggerModel::from_text(&read_file(path)).unwrap_or_else(|e| {
            fail(&format!("could not parse tagger model '{path}': {e}"));
        });
        tk = tk.with_tagger(model);
    } else if strategy == Strategy::Tagger {
        eprintln!(
            "warning: --strategy tagger needs a --tagger model file; \
             falling back to minwords"
        );
    }

    tk
}

/// Consume the value following a flag at `args[*i]`, advancing `i` past it.
fn value_for(args: &[String], i: &mut usize, flag: &str) -> String {
    *i += 1;
    match args.get(*i) {
        Some(v) => v.clone(),
        None => fail(&format!("{flag} requires a value")),
    }
}

fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("could not read '{path}': {e}")))
}

/// Parse a frequency table: one `word<whitespace>count` entry per line.
/// Blank lines and `#` comments are ignored (matching the dictionary file
/// convention). A malformed or non-numeric count is a hard error rather
/// than a silently-dropped line, so a bad table can't quietly skew scoring.
fn parse_freqs(text: &str, path: &str) -> Vec<(String, u64)> {
    let mut freqs = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.rsplitn(2, char::is_whitespace);
        let count_str = parts.next().unwrap_or_default();
        let Some(word) = parts.next() else {
            fail(&format!(
                "{path}:{}: expected 'word<whitespace>count', found '{line}'",
                n + 1
            ));
        };
        let count: u64 = count_str.parse().unwrap_or_else(|_| {
            fail(&format!(
                "{path}:{}: count '{count_str}' is not a non-negative integer",
                n + 1
            ))
        });
        freqs.push((word.trim().to_string(), count));
    }
    freqs
}

/// Print a message to stderr and exit non-zero. Never returns.
fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

/// Minimal JSON string escaping (sufficient for tokenizer output).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn print_help() {
    println!("khmer-tokenizer — segment Khmer text into words\n");
    println!("USAGE:");
    println!("  khmer-tokenizer [OPTIONS] [TEXT...]");
    println!("  echo TEXT | khmer-tokenizer [OPTIONS]\n");
    println!("OPTIONS:");
    println!("  -j, --json             Output a JSON array of tokens per line");
    println!("  -z, --zwsp             Join tokens with U+200B ZERO WIDTH SPACE (the");
    println!("                         Unicode-recommended Khmer word-boundary marker)");
    println!("  -s, --strategy <NAME>  Segmentation strategy: minwords (default), fmm,");
    println!("                         bimm, unigram (needs --freq), or tagger (needs --tagger)");
    println!("  -d, --dict <FILE>      Use a custom dictionary (one word per line;");
    println!("                         '#' comments and blank lines ignored)");
    println!("  -f, --freq <FILE>      Word-frequency table for unigram scoring");
    println!("                         (one 'word<whitespace>count' per line)");
    println!("  -t, --tagger <FILE>    Averaged-perceptron model (TaggerModel::to_text");
    println!("                         format). Full segmenter under '--strategy tagger',");
    println!("                         or an out-of-vocabulary fallback otherwise");
    println!("  -h, --help             Show this help and exit");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_freq_table_ignoring_comments_and_blanks() {
        let text = "# comment\n\nភាសា 500\nខ្មែរ\t800\n";
        let freqs = parse_freqs(text, "test");
        assert_eq!(
            freqs,
            vec![("ភាសា".to_string(), 500), ("ខ្មែរ".to_string(), 800)]
        );
    }

    #[test]
    fn freq_words_may_be_absent_of_spaces_only_at_line_end() {
        // The count is the last whitespace-delimited field; the word is
        // everything before it (Khmer words never contain spaces, but this
        // keeps a stray leading space from corrupting the parse).
        let freqs = parse_freqs("  ក   42  ", "test");
        assert_eq!(freqs, vec![("ក".to_string(), 42)]);
    }
}
