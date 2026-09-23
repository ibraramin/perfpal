//! cycle_b.rs — the other half of the PERF-RS-ARCH-001 `use` import cycle.
//!
//! Expected findings: the cycle is anchored at the lowest-sorted broken edge
//! (see `cycle_a.rs`); this file supplies its `use crate::cycle_a::…` half.

use crate::cycle_a::cycle_a_root;

/// Root item imported back by `cycle_a.rs`.
pub fn cycle_b_step(value: u64) -> u64 {
    value.wrapping_mul(3)
}

/// Calls across the cycle (trivial; the finding is the import cycle itself).
pub fn cycle_b_root(value: u64) -> u64 {
    cycle_a_root(value)
}
