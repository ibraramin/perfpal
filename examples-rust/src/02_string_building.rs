//! 02_string_building.rs — string accumulation, length queries, unsafe memory.
//!
//! Mirrors `examples/02_string_building.c`.
//!
//! Expected findings:
//!   PERF-RS-STR-002  warn  len() recomputed inside a loop
//!   PERF-RS-STR-003  warn  unsafe std::slice::from_raw_parts (any depth)
//!
//! Note: PERF-RS-STR-001 was RETIRED for Rust (the `Vec::push` name collision
//! dominated the self-scan), so `accumulate` now produces NO finding — it is a
//! retire-probe, not a positive.
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `len_with_reassignment` assigns to the receiver inside the loop, which
//!     suppresses STR-002 for that call.
//!   * `bounded_copy` uses `copy_from_slice`, the MEM-002 family, never
//!     STR-003, so STR-003 must NOT fire there.

/// Accumulate into a growable string per iteration (retired STR-001 probe).
pub fn accumulate(parts: &[&str], out: &mut String) {
    for part in parts {
        out.push_str(part); // retired STR-001: no finding
        out.push(';'); // retired STR-001: no finding
    }
}

/// Recompute the length of an unmutated string per iteration (STR-002).
pub fn scan_lengths(words: &[String]) -> usize {
    let joined = words.join(" ");
    let mut total = 0usize;
    for _ in 0..words.len() {
        total += joined.len(); // STR-002
    }
    total
}

/// Reassigning the receiver inside the loop suppresses STR-002.
pub fn len_with_reassignment(n: usize) -> usize {
    let mut s = String::new();
    let mut total = 0usize;
    for i in 0..n {
        s = format!("{s}{i}"); // assignment suppresses STR-002 below
        total += s.len(); // control: STR-002 must NOT fire
    }
    total
}

/// Compound assignment produced no CallEvent for the retired STR-001.
pub fn append_via_add_assign(parts: &[&str]) -> String {
    let mut s = String::new();
    for part in parts {
        s += part; // retired STR-001: no CallEvent, no finding
    }
    s
}

/// Unchecked raw-memory view (STR-003).
///
/// # Safety
/// `ptr` must point to `len` readable bytes.
pub unsafe fn unsafe_slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    std::slice::from_raw_parts(ptr, len) // STR-003
}

/// Bounded block copy: the MEM-002 family, never STR-003.
pub fn bounded_copy(src: &[u8], dst: &mut [u8]) {
    dst.copy_from_slice(src); // control: STR-003 must NOT fire
}
