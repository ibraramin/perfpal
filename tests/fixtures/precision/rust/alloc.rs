//! Precision corpus — Rust alloc family (PERF-RS-ALLOC-001): planted true positive.
//! The TP uses a REAL per-iteration allocation (zero-alloc ctors are silent since fix 3).

/// TP ALLOC-001: a single innermost loop that allocates per iteration. The
/// finding anchors at the LOOP span (the `for` line), not the alloc call.
pub fn alloc_in_loop(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        let v: Vec<usize> = vec![i; 4];
        total += v.len() + i;
    }
    total
}
