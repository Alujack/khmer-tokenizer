//! Prints evaluation results to stdout in a scannable table.

use khmer_tokenizer_eval::Metrics;

pub fn print_table(metrics: &Metrics) {
    println!("sentences      : {}", metrics.sentences);
    println!("precision      : {:.4}", metrics.precision);
    println!("recall         : {:.4}", metrics.recall);
    println!("f1             : {:.4}", metrics.f1);
    println!("r_iv           : {:.4}", metrics.r_iv);
    println!("r_oov          : {:.4}", metrics.r_oov);
    println!("word_accuracy  : {:.4}", metrics.word_accuracy);
}
