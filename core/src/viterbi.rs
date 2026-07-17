//! Shared BMES Viterbi machinery for the two statistical OOV segmenters — the
//! [`HmmModel`](crate::hmm) and the averaged-perceptron
//! [`TaggerModel`](crate::tagger). Both decode the same 4-state
//! Begin/Middle/End/Single lattice and turn the resulting tag sequence into
//! tokens the same way; only the *emission* scoring differs, which each
//! caller supplies as a closure. Keeping the lattice and the tags→tokens
//! conversion here means there is exactly one Viterbi implementation to get
//! right.

/// BMES tag set size.
pub(crate) const NUM_STATES: usize = 4;
pub(crate) const BEGIN: usize = 0;
pub(crate) const MIDDLE: usize = 1;
pub(crate) const END: usize = 2;
pub(crate) const SINGLE: usize = 3;

/// Viterbi-decode the highest-scoring BMES tag sequence over `n` positions.
///
/// `start[s]` scores tag `s` at position 0; `trans[i][j]` scores tag `j`
/// immediately following tag `i`; `emit(t)` returns the per-tag emission
/// score at position `t`. Ties break toward the lower tag index, so decoding
/// is deterministic. `n` must be `>= 1`.
// The DP indexes several parallel `[f64; NUM_STATES]` structures by the same
// state index at once, which a range loop expresses more clearly than an
// iterator chain would.
#[allow(clippy::needless_range_loop)]
pub(crate) fn viterbi(
    n: usize,
    start: &[f64; NUM_STATES],
    trans: &[[f64; NUM_STATES]; NUM_STATES],
    emit: impl Fn(usize) -> [f64; NUM_STATES],
) -> Vec<usize> {
    let mut score = vec![[f64::NEG_INFINITY; NUM_STATES]; n];
    let mut back = vec![[0usize; NUM_STATES]; n];

    let first_emit = emit(0);
    for s in 0..NUM_STATES {
        score[0][s] = start[s] + first_emit[s];
    }
    for t in 1..n {
        let emit_t = emit(t);
        for s in 0..NUM_STATES {
            let mut best_score = f64::NEG_INFINITY;
            let mut best_prev = 0;
            for ps in 0..NUM_STATES {
                let candidate = score[t - 1][ps] + trans[ps][s];
                if candidate > best_score {
                    best_score = candidate;
                    best_prev = ps;
                }
            }
            back[t][s] = best_prev;
            score[t][s] = best_score + emit_t[s];
        }
    }

    let mut best_final = 0;
    for s in 1..NUM_STATES {
        if score[n - 1][s] > score[n - 1][best_final] {
            best_final = s;
        }
    }

    let mut tags = vec![0usize; n];
    tags[n - 1] = best_final;
    for t in (1..n).rev() {
        tags[t - 1] = back[t][tags[t]];
    }
    tags
}

/// Convert a BMES `tags` sequence over `clusters` into tokens (each a `Vec` of
/// its constituent clusters). `clusters` and `tags` must have equal length.
pub(crate) fn bmes_to_tokens(clusters: &[String], tags: &[usize]) -> Vec<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for (cluster, &tag) in clusters.iter().zip(tags) {
        match tag {
            BEGIN => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                current.push(cluster.clone());
            }
            MIDDLE => current.push(cluster.clone()),
            END => {
                current.push(cluster.clone());
                tokens.push(std::mem::take(&mut current));
            }
            SINGLE => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                tokens.push(vec![cluster.clone()]);
            }
            _ => unreachable!("tags are always in 0..NUM_STATES"),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
