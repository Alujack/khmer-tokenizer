//! Development tooling for khmer-tokenizer.
//!
//! ```text
//! cargo xtask eval   # download khPOS, run the eval harness, print P/R/F1
//! ```

mod download;
mod report;

use std::path::Path;

use khmer_tokenizer_core::KhmerTokenizer;
use khmer_tokenizer_eval::corpus::{self, Split};
use khmer_tokenizer_eval::evaluate;

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("eval") => run_eval(),
        _ => print_usage(),
    }
}

fn print_usage() {
    eprintln!("USAGE: cargo xtask <COMMAND>\n");
    eprintln!("COMMANDS:");
    eprintln!("  eval    Download khPOS and print P/R/F1 for the current tokenizer");
}

fn run_eval() {
    let data_dir = Path::new("data");
    let repo_dir = download::ensure_khpos(data_dir).unwrap_or_else(|e| {
        eprintln!("error: could not fetch khPOS corpus: {e}");
        std::process::exit(1);
    });

    let examples = corpus::load_khpos_dir(&repo_dir, Split::OpenTest).unwrap_or_else(|e| {
        eprintln!("error: could not read khPOS OPEN-TEST split: {e}");
        std::process::exit(1);
    });

    let tokenizer = KhmerTokenizer::with_default_dict();
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table(&metrics);
}
