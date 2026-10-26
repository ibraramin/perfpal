//! `examples-js/` double-run byte-determinism (plan `js-ts`, B2-W2 / W3a;
//! modules.md §7 / §11 "double-run byte equality on `examples-js`").
//!
//! Two complete `run_with` passes over the COMMITTED `examples-js/` corpus
//! into two SEPARATE temp output dirs, then a byte comparison of every
//! artifact:
//!
//! - `metrics.json` — equal after masking `scan.started_at` /
//!   `scan.duration_ms` (the only sanctioned provenance fields);
//! - `report.md` — equal after masking the same fields where they surface in
//!   the header blockquote;
//! - `report.html`, `findings-detail.html`, `profile-queue.html`, every
//!   `charts/*` artifact and the bundled `metrics.schema.v1.8.json` —
//!   byte-identical UNMASKED (no provenance by construction);
//! - `findings-detail.md` / `profile-queue.md` — equal after the same
//!   provenance mask (defensive: they carry no provenance today);
//! - the artifact SET (rel paths) is identical across runs.
//!
//! The scan target is read-only and both reports live OUTSIDE it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

mod common;

/// The committed corpus root (`<repo>/examples-js`).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples-js")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-js-det-{tag}-{}-{nanos}",
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

fn text(bytes: &[u8], name: &str) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| panic!("{name} is UTF-8"))
}

#[test]
fn double_run_all_js_artifacts_byte_equal_modulo_provenance() {
    let corpus = corpus_root();
    assert!(
        corpus.join("package.json").is_file() && corpus.join("index.js").is_file(),
        "committed corpus missing at {}",
        corpus.display()
    );
    let target = corpus.to_string_lossy().into_owned();

    // Two SEPARATE output dirs (not a same-dir swap).
    let root1 = temp_dir("a");
    let root2 = temp_dir("b");
    let out1 = root1.join("perfpal-report");
    let out2 = root2.join("perfpal-report");

    assert_eq!(
        run_with(argv(out1.to_string_lossy().as_ref(), &target)),
        0,
        "first examples-js run must exit 0"
    );
    assert_eq!(
        run_with(argv(out2.to_string_lossy().as_ref(), &target)),
        0,
        "second examples-js run must exit 0"
    );

    let run1 = collect_artifacts(&out1);
    let run2 = collect_artifacts(&out2);

    // Same artifact set in both runs.
    let names1: Vec<&String> = run1.keys().collect();
    let names2: Vec<&String> = run2.keys().collect();
    assert_eq!(names1, names2, "artifact set diverged between runs");

    for (name, bytes1) in &run1 {
        let bytes2 = &run2[name];
        match name.as_str() {
            "metrics.json" => {
                assert_eq!(
                    common::mask_metrics_provenance(&text(bytes1, name)),
                    common::mask_metrics_provenance(&text(bytes2, name)),
                    "metrics.json diverged beyond provenance"
                );
            }
            "report.md" | "findings-detail.md" | "profile-queue.md" => {
                assert_eq!(
                    common::mask_report_provenance(&text(bytes1, name)),
                    common::mask_report_provenance(&text(bytes2, name)),
                    "{name} diverged beyond provenance"
                );
            }
            n if n.starts_with("charts/") => {
                assert_eq!(
                    bytes1, bytes2,
                    "{n} must be byte-identical UNMASKED across runs"
                );
            }
            // report.html / findings-detail.html / profile-queue.html and the
            // bundled schema copy: exact bytes.
            _ => assert_eq!(bytes1, bytes2, "{name} not byte-stable"),
        }
    }

    // Non-vacuity: every artifact kind this suite claims to compare exists.
    for required in [
        "metrics.json",
        "metrics.schema.v1.8.json",
        "report.md",
        "report.html",
        "findings-detail.md",
        "findings-detail.html",
        "profile-queue.md",
        "profile-queue.html",
        "charts/graph.dot",
        "charts/hot-components.svg",
        "charts/component-01-root.svg",
    ] {
        assert!(
            run1.contains_key(required),
            "expected artifact missing from JS report: {required}"
        );
    }

    fs::remove_dir_all(&root1).ok();
    fs::remove_dir_all(&root2).ok();
}
