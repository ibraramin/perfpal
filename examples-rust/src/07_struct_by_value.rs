//! 07_struct_by_value.rs — large by-value structs and padding candidates.
//!
//! Mirrors `examples/07_struct_by_value.c`.
//!
//! Expected findings:
//!   PERF-RS-TYPE-001  warn  BigBlob (>= 32 bytes under LP64) passed by value
//!   PERF-RS-TYPE-002  info  PaddedLayout shows >= 8 bytes of declared-order
//!                           padding
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `consume_blob_ref(&BigBlob)` takes the struct by reference, so
//!     TYPE-001 must NOT fire (the bare-named-type gate rejects `&T`).
//!   * `PackedTight` is padding-free (aligned 16 == packed 16), so TYPE-002
//!     must NOT fire.

/// Four 8-byte words = 32 bytes under the LP64 model.
pub struct BigBlob {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}

/// Declared-order padding: u8, u64, u8, u64 => aligned 32, packed 18,
/// padding 14 bytes.
pub struct PaddedLayout {
    pub tag: u8,
    pub wide_a: u64,
    pub flag: u8,
    pub wide_b: u64,
}

/// Padding-free: aligned 16, packed 16, padding 0.
pub struct PackedTight {
    pub x: u64,
    pub y: u64,
}

/// By-value parameter (TYPE-001).
pub fn consume_blob(blob: BigBlob) -> u64 {
    blob.a ^ blob.b ^ blob.c ^ blob.d
}

/// By-reference parameter — the TYPE-001 silent control.
pub fn consume_blob_ref(blob: &BigBlob) -> u64 {
    blob.a + blob.d
}

/// Reads the padded layout so its definition is live.
pub fn layout_total(layout: &PaddedLayout) -> u64 {
    u64::from(layout.tag) + layout.wide_a + u64::from(layout.flag) + layout.wide_b
}
