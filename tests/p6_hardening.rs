//! P6 hardening gates (modules.md §7): finding-id uniqueness (composite-id
//! contract, modules.md §1 analyzer row) and golden-report locking.
//!
//! Golden flow (tests/golden/report/): the committed tree is a FULL pipeline
//! output over `tests/fixtures/e2e`, masked in place for every run-varying
//! field. Regenerate with:
//!
//! ```text
//! PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens
//! ```
//!
//! The comparator test (default `cargo test`) re-runs the pipeline into a
//! temp dir, applies the SAME masking, and compares each artifact against
//! the committed goldens: metrics.json canonicalized via a serde round-trip,
//! report.md as line-masked text, schema copy + SVGs byte-exact.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::analyze;
use perfpal::cli::run_with;

mod common;

/// The committed fixture repo (must never be edited — goldens derive from
/// it; only pipeline OUTPUTS are masked).
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

/// The committed golden report tree.
fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/report")
}

/// Run-varying-masked artifacts under comparison. metrics.json and report.md
/// are compared as masked text; schema + SVGs carry no provenance and are
/// compared byte-exact (verified by `regenerate_goldens`).
const GOLDEN_ARTIFACTS: [&str; 6] = [
    "metrics.json",
    "metrics.schema.v1.1.json",
    "report.md",
    "charts/include-graph.svg",
    "charts/component-coupling.svg",
    "charts/hot-components.svg",
];

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("perfpal-p6-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<std::ffi::OsString> {
    std::iter::once(std::ffi::OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// Run the full pipeline over the e2e fixture into a fresh temp output dir
/// and return the parsed metrics.json document.
fn run_e2e_and_parse(tag: &str) -> (PathBuf, serde_json::Value) {
    let out = temp_dir(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline over the e2e fixture must succeed");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses as JSON");
    (out, doc)
}

/// Assert that the ONLY `finding_id` occurrences in the document are direct
/// members of `findings[]` — i.e. findings[] is the id-bearing list and the
/// ids are globally unique across the whole document (nothing else may
/// mint ids that could collide with them).
fn assert_ids_globally_unique(doc: &serde_json::Value) {
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(
        !findings.is_empty(),
        "e2e fixture must produce findings for a meaningful uniqueness gate"
    );

    let mut ids: Vec<&str> = Vec::new();
    let mut pointers: Vec<String> = Vec::new();
    collect_finding_id_pointers(doc, &mut Vec::new(), &mut pointers, &mut ids);

    assert_eq!(
        ids.len(),
        findings.len(),
        "every finding_id occurrence must belong to findings[] — \
         pointers found: {pointers:?}"
    );
    for pointer in &pointers {
        assert!(
            pointer.starts_with("/findings/"),
            "finding_id found outside findings[] at {pointer} — findings[] \
             must be the ONLY id-bearing list in the document"
        );
    }

    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "finding_ids must be pairwise unique; duplicates: {:?}",
        ids.iter()
            .filter(|id| ids.iter().filter(|o| o == id).count() > 1)
            .collect::<Vec<_>>()
    );
}

/// Depth-first walk collecting JSON pointers of every `finding_id` key.
fn collect_finding_id_pointers<'a>(
    value: &'a serde_json::Value,
    path: &mut Vec<String>,
    pointers: &mut Vec<String>,
    ids: &mut Vec<&'a str>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                path.push(key.clone());
                if key == "finding_id" {
                    pointers.push(format!("/{}", path.join("/")));
                    if let Some(s) = v.as_str() {
                        ids.push(s);
                    }
                }
                collect_finding_id_pointers(v, path, pointers, ids);
                path.pop();
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                path.push(i.to_string());
                collect_finding_id_pointers(v, path, pointers, ids);
                path.pop();
            }
        }
        _ => {}
    }
}

// ===========================================================================
// T1 — finding_id uniqueness gate
// ===========================================================================

#[test]
fn e2e_finding_ids_are_globally_unique() {
    let (_out, doc) = run_e2e_and_parse("ids");
    assert_ids_globally_unique(&doc);
}

#[test]
fn kitchen_sink_finding_ids_are_globally_unique() {
    // Reuse the exact all-23-rule corpus from the wave-A kitchen sink (now
    // shared via tests/common): check the analyzer-level ids BEFORE lift —
    // `finding_id` is derived deterministically from (rule, path, line,
    // col), so uniqueness at the Finding level is the same contract the
    // aggregator lifts into the document.
    let mut units = common::kitchen_units();
    units.push(common::helper_unit());
    let findings = analyze(&units);
    assert!(
        findings.len() >= 23,
        "kitchen sink must trip every rule (≥23 findings), got {}",
        findings.len()
    );
    let ids: Vec<String> = findings.iter().map(|f| f.finding_id()).collect();
    let unique: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "kitchen-sink finding_ids must be pairwise unique; got {ids:?}"
    );
}

// ===========================================================================
// T4 — golden report directories
// ===========================================================================

