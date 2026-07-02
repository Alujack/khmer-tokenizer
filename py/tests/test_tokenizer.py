"""Behavioral tests for the Python bindings.

These mirror the core crate's own unit tests, so a binding-layer bug
(argument mapping, string conversion, strategy dispatch) shows up even
though the underlying Rust logic is already tested in `core/`.
"""

import pytest

from khmer_tokenizer import KhmerTokenizer, is_khmer, normalize, split_kcc


def test_default_dict_segments():
    tk = KhmerTokenizer()
    assert tk.segment("សួស្តីអ្នកទាំងអស់គ្នា") == ["សួស្តី", "អ្នក", "ទាំងអស់គ្នា"]


def test_default_dict_is_loaded():
    tk = KhmerTokenizer()
    assert len(tk) > 50_000
    assert tk.contains("កម្ពុជា")
    assert not tk.contains("not-a-khmer-word")


def test_custom_word_list():
    tk = KhmerTokenizer(words=["ភាសា", "ខ្មែរ"])
    assert tk.segment("ភាសាខ្មែរ") == ["ភាសា", "ខ្មែរ"]
    assert len(tk) == 2


def test_mixed_scripts_and_whitespace():
    tk = KhmerTokenizer()
    assert tk.segment("ខ្ញុំស្រឡាញ់ Rust 2026 កម្ពុជា") == [
        "ខ្ញុំ",
        "ស្រឡាញ់",
        "Rust",
        "2026",
        "កម្ពុជា",
    ]


def test_zwsp_is_a_trusted_boundary():
    tk = KhmerTokenizer(words=["កខ"])
    assert tk.segment("កខ") == ["កខ"]  # sanity: merges without ZWSP
    assert tk.segment("ក\u200bខ") == ["ក", "ខ"]  # ZWSP boundary wins


def test_bimm_strategy():
    tk = KhmerTokenizer(strategy="bimm")
    assert tk.segment("សួស្តីអ្នក") == ["សួស្តី", "អ្នក"]


def test_unigram_strategy_beats_greedy_with_frequencies():
    # The core crate's synthetic ambiguity: greedy always picks ["កខ", "គ"];
    # only frequency-scored DP can reach ["ក", "ខគ"].
    words = ["ក", "កខ", "ខគ", "គ"]
    greedy = KhmerTokenizer(words=words)
    assert greedy.segment("កខគ") == ["កខ", "គ"]

    dp = KhmerTokenizer(
        words=words,
        strategy="unigram",
        frequencies={"ក": 100, "ខគ": 100, "កខ": 1, "គ": 1},
    )
    assert dp.segment("កខគ") == ["ក", "ខគ"]


def test_unknown_strategy_raises_value_error():
    with pytest.raises(ValueError, match="unknown strategy"):
        KhmerTokenizer(strategy="crf")


def test_normalization_on_by_default_and_toggleable():
    # "សិទិ្ធ" is a real-world malformed spelling of the dictionary word
    # "សិទ្ធិ"; normalization (default) repairs it before matching.
    tk = KhmerTokenizer(words=["សិទ្ធិ"])
    assert tk.segment("សិទិ្ធ") == ["សិទ្ធិ"]

    raw = KhmerTokenizer(words=["សិទ្ធិ"], normalization=False)
    assert raw.segment("សិទិ្ធ") != ["សិទ្ធិ"]


def test_normalize_function():
    assert normalize("សិទិ្ធ") == "សិទ្ធិ"
    assert normalize("កម្ពុជា") == "កម្ពុជា"  # canonical text is a no-op


def test_split_kcc_function():
    assert split_kcc("ខ្មែរ") == ["ខ្មែ", "រ"]
    assert split_kcc("ស្ត្រី") == ["ស្ត្រី"]  # stacked subscripts stay whole


def test_is_khmer_function():
    assert is_khmer("ខ")
    assert not is_khmer("a")


def test_repr():
    tk = KhmerTokenizer(words=["ភាសា", "ខ្មែរ"])
    assert repr(tk) == "KhmerTokenizer(2 words)"
