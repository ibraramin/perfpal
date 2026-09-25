//! Precision corpus — Rust alloc-family silent controls (must stay silent).

/// SILENT ALLOC-001: the allocation is hoisted OUT of any loop.
pub fn alloc_outside_loop() -> Vec<u8> {
    let buf: Vec<u8> = Vec::new(); // silent PERF-RS-ALLOC-001 (outside loop)
    buf
}

/// SILENT ALLOC-001: literal-receiver `to_string` is suppressed by the M1
/// literal-receiver gate. SILENT STR-001: the retired rule can never fire, so
/// this loop's `push` is silent for PERF-RS-STR-001 too.
pub fn literal_receiver(items: &[u8], other: &mut Vec<u8>) -> usize {
    let mut count = 0usize;
    for it in items {
        let s = "x".to_string(); // silent PERF-RS-ALLOC-001 (literal receiver)
        count += s.len() + usize::from(*it);
        other.push(*it); // silent PERF-RS-STR-001 (retired rule)
    }
    count
}
