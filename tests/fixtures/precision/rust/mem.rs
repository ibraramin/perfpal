//! Precision corpus — Rust block-copy family (PERF-RS-MEM-002): planted true
//! positive. Silent controls live in `mem_silent.rs`.

/// TP MEM-002: `copy_from_slice` is a block-copy method in an innermost loop;
/// the finding anchors at the call.
pub fn copy_block(dst: &mut [u8], src: &[u8]) {
    for i in 0..src.len() {
        dst.copy_from_slice(src); // TP PERF-RS-MEM-002
        let _ = i;
    }
}
