//! WebAssembly bindings for `khmer-tokenizer-core` (wasm-bindgen).
//!
//! Mirrors the Python bindings' surface (`py/src/lib.rs`): one
//! `KhmerTokenizer` class configured entirely at construction time via an
//! options object, plus the three free functions.
//!
//! ```js
//! import { KhmerTokenizer, splitKcc, normalize, isKhmer } from "kh-tokenizer";
//!
//! const tk = new KhmerTokenizer(); // embedded default dictionary, fewest-words DP
//! tk.segment("សួស្តីអ្នកទាំងអស់គ្នា");
//! // ["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
//!
//! new KhmerTokenizer({ strategy: "bimm" });
//! new KhmerTokenizer({ words: ["ភាសា", "ខ្មែរ"] });
//! new KhmerTokenizer({ strategy: "unigram", frequencies: { "ភាសា": 500 } });
//! new KhmerTokenizer({ normalization: false });
//! ```

use js_sys::{Array, Object, Reflect};
use khmer_tokenizer_core as core;
use wasm_bindgen::prelude::*;

/// Read `options[key]`, treating a missing key, `undefined`, and `null`
/// all as "not provided".
fn opt(options: &Object, key: &str) -> Option<JsValue> {
    let v = Reflect::get(options.as_ref(), &JsValue::from_str(key)).ok()?;
    if v.is_undefined() || v.is_null() {
        None
    } else {
        Some(v)
    }
}

#[wasm_bindgen]
pub struct KhmerTokenizer {
    inner: core::KhmerTokenizer,
}

#[wasm_bindgen]
impl KhmerTokenizer {
    /// `new KhmerTokenizer(options?)`
    ///
    /// Options (all optional, matching the Python constructor):
    /// - `words: string[]` — custom word list instead of the embedded
    ///   default dictionary
    /// - `strategy: "minwords" | "fmm" | "bimm" | "unigram" | "tagger"` —
    ///   segmentation strategy (default "minwords")
    /// - `frequencies: Record<string, number>` — word counts for `"unigram"`
    /// - `normalization: boolean` — orthographic normalization (default true)
    /// - `oovGrouping: boolean` — group unmatched cluster runs into one
    ///   unknown-word token each (default true)
    #[wasm_bindgen(constructor)]
    pub fn new(options: Option<Object>) -> Result<KhmerTokenizer, JsError> {
        let options = options.unwrap_or_else(Object::new);

        // Catch typos ({ strateggy: … }) instead of silently ignoring them,
        // the way Python's keyword arguments would.
        for key in Object::keys(&options).iter() {
            let key = key.as_string().unwrap_or_default();
            if !matches!(
                key.as_str(),
                "words" | "strategy" | "frequencies" | "tagger" | "normalization" | "oovGrouping"
            ) {
                return Err(JsError::new(&format!(
                    "unknown option {key:?}; expected words, strategy, frequencies, tagger, normalization, oovGrouping"
                )));
            }
        }

        let mut tokenizer = match opt(&options, "words") {
            None => core::KhmerTokenizer::with_default_dict(),
            Some(v) => {
                if !Array::is_array(&v) {
                    return Err(JsError::new("words must be an array of strings"));
                }
                let mut words = Vec::new();
                for item in Array::from(&v).iter() {
                    words.push(
                        item.as_string()
                            .ok_or_else(|| JsError::new("words must be an array of strings"))?,
                    );
                }
                core::KhmerTokenizer::from_words(words)
            }
        };

        if let Some(v) = opt(&options, "strategy") {
            let name = v
                .as_string()
                .ok_or_else(|| JsError::new("strategy must be a string"))?;
            let strategy = match name.as_str() {
                "minwords" => core::Strategy::MinWordsDp,
                "fmm" => core::Strategy::ForwardMaxMatch,
                "bimm" => core::Strategy::BiMaxMatch,
                "unigram" => core::Strategy::UnigramDp,
                "tagger" => core::Strategy::Tagger,
                other => {
                    return Err(JsError::new(&format!(
                        "unknown strategy {other:?}; expected \"minwords\", \"fmm\", \"bimm\", \"unigram\", or \"tagger\""
                    )))
                }
            };
            tokenizer = tokenizer.with_strategy(strategy);
        }

        if let Some(v) = opt(&options, "frequencies") {
            if !v.is_object() || Array::is_array(&v) {
                return Err(JsError::new(
                    "frequencies must be an object mapping word -> count",
                ));
            }
            let mut counts = Vec::new();
            for entry in Object::entries(&Object::from(v)).iter() {
                let entry = Array::from(&entry);
                let word = entry.get(0).as_string().unwrap_or_default();
                let count = entry.get(1).as_f64().ok_or_else(|| {
                    JsError::new("frequencies values must be non-negative numbers")
                })?;
                if !count.is_finite() || count < 0.0 {
                    return Err(JsError::new(
                        "frequencies values must be non-negative numbers",
                    ));
                }
                counts.push((word, count as u64));
            }
            tokenizer = tokenizer.with_frequencies(counts);
        }

        if let Some(v) = opt(&options, "tagger") {
            let tagger_str = v
                .as_string()
                .ok_or_else(|| JsError::new("tagger must be a string"))?;
            let model = core::TaggerModel::from_text(&tagger_str)
                .map_err(|e| JsError::new(&format!("invalid tagger model: {e}")))?;
            tokenizer = tokenizer.with_tagger(model);
        }

        if let Some(v) = opt(&options, "normalization") {
            let on = v
                .as_bool()
                .ok_or_else(|| JsError::new("normalization must be a boolean"))?;
            if !on {
                tokenizer = tokenizer.without_normalization();
            }
        }

        if let Some(v) = opt(&options, "oovGrouping") {
            let on = v
                .as_bool()
                .ok_or_else(|| JsError::new("oovGrouping must be a boolean"))?;
            if !on {
                tokenizer = tokenizer.without_oov_grouping();
            }
        }

        Ok(KhmerTokenizer { inner: tokenizer })
    }

    /// Segment Khmer text into words. Returns an array of strings.
    pub fn segment(&self, text: &str) -> Vec<String> {
        self.inner.segment(text)
    }

    /// Whether `word` is in this tokenizer's dictionary.
    pub fn contains(&self, word: &str) -> bool {
        self.inner.contains(word)
    }

    /// Number of words in the dictionary (like `Map#size`).
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> usize {
        self.inner.len()
    }
}

/// Split text into Khmer Character Clusters (orthographic syllables).
#[wasm_bindgen(js_name = splitKcc)]
pub fn split_kcc(text: &str) -> Vec<String> {
    core::split_kcc(text)
}

/// Repair common orthographic corruptions (wrong mark/COENG ordering,
/// including the damage Unicode NFC itself inflicts on Khmer).
#[wasm_bindgen]
pub fn normalize(text: &str) -> String {
    core::normalize(text)
}

/// Fully normalize text: performs combining character ordering, orthographic
/// replacements, common spelling corrections, and punctuation/whitespace cleanup.
#[wasm_bindgen(js_name = normalizeFull)]
pub fn normalize_full(text: &str) -> String {
    core::normalize_full(text)
}

/// Whether a single character (one code point) is in the Khmer block.
#[wasm_bindgen(js_name = isKhmer)]
pub fn is_khmer(c: char) -> bool {
    core::is_khmer(c)
}
