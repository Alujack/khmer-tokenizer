//! Development tooling for khmer-tokenizer.
//!
//! ```text
//! cargo xtask eval           # download khPOS, run the eval harness, print P/R/F1
//! cargo xtask prepare-dict   # rebuild core/src/dict.txt from chamkho's khmerdict.txt
//! ```

mod dict;
mod download;
mod report;

use std::fs;
use std::path::Path;

use khmer_tokenizer_core::{KhmerTokenizer, Strategy};
use khmer_tokenizer_eval::corpus::{self, Split};
use khmer_tokenizer_eval::{count_frequencies, evaluate};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("eval") => run_eval(),
        Some("prepare-dict") => run_prepare_dict(),
        _ => print_usage(),
    }
}

fn print_usage() {
    eprintln!("USAGE: cargo xtask <COMMAND>\n");
    eprintln!("COMMANDS:");
    eprintln!("  eval           Download khPOS and print P/R/F1 for the current tokenizer");
    eprintln!("  prepare-dict   Rebuild core/src/dict.txt from chamkho's khmerdict.txt");
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

    for (label, strategy) in [
        ("ForwardMaxMatch", Strategy::ForwardMaxMatch),
        ("BiMaxMatch", Strategy::BiMaxMatch),
    ] {
        let tokenizer = KhmerTokenizer::with_default_dict().with_strategy(strategy);
        let metrics = evaluate(&examples, &tokenizer);
        report::print_table(label, &metrics);
    }

    // UnigramDp needs frequencies. khPOS is CC BY-NC-SA, so these are
    // computed here for local evaluation only — never bundled or shipped
    // (see ATTRIBUTION.md and docs/ROADMAP.md Phase 3). Split::Train is
    // confirmed disjoint from OpenTest, so this doesn't leak the eval set.
    let train_examples = corpus::load_khpos_dir(&repo_dir, Split::Train).unwrap_or_else(|e| {
        eprintln!("error: could not read khPOS train split: {e}");
        std::process::exit(1);
    });
    let freqs = count_frequencies(&train_examples);

    let tokenizer = KhmerTokenizer::with_default_dict()
        .with_strategy(Strategy::UnigramDp)
        .with_frequencies(freqs);
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table("UnigramDp (freq: khPOS train, local-only)", &metrics);
}

fn run_prepare_dict() {
    let data_dir = Path::new("data");
    let repo_dir = download::ensure_chamkho(data_dir).unwrap_or_else(|e| {
        eprintln!("error: could not fetch chamkho: {e}");
        std::process::exit(1);
    });

    let raw = fs::read_to_string(repo_dir.join("data/khmerdict.txt")).unwrap_or_else(|e| {
        eprintln!("error: could not read chamkho's khmerdict.txt: {e}");
        std::process::exit(1);
    });

    let words = dict::clean_wordlist(&raw);
    let rendered = dict::render_dict_txt(&words);

    let out_path = Path::new("core/src/dict.txt");
    fs::write(out_path, &rendered).unwrap_or_else(|e| {
        eprintln!("error: could not write {}: {e}", out_path.display());
        std::process::exit(1);
    });

    println!("wrote {} words to {}", words.len(), out_path.display());
}
