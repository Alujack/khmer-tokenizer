//! Corpus loading and evaluation metrics for `khmer-tokenizer`.
//!
//! Not published — internal tooling consumed by `xtask eval`.

pub mod corpus;
pub mod freq;
pub mod hmm;
pub mod kh10000b;
pub mod metrics;
pub mod tagger;

pub use corpus::{load_alt_dir, load_khpos_dir, parse_alt, parse_khpos, AltSplit, Example};
pub use freq::count_frequencies;
pub use hmm::{train_hmm, HmmCounts};
pub use metrics::{evaluate, Metrics};
pub use tagger::train_tagger;
