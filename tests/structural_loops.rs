//! Structural suite v1 / metrics schema 1.9 locked fixtures: `loop_count`,
//! `loop_body_loc`, `max_loop_depth` consistency, and per-function
//! `parse_health` across C, Rust, Python, JavaScript, and TypeScript.
//!
//! The fixtures are minimal temp files driven through the REAL pipeline
//! (`cli::run_with`) so the aggregator-owned `loop_body_loc` (union of loop
//! body intervals) and `parse_health` (file-region overlap) are exercised, not
//! just the backend traversal.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

const C_LOOPS: &str = "\
int c_loops(int n) {
    int s = 0;
    for (int i = 0; i < n; i++) { s += i; }
    while (s < 10) { s++; }
    do { s--; } while (s > 0);
    for (int i = 0; i < 3; i++) {
        for (int j = 0; j < 3; j++) {
            s += i * j;
        }
    }
    for (int i = 0; i < 0; i++) { }
    for (int i = 0; i < 1; i++) s++;
#if 0
    for (;;) { s--; }
#endif
    return s;
}

int c_outer(int n) {
    int s = 0;
    for (int i = 0; i < n; i++) { s++; }
    int nested(int x) {
        for (int j = 0; j < x; j++) { s++; }
        return s;
    }
    return s;
}
";

const RUST_LOOPS: &str = "\
fn r_loops(mut n: i32) -> i32 {
    let mut s = 0;
    for i in 0..n { s += i; }
    while s < 10 { s += 1; }
    while let Some(x) = None::<i32> { s += x; }
    loop { s += 1; break; }
    for i in 0..2 {
        for j in 0..2 {
            s += i + j;
        }
    }
    let f = || { for k in 0..2 { s += k; } };
    f();
    s
}

fn r_outer() -> i32 {
    fn inner() -> i32 {
        for i in 0..3 { }
        0
    }
    for i in 0..2 { }
    inner()
}
";

const PY_LOOPS: &str = "\
def p_loops(items):
    total = 0
    for x in items:
        total += x
    while total > 0:
        total -= 1
    for i in range(3):
        for j in range(3):
            total += i * j
    return total

def p_comp(xs):
    return [x * 2 for x in xs]

def p_outer(xs):
    def inner(ys):
        for y in ys:
            print(y)
    for x in xs:
        print(x)
    inner(xs)
";

const JS_LOOPS: &str = "\
function jsLoops(arr) {
  let s = 0;
  for (let i = 0; i < arr.length; i++) { s += arr[i]; }
  for (const k in arr) { s += 1; }
  for (const v of arr) { s += v; }
  while (s < 10) { s++; }
  do { s--; } while (s > 0);
  for (let i = 0; i < 2; i++) {
    for (let j = 0; j < 2; j++) {
      s += i * j;
    }
  }
  function innerCb(x) {
    for (let k = 0; k < 1; k++) { s += x; }
  }
  innerCb(1);
  return s;
}
";

const TS_LOOPS: &str = "\
function tsLoops(xs: number[]): number {
  let s = 0;
  for (const x of xs) {
    s += x;
  }
  while (s > 0) s--;
  return s;
}
";

/// A function whose body contains an unclosed expression: the file is
/// `partial`, and the error region intersects the function span.
const PY_BROKEN: &str = "\
def broken():
    x = (
    return 1
";

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-structural-loops-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(output: &str, target: &str) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain([
            OsString::from("--output"),
            OsString::from(output),
            OsString::from(target),
        ])
        .collect()
}

fn build_fixture_tree() -> PathBuf {
    let root = temp_dir("fixtures");
    fs::write(root.join("c_loops.c"), C_LOOPS).expect("write C fixture");
    fs::write(root.join("r_loops.rs"), RUST_LOOPS).expect("write Rust fixture");
    fs::write(root.join("p_loops.py"), PY_LOOPS).expect("write Python fixture");
    fs::write(root.join("js_loops.js"), JS_LOOPS).expect("write JS fixture");
    fs::write(root.join("ts_loops.ts"), TS_LOOPS).expect("write TS fixture");
    fs::write(root.join("broken.py"), PY_BROKEN).expect("write broken fixture");
    root
}

fn function<'a>(doc: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    doc["stats"]["functions"]
        .as_array()
        .expect("stats.functions[]")
        .iter()
        .find(|f| f["name"] == name)
        .unwrap_or_else(|| panic!("function {name} missing from metrics.json"))
}

