//! 10_cross_file_and_arch.rs — architecture-level coupling signals.
//!
//! Mirrors `examples/10_cross_file_and_arch.c`.
//!
//! Expected findings:
//!   PERF-RS-ARCH-003  info  calls to functions DEFINED in other corpus files
//!                           (01_alloc_churn.rs, 03_deep_nesting.rs, common.rs)
//!                           — LOW confidence, name-based.
//!   PERF-RS-ARCH-002  hot   the `examples-rust/src` component is the hottest
//!                           in the scan; it anchors at its highest-complexity
//!                           function (classify() in 05_recursion_complexity.rs).
//!   PERF-RS-ARCH-001  warn  the cycle_a ↔ cycle_b import cycle (reported
//!                           against cycle_a.rs / cycle_b.rs).
//!
//! Silent control (must produce ZERO findings of the named rule):
//!   * `clamp_all` only calls `clamp_int`, which is DEFINED in THIS file, so
//!     file-local-first suppression means ARCH-003 must NOT fire for it.

/// Cross-file calls (ARCH-003).
pub fn orchestrate(n: usize) -> u64 {
    let rows = build_row_labels(n); // defined in 01_alloc_churn.rs
    let checksum = scale_checksum(n as u64); // defined in 03_deep_nesting.rs
    let salted = shared_checksum(checksum); // defined in common.rs
    let _ = rows.len();
    salted
}

/// File-local helper: callers here resolve locally, so no ARCH-003 edge.
pub fn clamp_int(value: i64, lo: i64, hi: i64) -> i64 {
    if value < lo {
        return lo;
    }
    if value > hi {
        return hi;
    }
    value
}

/// Only calls the file-local helper — the ARCH-003 silent control.
pub fn clamp_all(values: &[i64], lo: i64, hi: i64) -> i64 {
    let mut total = 0i64;
    for value in values {
        total += clamp_int(*value, lo, hi); // control: file-local -> no ARCH-003
    }
    total
}
