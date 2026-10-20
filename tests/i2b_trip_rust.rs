//! I2b Rust canonical trip-count extraction (plan `map-build`, founder-
//! approved extension of I2 to Rust).
//!
//! Locks the contract:
//! - ONLY a `for` over a range with BOTH bounds as non-negative integer
//!   literals, ascending, is KNOWN: `0..10` ⇒ 10, `5..15` ⇒ 10, `0..=10` ⇒ 11;
//! - radix prefixes (`0x`/`0o`/`0b`), `_` separators and type suffixes parse;
//! - variable/expression bounds, `step_by(2)`, `.rev()`, descending ranges,
//!   open-ended ranges, floats, macro ranges and `while`/`loop` stay `None`;
//! - saturating arithmetic clamped at [`MAX_TRIP`];
//! - real scan + aggregator: a literal-range function gets
//!   `function_cost_v2 > v1` heat;
//! - double CLI scan is byte-identical (provenance masked).

mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::cli::run_with;
use perfpal::ir::MAX_TRIP;
use perfpal::scanner::scan;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("perfpal-i2b-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-10-21T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

/// Scan one Rust fixture and map `function name` → its loops' trip counts.
fn scan_trip_counts(tag: &str, source: &str) -> BTreeMap<String, Vec<Option<u64>>> {
    let root = temp_dir(tag);
    fs::write(root.join("lib.rs"), source).expect("write Rust fixture");
    let units = scan(&root).expect("scan Rust fixture");
    fs::remove_dir_all(&root).ok();
    let mut out = BTreeMap::new();
    for f in &units[0].functions {
        out.insert(
            f.record.name.clone(),
            f.loops.iter().map(|l| l.trip_count).collect(),
        );
    }
    out
}

const FIXTURE: &str = r#"
pub fn half_open_zero() { for i in 0..10 { let _ = i; } }
pub fn half_open_nonzero() { for i in 5..15 { let _ = i; } }
pub fn inclusive() { for i in 0..=10 { let _ = i; } }
pub fn hex() { for i in 0x10..0x20 { let _ = i; } }
pub fn underscore_suffix() { for i in 1_000u32..2_000usize { let _ = i; } }
pub fn octal() { for i in 0o10..0o20 { let _ = i; } }
pub fn binary() { for i in 0b1010..0b1111 { let _ = i; } }
pub fn variable_bound(n: usize) { for i in 0..n { let _ = i; } }
pub fn expression_bound(parts: &[i32]) { for i in 0..parts.len() { let _ = i; } }
pub fn step_by() { for i in (0..10).step_by(2) { let _ = i; } }
pub fn rev() { for i in (0..10).rev() { let _ = i; } }
pub fn descending() { for i in 10..0 { let _ = i; } }
pub fn open_end() { for i in 0.. { if i > 3 { break; } } }
pub fn float_bound() { for i in 0..10.0 { let _ = i; } }
pub fn macro_range() { for i in range!(0..10) { let _ = i; } }
pub fn while_loop(n: usize) { let mut i = 0; while i < 10 { i += 1; let _ = n; } }
pub fn loop_kind() { loop { break; } }
pub fn clamp_open() { for i in 0..1_000_001 { let _ = i; } }
pub fn clamp_inclusive() { for i in 0..=1_000_000 { let _ = i; } }
pub fn clamp_huge() { for i in 0..18446744073709551615u64 { let _ = i; } }
"#;

#[test]
fn rust_literal_ranges_are_known_and_everything_else_is_honest() {
    let trips = scan_trip_counts("canonical", FIXTURE);

    // Canonical shapes.
    assert_eq!(trips["half_open_zero"], vec![Some(10)]);
    assert_eq!(trips["half_open_nonzero"], vec![Some(10)]);
    assert_eq!(trips["inclusive"], vec![Some(11)]);
    // Literal syntaxes.
    assert_eq!(trips["hex"], vec![Some(16)]);
    assert_eq!(trips["underscore_suffix"], vec![Some(1_000)]);
    assert_eq!(trips["octal"], vec![Some(8)]);
    assert_eq!(trips["binary"], vec![Some(5)]);

    // Honest unknown — never a guess.
    for name in [
        "variable_bound",
        "expression_bound",
        "step_by",
        "rev",
        "descending",
        "open_end",
        "float_bound",
        "macro_range",
        "while_loop",
        "loop_kind",
    ] {
        assert_eq!(
            trips[name],
            vec![None],
            "{name}: non-canonical shape must stay honestly unknown"
        );
    }

    // Saturating arithmetic + clamp.
    for name in ["clamp_open", "clamp_inclusive", "clamp_huge"] {
        assert_eq!(
            trips[name],
            vec![Some(MAX_TRIP)],
            "{name}: trip count must clamp at MAX_TRIP"
        );
    }
}

#[test]
fn literal_range_raises_v2_heat_over_v1() {
    let root = temp_dir("heat");
    fs::write(
        root.join("lib.rs"),
        "pub fn run() { let mut acc = 0; for i in 0..10 { acc += i; } let _ = acc; }\n",
    )
    .expect("write Rust fixture");
    let units = scan(&root).expect("scan Rust fixture");
    let doc = assemble(&units, &[], &prov());

    let f = doc
        .stats
        .functions
        .iter()
        .find(|f| f.name == "run")
        .expect("run function present");
    let v1 = u64::from(f.cyclomatic_complexity)
        + u64::from(f.cognitive_complexity)
        + 2 * u64::from(f.max_loop_depth)
        + 3 * u64::from(f.alloc_site_count)
        + u64::from(f.call_fanout.unwrap_or(0))
        + u64::from(f.call_fanin.unwrap_or(0))
        + u64::from(f.call_depth.unwrap_or(0))
        + if f.is_recursive.unwrap_or(false) {
            2
        } else {
            0
        }
        + if f.error_path.unwrap_or(false) { 1 } else { 0 }
        + if f.alloc_family.is_some() { 1 } else { 0 };
    let v2 = f.function_cost_v2.expect("v2 heat emitted");
    assert_eq!(f.unknown_trip_loops, 0, "literal range is a known trip");
    assert!(
        v2 > v1,
        "known trip 10 must raise v2 heat above v1 ({v2} <= {v1})"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn double_scan_is_byte_identical() {
    let root = temp_dir("determinism");
    fs::write(
        root.join("lib.rs"),
        "pub fn a() { for i in 0..7 { let _ = i; } }\npub fn b(n: usize) { for i in 0..n { let _ = i; } }\n",
    )
    .expect("write Rust fixture");

    let run = |tag: &str| {
        let out = temp_dir(tag).join("perfpal-report");
        let code = run_with(argv(&[
            "--output",
            out.to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
        ]));
        assert_eq!(code, 0, "pipeline must exit 0");
        fs::read_to_string(out.join("metrics.json")).expect("metrics.json present")
    };
    let first = run("scan1");
    let second = run("scan2");
    assert_eq!(
        common::mask_metrics_provenance(&first),
        common::mask_metrics_provenance(&second),
        "double-scan metrics must be byte-identical modulo provenance"
    );
    assert!(
        first.contains("\"function_cost_v2\""),
        "v2 heat must be emitted for the real scan"
    );
    fs::remove_dir_all(&root).ok();
}
