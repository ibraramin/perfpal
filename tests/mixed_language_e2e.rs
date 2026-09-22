//! Mixed C+Rust end-to-end lock (plan `rust-backend`, R3 workstream E).
//!
//! Runs the REAL pipeline (`cli::run_with` = scanner::scan → analyzer::analyze
//! → aggregator → writer/reporter) over a tempdir holding BOTH a `.c` and a
//! `.rs` file, and locks:
//! - `metrics.json` language derivation (`scan.languages`, `files[].language`);
//! - the analyzer registry's language-gating contract end-to-end: `PERF-C-*`
//!   ids never attach to Rust files and `PERF-RS-*` ids never attach to C
//!   files;
//! - `report.md` rendering (Appendix A catalogs the emitted `PERF-RS-*` ids)
//!   without the D1 uncataloged-rule panic;
//! - run-to-run byte determinism modulo provenance.
//!
//! The scanner → analyzer chain is additionally exercised directly through the
//! public `scanner::scan` / `analyzer::analyze` seam (the aggregation/writer
//! stages are only reachable through `run_with`, which is why the document
//! assertions go through it).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::analyze;
use perfpal::cli::run_with;
use perfpal::scanner::scan;

mod common;

/// C source with a couple of findings: `malloc`+`free` in loop (ALLOC-001 /
/// ALLOC-002) and `strcat` accumulation in loop (STR-001).
const C_SRC: &str = r#"#include <stdlib.h>
#include <string.h>

void grow(int n) {
    char *acc = malloc(8);
    int i;
    for (i = 0; i < n; i++) {
        char *tmp = malloc(16);
        strcat(acc, "x");
        free(tmp);
    }
    free(acc);
}
"#;

/// Rust source spanning several RS families: allocation in loop (ALLOC-001),
/// additive `reserve` growth in loop (ALLOC-003), `push_str` accumulation
/// (STR-001), `len` recomputation (STR-002), and `size_of` used as an element
/// count (MEM-001).
const RS_SRC: &str = r#"pub fn build(parts: &[&str], n: usize) -> String {
    let mut s = String::new();
    let mut out: Vec<u8> = Vec::new();
    for p in parts {
        let tmp: Vec<u8> = Vec::new();
        out.reserve(1);
        s.push_str(p);
        let _ = s.len();
        let _ = tmp.len();
    }
    let v: Vec<u8> = Vec::with_capacity(std::mem::size_of::<u64>());
    let _ = v;
    let _ = n;
    s
}
"#;

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-mixed-e2e-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

fn write_source_tree() -> PathBuf {
    let root = temp_root("src");
    fs::write(root.join("hot.c"), C_SRC).expect("write hot.c");
    fs::write(root.join("lib.rs"), RS_SRC).expect("write lib.rs");
    root
}

fn argv(args: &[&str]) -> Vec<std::ffi::OsString> {
    std::iter::once(std::ffi::OsString::from("perfpal"))
        .chain(args.iter().map(std::ffi::OsString::from))
        .collect()
}

/// Run the real pipeline over `target` into a fresh output dir; return the
/// output dir and the raw metrics.json text (so both the parsed document and
/// the byte-determinism check can use one run).
fn run_pipeline(tag: &str, target: &Path) -> (PathBuf, String) {
    let out = temp_root(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "mixed C+Rust pipeline must exit 0");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    (out, raw)
}

/// Slice `text` from the END of the `start` marker up to the `end` marker.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text.find(start).expect("section start marker") + start.len();
    let e = s + text[s..].find(end).expect("section end marker");
    &text[s..e]
}

