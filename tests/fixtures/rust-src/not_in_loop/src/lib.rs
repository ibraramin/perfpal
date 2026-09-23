//! Real-parse lock source for the `PERF-RS-ALLOC-001` / `PERF-RS-LOOP-002`
//! not-in-loop negatives (plan `rust-backend` R5 lane A, LL-009 pairing).
//!
//! Each rule gets an in-loop site (must fire) and an out-of-loop site of the
//! SAME family (must stay silent), so the real backend's loop-depth derivation
//! is exercised in both directions instead of asserting on an empty document.

pub fn fill(n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let tmp = Vec::new();
        out.push(tmp.len() as u8);
        let _ = i;
    }
    // Out of any loop: per the catalog this allocation is not churn.
    let fresh = Vec::new();
    let _ = fresh;
    out
}

pub fn log_loop(n: usize) {
    for i in 0..n {
        println!("{i}");
    }
    // Out of any loop: a one-shot log is not per-iteration I/O.
    println!("done");
}
