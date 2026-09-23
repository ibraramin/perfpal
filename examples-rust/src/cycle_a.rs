//! cycle_a.rs — one half of the PERF-RS-ARCH-001 `use` import cycle.
//!
//! Expected findings:
//!   PERF-RS-ARCH-001  warn  kind=cycle — the `use crate::cycle_b::…` edge
//!                           below pairs with cycle_b's `use crate::cycle_a::…`
//!                           to form a module cycle (reported against the
//!                           lowest-sorted broken edge).

use crate::cycle_b::cycle_b_step;

/// Root item imported back by `cycle_b.rs`.
pub fn cycle_a_root(value: u64) -> u64 {
    value.wrapping_add(1)
}

/// Calls across the cycle (trivial; the finding is the import cycle itself).
pub fn cycle_a_step(value: u64) -> u64 {
    cycle_b_step(value) ^ 0xA5A5
}