/// Assert the three loop fields plus per-function parse health exactly.
fn assert_loop_fields(
    doc: &serde_json::Value,
    name: &str,
    loop_count: u64,
    loop_body_loc: u64,
    max_loop_depth: u64,
) {
    let f = function(doc, name);
    assert_eq!(f["loop_count"], loop_count, "{name}.loop_count");
    assert_eq!(f["loop_body_loc"], loop_body_loc, "{name}.loop_body_loc");
    assert_eq!(f["max_loop_depth"], max_loop_depth, "{name}.max_loop_depth");
    assert_eq!(f["parse_health"]["status"], "ok", "{name}.parse_health");
    assert_eq!(
        f["parse_health"]["overlapping_error_regions"], 0,
        "{name}.overlapping_error_regions"
    );
    // Invariants (spec §5.2): count/depth agree, and body LOC cannot exceed
    // the function length or be zero when loops exist.
    assert_eq!(
        loop_count >= 1,
        max_loop_depth >= 1,
        "{name}: loop_count/max_loop_depth disagree"
    );
    assert!(loop_body_loc <= f["length_lines"].as_u64().unwrap());
    if loop_count >= 1 {
        assert!(loop_body_loc >= 1, "{name}: loops but no body LOC");
    }
}

#[test]
fn loop_fields_and_parse_health_across_languages() {
    let fixture = build_fixture_tree();
    let base = temp_dir("out");
    let out = base.join("report");
    let code = run_with(argv(
        out.to_string_lossy().as_ref(),
        fixture.to_string_lossy().as_ref(),
    ));
    assert_eq!(code, 0, "scan must succeed");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");
    assert_eq!(doc["schema_version"], "1.9");

    // C: for + while + do + nested (x2) + empty + single-line + dead `#if 0`
    // loop; bodies on lines 3,4,5,6-10,7-9,11,12,14 = 11 distinct lines.
    assert_loop_fields(&doc, "c_loops", 8, 11, 2);
    // C nested GCC function: its loop must NOT leak into the outer record.
    assert_loop_fields(&doc, "c_outer", 1, 1, 1);

    // Rust: for + while + while-let + loop + nested (x2) + closure loop;
    // bodies on lines 3,4,5,6,7-11,8-10,12 = 10 distinct lines.
    assert_loop_fields(&doc, "r_loops", 7, 10, 2);
    // Rust nested `fn` is its own scope (not emitted as a file-level record);
    // its loop must not leak into the outer record.
    assert_loop_fields(&doc, "r_outer", 1, 1, 1);

    // Python: for + while + nested (x2); bodies on lines 4,6,8-9,9 = 4 lines.
    assert_loop_fields(&doc, "p_loops", 4, 4, 2);
    // Comprehension body = element expression (`x * 2`), one line.
    assert_loop_fields(&doc, "p_comp", 1, 1, 1);
    // Nested `def` is its own scope (not emitted as a file-level record).
    assert_loop_fields(&doc, "p_outer", 1, 1, 1);

    // JS: for + for-in + for-of + while + do + nested (x2); callback loop
    // belongs to `innerCb`. Bodies on lines 3,4,5,6,7,8-12,9-11 = 10 lines.
    assert_loop_fields(&doc, "jsLoops", 7, 10, 2);
    assert_loop_fields(&doc, "innerCb", 1, 1, 1);

    // TypeScript: for-of block body (header shares line 3, brace spans 3-5)
    // plus the single-line while body on line 6; union = 4 lines.
    assert_loop_fields(&doc, "tsLoops", 2, 4, 1);

    // Broken file: partial parse, one error region intersecting `broken`.
    let broken = function(&doc, "broken");
    assert_eq!(broken["parse_health"]["status"], "partial");
    assert_eq!(
        broken["parse_health"]["overlapping_error_regions"], 1,
        "exactly one file error region intersects the function span"
    );
    assert_eq!(broken["loop_count"], 0);
    assert_eq!(broken["loop_body_loc"], 0);
    assert_eq!(broken["max_loop_depth"], 0);

    // The file-level summary still carries the partial status.
    let file_health = doc["files"]
        .as_array()
        .expect("files[]")
        .iter()
        .find(|f| f["path"] == "broken.py")
        .expect("broken.py record")
        .clone();
    assert_eq!(file_health["parse_health"]["status"], "partial");
}
