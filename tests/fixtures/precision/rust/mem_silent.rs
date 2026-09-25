//! Precision corpus — Rust block-copy silent controls (must stay silent).

/// SILENT MEM-002: `clone` was removed from the family by the M1 gate
/// (ubiquitous ownership clones are not block copies).
pub fn clone_in_loop(items: &[u8]) -> u64 {
    let mut sum = 0u64;
    for it in items {
        let c = it.clone(); // silent PERF-RS-MEM-002 (clone not in family)
        sum += u64::from(c);
    }
    sum
}

/// SILENT MEM-002: a block copy outside any loop.
pub fn copy_once(dst: &mut [u8], src: &[u8]) {
    dst.copy_from_slice(src); // silent PERF-RS-MEM-002 (outside loop)
}
