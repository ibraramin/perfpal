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
