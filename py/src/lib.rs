//! Python bindings for `khmer-tokenizer-core` (PyO3 / maturin).
//!
//! ```python
//! from khmer_tokenizer import KhmerTokenizer, split_kcc, normalize
//!
//! tk = KhmerTokenizer()  # embedded default dictionary, forward max-match
//! tk.segment("សួស្តីអ្នកទាំងអស់គ្នា")
//! # ['សួស្តី', 'អ្នក', 'ទាំងអស់គ្នា']
//! ```
//!
//! The Rust builder methods are folded into keyword arguments on the
//! constructor — the tokenizer is immutable after construction, matching
//! the core crate's builder-then-segment design.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use khmer_tokenizer_core as core;

/// Dictionary-backed Khmer word segmenter.
///
/// Args:
///     words: Optional list of dictionary words. Omit to use the embedded
///         59,526-word default dictionary (chamkho / SIL NRSI, MIT — see
///         the crate's ATTRIBUTION.md).
///     strategy: "fmm" (forward max-match, default), "bimm" (bidirectional
///         max-match), or "unigram" (unigram-DP over word frequencies —
///         needs `frequencies`, otherwise falls back to "fmm").
///     frequencies: Optional dict of word -> count for the "unigram"
///         strategy. No table ships with the package; supply counts from a
///         corpus you're licensed to use.
///     normalization: Whether to repair known Khmer Unicode encoding
///         errors before segmenting (default True; byte-length-preserving).
#[pyclass(frozen)]
struct KhmerTokenizer {
    inner: core::KhmerTokenizer,
}

#[pymethods]
impl KhmerTokenizer {
    #[new]
    #[pyo3(signature = (words=None, *, strategy="fmm", frequencies=None, tagger=None, normalization=true))]
    fn new(
        words: Option<Vec<String>>,
        strategy: &str,
        frequencies: Option<HashMap<String, u64>>,
        tagger: Option<String>,
        normalization: bool,
    ) -> PyResult<Self> {
        let strategy = match strategy {
            "fmm" => core::Strategy::ForwardMaxMatch,
            "bimm" => core::Strategy::BiMaxMatch,
            "unigram" => core::Strategy::UnigramDp,
            "tagger" => core::Strategy::Tagger,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown strategy '{other}' (expected 'fmm', 'bimm', 'unigram', or 'tagger')"
                )))
            }
        };

        let mut inner = match words {
            Some(words) => core::KhmerTokenizer::from_words(words),
            None => core::KhmerTokenizer::with_default_dict(),
        }
        .with_strategy(strategy);

        if let Some(frequencies) = frequencies {
            inner = inner.with_frequencies(frequencies);
        }
        if let Some(tagger_str) = tagger {
            let model = core::TaggerModel::from_text(&tagger_str)
                .map_err(|e| PyValueError::new_err(format!("invalid tagger model: {e}")))?;
            inner = inner.with_tagger(model);
        }
        if !normalization {
            inner = inner.without_normalization();
        }

        Ok(Self { inner })
    }

    /// Segment Khmer text into a list of word tokens.
    fn segment(&self, text: &str) -> Vec<String> {
        self.inner.segment(text)
    }

    /// True if `word` is an exact entry in the dictionary.
    fn contains(&self, word: &str) -> bool {
        self.inner.contains(word)
    }

    /// Number of distinct words in the dictionary.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("KhmerTokenizer({} words)", self.inner.len())
    }
}

/// Split text into Khmer Character Clusters (orthographic syllable units).
#[pyfunction]
fn split_kcc(text: &str) -> Vec<String> {
    core::split_kcc(text)
}

/// Repair known Khmer Unicode encoding errors (mark/subscript ordering,
/// NFC damage). Byte-length-preserving; idempotent.
#[pyfunction]
fn normalize(text: &str) -> String {
    core::normalize(text)
}

/// Fully normalize text: performs combining character ordering, orthographic
/// replacements, common spelling corrections, and punctuation/whitespace cleanup.
#[pyfunction]
fn normalize_full(text: &str) -> String {
    core::normalize_full(text)
}

/// True if the character falls in the Khmer Unicode block.
#[pyfunction]
fn is_khmer(c: char) -> bool {
    core::is_khmer(c)
}

#[pymodule]
fn khmer_tokenizer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<KhmerTokenizer>()?;
    m.add_function(wrap_pyfunction!(split_kcc, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_full, m)?)?;
    m.add_function(wrap_pyfunction!(is_khmer, m)?)?;
    Ok(())
}
