#![no_main]
//! Fuzz `normalize` for panics and for its two documented invariants on
//! *arbitrary* text: it is byte-length-preserving (only reorders characters)
//! and idempotent (normalizing twice equals normalizing once).

use khmer_tokenizer_core::normalize;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let once = normalize(s);
        // Documented contract: only reorders, never adds/removes characters.
        assert_eq!(
            once.len(),
            s.len(),
            "normalize changed byte length: {s:?} -> {once:?}"
        );
        // Documented contract: idempotent.
        let twice = normalize(&once);
        assert_eq!(twice, once, "normalize is not idempotent on {s:?}");
    }
});
