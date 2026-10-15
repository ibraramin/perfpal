//! IR v1.4 STEP 2 end-to-end lock (plan `divergence-revamp`): slices A→D
//! through the REAL pipeline over a purpose-built trilingual tree — call-shape
//! values, alloc-family tags, error-path flags, idiom hashes, the wired
//! `charts/graph.dot` artifact — plus the double-scan byte-identical proof.
//!
//! This is the instrument-population counterpart to the `tests/schema_1_4.rs`
//! contract locks: values asserted here, shapes locked there.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

const C_SRC: &str = r#"#include <stdlib.h>

void helper(int n) {
    int i;
    for (i = 0; i < n; i++) {
        char *buf = malloc(16);
        free(buf);
    }
}

int fact(int n) {
    if (n <= 1) {
        return 1;
    }
    return n * fact(n - 1);
}

void driver(int n) {
    do {
        helper(n);
    } while (0);
    helper(n);
}
"#;

const RS_SRC: &str = r#"pub fn build(parts: &[&str]) -> String {
    let mut s = String::new();
    for p in parts {
        s.push_str(p);
    }
    s
}

pub fn load(path: &str) -> Result<String, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    Ok(text)
}
"#;

const PY_SRC: &str = r#"def fetch_items(n):
    rows = []
    try:
        for i in range(n):
            rows.append(i)
    except ValueError:
        rows = []
    return rows


def pack(items):
    return list(items)
"#;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("perfpal-v14-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

fn fixture_tree(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    fs::write(root.join("a.c"), C_SRC).expect("write a.c");
    fs::write(root.join("b.rs"), RS_SRC).expect("write b.rs");
    fs::write(root.join("c.py"), PY_SRC).expect("write c.py");
    root
}

fn run_scan(tag: &str, fixture: &std::path::Path) -> (PathBuf, serde_json::Value) {
    let out = temp_dir(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline must exit 0");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    (
        out,
        serde_json::from_str(&raw).expect("metrics.json parses"),
    )
}

fn func<'a>(doc: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    doc["stats"]["functions"]
        .as_array()
        .expect("functions[]")
        .iter()
        .find(|f| f["name"] == name)
        .unwrap_or_else(|| panic!("function {name} present"))
}

