//! 08_globals_and_chains.rs — globals, indirect calls, deep member chains.
//!
//! Mirrors `examples/08_globals_and_chains.c`.
//!
//! Expected findings:
//!   PERF-RS-LOOP-005  info  `static mut` written inside a loop
//!   PERF-RS-LOOP-006  info  indirect call `handlers[i]()` in the innermost loop
//!   PERF-RS-LOOP-008  info  `cfg.region.zone.depth` member chain of depth 3
//!
//! Silent controls (each must produce ZERO findings of the named rule):
//!   * `call_directly` calls a plain function (`Direct` callee), so LOOP-006
//!     must NOT fire.
//!   * `read_zone_ref` only forms a depth-2 chain, below the LOOP-008 floor,
//!     so LOOP-008 must NOT fire.

/// File-level global: an assignment to it inside a loop is `Global` scoped.
pub static mut GLOBAL_ACC: u64 = 0;

/// A simple direct-call target (same file, so no ARCH-003 edge either).
pub fn direct_handler() {}

pub struct Zone {
    pub depth: u64,
}

pub struct Region {
    pub zone: Zone,
}

pub struct Config {
    pub region: Region,
}

/// Global store plus an indirect call inside a loop (LOOP-005, LOOP-006).
pub fn run_handlers(handlers: &[fn()], n: usize) {
    let mut i = 0usize;
    while i < n {
        unsafe {
            GLOBAL_ACC = GLOBAL_ACC.wrapping_add(1); // LOOP-005
        }
        handlers[i](); // LOOP-006 (index-expression callee => Indirect)
        i += 1;
    }
}

/// Direct call inside a loop — the LOOP-006 silent control.
pub fn call_directly(n: usize) {
    for _ in 0..n {
        direct_handler(); // control: Direct callee -> no LOOP-006
    }
}

/// Deep chain in the innermost loop (LOOP-008).
pub fn read_depth(cfg: &Config, n: usize) -> u64 {
    let mut total = 0u64;
    for _ in 0..n {
        let d = cfg.region.zone.depth; // LOOP-008 (chain depth 3)
        total += d;
    }
    total
}

/// Depth-2 chain — the LOOP-008 silent control.
pub fn read_zone_ref(cfg: &Config, n: usize) -> u64 {
    let mut total = 0u64;
    for _ in 0..n {
        let zone = &cfg.region.zone; // control: chain depth 2 -> no LOOP-008
        total += zone.depth;
    }
    total
}
