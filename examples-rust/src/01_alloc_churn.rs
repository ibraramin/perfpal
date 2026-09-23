//! 01_alloc_churn.rs — per-iteration allocation and teardown.
//!
//! Mirrors `examples/01_allocator_churn.c`.
//!
//! Expected findings:
//!   PERF-RS-ALLOC-001  hot   heap allocation inside a loop (format!,
//!                            to_string, vec! — plus Vec::with_capacity below)
//!   PERF-RS-ALLOC-002  hot   drop() inside a loop, escalated to Hot by
//!                            same-function ALLOC-001 co-occurrence
//!   PERF-RS-ALLOC-003  hot   reserve() additive growth inside a loop
//!   PERF-RS-LOOP-002   hot   println! macro inside a loop
//!   PERF-RS-MEM-002    warn  copy_from_slice in the innermost loop
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `make_scratch` allocates and reserves OUTSIDE any loop, so
//!     ALLOC-001 and ALLOC-003 must NOT fire there.
//!   * `drop_outside_loop` drops at loop depth 0, so ALLOC-002 must NOT fire.

/// Allocations hoisted out of every loop — the ALLOC-001/003 silent control.
pub fn make_scratch() -> String {
    let mut scratch: Vec<u8> = Vec::new(); // control: outside a loop
    scratch.reserve(8); // control: outside a loop (ALLOC-003 needs a loop)
    String::from("scratch")
}

/// Drop at loop depth 0 — the ALLOC-002 silent control.
pub fn drop_outside_loop(value: String) {
    drop(value); // control: no enclosing loop
}

/// Allocate, format, duplicate, and drop one heap value per iteration.
pub fn build_row_labels(rows: usize) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    for i in 0..rows {
        let scratch = format!("row-{i}"); // ALLOC-001 (format! macro)
        let owned = scratch.to_string(); // ALLOC-001 (to_string method)
        let cell: Vec<u8> = vec![i as u8]; // ALLOC-001 (vec! macro)
        labels.push(owned); // Vec::push (STR-001 documented FP class)
        drop(cell); // ALLOC-002 (escalated: alloc co-occurrence)
        println!("row {i}"); // LOOP-002
    }
    labels
}

/// Additive growth: reserve() a little more every iteration.
pub fn grow_one_at_a_time(n: usize) -> Vec<u64> {
    let mut buffer: Vec<u64> = Vec::new();
    for i in 0..n {
        buffer.reserve(1); // ALLOC-003
        buffer.push(i as u64);
    }
    buffer
}

/// Block copy + allocation inside a two-level nest.
pub fn grid_reset(grid: &mut [Vec<u8>], rows: usize, cols: usize, width: usize) {
    for _r in 0..rows {
        for c in 0..cols {
            let mut cell = vec![0u8; width]; // ALLOC-001 (vec! macro)
            let src = &grid[c];
            cell.copy_from_slice(&src[..width]); // MEM-002 (innermost)
            grid[c] = cell;
        }
    }
}