/// Regeneration path (env-gated, not part of the default gate): run the REAL
/// pipeline (through the real writer) directly into tests/golden/report/,
/// then mask the two text artifacts IN PLACE. SVGs and the bundled schema
/// are committed byte-exact — they contain no provenance (asserted below at
/// generation time).
#[test]
fn regenerate_goldens() {
    if std::env::var("PERFPAL_REGEN_GOLDEN").ok().as_deref() != Some("1") {
        // Default gate: a no-op. Run with PERFPAL_REGEN_GOLDEN=1 to refresh
        // the committed tree (see the module docs for the exact command).
        return;
    }

    let golden = golden_dir();
    // --force is REQUIRED here by design: the committed golden tree is a
    // plain directory in git and may exist without a fresh metrics.json
    // (e.g. right after checkout with only .gitkeep) — regeneration is the
    // one sanctioned overwrite of a non-report directory.
    let code = run_with(argv(&[
        "--output",
        golden.to_string_lossy().as_ref(),
        "--force",
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "golden regeneration run must succeed");

    // Mask metrics.json in place (provenance + root path).
    let metrics_raw = fs::read_to_string(golden.join("metrics.json")).expect("metrics.json");
    fs::write(
        golden.join("metrics.json"),
        common::mask_golden_metrics(&metrics_raw),
    )
    .expect("write masked metrics.json");

    // Mask report.md in place (provenance + every root-path occurrence).
    let report_raw = fs::read_to_string(golden.join("report.md")).expect("report.md");
    fs::write(
        golden.join("report.md"),
        common::mask_golden_report(&report_raw, &fixture_root().to_string_lossy()),
    )
    .expect("write masked report.md");

    // SVGs + schema: committed byte-exact; assert they carry no provenance
    // (no year-like RFC3339 fragments, no absolute fixture path).
    let root_str = fixture_root().to_string_lossy().into_owned();
    for name in [
        "charts/include-graph.svg",
        "charts/component-coupling.svg",
        "charts/hot-components.svg",
        "metrics.schema.v1.1.json",
    ] {
        let text = fs::read_to_string(golden.join(name)).expect(name);
        assert!(
            !text.contains("2026-") && !text.contains(&root_str),
            "{name} must be provenance-free to be committed byte-exact"
        );
    }

    // Sanity: the masked text artifacts must contain the <ROOT> sentinel.
    assert!(
        fs::read_to_string(golden.join("metrics.json"))
            .unwrap()
            .contains("<ROOT>"),
        "metrics.json masking must have replaced the scan root"
    );
    assert!(
        fs::read_to_string(golden.join("report.md"))
            .unwrap()
            .contains("<ROOT>"),
        "report.md masking must have replaced the scan root"
    );
}

/// Comparator (default gate, always runs): fresh pipeline output into a temp
/// dir, SAME masking, byte-compare each of the six artifacts against the
/// committed goldens. On mismatch the assertion lists every differing
/// artifact plus the regeneration command.
#[test]
fn report_matches_committed_goldens() {
    let (out, _doc) = run_e2e_and_parse("golden");

    let root_str = fixture_root().to_string_lossy().into_owned();
    let mut diffs: Vec<String> = Vec::new();
    for name in GOLDEN_ARTIFACTS {
        let expected_path = golden_dir().join(name);
        let actual_path = out.join(name);
        let Ok(actual) = fs::read(&actual_path) else {
            diffs.push(format!("{name}: MISSING from fresh output"));
            continue;
        };
        let Ok(expected) = fs::read(&expected_path) else {
            diffs.push(format!(
                "{name}: committed golden missing — regenerate the tree"
            ));
            continue;
        };
        let equal = match name {
            // Masked text comparison for the provenance-bearing artifacts.
            // The resolver field is neutralized on BOTH sides (feature builds
            // legitimately produce "ok" vs the goldens' default-build
            // "absent" — see common::mask_golden_resolver), as are the Wave
            // B1 resolved-edge deltas (MEDIUM resolved ARCH-003 finding +
            // call-graph edge rewritten to name-based form + §4 re-sort —
            // see common::mask_golden_semantic).
            "metrics.json" => {
                let actual = String::from_utf8(actual).expect("metrics.json is UTF-8");
                let expected = String::from_utf8(expected).expect("golden is UTF-8");
                let actual = common::mask_golden_metrics(&actual);
                let expected = common::mask_golden_metrics(&expected);
                let actual = common::mask_golden_semantic(&actual);
                let expected = common::mask_golden_semantic(&expected);
                common::mask_golden_resolver(&actual) == common::mask_golden_resolver(&expected)
            }
            "report.md" => {
                let actual = String::from_utf8(actual).expect("report.md is UTF-8");
                let expected = String::from_utf8(expected).expect("golden is UTF-8");
                common::mask_golden_report(&actual, &root_str)
                    == common::mask_golden_report(&expected, &root_str)
            }
            // Byte-exact for schema + SVGs (no provenance inside).
            _ => actual == expected,
        };
        if !equal {
            diffs.push(name.to_string());
        }
    }

    assert!(
        diffs.is_empty(),
        "fresh output diverged from committed goldens in: {diffs:?}\n\
         regenerate with:\n  \
         PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens"
    );
}
