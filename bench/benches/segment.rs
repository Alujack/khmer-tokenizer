//! Throughput benchmarks for the segmenter and the one-time dictionary build.
//!
//! Run: `cargo bench --manifest-path bench/Cargo.toml`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use khmer_tokenizer_core::{KhmerTokenizer, Strategy};

/// One realistic news-style Khmer sentence (mixed letters, Khmer digits,
/// a Latin/number run, and a khan). Repeated to make a corpus.
const SENTENCE: &str = "រដ្ឋាភិបាលកម្ពុជាបានប្រកាសនៅថ្ងៃទី១៥ខែកក្កដាឆ្នាំ២០២៥ថានឹងបង្កើនកិច្ចសហប្រតិបត្តិការជាមួយប្រទេសជប៉ុន Rust 2026។";

fn throughput(c: &mut Criterion) {
    let corpus = SENTENCE.repeat(200); // ~a page of dense Khmer text
    let bytes = corpus.len() as u64;

    let minwords = KhmerTokenizer::with_default_dict();
    let fmm = KhmerTokenizer::with_default_dict().with_strategy(Strategy::ForwardMaxMatch);
    let bimm = KhmerTokenizer::with_default_dict().with_strategy(Strategy::BiMaxMatch);

    let mut group = c.benchmark_group("segment");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function("minwords_default", |b| {
        b.iter(|| minwords.segment(black_box(&corpus)))
    });
    group.bench_function("forward_max_match", |b| {
        b.iter(|| fmm.segment(black_box(&corpus)))
    });
    group.bench_function("bidirectional_max_match", |b| {
        b.iter(|| bimm.segment(black_box(&corpus)))
    });
    group.finish();
}

fn build(c: &mut Criterion) {
    // The one-time cost of parsing the embedded ~59k-word dictionary into the
    // cluster trie — amortized across every segment() call in a real program.
    c.bench_function("build_default_dict", |b| {
        b.iter(|| black_box(KhmerTokenizer::with_default_dict()))
    });
}

fn splitting(c: &mut Criterion) {
    let corpus = SENTENCE.repeat(200);
    c.bench_function("split_kcc", |b| {
        b.iter(|| black_box(khmer_tokenizer_core::split_kcc(black_box(&corpus))))
    });
}

criterion_group!(benches, throughput, build, splitting);
criterion_main!(benches);
