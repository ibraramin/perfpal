//! Mixed C+Rust end-to-end lock (plan `rust-backend`, R5 lane C1).
//!
//! Runs the REAL pipeline (`cli::run_with` = scanner::scan → analyzer::analyze
//! → aggregator → writer/reporter) over a tempdir copy of the COMMITTED mixed
//! corpus `tests/fixtures/e2e-mixed/` (`.c` + `.h` + `.rs`), and locks:
//! - `metrics.json` language derivation (`scan.languages`, `files[].language`);
//! - the analyzer registry's language-gating contract end-to-end: `PERF-C-*`
//!   ids never attach to Rust files and `PERF-RS-*` ids never attach to C
//!   files;
//! - the EXACT `PERF-C-*` / `PERF-RS-*` rule-id sets the corpus fires;
//! - same-language cross-file ARCH-003 coupling (src/main.c → src/util.c)
//!   fires, AND the deliberate cross-language gate holds: the corpus contains
//!   BOTH directions of a C↔Rust call, and neither emits a call-graph edge or
//!   an ARCH-003 finding, because `cross_file_call_edges` keys its definition
//!   map on `(language, name)` (`src/analyzer/heuristics/arch.rs`, R3 C1);
//! - `report.md` rendering (Appendix A catalogs every emitted `PERF-RS-*` id)
//!   without the D1 uncataloged-rule panic;
//! - run-to-run byte determinism modulo provenance.
//!
//! The scanner → analyzer chain is additionally exercised directly through the
//! public `scanner::scan` / `analyzer::analyze` seam (the aggregation/writer
//! stages are only reachable through `run_with`, which is why the document
//! assertions go through it).
//!
//! The committed corpus is copied into a tempdir before scanning so the report
//! output directory never sits inside the scanned tree, and the committed
//! fixture is never mutated or deleted.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::analyze;
use perfpal::analyzer::finding::{
    ALLOC_001_RULE_ID, ALLOC_002_RULE_ID, ARCH_003_RULE_ID, RS_ALLOC_001_RULE_ID,
    RS_ALLOC_003_RULE_ID, RS_ARCH_003_RULE_ID, RS_MEM_001_RULE_ID, RS_STR_002_RULE_ID,
    STR_001_RULE_ID, STR_003_RULE_ID,
};
use perfpal::cli::run_with;
use perfpal::scanner::scan;

mod common;

/// The committed mixed-language e2e corpus (mirrors `tests/fixtures/e2e/`).
fn committed_corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e-mixed")
}

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

/// Recursively copy `from` into `to` (small committed fixture: files + `src/`).
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy dir");
    for entry in fs::read_dir(from).expect("read committed corpus").flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).expect("copy corpus file");
        }
    }
}

/// Materialize the committed corpus into a fresh tempdir (output + cleanup
/// stay entirely outside the committed tree).
fn materialize_corpus() -> PathBuf {
    let root = temp_root("src");
    copy_tree(&committed_corpus(), &root);
    root
}

fn is_c_path(path: &str) -> bool {
    path.ends_with(".c") || path.ends_with(".h")
}

