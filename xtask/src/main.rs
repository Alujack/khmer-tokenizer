//! Development tooling for khmer-tokenizer.
//!
//! ```text
//! cargo xtask eval             # download khPOS, run the eval harness, print P/R/F1
//! cargo xtask eval-kh10000b    # run against the local (manually-placed) kh_data_10000b corpus
//! cargo xtask prepare-dict     # rebuild core/src/dict.txt from chamkho's khmerdict.txt
//! ```

mod dict;
mod download;
mod report;

use std::fs;
use std::path::Path;

use khmer_tokenizer_core::{HmmModel, KhmerTokenizer, Strategy};
use khmer_tokenizer_eval::corpus::{self, Split};
use khmer_tokenizer_eval::{count_frequencies, evaluate, kh10000b, train_hmm};

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("eval") => run_eval(),
        Some("eval-kh10000b") => run_eval_kh10000b(),
        Some("prepare-dict") => run_prepare_dict(),
        _ => print_usage(),
    }
}

fn print_usage() {
    eprintln!("USAGE: cargo xtask <COMMAND>\n");
    eprintln!("COMMANDS:");
    eprintln!("  eval             Download khPOS and print P/R/F1 for the current tokenizer");
    eprintln!("  eval-kh10000b    Evaluate against the local kh_data_10000b corpus (silver reference)");
    eprintln!("  prepare-dict     Rebuild core/src/dict.txt from chamkho's khmerdict.txt");
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

    // These rows opt out of Phase 5's normalization pass (on by default as
    // of this phase) so they keep reproducing their exact historical
    // Phase 1-4 numbers in docs/BENCHMARKS.md. Normalization's own
    // contribution is isolated separately, at the end of this function.
    for (label, strategy) in [
        ("ForwardMaxMatch", Strategy::ForwardMaxMatch),
        ("BiMaxMatch", Strategy::BiMaxMatch),
    ] {
        let tokenizer = KhmerTokenizer::with_default_dict()
            .with_strategy(strategy)
            .without_normalization();
        let metrics = evaluate(&examples, &tokenizer);
        report::print_table(label, &metrics);
    }

    // UnigramDp needs frequencies. khPOS is CC BY-NC-SA, so these are
    // computed here for local evaluation only — never bundled or shipped
    // (see core/ATTRIBUTION.md and docs/ROADMAP.md Phase 3). Split::Train is
    // confirmed disjoint from OpenTest, so this doesn't leak the eval set.
    let train_examples = corpus::load_khpos_dir(&repo_dir, Split::Train).unwrap_or_else(|e| {
        eprintln!("error: could not read khPOS train split: {e}");
        std::process::exit(1);
    });
    let freqs = count_frequencies(&train_examples);

    let tokenizer = KhmerTokenizer::with_default_dict()
        .with_strategy(Strategy::UnigramDp)
        .with_frequencies(freqs.clone())
        .without_normalization();
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table("UnigramDp (freq: khPOS train, local-only)", &metrics);

    // Phase 4: HMM fallback for clusters that no strategy matched in the
    // dictionary at all. Same licensing posture as the frequencies above:
    // BMES tag counts are trained from khPOS's train split for local
    // evaluation only — never bundled, committed, or shipped.
    let hmm_counts = train_hmm(&train_examples);
    let hmm_model = HmmModel::from_counts(hmm_counts.start, hmm_counts.trans, hmm_counts.emit);

    let tokenizer = KhmerTokenizer::with_default_dict()
        .with_hmm(hmm_model.clone())
        .without_normalization();
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table("ForwardMaxMatch + HMM (khPOS train, local-only)", &metrics);

    let tokenizer = KhmerTokenizer::with_default_dict()
        .with_strategy(Strategy::UnigramDp)
        .with_frequencies(freqs.clone())
        .with_hmm(hmm_model.clone())
        .without_normalization();
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table("UnigramDp + HMM (khPOS train, local-only)", &metrics);

    // Phase 5: orthographic normalization, on by default as of this phase.
    // Isolate its contribution on top of the weakest and strongest
    // configurations above by comparing against their without_normalization()
    // numbers just printed.
    let tokenizer = KhmerTokenizer::with_default_dict().with_strategy(Strategy::ForwardMaxMatch);
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table("ForwardMaxMatch + Normalization", &metrics);

    let tokenizer = KhmerTokenizer::with_default_dict()
        .with_strategy(Strategy::UnigramDp)
        .with_frequencies(freqs)
        .with_hmm(hmm_model);
    let metrics = evaluate(&examples, &tokenizer);
    report::print_table(
        "UnigramDp + HMM + Normalization (khPOS train, local-only)",
        &metrics,
    );
}

/// `kh_data_10000b` isn't auto-downloaded like khPOS/chamkho — its source
/// and license are unclear, so it's never fetched automatically, only read
/// if a caller has manually placed it under `data/`. Its `_seg_200b`
/// segmentation looks machine-produced, not human-verified (see
/// `eval/src/kh10000b.rs`), so results here are a **silver-reference**
/// comparison, not accuracy in the same sense as the khPOS numbers above.
fn run_eval_kh10000b() {
    let dir = Path::new("data/kh_data_10000b");
    if !dir.exists() {
        eprintln!(
            "error: {} not found. This corpus isn't auto-downloaded (source/license \
             unclear) -- place it there manually to run this command.",
            dir.display()
        );
        std::process::exit(1);
    }

    let result = kh10000b::load_dir(dir).unwrap_or_else(|e| {
        eprintln!("error: could not read {}: {e}", dir.display());
        std::process::exit(1);
    });

    println!(
        "loaded {} examples from {}/{} pairs ({} skipped: missing counterpart or misaligned text)\n",
        result.examples.len(),
        result.total_pairs - result.skipped_pairs,
        result.total_pairs,
        result.skipped_pairs,
    );
    println!(
        "NOTE: kh_data_10000b's segmentation is a SILVER reference of unknown \
         provenance (looks machine-produced, not human-verified) -- scores below \
         measure agreement with it, not verified accuracy. See docs/BENCHMARKS.md.\n"
    );

    for (label, strategy) in [
        ("ForwardMaxMatch", Strategy::ForwardMaxMatch),
        ("BiMaxMatch", Strategy::BiMaxMatch),
    ] {
        let tokenizer = KhmerTokenizer::with_default_dict()
            .with_strategy(strategy)
            .without_normalization();
        let metrics = evaluate(&result.examples, &tokenizer);
        report::print_table(label, &metrics);
    }

    let tokenizer = KhmerTokenizer::with_default_dict().with_strategy(Strategy::ForwardMaxMatch);
    let metrics = evaluate(&result.examples, &tokenizer);
    report::print_table("ForwardMaxMatch + Normalization", &metrics);
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
