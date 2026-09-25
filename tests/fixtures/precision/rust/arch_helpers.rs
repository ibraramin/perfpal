//! Precision corpus — Rust cross-file callee definitions (part 1).

/// Uniquely defined corpus callee (the ARCH-003 true-positive target).
pub fn compute_total(x: u64) -> u64 {
    x + 1
}

/// One of TWO definitions of `new` (the ambiguity half of the silent control).
pub fn new(y: u64) -> u64 {
    y
}
