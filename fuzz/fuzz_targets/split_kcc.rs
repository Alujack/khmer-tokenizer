#![no_main]
//! Fuzz `split_kcc` for panics and for its core invariant on arbitrary text:
//! the clusters it emits concatenate back to exactly the input (no character
//! is dropped, duplicated, or reordered — only boundaries are inserted).

use khmer_tokenizer_core::split_kcc;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let clusters = split_kcc(s);
        let rejoined: String = clusters.concat();
        assert_eq!(rejoined, s, "split_kcc did not preserve the input exactly");
    }
});
