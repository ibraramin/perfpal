//! Precision corpus — PERF-RS-ALLOC-001 empty-ctor FP class (fix 3).
//! Zero-alloc constructors (`String::new` / `Vec::new` / map-set `::new`,
//! empty `vec![]`) never touch the allocator, so per-iteration "allocation"
//! findings on them are false positives. Real per-iteration allocation keeps
//! firing (`real_alloc_in_loop`).
//! (No `.len()` / `.capacity()` calls here: they would fire PERF-RS-STR-002,
//! which this file does not plant.)

use std::collections::HashMap;

/// SILENT: every site in this loop is a zero-alloc constructor.
pub fn empty_ctors_in_loop(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        let a = String::new();
        let b: Vec<usize> = Vec::new();
        let c: HashMap<usize, usize> = HashMap::new();
        let d: Vec<usize> = vec![];
        let _ = (&a, &b, &c, &d);
        total += i;
    }
    total
}

/// TP: a real per-iteration allocation still fires (loop-span anchored).
pub fn real_alloc_in_loop(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        let v: Vec<usize> = vec![i; 4];
        total += v[0] + i;
    }
    total
}
