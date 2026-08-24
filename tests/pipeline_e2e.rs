//! End-to-end pipeline integration (modules.md §2 stage order, driven through
//! the real cli entry): fixture C repo under tests/fixtures/e2e → scan →
//! analyze → assemble → serialize → atomic write. Locks the exit-code
//! contract (§8.2), the report-directory layout (output-contract §2), and
//! run-to-run byte determinism modulo provenance fields.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;
use perfpal::schema;

/// The committed fixture repo (tiny C project with an alloc-in-loop).
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("perfpal-e2e-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<std::ffi::OsString> {
    std::iter::once(std::ffi::OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

#[test]
fn full_pipeline_writes_valid_report_and_is_deterministic() {
    let out = temp_dir("happy").join("perfpal-report");
    let out_str = out.to_string_lossy().into_owned();

    // --- First run ---------------------------------------------------------
    let code = run_with(argv(&[
        "--output",
        &out_str,
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(
        code, 0,
        "findings never affect the exit code (modules.md P4)"
    );

    let metrics_path = out.join("metrics.json");
    assert!(metrics_path.is_file(), "metrics.json missing");
    assert!(out.join("charts").is_dir(), "charts/ always created");

    // Bundled schema is a byte-copy of the canonical artifact.
    let bundled = fs::read(out.join(schema::SCHEMA_ARTIFACT_NAME)).expect("bundled schema");
    let canonical =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/schema/metrics.schema.v1.json"))
            .expect("canonical schema");
    assert_eq!(bundled, canonical, "bundled schema drifted from docs copy");

    // Document shape.
    let raw1 = fs::read_to_string(&metrics_path).unwrap();
    let doc1: serde_json::Value = serde_json::from_str(&raw1).expect("metrics.json parses");
    assert_eq!(doc1["schema_version"], "1.0");
    assert_eq!(doc1["tool"]["name"], "perfpal");
    assert_eq!(doc1["scan"]["languages"], serde_json::json!(["c"]));

    // files[] sorted by path; both fixture files present.
    let paths: Vec<&str> = doc1["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["src/main.c", "src/util.h"]);
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);

    // The alloc-in-loop finding fired with a well-formed composite id.
    let findings = doc1["findings"].as_array().unwrap();
    let alloc: Vec<&str> = findings
        .iter()
        .filter_map(|f| f["finding_id"].as_str())
        .filter(|id| id.starts_with("PERF-C-ALLOC-001:src/main.c:"))
        .collect();
    assert!(
        !alloc.is_empty(),
        "expected PERF-C-ALLOC-001 on src/main.c, got {findings:?}"
    );
    for id in &alloc {
        let tail = id.rsplit(':').take(2).count();
        assert_eq!(tail, 2, "finding_id must end line:col — {id}");
    }

    // Wave A rules trip through the real tree-sitter-c backend on main.c:
    // drain's free-in-for (ALLOC-002), join's strcat-in-while (STR-001 and
    // STR-003 double-fire), deep_sum's triple-nested for (LOOP-001).
    // Wave B rules on the same file: grow's additive realloc-in-loop
    // (ALLOC-003), report's printf-in-loop (LOOP-002), deep_sum's
    // variable-divisor division in the innermost loop (LOOP-004).
    for rule in [
        "PERF-C-ALLOC-002",
        "PERF-C-STR-001",
        "PERF-C-STR-003",
        "PERF-C-LOOP-001",
        "PERF-C-ALLOC-003",
        "PERF-C-LOOP-002",
        "PERF-C-LOOP-004",
    ] {
        let prefix = format!("{rule}:src/main.c:");
        assert!(
            findings
                .iter()
                .filter_map(|f| f["finding_id"].as_str())
                .any(|id| id.starts_with(&prefix)),
            "expected {rule} on src/main.c, got {findings:?}"
        );
    }

    // CALL-002 threshold check against the ACTUAL computed metric: read
    // dispatch's cyclomatic complexity from stats.functions[] first, then
    // require the finding only because the real backend counted ≥ 15.
    let functions = doc1["stats"]["functions"].as_array().unwrap();
    let dispatch = functions
        .iter()
        .find(|f| f["name"] == "dispatch")
        .expect("dispatch in stats.functions[]");
    let complexity = dispatch["cyclomatic_complexity"].as_u64().unwrap();
    assert!(
        complexity >= 15,
        "fixture drift: dispatch complexity {complexity} fell below the \
         CALL-002 threshold — extend the branch chain"
    );
    let call002_prefix = "PERF-C-CALL-002:src/main.c:";
    assert!(
        findings
            .iter()
            .filter_map(|f| f["finding_id"].as_str())
            .any(|id| id.starts_with(call002_prefix)),
        "expected PERF-C-CALL-002 on src/main.c (dispatch complexity \
         {complexity}), got {findings:?}"
    );

    // Include graph carries the resolved quoted edge main.c → util.h.
    let edges = &doc1["graphs"]["include_graph"]["edges"];
    assert!(
        edges.as_array().is_some_and(|e| e.iter().any(|e| {
            e["from"] == "src/main.c" && e["to"] == "src/util.h" && e["resolved"] == true
        })),
        "resolved include edge missing: {edges}"
    );

    // --- Second run into the SAME directory (swap path) ---------------------
    let code = run_with(argv(&[
        "--output",
        &out_str,
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);
    let raw2 = fs::read_to_string(&metrics_path).unwrap();

    // Byte-equality modulo provenance: mask started_at/duration_ms, compare.
    let mask = |raw: &str| {
        let mut v: serde_json::Value = serde_json::from_str(raw).unwrap();
        v["scan"]["started_at"] = serde_json::Value::Null;
        v["scan"]["duration_ms"] = serde_json::Value::Null;
        v
    };
    assert_eq!(
        mask(&raw1),
        mask(&raw2),
        "two runs over identical input diverged beyond provenance"
    );
}

#[test]
fn unreadable_target_maps_to_usage_exit_1() {
    let out = temp_dir("usage");
    let code = run_with(argv(&[
        "--output",
        out.join("r").to_string_lossy().as_ref(),
        "/nonexistent/perfpal-target-xyz",
    ]));
    assert_eq!(code, 1, "missing target ⇒ usage error (exit 1)");
}

#[test]
fn safety_guard_refusal_maps_to_output_exit_2() {
    let root = temp_dir("guard");
    let out = root.join("occupied");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("user-data.txt"), "precious\n").unwrap();

    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 2, "safety-guard refusal without --force ⇒ exit 2");
    assert_eq!(
        fs::read_to_string(out.join("user-data.txt")).unwrap(),
        "precious\n",
        "guarded directory must be untouched"
    );

    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        "--force",
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "--force overrides the guard");
    assert!(out.join("metrics.json").is_file());
}

#[test]
fn unwritable_output_target_maps_to_output_exit_2() {
    let root = temp_dir("unwritable");
    let blocker = root.join("blocker.txt");
    fs::write(&blocker, "a file, not a dir\n").unwrap();

    // Output path sits UNDER a regular file ⇒ cannot create the directory.
    let code = run_with(argv(&[
        "--output",
        blocker.join("sub").to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 2, "cannot create output dir ⇒ exit 2");
}

#[test]
fn unknown_flag_maps_to_usage_exit_1() {
    let out = temp_dir("flag");
    let code = run_with(argv(&[
        "--bogus-flag",
        "--output",
        out.join("r").to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 1, "unknown flag ⇒ usage error (exit 1)");
}

#[test]
fn help_and_version_exit_zero() {
    assert_eq!(run_with(argv(&["--help"])), 0);
    assert_eq!(run_with(argv(&["--version"])), 0);
}
