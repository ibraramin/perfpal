//! 04_stdio_churn.rs — character/byte-wise processing and per-iteration I/O.
//!
//! Mirrors `examples/04_stdio_churn.c`.
//!
//! Expected findings:
//!   PERF-RS-LOOP-003  info  chars() as the loop body's PRIMARY statement
//!   PERF-RS-LOOP-002  hot   println!/write_all inside a loop
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `consume_all` nests `chars()` inside another call, so it is not the
//!     loop body's primary statement and LOOP-003 must NOT fire there.

use std::io::Write;

/// Per-character work as the loop body's primary statement (LOOP-003).
pub fn count_chars_per_iteration(text: &str, n: usize) -> usize {
    let mut total = 0usize;
    for _ in 0..n {
        text.chars(); // LOOP-003 (primary statement)
        total += 1;
    }
    total
}

/// Per-iteration I/O (LOOP-002).
pub fn emit_rows(rows: &[String], sink: &mut dyn Write) {
    for row in rows {
        println!("{row}"); // LOOP-002 (println! macro)
        let _ = sink.write_all(row.as_bytes()); // LOOP-002 (write_all method)
    }
}

/// `chars()` feeding another call is nested, not primary — LOOP-003 silent
/// control.
pub fn consume_all(text: &str, out: &mut Vec<char>) {
    for _ in 0..1 {
        out.extend(text.chars()); // control: nested, not primary -> no LOOP-003
    }
}
