//! Corpus loading and evaluation metrics for `khmer-tokenizer`.
//!
//! Not published — internal tooling consumed by `xtask eval`.

pub mod corpus;
pub mod freq;
pub mod hmm;
pub mod metrics;

pub use corpus::{load_khpos_dir, parse_khpos, Example};
pub use freq::count_frequencies;
pub use hmm::{train_hmm, HmmCounts};
pub use metrics::{evaluate, Metrics};
