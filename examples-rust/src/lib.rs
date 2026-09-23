//! Perfpal Rust demo corpus — mini-crate root (plan `rust-backend`, phase R4).
//!
//! Only a valid Rust identifier can carry a `mod` declaration, and Rust
//! identifiers cannot begin with a digit. The numbered demo programs
//! (`01_alloc_churn.rs` … `10_cross_file_and_arch.rs`) therefore CANNOT be
//! module-declared and are deliberately absent from the module graph below.
//! They are still discovered and scanned as ordinary `.rs` files — the scanner
//! walks the directory tree by extension, not by module declaration — which is
//! exactly what this corpus needs.
//!
//! The three modules below exist to give PERF-RS-ARCH-001 a resolvable
//! internal `use crate::` import cycle (`cycle_a` ↔ `cycle_b`) plus the shared
//! helpers analogous to `examples/perf_common.h`.

mod common;
mod cycle_a;
mod cycle_b;