#[test]
fn mixed_c_and_rust_tree_locks_language_gating_and_report() {
    let target = write_source_tree();
    let (out, raw1) = run_pipeline("mixed", &target);
    let doc: serde_json::Value = serde_json::from_str(&raw1).expect("metrics.json parses");

    // --- Language derivation ------------------------------------------------
    assert_eq!(
        doc["scan"]["languages"],
        serde_json::json!(["c", "rust"]),
        "mixed tree must derive the sorted-unique language set"
    );
    let files = doc["files"].as_array().expect("files[] present");
    let lang_of = |path: &str| -> String {
        files
            .iter()
            .find(|f| f["path"] == path)
            .unwrap_or_else(|| panic!("{path} missing from files[]: {files:?}"))["language"]
            .as_str()
            .expect("language token")
            .to_string()
    };
    assert_eq!(lang_of("hot.c"), "c");
    assert_eq!(lang_of("lib.rs"), "rust");

    // --- Findings: per-language presence AND the gating contract ------------
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(!findings.is_empty(), "mixed tree must emit findings");

    let c_on_c: Vec<&str> = findings
        .iter()
        .filter(|f| f["path"] == "hot.c")
        .filter_map(|f| f["rule_id"].as_str())
        .filter(|r| r.starts_with("PERF-C-"))
        .collect();
    assert!(
        !c_on_c.is_empty(),
        "expected at least one PERF-C-* finding on hot.c, got {findings:?}"
    );

    let rs_on_rust: Vec<&str> = findings
        .iter()
        .filter(|f| f["path"] == "lib.rs")
        .filter_map(|f| f["rule_id"].as_str())
        .filter(|r| r.starts_with("PERF-RS-"))
        .collect();
    assert!(
        !rs_on_rust.is_empty(),
        "expected at least one PERF-RS-* finding on lib.rs, got {findings:?}"
    );

    // The registry is language-tagged: no cross-language leakage, per file.
    for f in findings {
        let path = f["path"].as_str().expect("finding path");
        let rule = f["rule_id"].as_str().expect("finding rule_id");
        if path.ends_with(".rs") {
            assert!(
                rule.starts_with("PERF-RS-"),
                "PERF-C-* rule {rule} leaked onto Rust file {path}"
            );
        } else if path.ends_with(".c") || path.ends_with(".h") {
            assert!(
                rule.starts_with("PERF-C-"),
                "PERF-RS-* rule {rule} leaked onto C file {path}"
            );
        } else {
            panic!("unexpected finding path extension: {path}");
        }
    }

    // Several Rust families must fire (alloc / str / mem at minimum).
    let rs_families: BTreeSet<&str> = rs_on_rust
        .iter()
        .filter_map(|r| r.strip_prefix("PERF-RS-"))
        .collect();
    assert!(
        rs_families.len() >= 3,
        "expected ≥3 distinct PERF-RS-* rule ids, got {rs_families:?}"
    );

    // --- Direct scanner → analyzer seam (gating at the registry level) ------
    let units = scan(&target).expect("scan must succeed");
    let langs: Vec<&str> = units.iter().map(|u| u.file.language.as_str()).collect();
    assert_eq!(langs, ["c", "rust"], "scan records per-file language");
    let direct = analyze(&units);
    for f in &direct {
        if f.path.ends_with(".rs") {
            assert!(f.rule_id.starts_with("PERF-RS-"), "{}", f.rule_id);
        } else {
            assert!(f.rule_id.starts_with("PERF-C-"), "{}", f.rule_id);
        }
    }

    // --- report.md renders and Appendix A catalogs every emitted RS id ------
    let report = fs::read_to_string(out.join("report.md")).expect("report.md present");
    assert!(
        report.contains("## 4. Findings"),
        "report.md must render §4 without the uncataloged-rule panic"
    );
    // Rule ids are Markdown-escape'd (`-` → `\-`); compare de-escaped.
    let appendix_a = section(&report, "## Appendix A.", "## Appendix B.").replace('\\', "");
    for rule in &rs_on_rust {
        assert!(
            appendix_a.contains(rule),
            "Appendix A must catalog emitted Rust rule {rule}"
        );
    }

    // --- Run-to-run byte determinism modulo provenance ----------------------
    let (out2, raw2) = run_pipeline("mixed-2", &target);
    assert_eq!(
        common::mask_metrics_provenance(&raw1),
        common::mask_metrics_provenance(&raw2),
        "two runs over identical input diverged beyond provenance"
    );
    let report2 = fs::read_to_string(out2.join("report.md")).expect("second report.md");
    assert_eq!(
        common::mask_report_provenance(&report),
        common::mask_report_provenance(&report2),
        "report.md diverged beyond provenance between runs"
    );

    fs::remove_dir_all(&target).ok();
}
