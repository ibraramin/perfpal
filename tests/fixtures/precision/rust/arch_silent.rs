//! Precision corpus — Rust cross-module coupling silent control.

/// SILENT ARCH-003: `new` is defined in TWO other files (`arch_helpers.rs`
/// and `arch_extra.rs`) — ambiguous names emit no edge.
pub fn call_ambiguous(y: u64) -> u64 {
    new(y) // silent PERF-RS-ARCH-003 (ambiguous callee)
}
