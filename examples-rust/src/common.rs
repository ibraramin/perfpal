//! common.rs — shared helpers for the Perfpal Rust demo corpus.
//!
//! This is the Rust analogue of `examples/perf_common.h`: a real internal
//! module that several programs reference. It defines the cross-file call
//! target consumed by the PERF-RS-ARCH-003 demo.
//!
//! Expected findings in THIS file: none (shared helpers only).

/// Baseline chunk size, mirroring `perf_common.h`'s `LABEL_BYTES`.
pub const CHUNK_BYTES: usize = 64;

/// A deterministic checksum helper defined in `common` and called from
/// `10_cross_file_and_arch.rs` by bare name, so PERF-RS-ARCH-003 reports a
/// name-based cross-file coupling edge for it.
pub fn shared_checksum(value: u64) -> u64 {
    value.rotate_left(7) ^ 0x9E37_79B9
}