fn is_hex16(v: &serde_json::Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 16
            && s.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

// ---------------------------------------------------------------------------
// Slice A — call-shape values
// ---------------------------------------------------------------------------

#[test]
fn call_shape_values_on_trilingual_tree() {
    let fixture = fixture_tree("shape");
    let (_out, doc) = run_scan("shape", &fixture);

    // driver → helper (distinct count 1 despite two call sites); nothing
    // calls driver.
    let driver = func(&doc, "driver");
    assert_eq!(driver["call_fanout"], 1);
    assert_eq!(driver["call_fanin"], 0);
    assert_eq!(driver["is_recursive"], false);
    // helper ← driver only; sits one layer below its caller.
    let helper = func(&doc, "helper");
    assert_eq!(helper["call_fanout"], 0);
    assert_eq!(helper["call_fanin"], 1);
    assert_eq!(helper["call_depth"], 1);
    assert_eq!(driver["call_depth"], 0);
    // fact is self-recursive.
    let fact = func(&doc, "fact");
    assert_eq!(fact["is_recursive"], true);
    assert_eq!(fact["call_fanout"], 1);
    assert_eq!(fact["call_fanin"], 1);
    // Cross-language isolation: no C/Rust/Python edge ever crosses the
    // language boundary (every fanout here resolves within its own file).
    for name in ["build", "load", "fetch_items", "pack"] {
        let f = func(&doc, name);
        assert_eq!(f["call_fanout"], 0, "{name} must have no callees");
    }

    fs::remove_dir_all(&fixture).ok();
}

// ---------------------------------------------------------------------------
// Slice B — alloc_family + error_path values
// ---------------------------------------------------------------------------

#[test]
fn alloc_family_and_error_path_values() {
    let fixture = fixture_tree("families");
    let (_out, doc) = run_scan("families", &fixture);

    // C: malloc site ⇒ real_alloc; free without error vocabulary ⇒ no flag.
    assert_eq!(func(&doc, "helper")["alloc_family"], "real_alloc");
    assert_eq!(func(&doc, "helper")["error_path"], false);
    // C: literal do-while(0) body ⇒ error path; no alloc sites ⇒ omitted.
    assert_eq!(func(&doc, "driver")["error_path"], true);
    assert!(
        func(&doc, "driver").get("alloc_family").is_none(),
        "driver has no alloc sites"
    );
    // Rust: empty ctors only ⇒ empty_ctor; `?` ⇒ error path.
    assert_eq!(func(&doc, "build")["alloc_family"], "empty_ctor");
    assert_eq!(func(&doc, "build")["error_path"], false);
    assert_eq!(func(&doc, "load")["error_path"], true);
    assert!(
        func(&doc, "load").get("alloc_family").is_none(),
        "read_to_string is not an alloc-family site"
    );
    // Python: try/except ⇒ error path; list(items) ⇒ real_alloc.
    assert_eq!(func(&doc, "fetch_items")["error_path"], true);
    assert_eq!(func(&doc, "pack")["alloc_family"], "real_alloc");
    assert_eq!(func(&doc, "pack")["error_path"], false);

    fs::remove_dir_all(&fixture).ok();
}

// ---------------------------------------------------------------------------
// Slice C — idiom_hash presence, shape, and comment-stability
// ---------------------------------------------------------------------------

#[test]
fn idiom_hash_present_and_stable_shape() {
    let fixture = fixture_tree("idiom");
    let (_out, doc) = run_scan("idiom", &fixture);

    let functions = doc["stats"]["functions"].as_array().expect("functions[]");
    assert!(!functions.is_empty());
    for f in functions {
        assert!(
            is_hex16(&f["idiom_hash"]),
            "idiom_hash must be 16 lowercase hex: {f:?}"
        );
    }
    // Distinct bodies ⇒ distinct hashes (all six functions differ).
    let hashes: std::collections::BTreeSet<&str> = functions
        .iter()
        .map(|f| f["idiom_hash"].as_str().expect("hash"))
        .collect();
    assert_eq!(hashes.len(), functions.len(), "hashes must discriminate");

    fs::remove_dir_all(&fixture).ok();
}

// ---------------------------------------------------------------------------
// Slice D — graph.dot wired as a CLI artifact + double-scan proof
// ---------------------------------------------------------------------------

#[test]
fn graph_dot_wired_and_double_scan_byte_identical() {
    let fixture = fixture_tree("dot");
    let (out1, _) = run_scan("dot-1", &fixture);
    let (out2, _) = run_scan("dot-2", &fixture);

    for name in ["metrics.json", "report.md"] {
        let a = fs::read_to_string(out1.join(name)).expect(name);
        let b = fs::read_to_string(out2.join(name)).expect(name);
        let mask = if name == "metrics.json" {
            // Mask the two provenance fields; root_path is verbatim-equal
            // here (same fixture path both runs).
            |raw: &str| {
                let mut value: serde_json::Value =
                    serde_json::from_str(raw).expect("metrics parses");
                value["scan"]["started_at"] = serde_json::Value::Null;
                value["scan"]["duration_ms"] = serde_json::Value::Null;
                value.to_string()
            }
        } else {
            |raw: &str| {
                raw.lines()
                    .map(|line| {
                        let mut masked = line.to_string();
                        if let Some(i) = masked.find("started ") {
                            let from = i + "started ".len();
                            if let Some(z) = masked[from..].find('Z') {
                                masked.replace_range(from..=from + z, "<MASKED>");
                            }
                        }
                        if let Some(i) = masked.find("duration ") {
                            let from = i + "duration ".len();
                            let digits = masked[from..]
                                .chars()
                                .take_while(char::is_ascii_digit)
                                .count();
                            if digits > 0 {
                                masked.replace_range(from..from + digits, "<MASKED>");
                            }
                        }
                        masked
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };
        assert_eq!(mask(&a), mask(&b), "{name} diverged between scans");
    }

    // charts/ + graph.dot: byte-identical UNMASKED (no provenance inside).
    let charts1: Vec<String> = fs::read_dir(out1.join("charts"))
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    assert!(
        charts1.contains(&"graph.dot".to_string()),
        "graph.dot must ship in charts/: {charts1:?}"
    );
    for name in &charts1 {
        let a = fs::read(out1.join("charts").join(name)).unwrap();
        let b = fs::read(out2.join("charts").join(name)).unwrap();
        assert_eq!(a, b, "charts/{name} not byte-stable across scans");
    }

    // DOT content sanity: file nodes + component node present (the call
    // section is empty here — call_graph carries cross-file edges only and
    // this tree has one file per language; edge rendering is locked by the
    // dot.rs unit tests).
    let dot = fs::read_to_string(out1.join("charts/graph.dot")).expect("graph.dot");
    assert!(dot.starts_with("digraph perfpal {"));
    assert!(dot.ends_with("}\n"));
    for node in ["\"a.c\";", "\"b.rs\";", "\"c.py\";"] {
        assert!(dot.contains(node), "file node {node} missing:\n{dot}");
    }
    assert!(dot.contains("component:."), "component node missing");

    // report.md §5 references the DOT export and the cost formula.
    let report = fs::read_to_string(out1.join("report.md")).expect("report.md");
    assert!(
        report.contains("charts/graph.dot"),
        "§5 must reference graph.dot"
    );
    assert!(
        report.contains("pure functions of metrics.json"),
        "§5 must document the cost overlay"
    );

    fs::remove_dir_all(&fixture).ok();
}
