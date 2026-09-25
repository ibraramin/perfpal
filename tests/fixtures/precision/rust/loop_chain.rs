//! Precision corpus — Rust pointer/lifetime-chain chasing (PERF-RS-LOOP-008):
//! planted true positive. Silent controls live in `loop_silent.rs`.

pub struct Leaf {
    pub value: u64,
}

pub struct Row {
    pub cells: [Leaf; 4],
}

/// TP LOOP-008: an indexed chain (a subscript step) of depth 3 in an
/// innermost loop; the finding anchors at the chain.
pub fn indexed_chain(row: &Row, i: usize) -> u64 {
    let mut sum = 0u64;
    for k in 0..4 {
        sum += row.cells[i].value + k as u64; // TP PERF-RS-LOOP-008
    }
    sum
}
