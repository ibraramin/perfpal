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

mod common;

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

/// Whether libclang can actually be loaded in this environment. Feature
/// builds without an installed libclang degrade to the syntactic tier, and
/// build-variant expectations must follow the degraded path (graceful
/// absence is first-class — cf. tests/resolver_health.rs).
#[cfg(feature = "semantic-clang")]
fn libclang_loadable() -> bool {
    clang_sys::load().is_ok()
}

/// Without the feature there is no libclang to load, ever.
#[cfg(not(feature = "semantic-clang"))]
fn libclang_loadable() -> bool {
    false
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
    let canonical = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/schema/metrics.schema.v1.2.json"),
    )
    .expect("canonical schema");
    assert_eq!(bundled, canonical, "bundled schema drifted from docs copy");

    // Document shape.
    let raw1 = fs::read_to_string(&metrics_path).unwrap();
    let doc1: serde_json::Value = serde_json::from_str(&raw1).expect("metrics.json parses");
    assert_eq!(doc1["schema_version"], "1.2");
    assert_eq!(doc1["tool"]["name"], "perfpal");
    assert_eq!(doc1["scan"]["languages"], serde_json::json!(["c"]));

    // files[] sorted by path; all three fixture files present.
    let paths: Vec<&str> = doc1["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["src/main.c", "src/util.c", "src/util.h"]);
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
    // P6 threshold lift: CALL-002 findings carry their fired threshold as
    // a schema name/value pair (metrics-json-v1 $defs.threshold).
    let call002_threshold_names: Vec<&str> = findings
        .iter()
        .filter(|f| {
            f["finding_id"]
                .as_str()
                .unwrap_or("")
                .starts_with(call002_prefix)
        })
        .filter_map(|f| f["threshold"]["name"].as_str())
        .collect();
    assert_eq!(
        call002_threshold_names,
        vec!["min_complexity"; call002_threshold_names.len()],
        "every CALL-002 finding lifts threshold min_complexity"
    );
    assert!(
        findings
            .iter()
            .filter(|f| f["finding_id"]
                .as_str()
                .unwrap_or("")
                .starts_with(call002_prefix))
            .all(|f| f["threshold"]["value"].as_f64() == Some(15.0)),
        "CALL-002 threshold value must be the catalog default 15"
    );

    // P5 Tier 2 through the real backend: weigh's by-value struct Big
    // (TYPE-001) and clone_row's pointer-param sizeof misuse (MEM-001).
    for rule in ["PERF-C-TYPE-001", "PERF-C-MEM-001"] {
        let prefix = format!("{rule}:src/main.c:");
        assert!(
            findings
                .iter()
                .filter_map(|f| f["finding_id"].as_str())
                .any(|id| id.starts_with(&prefix)),
            "expected {rule} on src/main.c, got {findings:?}"
        );
    }
    // The real-backend sizeof operand must classify as the flagship
    // misuse class — this is the end-to-end lock on Shape.sizeof_operand
    // extraction feeding MEM-001.
    let mem001 = findings
        .iter()
        .find(|f| {
            f["finding_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("PERF-C-MEM-001:src/main.c:"))
        })
        .expect("MEM-001 finding on src/main.c");
    assert_eq!(
        mem001["evidence"]["misuse"].as_str(),
        Some("pointer_param_sizeof"),
        "real-backend sizeof(row) with pointer-typed row must classify \
         as pointer_param_sizeof, got {mem001}"
    );

    // Include graph carries the resolved quoted edge main.c → util.h.
    let edges = &doc1["graphs"]["include_graph"]["edges"];
    assert!(
        edges.as_array().is_some_and(|e| e.iter().any(|e| {
            e["from"] == "src/main.c" && e["to"] == "src/util.h" && e["resolved"] == true
        })),
        "resolved include edge missing: {edges}"
    );
    // P4: util.c joined the graph (node) with its own resolved edge.
    let ig_nodes = doc1["graphs"]["include_graph"]["nodes"].as_array().unwrap();
    assert!(
        ig_nodes.iter().any(|n| n["path"] == "src/util.c"),
        "src/util.c missing from include-graph nodes: {ig_nodes:?}"
    );
    assert!(
        edges.as_array().is_some_and(|e| e.iter().any(|e| {
            e["from"] == "src/util.c" && e["to"] == "src/util.h" && e["resolved"] == true
        })),
        "resolved include edge util.c → util.h missing: {edges}"
    );

    // Call graph: the TRUE cross-file pair main() → util_init (defined in
    // src/util.c, so not file-local). Name-based matching ⇒ confidence LOW.
    let call_edges = doc1["graphs"]["call_graph"]["edges"]
        .as_array()
        .expect("call_graph.edges present");
    let arch003_edge = call_edges.iter().find(|e| {
        e["from_function"]
            .as_str()
            .is_some_and(|f| f.starts_with("src/main.c#main#"))
            && e["to_name"] == "util_init"
    });
    let arch003_edge = arch003_edge.unwrap_or_else(|| {
        panic!("expected main→util_init cross-file call edge, got {call_edges:?}")
    });
    assert_eq!(arch003_edge["from_path"], "src/main.c");
    assert_eq!(arch003_edge["to_path"], "src/util.c");
    // Build-variant confidence: the default build matches by name only
    // (LOW); a `semantic-clang` feature build WITH a loadable libclang
    // confirms main()→util_init via USR pairing (resolved, MEDIUM); without
    // libclang it degrades to the name-based LOW path (graceful absence is
    // a first-class path, never a failure).
    let expected_confidence = if cfg!(feature = "semantic-clang") && libclang_loadable() {
        "MEDIUM"
    } else {
        "LOW"
    };
    assert_eq!(arch003_edge["confidence"], expected_confidence);

    // components[] present and ranked 1..n in emission order; every entry
    // carries a non-empty path_prefix (root-level files would map to ".").
    let components = doc1["components"].as_array().expect("components present");
    assert!(!components.is_empty(), "components must be non-empty");
    for (i, c) in components.iter().enumerate() {
        assert_eq!(c["rank"], serde_json::json!(i + 1), "rank gap: {c}");
        assert!(
            c["path_prefix"].as_str().is_some_and(|p| !p.is_empty()),
            "path_prefix must be a non-empty string: {c}"
        );
    }

    // --- Derived artifacts: report.md + charts/ -----------------------------
    let report_path = out.join("report.md");
    assert!(report_path.is_file(), "report.md missing");
    let report1 = fs::read_to_string(&report_path).unwrap();

    // Section headings present, in the fixed §5 template order.
    let headings = [
        "# Perfpal Report\n",
        "## 1. Executive summary\n",
        "## 2. Scan provenance & parse health\n",
        "## 3. Codebase statistics\n",
        "## 4. Findings\n",
        "## 5. Architecture\n",
        "## Appendix A. Heuristic reference\n",
        "## Appendix B. How to read this report\n",
    ];
    let mut cursor = 0;
    for h in headings {
        let pos = report1[cursor..]
            .find(h)
            .unwrap_or_else(|| panic!("heading {h:?} missing or out of order"));
        cursor += pos + h.len();
    }

    // The ARCH-003 finding is cited by EXACT finding_id inside §4.
    let findings = doc1["findings"].as_array().unwrap();
    let arch003_id = findings
        .iter()
        .find(|f| f["rule_id"] == "PERF-C-ARCH-003")
        .map(|f| f["finding_id"].as_str().unwrap().to_string())
        .expect("PERF-C-ARCH-003 finding in metrics.json");
    let sec4 = section(&report1, "## 4. Findings\n", "## 5. Architecture\n");
    // §4 embeds ids through the reporter's Markdown escaper (pinned set
    // includes '-' and '.'), so compare against the DE-ESCAPED section.
    let sec4_plain = sec4.replace('\\', "");
    assert!(
        sec4_plain.contains(&arch003_id),
        "§4 must cite {arch003_id} verbatim"
    );

    // Appendix A covers EVERY rule_id present in the findings (same
    // Markdown-escape consideration as §4 — compare de-escaped).
    let appendix_a = section(&report1, "## Appendix A.", "## Appendix B.").replace('\\', "");
    for rule_id in findings
        .iter()
        .filter_map(|f| f["rule_id"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
    {
        assert!(
            appendix_a.contains(rule_id),
            "Appendix A must cover {rule_id}"
        );
    }

    // charts/: exactly the emission-rule-expected set — all three charts,
    // because include-graph nodes AND components are non-empty here.
    let charts_dir = out.join("charts");
    let mut chart_files: Vec<String> = fs::read_dir(&charts_dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    chart_files.sort();
    chart_files.retain(|n| n.ends_with(".svg"));
    assert_eq!(
        chart_files,
        vec![
            "component-coupling.svg".to_string(),
            "hot-components.svg".to_string(),
            "include-graph.svg".to_string()
        ],
        "chart set must match the emission rule exactly"
    );
    let mut svg_bytes1 = std::collections::BTreeMap::new();
    for name in &chart_files {
        let bytes = fs::read(charts_dir.join(name)).unwrap();
        let text = String::from_utf8(bytes.clone()).expect("SVG is UTF-8");
        assert!(text.starts_with("<svg"), "{name} must start with <svg>");
        assert!(
            text.trim_end().ends_with("</svg>"),
            "{name} must end with </svg>"
        );
        svg_bytes1.insert(name.clone(), bytes);
    }

    // --- Second run into the SAME directory (swap path) ---------------------
    let code = run_with(argv(&[
        "--output",
        &out_str,
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);
    let raw2 = fs::read_to_string(&metrics_path).unwrap();
    let report2 = fs::read_to_string(&report_path).unwrap();

    // Byte-equality modulo provenance: mask started_at/duration_ms, compare.
    let mask = common::mask_metrics_provenance;
    assert_eq!(
        mask(&raw1),
        mask(&raw2),
        "two runs over identical input diverged beyond provenance"
    );

    // report.md: byte-equality modulo the SAME provenance fields, which
    // surface in the header blockquote (`started <RFC3339>`,
    // `duration <n> ms`).
    assert_eq!(
        common::mask_report_provenance(&report1),
        common::mask_report_provenance(&report2),
        "report.md diverged beyond provenance between runs"
    );

    // SVGs: byte-identical UNMASKED across runs (no provenance inside).
    for name in &chart_files {
        let bytes2 = fs::read(charts_dir.join(name)).unwrap();
        assert_eq!(
            svg_bytes1[name.as_str()],
            bytes2,
            "{name} not byte-stable across runs"
        );
    }
}

/// Slice `text` from the END of the `start` marker up to the `end` marker.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text.find(start).expect("section start marker") + start.len();
    let e = s + text[s..].find(end).expect("section end marker");
    &text[s..e]
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
