//! 09_membership_scan.rs — linear membership scans nested in loops.
//!
//! Mirrors `examples/09_membership_scan.c`.
//!
//! Expected findings:
//!   PERF-RS-LOOP-007  info  `table[j] == keys[i]` inside a two-level nest
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `first_match_shallow` compares a subscript at loop depth 1, below the
//!     LOOP-007 floor of 2, so LOOP-007 must NOT fire.

/// O(n*m) membership scan (LOOP-007).
pub fn contains_nested(table: &[u64], keys: &[u64]) -> bool {
    for i in 0..keys.len() {
        for j in 0..table.len() {
            if table[j] == keys[i] {
                // LOOP-007 (depth 2, index is inner induction var)
                return true;
            }
        }
    }
    false
}

/// Depth-1 lookup — the LOOP-007 silent control.
pub fn first_match_shallow(table: &[u64], key: u64) -> bool {
    for j in 0..table.len() {
        if table[j] == key {
            // control: depth 1, below the LOOP-007 floor
            return true;
        }
    }
    false
}
