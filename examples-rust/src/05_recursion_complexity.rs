//! 05_recursion_complexity.rs — direct recursion and high complexity.
//!
//! Mirrors `examples/05_recursion_complexity.c`.
//!
//! Expected findings:
//!   PERF-RS-CALL-001  info  fib() calls itself directly (two self-call sites)
//!   PERF-RS-CALL-002  warn  classify() branches >= 15 ways

/// Direct recursion (CALL-001).
pub fn fib(n: u64) -> u64 {
    if n < 2 {
        n
    } else {
        fib(n - 1) + fib(n - 2) // CALL-001 (two self-call sites)
    }
}

/// Match-heavy dispatch: 16 arms plus the function base gives a cyclomatic
/// complexity of 17 (>= the CALL-002 threshold of 15).
pub fn classify(code: u32) -> &'static str {
    match code {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        11 => "eleven",
        12 => "twelve",
        13 => "thirteen",
        14 => "fourteen",
        _ => "other",
    }
}
