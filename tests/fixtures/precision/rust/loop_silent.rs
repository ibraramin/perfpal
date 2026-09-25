//! Precision corpus — Rust pointer/lifetime-chain silent control.

pub struct Leaf {
    pub value: u64,
}

pub struct Mid {
    pub leaf: Leaf,
}

pub struct Top {
    pub mid: Mid,
}

/// SILENT LOOP-008: a pure member chain of depth 3 with NO subscript step —
/// the M1 gate requires an index step.
pub fn member_chain_only(top: &Top) -> u64 {
    let mut sum = 0u64;
    for k in 0..4 {
        sum += top.mid.leaf.value + k as u64; // silent PERF-RS-LOOP-008 (no subscript)
    }
    sum
}
