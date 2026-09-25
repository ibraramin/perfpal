// End-to-end MIXED C+Rust fixture — Rust side (plan `rust-backend`, R5 C1).
//
// `build` deliberately fires four PERF-RS-* families:
//   - PERF-RS-ALLOC-001 (Vec::new inside the for loop)
//   - PERF-RS-ALLOC-003 (out.reserve additive growth inside the loop)
//   - PERF-RS-STR-002   (s.len / tmp.len recomputed inside the loop)
//   - PERF-RS-MEM-001   (size_of used as an element count)
//
// PERF-RS-STR-001 is RETIRED for Rust, so `s.push_str(p)` is a silence probe.
//
// Cross-file, CROSS language (deliberate): `util_init` is defined in
// src/util.c. `cross_file_call_edges` is language-scoped, so this call emits
// NO Rust edge and NO PERF-RS-ARCH-003 finding. The mirror-direction call
// lives in src/main.c; tests/mixed_language_e2e.rs locks both.
pub fn build(parts: &[&str], n: usize) -> String {
    let mut s = String::new();
    let mut out: Vec<u8> = Vec::new();
    for p in parts {
        let tmp: Vec<u8> = Vec::new();
        out.reserve(1);
        s.push_str(p);
        let _ = s.len();
        let _ = tmp.len();
    }
    let v: Vec<u8> = Vec::with_capacity(std::mem::size_of::<u64>());
    let _ = v;
    let _ = n;
    s
}

/// Cross-language call into the C tier (src/util.c defines `util_init`):
/// deliberately unmatched by the language-scoped call-graph builder.
pub fn rust_surface(n: usize) -> usize {
    util_init(n as i32) as usize + n
}
