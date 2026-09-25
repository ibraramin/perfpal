//! M2 benchmark suite entry point (plan `evidence-beta` M2).
//!
//! `cargo test --locked --test benchmark -- --nocapture` prints the scale
//! ladder (files, LOC, findings), a per-rule TP/FP/FN/precision/recall table
//! for every rung, and the two-run byte-determinism verdict. The whole suite
//! is always-on and hermetic: every generated corpus and report lives in a
//! fresh tempdir and is removed on completion.
//!
//! See `tests/benchmark/mod.rs` (harness) and `tests/benchmark/corpus.rs`
//! (mutation catalog + the extension point for the remaining 36 rules).

// The mandated layout puts the test entry at `tests/benchmark.rs` and the
// harness at `tests/benchmark/mod.rs`; an explicit `#[path]` disambiguates
// (rustc otherwise reports E0761 for the same-named file + directory).
#[path = "benchmark/mod.rs"]
mod benchmark;
mod common;

#[test]
fn benchmark_suite() {
    benchmark::run_suite();
}
