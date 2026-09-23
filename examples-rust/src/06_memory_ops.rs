//! 06_memory_ops.rs — size_of misuse and block copies inside loops.
//!
//! Mirrors `examples/06_memory_misuse.c`.
//!
//! Expected findings:
//!   PERF-RS-MEM-001  warn  with_capacity(size_of::<T>()) element-count misuse
//!   PERF-RS-MEM-002  info  to_vec()/clone() block copies inside a loop
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `capacity_from_count` passes a plain element COUNT to
//!     `with_capacity`, so MEM-001 must NOT fire.

/// A byte count passed where an element count is expected (MEM-001).
pub fn over_reserve() -> Vec<u64> {
    Vec::with_capacity(std::mem::size_of::<u64>()) // MEM-001
}

/// Correct element-count capacity — the MEM-001 silent control.
pub fn capacity_from_count(count: usize) -> Vec<u64> {
    Vec::with_capacity(count) // control: no size_of argument
}

/// Per-iteration block copy/clone (MEM-002).
pub fn duplicate_rows(rows: &[Vec<u8>], repeats: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for _ in 0..repeats {
        for row in rows {
            out.push(row.clone()); // MEM-002 (clone, innermost)
            out.extend(row.to_vec()); // MEM-002 (to_vec)
        }
    }
    out
}
