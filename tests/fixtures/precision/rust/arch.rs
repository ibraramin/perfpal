//! Precision corpus — Rust cross-module call coupling (PERF-RS-ARCH-003):
//! planted true positive. Silent controls live in `arch_silent.rs`.

/// TP ARCH-003: `compute_total` is defined in exactly ONE other file
/// (`rust/arch_helpers.rs`), so the M1 uniqueness gate emits an edge.
pub fn call_unique(x: u64) -> u64 {
    compute_total(x) // TP PERF-RS-ARCH-003
}
