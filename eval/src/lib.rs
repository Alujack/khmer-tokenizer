//! Corpus loading and evaluation metrics for `khmer-tokenizer`.
//!
//! Not published — internal tooling consumed by `xtask eval`.

pub mod corpus;
pub mod freq;
pub mod metrics;

pub use corpus::{load_khpos_dir, parse_khpos, Example};
pub use freq::count_frequencies;
pub use metrics::{evaluate, Metrics};
