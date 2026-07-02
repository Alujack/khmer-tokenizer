//! Behavioral tests for the WASM bindings, run inside a real wasm engine
//! with `wasm-pack test --node wasm`. Mirrors py/tests/test_tokenizer.py.

use js_sys::{Array, Object, Reflect};
use kh_tokenizer::{is_khmer, normalize, split_kcc, KhmerTokenizer};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn options(pairs: &[(&str, JsValue)]) -> Object {
    let o = Object::new();
    for (key, value) in pairs {
        Reflect::set(&o, &JsValue::from_str(key), value).unwrap();
    }
    o
}

fn string_array(items: &[&str]) -> JsValue {
    let arr = Array::new();
    for item in items {
        arr.push(&JsValue::from_str(item));
    }
    arr.into()
}

fn freq_object(pairs: &[(&str, f64)]) -> JsValue {
    let o = Object::new();
    for (word, count) in pairs {
        Reflect::set(&o, &JsValue::from_str(word), &JsValue::from_f64(*count)).unwrap();
    }
    o.into()
}

#[wasm_bindgen_test]
fn default_dictionary_segments_greeting() {
    let tk = KhmerTokenizer::new(None).unwrap();
    assert_eq!(
        tk.segment("សួស្តីអ្នកទាំងអស់គ្នា"),
        vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
    );
    assert!(tk.size() > 50_000);
    assert!(tk.contains("សួស្តី"));
}

#[wasm_bindgen_test]
fn custom_word_list() {
    let opts = options(&[("words", string_array(&["ភាសា", "ខ្មែរ"]))]);
    let tk = KhmerTokenizer::new(Some(opts)).unwrap();
    assert_eq!(tk.segment("ភាសាខ្មែរ"), vec!["ភាសា", "ខ្មែរ"]);
    assert_eq!(tk.size(), 2);
}

#[wasm_bindgen_test]
fn mixed_scripts_become_their_own_tokens() {
    let tk = KhmerTokenizer::new(None).unwrap();
    let tokens = tk.segment("ភាសាខ្មែរ ABC 123");
    assert!(tokens.contains(&"ABC".to_string()));
    assert!(tokens.contains(&"123".to_string()));
}

#[wasm_bindgen_test]
fn zwsp_is_a_hard_word_boundary() {
    let opts = options(&[("words", string_array(&["កខ"]))]);
    let tk = KhmerTokenizer::new(Some(opts)).unwrap();
    // A dictionary word must NOT be assembled across a ZERO WIDTH SPACE.
    let tokens = tk.segment("ក\u{200B}ខ");
    assert_eq!(tokens, vec!["ក", "ខ"]);
}

#[wasm_bindgen_test]
fn bimm_strategy_is_accepted() {
    let opts = options(&[("strategy", JsValue::from_str("bimm"))]);
    let tk = KhmerTokenizer::new(Some(opts)).unwrap();
    assert_eq!(
        tk.segment("សួស្តីអ្នកទាំងអស់គ្នា"),
        vec!["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]
    );
}

#[wasm_bindgen_test]
fn unigram_frequencies_beat_greedy_matching() {
    // Greedy FMM takes "កខ" first and strands "គ"; unigram DP with these
    // counts prefers ["ក", "ខគ"]. Same fixture as the core and py tests.
    let words = string_array(&["ក", "កខ", "ខគ", "គ"]);
    let freqs = freq_object(&[("ក", 100.0), ("ខគ", 100.0), ("កខ", 1.0), ("គ", 1.0)]);

    let greedy = KhmerTokenizer::new(Some(options(&[("words", words.clone())]))).unwrap();
    assert_eq!(greedy.segment("កខគ"), vec!["កខ", "គ"]);

    let opts = options(&[
        ("words", words),
        ("strategy", JsValue::from_str("unigram")),
        ("frequencies", freqs),
    ]);
    let unigram = KhmerTokenizer::new(Some(opts)).unwrap();
    assert_eq!(unigram.segment("កខគ"), vec!["ក", "ខគ"]);
}

#[wasm_bindgen_test]
fn unknown_strategy_is_an_error() {
    let opts = options(&[("strategy", JsValue::from_str("crf"))]);
    assert!(KhmerTokenizer::new(Some(opts)).is_err());
}

#[wasm_bindgen_test]
fn unknown_option_key_is_an_error() {
    let opts = options(&[("strateggy", JsValue::from_str("bimm"))]);
    assert!(KhmerTokenizer::new(Some(opts)).is_err());
}

#[wasm_bindgen_test]
fn normalization_repairs_malformed_input_and_can_be_disabled() {
    // "សិទិ្ធ" is the common mistyping of "សិទ្ធិ" (mark before the
    // COENG pair). With normalization (default) the dictionary word is
    // found; without it, the malformed run misses the dictionary.
    let words = string_array(&["សិទ្ធិ"]);

    let on = KhmerTokenizer::new(Some(options(&[("words", words.clone())]))).unwrap();
    assert_eq!(on.segment("សិទិ្ធ"), vec!["សិទ្ធិ"]);

    let opts = options(&[("words", words), ("normalization", JsValue::from_bool(false))]);
    let off = KhmerTokenizer::new(Some(opts)).unwrap();
    assert_ne!(off.segment("សិទិ្ធ"), vec!["សិទ្ធិ"]);
}

#[wasm_bindgen_test]
fn free_functions() {
    assert_eq!(split_kcc("ខ្មែរ"), vec!["ខ្មែ", "រ"]);
    assert_eq!(normalize("សិទិ្ធ"), "សិទ្ធិ");
    assert!(is_khmer('ក'));
    assert!(!is_khmer('a'));
}
