//! Real-parse lock source for the macro-origin downgrade hook (plan
//! `rust-backend` R5 lane A, LL-009 pairing).
//!
//! `tick!` is a local `macro_rules!` wrapper, so the real backend mints its
//! call event with `macro_origin = true` (the callee-name branch of
//! `macro_origin_of`). The event sits inside a depth-3 loop nest, so the
//! `PERF-RS-LOOP-001` finding anchored at the depth-3 ancestor intersects a
//! macro-origin event and the CENTRAL downgrade hook must flag + cap it.

macro_rules! tick {
    () => {
        0
    };
}

pub fn deep(n: usize) -> usize {
    let mut acc = 0;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                acc += tick!() + i + j + k;
            }
        }
    }
    acc
}
