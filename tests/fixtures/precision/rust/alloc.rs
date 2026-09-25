//! Precision corpus — Rust alloc family (PERF-RS-ALLOC-001): planted true
//! positive. Silent controls live in `alloc_silent.rs`.

/// TP ALLOC-001: a single innermost loop that allocates per iteration. The
/// finding anchors at the LOOP span (the `for` line), not the alloc call.
pub fn alloc_in_loop(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        let v: Vec<usize> = Vec::new();
        total += v.len() + i;
    }
    total
}