fn is_rs_path(path: &str) -> bool {
    path.ends_with(".rs")
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
    let target = materialize_corpus();
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
    assert_eq!(lang_of("src/main.c"), "c");
    assert_eq!(lang_of("src/util.c"), "c");
    assert_eq!(lang_of("src/util.h"), "c");
    assert_eq!(lang_of("src/lib.rs"), "rust");

    // --- Findings: per-language presence AND the gating contract ------------
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(!findings.is_empty(), "mixed tree must emit findings");

    // The corpus is designed to fire these EXACT rule ids (see the fixture
    // headers for the site-to-rule mapping). Compared as full ids rather than
    // a `≥3` family count, so corpus/backend drift fails the lock.
    let c_rules: BTreeSet<&str> = findings
        .iter()
        .filter_map(|f| {
            let path = f["path"].as_str()?;
            is_c_path(path).then(|| f["rule_id"].as_str()).flatten()
        })
        .collect();
    let rs_rules: BTreeSet<&str> = findings
        .iter()
        .filter_map(|f| {
            let path = f["path"].as_str()?;
            is_rs_path(path).then(|| f["rule_id"].as_str()).flatten()
        })
        .collect();

    let expected_c: BTreeSet<&str> = [
        ALLOC_001_RULE_ID,
        ALLOC_002_RULE_ID,
        STR_001_RULE_ID,
        STR_003_RULE_ID,
        ARCH_003_RULE_ID,
    ]
    .into_iter()
    .collect();
    assert_eq!(
        c_rules, expected_c,
        "expected exactly these PERF-C-* rule ids on C files, got {findings:?}"
    );

    let expected_rs: BTreeSet<&str> = [
        RS_ALLOC_001_RULE_ID,
        RS_ALLOC_003_RULE_ID,
        RS_STR_002_RULE_ID,
        RS_MEM_001_RULE_ID,
    ]
    .into_iter()
    .collect();
    assert_eq!(
        rs_rules, expected_rs,
        "expected exactly these PERF-RS-* rule ids on Rust files, got {findings:?}"
    );

    // Per-file presence (the original inline-fixture intent, pinned to the
    // committed corpus paths).
    assert!(
        findings.iter().any(|f| f["path"] == "src/main.c"
            && f["rule_id"]
                .as_str()
                .is_some_and(|r| r.starts_with("PERF-C-"))),
        "expected at least one PERF-C-* finding on src/main.c, got {findings:?}"
    );
    assert!(
        findings.iter().any(|f| f["path"] == "src/lib.rs"
            && f["rule_id"]
                .as_str()
                .is_some_and(|r| r.starts_with("PERF-RS-"))),
        "expected at least one PERF-RS-* finding on src/lib.rs, got {findings:?}"
    );

    // The registry is language-tagged: no cross-language leakage, per file.
    for f in findings {
        let path = f["path"].as_str().expect("finding path");
        let rule = f["rule_id"].as_str().expect("finding rule_id");
        if is_rs_path(path) {
            assert!(
                rule.starts_with("PERF-RS-"),
                "PERF-C-* rule {rule} leaked onto Rust file {path}"
            );
        } else if is_c_path(path) {
            assert!(
                rule.starts_with("PERF-C-"),
                "PERF-RS-* rule {rule} leaked onto C file {path}"
            );
        } else {
            panic!("unexpected finding path extension: {path}");
        }
    }

    // --- Same-language cross-file coupling fires ----------------------------
    // src/main.c's call to util_init (defined in src/util.c) is a genuine
    // cross-file, same-language edge.
    let c_arch003: Vec<&serde_json::Value> = findings
        .iter()
        .filter(|f| f["rule_id"] == ARCH_003_RULE_ID)
        .collect();
    assert_eq!(
        c_arch003.len(),
        1,
        "expected exactly one PERF-C-ARCH-003 edge, got {c_arch003:?}"
    );
    assert_eq!(c_arch003[0]["path"], "src/main.c");
    assert_eq!(c_arch003[0]["evidence"]["callee_name"], "util_init");
    assert_eq!(c_arch003[0]["evidence"]["matched_path"], "src/util.c");

    // --- Cross-language coupling is deliberately gated ----------------------
    // The corpus contains BOTH directions of a C↔Rust call:
    //   src/main.c  → rust_surface (defined in src/lib.rs)
    //   src/lib.rs  → util_init    (defined in src/util.c)
    // `cross_file_call_edges` keys its definition map on `(language, name)`
    // and checks resolved sites against the caller's language, so a
    // whole-program call can never emit a cross-language edge (rust-backend.md
    // §9.3 "Language tagging"). Lock that gate: only the same-language edge
    // exists, and no ARCH-003 finding crosses the boundary.
    let call_edges = doc["graphs"]["call_graph"]["edges"]
        .as_array()
        .expect("call_graph.edges present");
    assert_eq!(
        call_edges.len(),
        1,
        "only the same-language edge may be emitted, got {call_edges:?}"
    );
    assert_eq!(call_edges[0]["from_path"], "src/main.c");
    assert_eq!(call_edges[0]["to_name"], "util_init");
    assert_eq!(call_edges[0]["to_path"], "src/util.c");
    for edge in call_edges {
        let from = edge["from_path"].as_str().expect("from_path");
        let to = edge["to_path"].as_str().expect("to_path");
        assert_eq!(
            is_c_path(from),
            is_c_path(to),
            "cross-language call_graph edge emitted: {edge}"
        );
    }
    for f in findings {
        let rule = f["rule_id"].as_str().expect("finding rule_id");
        if rule != ARCH_003_RULE_ID && rule != RS_ARCH_003_RULE_ID {
            continue;
        }
        let caller = f["path"].as_str().expect("finding path");
        let matched = f["evidence"]["matched_path"].as_str().unwrap_or("");
        assert_eq!(
            is_c_path(caller),
            is_c_path(matched),
            "cross-language ARCH-003 finding emitted: {f}"
        );
    }

    // --- Direct scanner → analyzer seam (gating at the registry level) ------
    let units = scan(&target).expect("scan must succeed");
    let langs: Vec<&str> = units.iter().map(|u| u.file.language.as_str()).collect();
    assert_eq!(
        langs,
        ["rust", "c", "c", "c"],
        "scan records per-file language in lexicographic path order"
    );
    // The deliberate cross-language call sites must really exist in the IR,
    // so the "no cross-language edge" gate above is not vacuous:
    //   src/main.c#c_loop      calls rust_surface (defined in src/lib.rs)
    //   src/lib.rs#rust_surface calls util_init    (defined in src/util.c)
    let calls_in = |path: &str, func: &str| -> Vec<&str> {
        units
            .iter()
            .find(|u| u.file.path == path)
            .and_then(|u| u.functions.iter().find(|f| f.record.name == func))
            .map(|f| {
                f.calls
                    .iter()
                    .map(|c| c.callee_name.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    assert!(
        calls_in("src/main.c", "c_loop").contains(&"rust_surface"),
        "corpus drift: c_loop must call the Rust-defined rust_surface"
    );
    assert!(
        calls_in("src/lib.rs", "rust_surface").contains(&"util_init"),
        "corpus drift: rust_surface must call the C-defined util_init"
    );
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
    for rule in &rs_rules {
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

    // Cleanup only the temp copy — the committed corpus is never touched.
    fs::remove_dir_all(&target).ok();
}
