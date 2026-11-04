//! P4 integration determinism (modules.md §7 "double-render modulo
//! provenance" obligation at the FULL-PIPELINE level): two complete
//! `run_with` passes over the committed e2e fixture, then byte comparison of
//! EVERY artifact in the report directory.
//!
//! Contract locked here:
//! - `metrics.json` — byte-equal after masking `scan.started_at` /
//!   `scan.duration_ms` (the only sanctioned provenance fields).
//! - `report.md` — byte-equal after masking the SAME fields where they
//!   surface in the header blockquote.
//! - every `charts/*.svg` — byte-identical UNMASKED (charts carry no
//!   provenance by construction, ADR-0002 guardrail 2).
//! - the artifact SET (rel paths) is identical across runs.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

mod common;

/// The committed fixture repo (same one pipeline_e2e drives).
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-p4det-{tag}-{}-{nanos}",
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

/// Collect every artifact under `root` into a sorted rel-path → bytes map.
fn collect_artifacts(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let entry = entry.expect("dir entry readable");
            let name = entry
                .file_name()
                .into_string()
                .expect("UTF-8 artifact name");
            let path = entry.path();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, out);
            } else {
                out.insert(rel, fs::read(&path).expect("artifact readable"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, "", &mut out);
    out
}

#[test]
fn double_run_all_artifacts_byte_equal_modulo_provenance() {
    let root = temp_dir("full");
    let out = root.join("perfpal-report");
    let out_str = out.to_string_lossy().into_owned();
    let target = fixture_root().to_string_lossy().into_owned();

    // --- Run 1 ---------------------------------------------------------------
    assert_eq!(
        run_with(argv(&out_str, &target)),
        0,
        "findings never affect the exit code"
    );
    let run1 = collect_artifacts(&out);

    // --- Run 2 (swap path: same directory regenerated) -----------------------
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let run2 = collect_artifacts(&out);

    // Same artifact set both runs (no stale files survive the swap).
    let names1: Vec<&String> = run1.keys().collect();
    let names2: Vec<&String> = run2.keys().collect();
    assert_eq!(names1, names2, "artifact set diverged between runs");

    for (name, bytes1) in &run1 {
        let bytes2 = &run2[name];
        match name.as_str() {
            "metrics.json" => {
                let text1 = String::from_utf8(bytes1.clone()).expect("metrics.json UTF-8");
                let text2 = String::from_utf8(bytes2.clone()).expect("metrics.json UTF-8");
                assert_eq!(
                    common::mask_metrics_provenance(&text1),
                    common::mask_metrics_provenance(&text2),
                    "metrics.json diverged beyond provenance"
                );
            }
            "report.md" => {
                let text1 = String::from_utf8(bytes1.clone()).expect("report.md UTF-8");
                let text2 = String::from_utf8(bytes2.clone()).expect("report.md UTF-8");
                assert_eq!(
                    common::mask_report_provenance(&text1),
                    common::mask_report_provenance(&text2),
                    "report.md diverged beyond provenance"
                );
            }
            // C-split detail artifacts carry no provenance surfaces (no
            // started_at/duration_ms/root_path — relative paths only), so
            // the HTML detail comparison is byte-exact UNMASKED.
            "findings-detail.html" => {
                assert_eq!(
                    bytes1, bytes2,
                    "findings-detail.html must be byte-identical UNMASKED across runs"
                );
            }
            n if n.starts_with("charts/") && (n.ends_with(".svg") || n.ends_with(".dot")) => {
                assert_eq!(
                    bytes1, bytes2,
                    "{n} must be byte-identical UNMASKED across runs"
                );
            }
            // Bundled schema copy + any other fixed artifact: exact bytes.
            _ => assert_eq!(bytes1, bytes2, "{name} not byte-stable"),
        }
    }

    // Sanity: this fixture must actually exercise all derived-artifact
    // kinds, or the comparisons above prove nothing.
    assert!(run1.contains_key("metrics.json"));
    assert!(run1.contains_key("report.md"));
    assert!(run1.contains_key("report.html"));
    assert!(!run1.contains_key("findings-detail.md"));
    assert!(run1.contains_key("findings-detail.html"));
    // Three overview charts + five layered/map overviews + graph.dot + one
    // detail page per capped component (count derived from metrics.json, not
    // hardcoded).
    let metrics_raw = String::from_utf8(run1["metrics.json"].clone()).expect("metrics.json UTF-8");
    let metrics_doc: serde_json::Value =
        serde_json::from_str(&metrics_raw).expect("metrics.json parses");
    let component_count = metrics_doc["components"].as_array().unwrap().len();
    assert_eq!(
        run1.keys().filter(|n| n.starts_with("charts/")).count(),
        9 + component_count,
        "fixture should emit all three v1 charts + five layered/map overviews + graph.dot + one page per component"
    );
}

/// Structural suite v1 (schema 1.9): a fixture with loops and a parse-error
/// file must be byte-deterministic after provenance masking, and the function
/// order must stay `path → start_line → start_col` across runs.
#[test]
fn structural_loops_and_parse_errors_are_deterministic() {
    let root = temp_dir("structural");
    fs::write(
        root.join("loops.c"),
        "\
int loops(int n) {
    int s = 0;
    for (int i = 0; i < n; i++) {
        while (s < i) { s++; }
    }
    for (int j = 0; j < 2; j++) { s += j; }
    return s;
}
",
    )
    .expect("write C fixture");
    fs::write(
        root.join("broken.py"),
        "def broken():\n    x = (\n    return 1\n",
    )
    .expect("write broken fixture");

    let out = root.join("report");
    let out_str = out.to_string_lossy().into_owned();
    let target = root.to_string_lossy().into_owned();

    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let run1 = collect_artifacts(&out);
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let run2 = collect_artifacts(&out);

    assert_eq!(
        run1.keys().collect::<Vec<_>>(),
        run2.keys().collect::<Vec<_>>(),
        "artifact set diverged"
    );
    let metrics1 = String::from_utf8(run1["metrics.json"].clone()).expect("metrics.json UTF-8");
    let metrics2 = String::from_utf8(run2["metrics.json"].clone()).expect("metrics.json UTF-8");
    assert_eq!(
        common::mask_metrics_provenance(&metrics1),
        common::mask_metrics_provenance(&metrics2),
        "metrics.json diverged beyond provenance"
    );
    let report1 = String::from_utf8(run1["report.md"].clone()).expect("report.md UTF-8");
    let report2 = String::from_utf8(run2["report.md"].clone()).expect("report.md UTF-8");
    assert_eq!(
        common::mask_report_provenance(&report1),
        common::mask_report_provenance(&report2),
        "report.md diverged beyond provenance"
    );

    // Function order is the documented `(path, start_line, start_col)` total
    // order; field key order is fixed by the serde DTO (byte equality above).
    let doc: serde_json::Value = serde_json::from_str(&metrics1).expect("metrics.json parses");
    let functions = doc["stats"]["functions"].as_array().expect("functions[]");
    let keys: Vec<(String, u64, u64)> = functions
        .iter()
        .map(|f| {
            (
                f["path"].as_str().expect("path").to_string(),
                f["span"]["start_line"].as_u64().expect("start_line"),
                f["span"]["start_col"].as_u64().expect("start_col"),
            )
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "function order drifted from the contract");
    assert!(functions.iter().any(|f| {
        f["loop_count"].as_u64().unwrap_or(0) > 0 && f["loop_body_loc"].as_u64().unwrap_or(0) > 0
    }));
}

/// Stage 2 (schema 1.9): in-loop resource sites must be byte-deterministic
/// after provenance masking and keep the total `(line, column, kind,
/// language)` order across runs.
#[test]
fn resource_sites_are_deterministic() {
    let root = temp_dir("resources");
    fs::write(
        root.join("sites.c"),
        "\
int sites(int n) {
    for (int i = 0; i < n; i++) {
        malloc(4);
        printf(\"x\");
    }
    return 0;
}

#if 0
int dead_sites(int n) {
    for (int i = 0; i < n; i++) {
        malloc(8);
    }
    return 0;
}
#endif
",
    )
    .expect("write C fixture");
    fs::write(
        root.join("sites.rs"),
        "\
fn sites(n: usize) {
    for _ in 0..n {
        let v = Vec::new();
        println!(\"x\");
    }
}
",
    )
    .expect("write Rust fixture");
    fs::write(
        root.join("sites.py"),
        "\
def sites(items):
    for x in items:
        print(x)
",
    )
    .expect("write Python fixture");

    let out = root.join("report");
    let out_str = out.to_string_lossy().into_owned();
    let target = root.to_string_lossy().into_owned();

    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let run1 = collect_artifacts(&out);
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let run2 = collect_artifacts(&out);

    let metrics1 = String::from_utf8(run1["metrics.json"].clone()).expect("metrics.json UTF-8");
    let metrics2 = String::from_utf8(run2["metrics.json"].clone()).expect("metrics.json UTF-8");
    assert_eq!(
        common::mask_metrics_provenance(&metrics1),
        common::mask_metrics_provenance(&metrics2),
        "resource-site metrics diverged beyond provenance"
    );
    let report1 = String::from_utf8(run1["report.md"].clone()).expect("report.md UTF-8");
    let report2 = String::from_utf8(run2["report.md"].clone()).expect("report.md UTF-8");
    assert_eq!(
        common::mask_report_provenance(&report1),
        common::mask_report_provenance(&report2),
        "report.md diverged beyond provenance"
    );

    let doc: serde_json::Value = serde_json::from_str(&metrics1).expect("metrics.json parses");
    let mut total_sites = 0usize;
    for f in doc["stats"]["functions"].as_array().expect("functions[]") {
        let Some(sites) = f.get("resource_sites").and_then(|v| v.as_array()) else {
            continue;
        };
        total_sites += sites.len();
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
    assert_eq!(
        total_sites, 6,
        "C alloc+io (live) + C dead alloc + Rust alloc+io + Python io"
    );
    // The `#if 0` fixture also proves the dead-arm gate is stable across runs.
    let dead = doc["stats"]["functions"]
        .as_array()
        .expect("functions[]")
        .iter()
        .find(|f| f["name"] == "dead_sites")
        .expect("dead_sites record");
    assert_eq!(dead["cfg_gated"], true, "#if 0 function must be gated");
    assert_eq!(
        dead["resource_sites"][0]["cfg_gated"], true,
        "#if 0 site must be gated"
    );
}
