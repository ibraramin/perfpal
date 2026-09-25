//! Precision corpus — Rust cross-file callee definitions (part 2).

/// The SECOND definition of `new`; its existence makes the name ambiguous.
pub fn new(y: u64) -> u64 {
    y + 2
}
