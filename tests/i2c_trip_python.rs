//! I2c Python canonical trip-count extraction (plan `map-build`; completes
//! I2 for C/Rust/Python).
//!
//! Locks the contract:
//! - ONLY a `for`/comprehension `for_in_clause` over a canonical `range(...)`
//!   call with integer-literal args is KNOWN: `range(10)` ⇒ 10,
//!   `range(0, 10)` ⇒ 10, `range(5, 15)` ⇒ 10, `range(-5, 5)` ⇒ 10,
//!   `range(0, 10, 1)` ⇒ 10;
//! - literal steps other than 1, variable/computed bounds, floats,
//!   non-`range` iterables and empty/descending ranges stay `None`;
//! - radix prefixes (`0x`/`0o`/`0b`) and `_` separators parse;
//! - saturating arithmetic clamped at [`MAX_TRIP`];
//! - real scan + aggregator: a literal-range function gets
//!   `function_cost_v2 > v1` heat with `unknown_trip_loops == 0`;
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
        std::env::temp_dir().join(format!("perfpal-i2c-{tag}-{}-{nanos}", std::process::id()));
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

/// Scan one Python fixture and map `function name` → its loops' trip counts.
fn scan_trip_counts(tag: &str, source: &str) -> BTreeMap<String, Vec<Option<u64>>> {
    let root = temp_dir(tag);
    fs::write(root.join("app.py"), source).expect("write Python fixture");
    let units = scan(&root).expect("scan Python fixture");
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
def single_arg():
    for i in range(10):
        print(i)

def two_args():
    for i in range(0, 10):
        print(i)

def nonzero_start():
    for i in range(5, 15):
        print(i)

def negative_start():
    for i in range(-5, 5):
        print(i)

def explicit_unit_step():
    for i in range(0, 10, 1):
        print(i)

def literal_step_two():
    for i in range(0, 10, 2):
        print(i)

def negative_step():
    for i in range(10, 0, -1):
        print(i)

def variable_bound(n):
    for i in range(n):
        print(i)

def computed_bound(items):
    for i in range(len(items)):
        print(i)

def comprehension():
    return [x for x in range(10)]

def hex_literal():
    for i in range(0x10):
        print(i)

def underscore_literal():
    for i in range(1_000):
        print(i)

def octal_literal():
    for i in range(0o10):
        print(i)

def binary_literal():
    for i in range(0b1010):
        print(i)

def float_bound():
    for i in range(10.0):
        print(i)

def empty_range():
    for i in range(0):
        print(i)

def descending():
    for i in range(10, 0):
        print(i)

def no_args():
    for i in range():
        print(i)

def too_many_args():
    for i in range(1, 2, 3, 4):
        print(i)

def non_range_iterable(items):
    for item in items:
        print(item)

def attribute_callee():
    for i in builtins.range(10):
        print(i)

def clamp_single():
    for i in range(99999999999999999999999999):
        print(i)

def clamp_two_arg():
    for i in range(0, 99999999999999999999999999):
        print(i)

def clamp_sign_span():
    # i128-span edge: end saturates at i128::MAX while start is negative.
    for i in range(-1, 170141183460469231731687303715884105727):
        print(i)

def keyword_args():
    for i in range(stop=10):
        print(i)

def splat_args():
    for i in range(*args):
        print(i)
"#;

#[test]
fn python_literal_ranges_are_known_and_everything_else_is_honest() {
    let trips = scan_trip_counts("canonical", FIXTURE);

    // Canonical shapes.
    assert_eq!(trips["single_arg"], vec![Some(10)]);
    assert_eq!(trips["two_args"], vec![Some(10)]);
    assert_eq!(trips["nonzero_start"], vec![Some(10)]);
    assert_eq!(trips["negative_start"], vec![Some(10)]);
    assert_eq!(trips["explicit_unit_step"], vec![Some(10)]);
    assert_eq!(trips["comprehension"], vec![Some(10)]);

    // Literal syntaxes.
    assert_eq!(trips["hex_literal"], vec![Some(16)]);
    assert_eq!(trips["underscore_literal"], vec![Some(1_000)]);
    assert_eq!(trips["octal_literal"], vec![Some(8)]);
    assert_eq!(trips["binary_literal"], vec![Some(10)]);

    // Honest unknown — never a guess.
    for name in [
        "literal_step_two",
        "negative_step",
        "variable_bound",
        "computed_bound",
        "float_bound",
        "empty_range",
        "descending",
        "no_args",
        "too_many_args",
        "non_range_iterable",
        "attribute_callee",
        "keyword_args",
        "splat_args",
    ] {
        assert_eq!(
            trips[name],
            vec![None],
            "{name}: non-canonical shape must stay honestly unknown"
        );
    }

    // Saturating arithmetic + clamp.
    for name in ["clamp_single", "clamp_two_arg", "clamp_sign_span"] {
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
        root.join("app.py"),
        "def run():\n    total = 0\n    for i in range(10):\n        total += i\n    return total\n",
    )
    .expect("write Python fixture");
    let units = scan(&root).expect("scan Python fixture");
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
        root.join("app.py"),
        "def a():\n    for i in range(7):\n        print(i)\n\ndef b(n):\n    for i in range(n):\n        print(i)\n",
    )
    .expect("write Python fixture");

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
