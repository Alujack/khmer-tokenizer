#![no_main]
//! Fuzz the full `segment` pipeline (normalize -> cluster -> strategy -> OOV
//! grouping) against the embedded default dictionary. The property under test
//! is simply that no arbitrary input — malformed Khmer, stray combining marks,
//! mixed scripts, control characters — can make it panic or hang.

use khmer_tokenizer_core::KhmerTokenizer;
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

static TOKENIZER: OnceLock<KhmerTokenizer> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let tk = TOKENIZER.get_or_init(KhmerTokenizer::with_default_dict);
        // No output invariant (normalization reorders and separators are
        // dropped, so tokens need not rejoin to the input) — the property is
        // "does not panic".
        let _ = tk.segment(s);
    }
});
