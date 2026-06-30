//! Command-line interface for the Khmer tokenizer.
//!
//! ```text
//! khmer-tokenizer "សួស្តីអ្នកទាំងអស់គ្នា"   # segment argument(s)
//! echo "សួស្តីអ្នក" | khmer-tokenizer        # segment stdin
//! khmer-tokenizer --json "ភាសាខ្មែរ"         # JSON array per line
//! ```
//!
//! Output is space-separated tokens by default, or a JSON array with `--json`.
//! Input is read from the command-line arguments, or from stdin when no text
//! argument is given. Each input line is segmented independently.

use std::io::{self, Read, Write};

use khmer_tokenizer_core::KhmerTokenizer;

fn main() {
    let mut json = false;
    let mut text_args: Vec<String> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--json" | "-j" => json = true,
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => text_args.push(arg),
        }
    }

    // Prefer text from arguments; otherwise read stdin.
    let input = if !text_args.is_empty() {
        text_args.join(" ")
    } else {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("error: failed to read stdin: {e}");
            std::process::exit(1);
        }
        buf
    };

    let tk = KhmerTokenizer::with_default_dict();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in input.lines() {
        let tokens = tk.segment(line);
        let rendered = if json {
            let items: Vec<String> = tokens.iter().map(|t| json_escape(t)).collect();
            format!("[{}]", items.join(","))
        } else {
            tokens.join(" ")
        };
        let _ = writeln!(out, "{rendered}");
    }
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
    println!("  -j, --json    Output a JSON array of tokens per line");
    println!("  -h, --help    Show this help and exit");
}
