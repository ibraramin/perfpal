//! 03_deep_nesting.rs — deep loop nests and variable-divisor division.
//!
//! Mirrors `examples/03_deep_nesting.c`.
//!
//! Expected findings:
//!   PERF-RS-LOOP-001  warn  a depth-3 loop nest (one finding at the depth-3
//!                           ancestor)
//!   PERF-RS-LOOP-004  info  `acc % divisor` with a non-literal divisor in the
//!                           innermost loop
//!
//! `scale_checksum` is also the cross-file call target that
//! `10_cross_file_and_arch.rs` invokes, feeding PERF-RS-ARCH-003.

/// Three-level nest over a 3-D cube: the depth-3 loop is the finding.
pub fn grid_checksum(cube: &[u64], n: usize, divisor: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let idx = (i * n + j) * n + k;
                acc = acc.wrapping_add(cube[idx]);
                acc = acc % divisor; // LOOP-004 (innermost, variable divisor)
            }
        }
    }
    acc
}

/// Cross-file call target used by `10_cross_file_and_arch.rs` (ARCH-003).
pub fn scale_checksum(value: u64) -> u64 {
    value.wrapping_mul(31)
}
