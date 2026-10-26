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
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/schema/metrics.schema.v1.8.json"),
    )
    .expect("canonical schema");
    assert_eq!(bundled, canonical, "bundled schema drifted from docs copy");

    // Document shape.
    let raw1 = fs::read_to_string(&metrics_path).unwrap();
    let doc1: serde_json::Value = serde_json::from_str(&raw1).expect("metrics.json parses");
    assert_eq!(doc1["schema_version"], "1.8");
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

    // --- Derived artifacts: report.md + detail artifacts + charts/ --------
    let report_path = out.join("report.md");
    assert!(report_path.is_file(), "report.md missing");
    // C-split: the uncapped per-finding detail rides alongside the bounded
    // human files through the same CLI artifact pipe.
    assert!(
        out.join("findings-detail.md").is_file(),
        "findings-detail.md missing"
    );
    assert!(
        out.join("findings-detail.html").is_file(),
        "findings-detail.html missing"
    );
    let report1 = fs::read_to_string(&report_path).unwrap();
    // Human report.md carries zero per-finding rows (bounded at any scale).
    assert_eq!(
        report1.matches("#### ").count(),
        0,
        "report.md must carry zero per-finding detail rows"
    );
    assert!(
        report1.contains("[findings-detail.md](findings-detail.md)"),
        "report.md Appendix C must link the detail artifact"
    );

    // Section headings present, in the fixed §5 template order.
    let headings = [
        "# Perfpal Report\n",
        "## 1. Executive summary\n",
        "## 2. Scan provenance & parse health\n",
        "## 3. Codebase statistics\n",
        "## 4. Findings\n",
        "## 5. Explorer\n",
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
    let sec4 = section(&report1, "## 4. Findings\n", "## 5. Explorer\n");
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

    // charts/: exactly the emission-rule-expected set — all three overview
    // charts (include-graph nodes AND components are non-empty here) plus
    // the two layered overviews plus one per-component detail page per
    // capped component, in (rank, prefix) order after the overviews.
    let charts_dir = out.join("charts");
    let mut chart_files: Vec<String> = fs::read_dir(&charts_dir)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    chart_files.sort();
    chart_files.retain(|n| n.ends_with(".svg"));
    let mut expected = vec![
        "component-coupling.svg".to_string(),
        "hot-components.svg".to_string(),
        "include-graph.svg".to_string(),
        "layer-entry.svg".to_string(),
        "layer-file.svg".to_string(),
        "layer-bands.svg".to_string(),
        "layer-findings.svg".to_string(),
        "layer-workspace.svg".to_string(),
    ];
    let mut ranked: Vec<(u64, String)> = doc1["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["rank"].as_u64().unwrap(),
                c["path_prefix"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    ranked.sort();
    for (rank, prefix) in ranked.iter().take(50) {
        expected.push(perfpal::charts::component::component_filename(
            u32::try_from(*rank).unwrap(),
            prefix,
        ));
    }
    expected.sort();
    assert_eq!(
        chart_files, expected,
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

    // v1.4 step 2: the deterministic DOT export rides alongside the SVGs.
    let dot1 = fs::read_to_string(charts_dir.join("graph.dot")).expect("graph.dot present");
    assert!(
        dot1.starts_with("digraph perfpal {"),
        "graph.dot must be a DOT digraph"
    );

    // C-split detail artifacts: snapshot run-1 bytes for the UNMASKED
    // byte-identity check after the swap-path second run below.
    let mut detail_bytes1 = std::collections::BTreeMap::new();
    for name in ["findings-detail.md", "findings-detail.html", "report.html"] {
        detail_bytes1.insert(
            name.to_string(),
            fs::read(out.join(name)).unwrap_or_else(|_| panic!("{name} missing after run 1")),
        );
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

    // C-split detail artifacts: byte-identical UNMASKED across runs (no
    // provenance surfaces — relative paths only).
    for name in ["findings-detail.md", "findings-detail.html", "report.html"] {
        let bytes2 = fs::read(out.join(name)).expect("detail artifact readable");
        assert_eq!(
            detail_bytes1[name], bytes2,
            "{name} not byte-stable across runs"
        );
    }
    let detail_md1 = String::from_utf8(detail_bytes1["findings-detail.md"].clone())
        .expect("findings-detail.md is UTF-8");
    assert_eq!(
        detail_md1.matches("#### ").count(),
        doc1["findings"].as_array().unwrap().len(),
        "findings-detail.md must render every finding"
    );

    // SVGs + DOT: byte-identical UNMASKED across runs (no provenance inside).
    for name in &chart_files {
        let bytes2 = fs::read(charts_dir.join(name)).unwrap();
        assert_eq!(
            svg_bytes1[name.as_str()],
            bytes2,
            "{name} not byte-stable across runs"
        );
    }
    let dot2 = fs::read_to_string(charts_dir.join("graph.dot")).unwrap();
    assert_eq!(dot1, dot2, "graph.dot not byte-stable across runs");
}

/// `--story` is additive: it emits `atlas-layout.json` while `metrics.json`
/// and `report.html` stay byte-identical to the non-story run.
#[test]
fn story_flag_emits_atlas_layout_only_when_requested() {
    let plain = temp_dir("story-off").join("perfpal-report");
    let story = temp_dir("story-on").join("perfpal-report");

    let plain_code = run_with(argv(&[
        "--output",
        plain.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    let story_code = run_with(argv(&[
        "--output",
        story.to_string_lossy().as_ref(),
        "--story",
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(plain_code, 0);
    assert_eq!(story_code, 0);

    assert!(
        !plain.join("atlas-layout.json").exists(),
        "atlas-layout.json must not be emitted without --story"
    );
    assert!(
        story.join("atlas-layout.json").is_file(),
        "atlas-layout.json missing with --story"
    );

    let plain_metrics = fs::read_to_string(plain.join("metrics.json")).expect("plain metrics");
    let story_metrics = fs::read_to_string(story.join("metrics.json")).expect("story metrics");
    assert_eq!(
        common::mask_metrics_provenance(&plain_metrics),
        common::mask_metrics_provenance(&story_metrics),
        "metrics.json changed beyond provenance when --story was passed"
    );
    let plain_html = fs::read(plain.join("report.html")).expect("plain report");
    let story_html = fs::read(story.join("report.html")).expect("story report");
    assert_eq!(
        story_html, plain_html,
        "report.html changed when --story was passed"
    );

    let raw = fs::read_to_string(story.join("atlas-layout.json")).expect("layout JSON");
    let layout: serde_json::Value = serde_json::from_str(&raw).expect("layout JSON parses");
    assert_eq!(layout["layout_version"], 1);
    // P6 fallback: the committed fixture has no README at its root.
    assert_eq!(layout["purpose"]["kind"], "structural");
    assert_eq!(layout["purpose"]["source"], serde_json::Value::Null);
    assert_eq!(layout["purpose"]["purpose_version"], 1);
    assert!(
        layout["purpose"]["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("No README at the scan root; inferred entry: ")),
        "structural purpose text missing: {}",
        layout["purpose"]["text"]
    );

    // P7: the whole-repo profile queue is a projection of the authoritative
    // KV3 ordering — capped, reconciled, and joined to metrics function ids.
    let profile = &layout["profile"];
    assert_eq!(profile["queue_version"], 1, "profile.queue_version");
    let shown = profile["shown"].as_u64().expect("profile.shown");
    assert!(shown <= 12, "profile.shown exceeds the 12-row cap");
    let rows = profile["rows"].as_array().expect("profile.rows array");
    assert_eq!(rows.len() as u64, shown, "profile.shown != profile.rows");
    let total_ranked = profile["total_ranked"]
        .as_u64()
        .expect("profile.total_ranked");
    let production_ranked = profile["production_ranked"]
        .as_u64()
        .expect("profile.production_ranked");
    let candidate_total = profile["candidate_total"]
        .as_u64()
        .expect("profile.candidate_total");
    let demoted_production = profile["demoted_production"]
        .as_u64()
        .expect("profile.demoted_production");
    assert_eq!(
        production_ranked,
        candidate_total + demoted_production,
        "profile production counts do not reconcile"
    );
    assert!(total_ranked >= production_ranked);
    assert!(production_ranked >= candidate_total);
    assert!(candidate_total >= shown);

    let metrics_doc: serde_json::Value =
        serde_json::from_str(&story_metrics).expect("story metrics parses");
    let function_ids: Vec<&str> = metrics_doc["stats"]["functions"]
        .as_array()
        .expect("stats.functions array")
        .iter()
        .filter_map(|function| function["id"].as_str())
        .collect();
    for row in rows {
        let id = row["id"].as_str().expect("profile row id");
        assert!(
            function_ids.contains(&id),
            "profile row id {id} is not a metrics function id"
        );
        assert_eq!(row["production"], true, "non-production profile row: {row}");
        assert_eq!(
            row["demotions"],
            serde_json::json!([]),
            "demoted profile row: {row}"
        );
        assert!(
            row["cost"].is_number() && row["cost_text"].is_string(),
            "profile row cost fields missing: {row}"
        );
    }
    if !rows.is_empty() {
        assert_eq!(
            rows[0]["demotions"],
            serde_json::json!([]),
            "top profile row must be non-demoted"
        );
    }

    // P8: directory scenes cover every source file, respect the caps, and
    // reconcile shown + omitted against the scan totals.
    let file_paths: Vec<&str> = metrics_doc["files"]
        .as_array()
        .expect("files array")
        .iter()
        .filter_map(|file| file["path"].as_str())
        .collect();
    let source_file_count = metrics_doc["files"]
        .as_array()
        .expect("files array")
        .iter()
        .filter(|file| file["function_count"].as_u64().unwrap_or(0) > 0)
        .count();

    let scenes = &layout["scenes"];
    assert_eq!(scenes["scene_version"], 1, "scenes.scene_version");
    assert_eq!(scenes["scene_cap"], 24, "scenes.scene_cap");
    assert_eq!(scenes["file_cap"], 5, "scenes.file_cap");
    let scene_rows = scenes["rows"].as_array().expect("scenes.rows array");
    assert_eq!(
        scenes["shown_scenes"].as_u64().expect("shown_scenes"),
        scene_rows.len() as u64,
        "shown_scenes != rows.len()"
    );
    assert!(scene_rows.len() <= 24, "shown scenes exceed the cap");
    let mut shown_files = 0_usize;
    for scene in scene_rows {
        let files = scene["files"].as_u64().expect("scene files") as usize;
        shown_files += files;
        let top_files = scene["top_files"]
            .as_array()
            .expect("scene top_files array");
        assert!(
            top_files.len() <= 5,
            "top_files exceeds the per-scene cap: {scene}"
        );
        for file in top_files {
            let path = file["path"].as_str().expect("top file path");
            assert!(
                file_paths.contains(&path),
                "scene top file {path} is not a metrics file"
            );
            assert_eq!(
                file["name"],
                path.rsplit('/').next().unwrap_or(path),
                "top file basename mismatch: {file}"
            );
        }
        let path = scene["path"].as_str().expect("scene path");
        if path != "(root)" {
            let prefix = format!("{path}/");
            assert!(
                file_paths.iter().any(|file| file.starts_with(&prefix)),
                "scene path {path} owns no metrics file"
            );
        }
        assert!(scene["depth"].as_u64().is_some(), "scene depth missing");
        assert!(scene["cost"].is_number(), "scene cost missing");
        assert!(scene["scale"].is_string(), "scene scale missing");
    }
    let omitted = &scenes["omitted"];
    let total_files = scenes["total_files"].as_u64().expect("scenes.total_files") as usize;
    assert_eq!(total_files, source_file_count, "total_files mismatch");
    assert_eq!(
        shown_files + omitted["files"].as_u64().expect("omitted files") as usize,
        total_files,
        "shown + omitted files do not reconcile"
    );
    assert_eq!(
        scene_rows.len() + omitted["scenes"].as_u64().expect("omitted scenes") as usize,
        scenes["total_scenes"]
            .as_u64()
            .expect("scenes.total_scenes") as usize,
        "shown + omitted scenes do not reconcile"
    );

    for action in layout["actions"].as_array().expect("actions array") {
        assert!(
            action["scene"].is_string(),
            "story action missing its scene: {action}"
        );
    }

    let canvas_w = layout["canvas"]["w"].as_i64().expect("canvas width");
    let included = layout["meta"]["findings_included"]
        .as_u64()
        .expect("findings_included");
    let total = layout["meta"]["findings_total"]
        .as_u64()
        .expect("findings_total");
    assert!(included <= total, "included findings exceed the scan total");

    let nodes = layout["nodes"].as_array().expect("nodes array");
    for node in nodes {
        let path = node["path"].as_str().expect("node path");
        let scene = node["scene"].as_str().expect("drawn node scene");
        assert!(
            scene == "(root)" || path.starts_with(&format!("{scene}/")),
            "node {path} scene {scene} is not a partition prefix"
        );
    }
    let drawn: u64 = nodes
        .iter()
        .map(|node| {
            node["findings"]["hot"].as_u64().unwrap()
                + node["findings"]["warn"].as_u64().unwrap()
                + node["findings"]["info"].as_u64().unwrap()
        })
        .sum();
    assert_eq!(
        included, drawn,
        "findings_included != sum of drawn-node counts"
    );

    let columns = layout["columns"].as_array().expect("columns array");
    assert!(!columns.is_empty(), "layout must have columns");
    for column in columns {
        assert!(
            column["label"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "column label missing: {column}"
        );
        assert!(
            column["count_text"].as_str().is_some(),
            "column count_text missing: {column}"
        );
        assert!(
            column["width"].as_i64().is_some(),
            "column width missing: {column}"
        );
        let x = column["x"].as_i64().expect("column x");
        assert!(
            (0..=canvas_w).contains(&x),
            "column x outside [0, canvas.w]: {column}"
        );
    }

    for (index, column) in columns.iter().enumerate() {
        let centre = column["x"].as_i64().expect("column x");
        let width = column["width"].as_i64().expect("column width");
        for node in nodes
            .iter()
            .filter(|node| node["column"].as_u64() == Some(index as u64))
        {
            let node_x = node["x"].as_i64().expect("node x");
            let node_w = node["w"].as_i64().expect("node w");
            assert_eq!(
                node_x + node_w / 2,
                centre,
                "column header centre must equal the node centre: {column} vs {node}"
            );
        }
        assert!(width > 0, "column width must be positive: {column}");
    }

    for node in nodes {
        assert!(node["cost"].as_i64().is_some(), "node cost missing: {node}");
        let column = node["column"].as_u64().expect("node column");
        assert!(
            (column as usize) < columns.len(),
            "node column out of range: {node}"
        );
    }
}

/// S4/P6: the story-only purpose block reads the scan-root README, and adding
/// it changes nothing outside `atlas-layout.json` (F6).
#[test]
fn story_purpose_reads_root_readme_and_stays_story_only() {
    let root = temp_dir("purpose-src");
    fs::write(
        root.join("README.md"),
        "# Example\n\n\
         ![badge](https://example.invalid/badge.svg)\n\n\
         A deterministic C scanner for [Perfpal](https://example.invalid).\n\n\
         A second paragraph is ignored.\n",
    )
    .expect("write README");
    fs::write(root.join("main.c"), "int main(void) { return 0; }\n").expect("write main.c");

    let plain = temp_dir("purpose-off").join("perfpal-report");
    let story = temp_dir("purpose-on").join("perfpal-report");
    let plain_code = run_with(argv(&[
        "--output",
        plain.to_string_lossy().as_ref(),
        root.to_string_lossy().as_ref(),
    ]));
    let story_code = run_with(argv(&[
        "--output",
        story.to_string_lossy().as_ref(),
        "--story",
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(plain_code, 0);
    assert_eq!(story_code, 0);

    assert!(
        !plain.join("atlas-layout.json").exists(),
        "atlas-layout.json must not be emitted without --story"
    );

    let raw = fs::read_to_string(story.join("atlas-layout.json")).expect("layout JSON");
    let layout: serde_json::Value = serde_json::from_str(&raw).expect("layout JSON parses");
    assert_eq!(layout["layout_version"], 1);
    let purpose = &layout["purpose"];
    assert_eq!(purpose["kind"], "readme");
    assert_eq!(purpose["source"], "README.md");
    assert_eq!(purpose["purpose_version"], 1);
    assert_eq!(purpose["text"], "A deterministic C scanner for Perfpal.");

    // F6: --story changes nothing in the shared artifacts.
    let plain_metrics = fs::read_to_string(plain.join("metrics.json")).expect("plain metrics");
    let story_metrics = fs::read_to_string(story.join("metrics.json")).expect("story metrics");
    assert_eq!(
        common::mask_metrics_provenance(&plain_metrics),
        common::mask_metrics_provenance(&story_metrics),
        "metrics.json changed beyond provenance when --story was passed"
    );
    assert_eq!(
        fs::read(plain.join("report.html")).expect("plain report"),
        fs::read(story.join("report.html")).expect("story report"),
        "report.html changed when --story was passed"
    );
}

/// `atlas-layout.json` must be byte-identical across two independent scans.
#[test]
fn atlas_layout_json_is_byte_identical_on_double_scan() {
    let first = temp_dir("story-double-a").join("perfpal-report");
    let second = temp_dir("story-double-b").join("perfpal-report");
    for out in [&first, &second] {
        let code = run_with(argv(&[
            "--output",
            out.to_string_lossy().as_ref(),
            "--story",
            "--force",
            fixture_root().to_string_lossy().as_ref(),
        ]));
        assert_eq!(code, 0);
    }

    let bytes_a = fs::read(first.join("atlas-layout.json")).expect("first layout");
    let bytes_b = fs::read(second.join("atlas-layout.json")).expect("second layout");
    assert_eq!(
        bytes_a, bytes_b,
        "atlas-layout.json not byte-stable across scans"
    );
}

/// S2/P3: `--story` emits a syntactic signature block whose `id`s join the
/// metrics function table exactly (canonical IR identity).
#[test]
fn story_flow_signatures_join_metrics_function_ids() {
    let root = temp_dir("flow-src");
    fs::write(
        root.join("main.c"),
        "int parse(Config *cfg, const char *path) { return cfg != 0 && path != 0; }\n\
         int main(void) { return parse(0, 0); }\n",
    )
    .expect("write fixture");
    let out = temp_dir("flow-out").join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        "--story",
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);

    let raw = fs::read_to_string(out.join("atlas-layout.json")).expect("layout JSON");
    let layout: serde_json::Value = serde_json::from_str(&raw).expect("layout JSON parses");
    let flow = &layout["flow"];
    assert_eq!(flow["flow_version"], 1);
    assert_eq!(flow["fidelity"], "syntactic");
    assert!(flow["calls"].is_array(), "P1 calls array missing");
    assert!(flow["io"].is_array(), "P2 io array missing");
    assert_eq!(flow["io"], serde_json::json!([]), "no I/O in this fixture");

    let signatures = flow["signatures"]
        .as_array()
        .expect("flow signatures array");
    assert!(!signatures.is_empty(), "signatures must not be empty");
    assert!(
        signatures
            .iter()
            .any(|signature| signature["id"] == "main.c#parse#1:1"),
        "canonical IR id missing: {signatures:?}"
    );
    for signature in signatures {
        assert!(
            signature["params"].is_array(),
            "params missing: {signature}"
        );
        assert!(
            signature["unresolved"].is_array(),
            "unresolved missing: {signature}"
        );
        assert!(
            signature["ret"].is_string() || signature["ret"].is_null(),
            "ret must be a string or null: {signature}"
        );
    }

    let metrics_raw = fs::read_to_string(out.join("metrics.json")).expect("metrics JSON");
    let metrics: serde_json::Value =
        serde_json::from_str(&metrics_raw).expect("metrics JSON parses");
    let ids: Vec<&str> = metrics["stats"]["functions"]
        .as_array()
        .expect("stats.functions array")
        .iter()
        .filter_map(|function| function["id"].as_str())
        .collect();
    for signature in signatures {
        let id = signature["id"].as_str().expect("signature id");
        assert!(
            ids.contains(&id),
            "flow signature id {id} is not a metrics function id"
        );
    }
}

/// S2/P1: call payloads and per-edge labels ride on the story-only flow block.
#[test]
fn story_flow_calls_and_edge_labels_are_story_only() {
    let root = temp_dir("flow-calls-src");
    fs::write(
        root.join("main.c"),
        "int helper(int x);\n\
         int main(void) {\n\
             int value = 1;\n\
             return helper(value);\n\
         }\n",
    )
    .expect("write main.c");
    fs::write(root.join("util.c"), "int helper(int x) { return x; }\n").expect("write util.c");

    let out = temp_dir("flow-calls-out").join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        "--story",
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);

    let raw = fs::read_to_string(out.join("atlas-layout.json")).expect("layout JSON");
    let layout: serde_json::Value = serde_json::from_str(&raw).expect("layout JSON parses");
    let drawn: Vec<&str> = layout["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .filter_map(|node| node["path"].as_str())
        .collect();

    let calls = layout["flow"]["calls"]
        .as_array()
        .expect("flow calls array");
    assert!(!calls.is_empty(), "drawn call sites must be recorded");
    for call in calls {
        let from = call["from"].as_str().expect("call from");
        assert!(
            drawn.contains(&from),
            "call from is not a drawn path: {call}"
        );
        match call["to"].as_str() {
            Some(to) => assert!(drawn.contains(&to), "call to is not a drawn path: {call}"),
            None => assert!(call["to_id"].is_null(), "to_id set without to: {call}"),
        }
        assert!(
            call["unresolved"].is_boolean(),
            "unresolved must be a boolean: {call}"
        );
        for arg in call["args"].as_array().expect("args array") {
            assert!(arg["expr"].is_string(), "arg expr missing: {arg}");
            assert!(arg["literal"].is_boolean(), "arg literal missing: {arg}");
            assert!(
                matches!(arg["dir"].as_str(), Some("in" | "inout")),
                "arg dir outside in/inout: {arg}"
            );
        }
    }

    let allowed = ["type-only", "literal-observed", "unresolved"];
    let edges = layout["edges"].as_array().expect("edges array");
    assert!(
        !edges.is_empty(),
        "a two-file fixture must draw at least one edge"
    );
    for edge in edges {
        let label = edge["label"].as_str().expect("edge label");
        assert!(
            allowed.contains(&label),
            "edge label outside the contract: {edge}"
        );
        assert!(
            edge.get("payload").is_some(),
            "edge payload key missing: {edge}"
        );
    }
    let edge = edges
        .iter()
        .find(|edge| edge["from"] == "main.c" && edge["to"] == "util.c")
        .expect("main.c → util.c edge");
    assert_eq!(edge["label"], "type-only");
    assert_eq!(edge["payload"], "int");
}

/// S3/P2: `--story` emits the flat, sorted `flow.io[]` rows; every row belongs
/// to a drawn file, literals are plain string targets, and built values
/// (`sprintf`) never appear.
#[test]
fn story_flow_io_effects_are_story_only_and_conservative() {
    let root = temp_dir("flow-io-src");
    fs::write(
        root.join("io.c"),
        "#include <stdio.h>\n\
         #include <stdlib.h>\n\
         int main(void) {\n\
             FILE *f = fopen(\"cfg.ini\", \"r\");\n\
             const char *home = getenv(\"HOME\");\n\
             FILE *out = fopen(home, \"w\");\n\
             char buf[8];\n\
             sprintf(buf, \"%s\", home);\n\
             printf(\"%s\\n\", home);\n\
             return f != 0;\n\
         }\n",
    )
    .expect("write io.c");

    let out = temp_dir("flow-io-out").join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        "--story",
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);

    let raw = fs::read_to_string(out.join("atlas-layout.json")).expect("layout JSON");
    let layout: serde_json::Value = serde_json::from_str(&raw).expect("layout JSON parses");
    let drawn: Vec<&str> = layout["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .filter_map(|node| node["path"].as_str())
        .collect();
    let io = layout["flow"]["io"].as_array().expect("flow.io array");

    let rows: Vec<(u64, &str, &str, &str, Option<&str>)> = io
        .iter()
        .map(|row| {
            (
                row["line"].as_u64().expect("io line"),
                row["callee"].as_str().expect("io callee"),
                row["op"].as_str().expect("io op"),
                row["target"].as_str().expect("io target"),
                row["literal"].as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (4, "fopen", "read", "file", Some("cfg.ini")),
            (5, "getenv", "read", "env", Some("HOME")),
            (6, "fopen", "write", "file", None),
            (9, "printf", "write", "stdio", None),
        ],
        "sprintf is not I/O; a non-literal path stays null; stdio has no target literal"
    );
    for row in io {
        assert_eq!(
            row["file"].as_str(),
            Some("io.c"),
            "io.file must be a drawn call-site path: {row}"
        );
        assert!(
            drawn.contains(&row["file"].as_str().expect("io file")),
            "io.file is not drawn: {row}"
        );
        assert!(
            matches!(row["op"].as_str(), Some("read" | "write")),
            "io op outside read/write: {row}"
        );
        assert!(
            matches!(
                row["target"].as_str(),
                Some("file" | "env" | "net" | "stdio")
            ),
            "io target outside the contract: {row}"
        );
        if row["target"] == "stdio" {
            assert!(
                row["literal"].is_null(),
                "stdio rows never carry a path literal: {row}"
            );
        }
    }
    let order: Vec<(String, u64, String, String, String, Option<String>)> = io
        .iter()
        .map(|row| {
            (
                row["file"].as_str().expect("io file").to_string(),
                row["line"].as_u64().expect("io line"),
                row["callee"].as_str().expect("io callee").to_string(),
                row["op"].as_str().expect("io op").to_string(),
                row["target"].as_str().expect("io target").to_string(),
                row["literal"].as_str().map(str::to_string),
            )
        })
        .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] <= pair[1]),
        "flow.io must be sorted (file, line, callee, op, target, literal): {order:?}"
    );
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
