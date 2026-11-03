//! Structural suite v1 / schema 1.9 Stage 2 locked fixtures: allocation-in-
//! loop, external I/O, `resource_sites[]`, and per-site `cfg_gated` across C,
//! Rust, Python, JavaScript, and TypeScript.
//!
//! Fixtures are minimal temp files driven through the REAL pipeline
//! (`cli::run_with`), so the aggregator-owned classifier and the C dead-region
//! gate are exercised end to end.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

const C_SITES: &str = "\
#include <stdio.h>
#include <stdlib.h>

int c_sites(int n) {
    char *p = NULL;
    for (int i = 0; i < n; i++) {
        for (int j = 0; j < n; j++) {
            p = malloc(16);
            printf(\"x\");
        }
    }
    return 0;
}

int c_dead(int n) {
#if 0
    for (int i = 0; i < n; i++) {
        malloc(8);
    }
#endif
    return 0;
}

int c_live_io(int n) {
    for (int i = 0; i < n; i++) {
        fprintf(stderr, \"x\");
    }
    return 0;
}

#if 0
int c_whole(int n) {
    for (int i = 0; i < n; i++) {
        malloc(1);
    }
    return 0;
}
#endif
";

const RUST_SITES: &str = "\
fn r_sites(n: usize, mut out: String) -> String {
    for i in 0..n {
        let v = Vec::new();
        println!(\"{i}\");
        let f = || { let b = Box::new(1); b };
        f();
        out.push_str(\"x\");
    }
    out
}

fn r_io(n: usize) {
    for _ in 0..n {
        std::fs::write(\"a\", \"b\").ok();
        let _ = write!(String::new(), \"x\");
    }
}
";

const PY_SITES: &str = "\
def py_sites(items, path):
    for x in items:
        f = open(path)
        print(x)
        data = f.read()
        obj.read()
    return data

def py_nested(items):
    for x in items:
        for y in items:
            open(\"f\")
";

const JS_SITES: &str = "\
function jsSites(stream, obj) {
  for (let i = 0; i < 3; i++) {
    fs.readFileSync(\"a\");
    console.log(i);
  }
  for (const x of [1, 2]) {
    stream.write(x);
    obj.write(x);
  }
  return fetch(\"http://example\");
}
";

const TS_SITES: &str = "\
async function tsSites(xs: string[]) {
  for (const x of xs) {
    await fetch(x);
    console.log(x);
  }
}
";

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-structural-sites-{tag}-{}-{nanos}",
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
    fs::write(root.join("c_sites.c"), C_SITES).expect("write C fixture");
    fs::write(root.join("rust_sites.rs"), RUST_SITES).expect("write Rust fixture");
    fs::write(root.join("py_sites.py"), PY_SITES).expect("write Python fixture");
    fs::write(root.join("js_sites.js"), JS_SITES).expect("write JS fixture");
    fs::write(root.join("ts_sites.ts"), TS_SITES).expect("write TS fixture");
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

fn site(
    kind: &str,
    line: u32,
    column: u32,
    loop_depth: u32,
    cfg_gated: Option<bool>,
    lang: &str,
) -> serde_json::Value {
    serde_json::json!({
        "kind": kind,
        "line": line,
        "column": column,
        "loop_depth": loop_depth,
        "cfg_gated": cfg_gated,
        "language": lang,
    })
}

/// Assert exact counts + the full `resource_sites[]` array.
fn assert_function(
    doc: &serde_json::Value,
    name: &str,
    alloc_in_loops: Option<u64>,
    io_count: u64,
    io_in_loops: u64,
    sites: Vec<serde_json::Value>,
) {
    let f = function(doc, name);
    assert_eq!(
        f.get("alloc_sites_in_loops").and_then(|v| v.as_u64()),
        alloc_in_loops,
        "{name}.alloc_sites_in_loops"
    );
    assert_eq!(f["io_site_count"], io_count, "{name}.io_site_count");
    assert_eq!(
        f["io_sites_in_loops"], io_in_loops,
        "{name}.io_sites_in_loops"
    );
    assert_eq!(
        f["resource_sites"],
        serde_json::Value::Array(sites),
        "{name}.resource_sites"
    );

    // Invariants.
    if let Some(alloc) = alloc_in_loops {
        assert!(
            alloc <= f["alloc_site_count"].as_u64().unwrap(),
            "{name}: alloc_sites_in_loops exceeds alloc_site_count"
        );
    }
    assert!(io_in_loops <= io_count, "{name}: io-in-loop > io count");
    let span = &f["span"];
    let (start_line, end_line) = (
        span["start_line"].as_u64().unwrap(),
        span["end_line"].as_u64().unwrap(),
    );
    for s in f["resource_sites"].as_array().unwrap() {
        let line = s["line"].as_u64().unwrap();
        assert!(s["loop_depth"].as_u64().unwrap() >= 1);
        assert!(
            line >= start_line && line <= end_line,
            "{name}: site out of span"
        );
    }
}

#[test]
fn resource_sites_and_cfg_gating_across_languages() {
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

    // C: alloc + I/O in a doubly nested loop; nested `#if 0` alloc; live I/O;
    // and a function wholly inside `#if 0` (function-level cfg_gated true).
    assert_function(
        &doc,
        "c_sites",
        Some(1),
        1,
        1,
        vec![
            site("alloc", 8, 17, 2, Some(false), "c"),
            site("io", 9, 13, 2, Some(false), "c"),
        ],
    );
    assert_function(
        &doc,
        "c_dead",
        Some(1),
        0,
        0,
        vec![site("alloc", 18, 9, 1, Some(true), "c")],
    );
    assert_function(
        &doc,
        "c_live_io",
        Some(0),
        1,
        1,
        vec![site("io", 26, 9, 1, Some(false), "c")],
    );
    assert_function(
        &doc,
        "c_whole",
        Some(1),
        0,
        0,
        vec![site("alloc", 34, 9, 1, Some(true), "c")],
    );
    let whole = function(&doc, "c_whole");
    assert_eq!(whole["cfg_gated"], true, "function wholly in #if 0");
    for s in whole["resource_sites"].as_array().unwrap() {
        assert_eq!(s["cfg_gated"], true, "C function gate implies every site");
    }

    // Rust: alloc in loop + closure alloc + stdio macro; file I/O + write!
    // macro. Rust sites are null, never false.
    assert_function(
        &doc,
        "r_sites",
        Some(2),
        1,
        1,
        vec![
            site("alloc", 3, 17, 1, None, "rust"),
            site("io", 4, 9, 1, None, "rust"),
            site("alloc", 5, 30, 1, None, "rust"),
        ],
    );
    assert_function(
        &doc,
        "r_io",
        Some(0),
        2,
        2,
        vec![
            site("io", 14, 9, 1, None, "rust"),
            site("io", 15, 17, 1, None, "rust"),
        ],
    );

    // Python: bare open/print + documented file receiver `f.read`; an
    // undocumented receiver (`obj.read`) is a documented false negative.
    assert_function(
        &doc,
        "py_sites",
        None,
        3,
        3,
        vec![
            site("io", 3, 13, 1, None, "python"),
            site("io", 4, 9, 1, None, "python"),
            site("io", 5, 16, 1, None, "python"),
        ],
    );
    assert_function(
        &doc,
        "py_nested",
        None,
        1,
        1,
        vec![site("io", 12, 13, 2, None, "python")],
    );

    // JavaScript: fs + console + documented stream receiver; `obj.write` is
    // not classified; `fetch` outside the loop still counts toward
    // io_site_count but not resource_sites.
    assert_function(
        &doc,
        "jsSites",
        None,
        4,
        3,
        vec![
            site("io", 3, 5, 1, None, "javascript"),
            site("io", 4, 5, 1, None, "javascript"),
            site("io", 7, 5, 1, None, "javascript"),
        ],
    );

    // TypeScript inherits the JS table; the language token is `typescript`.
    assert_function(
        &doc,
        "tsSites",
        None,
        2,
        2,
        vec![
            site("io", 3, 11, 1, None, "typescript"),
            site("io", 4, 5, 1, None, "typescript"),
        ],
    );

    // Site arrays are globally sorted by (line, column, kind, language) and
    // have no duplicate full sort key.
    for f in doc["stats"]["functions"].as_array().unwrap() {
        let Some(sites) = f.get("resource_sites").and_then(|v| v.as_array()) else {
            continue;
        };
        let mut keys: Vec<(u64, u64, &str, &str)> = sites
            .iter()
            .map(|s| {
                (
                    s["line"].as_u64().unwrap(),
                    s["column"].as_u64().unwrap(),
                    s["kind"].as_str().unwrap(),
                    s["language"].as_str().unwrap(),
                )
            })
            .collect();
        let original = keys.clone();
        keys.sort();
        assert_eq!(original, keys, "resource_sites order drifted");
        let mut dedup = keys.clone();
        dedup.dedup();
        assert_eq!(dedup, keys, "duplicate resource-site sort key");
    }
}

/// Stage 2 size audit over the committed goldens: the `resource_sites[]`
/// addition must not more than double the largest `metrics.json`. The audit
/// never caps/truncates — if the bound fails, the panic carries the actual
/// numbers so the study can decide on an opt-in flag instead.
#[test]
fn resource_site_size_audit_does_not_double_largest_golden() {
    let roots = [
        "tests/golden/report/metrics.json",
        "tests/golden/report-mixed/metrics.json",
        "tests/golden/report-rust/metrics.json",
        "tests/golden/report-python/metrics.json",
        "tests/golden/report-js/metrics.json",
        "tests/golden/report-ts/metrics.json",
    ];
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let (path, raw) = roots
        .iter()
        .map(|rel| {
            let path = manifest.join(rel);
            let raw = fs::read_to_string(&path).expect("read golden metrics");
            (path, raw)
        })
        .max_by_key(|(_, raw)| raw.len())
        .expect("at least one golden");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("golden parses");
    let mut without = doc.clone();
    let mut site_count = 0usize;
    for f in without["stats"]["functions"]
        .as_array_mut()
        .expect("functions[]")
    {
        if let Some(sites) = f.get("resource_sites").and_then(|v| v.as_array()) {
            site_count += sites.len();
        }
        f.as_object_mut()
            .expect("function object")
            .remove("resource_sites");
    }
    let full_bytes = serde_json::to_vec(&doc).expect("serialize full").len();
    let without_bytes = serde_json::to_vec(&without)
        .expect("serialize without")
        .len();
    let ratio = full_bytes as f64 / without_bytes as f64;
    eprintln!(
        "size-audit: largest golden {} raw={} B, compact full={} B, without resource_sites={} B, \
         ratio={ratio:.3}, sites={}",
        path.display(),
        raw.len(),
        full_bytes,
        without_bytes,
        site_count
    );
    assert!(
        full_bytes <= without_bytes.saturating_mul(2),
        "resource_sites more than doubles the largest golden: full={full_bytes} B, \
         without={without_bytes} B, ratio={ratio:.3}, sites={site_count} (no silent cap)"
    );
}
