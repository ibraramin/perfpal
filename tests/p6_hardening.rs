//! P6 hardening gates (modules.md §7): finding-id uniqueness (composite-id
//! contract, modules.md §1 analyzer row) and golden-report locking.
//!
//! Golden flow (three committed trees, see [`GOLDEN_TREES`]): each tree is a
//! FULL pipeline output over its fixture, masked in place for every
//! run-varying field. Regenerate ALL trees with:
//!
//! ```text
//! PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens
//! ```
//!
//! The comparator test (default `cargo test`) re-runs the pipeline per tree
//! into a temp dir, applies the SAME masking, and compares each artifact
//! against the committed goldens: metrics.json canonicalized via a serde
//! round-trip, report.md as line-masked text, schema copy + SVGs byte-exact.
//!
//! R5 Wave 2 parameterized the original single (`tests/fixtures/e2e` →
//! `tests/golden/report`) tree into a descriptor array so the C, mixed C+Rust
//! and Rust-only trees share one regen + comparator path. The T1 uniqueness
//! gate stays pointed at the original C e2e fixture.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::analyze;
use perfpal::cli::run_with;

mod common;

/// One (fixture → committed golden) pair under lock. R5 Wave 2 generalized
/// the original single C tree to a descriptor array so the SAME regen +
/// comparator flow covers every language backend.
struct GoldenTree {
    /// Stable tag for temp-dir names + failure messages.
    tag: &'static str,
    /// Fixture repo relative to `CARGO_MANIFEST_DIR` (scan input).
    fixture: &'static str,
    /// Committed golden report tree relative to `CARGO_MANIFEST_DIR`.
    golden: &'static str,
}

/// The three locked trees. `c` is the original (T1 uniqueness stays pointed
/// here); `mixed` (`.c` + `.h` + `.rs`) and `rust` (`examples-rust/`) are the
/// R5 Wave 2 additions.
const GOLDEN_TREES: [GoldenTree; 3] = [
    GoldenTree {
        tag: "c",
        fixture: "tests/fixtures/e2e",
        golden: "tests/golden/report",
    },
    GoldenTree {
        tag: "mixed",
        fixture: "tests/fixtures/e2e-mixed",
        golden: "tests/golden/report-mixed",
    },
    GoldenTree {
        tag: "rust",
        fixture: "examples-rust",
        golden: "tests/golden/report-rust",
    },
];

/// Resolve a repo-relative path against the crate manifest dir.
fn manifest_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// The committed C fixture repo (original golden tree; must never be edited —
/// goldens derive from it; only pipeline OUTPUTS are masked).
fn fixture_root() -> PathBuf {
    manifest_path(GOLDEN_TREES[0].fixture)
}

/// Run-varying-masked artifacts under comparison. metrics.json and report.md
/// are compared as masked text; schema + SVGs carry no provenance and are
/// compared byte-exact (verified by `regenerate_goldens`).
const GOLDEN_ARTIFACTS: [&str; 6] = [
    "metrics.json",
    "metrics.schema.v1.3.json",
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

/// Run the full pipeline over `fixture` into a fresh temp output dir and
/// return the parsed metrics.json document.
fn run_pipeline_and_parse(tag: &str, fixture: &Path) -> (PathBuf, serde_json::Value) {
    let out = temp_dir(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline over the fixture must succeed");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses as JSON");
    (out, doc)
}

/// C-tree convenience wrapper: the T1 uniqueness gate stays pointed at the
/// original e2e fixture.
fn run_e2e_and_parse(tag: &str) -> (PathBuf, serde_json::Value) {
    run_pipeline_and_parse(tag, &fixture_root())
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

/// Materialize the scan target for a tree. The Rust tree (`examples-rust`) is
/// snapshotted into a tempdir EXCLUDING any `target/` directory: `scanner::discover`
/// does not skip it, so a future `cargo build` in the corpus would silently
/// inflate the scan. Relative paths are preserved, so the snapshot is
/// byte-identical to the live tree modulo the (masked) absolute root. The C and
/// mixed fixtures are scanned in place.
///
/// Returns the scan root plus, for snapshotted trees, the temp parent to clean
/// up once the run is done.
fn materialize_fixture(tree: &GoldenTree) -> (PathBuf, Option<PathBuf>) {
    if tree.tag == "rust" {
        let parent = temp_dir(&format!("corpus-{}", tree.tag));
        let snapshot = parent.join("corpus");
        common::copy_tree_excluding(&manifest_path(tree.fixture), &snapshot, "target");
        (snapshot, Some(parent))
    } else {
        (manifest_path(tree.fixture), None)
    }
}

/// Regeneration path (env-gated, not part of the default gate): run the REAL
/// pipeline (through the real writer) directly into each committed golden
/// tree, then mask the two text artifacts IN PLACE. SVGs and the bundled
/// schema are committed byte-exact — they contain no provenance (asserted
/// below at generation time for EVERY tree).
#[test]
fn regenerate_goldens() {
    if std::env::var("PERFPAL_REGEN_GOLDEN").ok().as_deref() != Some("1") {
        // Default gate: a no-op. Run with PERFPAL_REGEN_GOLDEN=1 to refresh
        // the committed trees (see the module docs for the exact command).
        return;
    }

    for tree in &GOLDEN_TREES {
        let (fixture, snapshot_parent) = materialize_fixture(tree);
        let golden = manifest_path(tree.golden);
        // --force is REQUIRED here by design: the committed golden tree is a
        // plain directory in git and may exist without a fresh metrics.json
        // (e.g. right after checkout with only .gitkeep) — regeneration is the
        // one sanctioned overwrite of a non-report directory.
        let code = run_with(argv(&[
            "--output",
            golden.to_string_lossy().as_ref(),
            "--force",
            fixture.to_string_lossy().as_ref(),
        ]));
        assert_eq!(
            code, 0,
            "golden regeneration run must succeed for tree '{}'",
            tree.tag
        );

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
            common::mask_golden_report(&report_raw, &fixture.to_string_lossy()),
        )
        .expect("write masked report.md");

        // SVGs + schema: committed byte-exact; assert they carry no provenance
        // (no year-like RFC3339 fragments, no absolute fixture path).
        let root_str = fixture.to_string_lossy().into_owned();
        for name in [
            "charts/include-graph.svg",
            "charts/component-coupling.svg",
            "charts/hot-components.svg",
            "metrics.schema.v1.3.json",
        ] {
            let text = fs::read_to_string(golden.join(name)).expect(name);
            assert!(
                !text.contains("2026-") && !text.contains(&root_str),
                "{name} (tree '{}') must be provenance-free to be committed byte-exact",
                tree.tag
            );
        }

        // Sanity: the masked text artifacts must contain the <ROOT> sentinel.
        assert!(
            fs::read_to_string(golden.join("metrics.json"))
                .unwrap()
                .contains("<ROOT>"),
            "metrics.json masking (tree '{}') must have replaced the scan root",
            tree.tag
        );
        assert!(
            fs::read_to_string(golden.join("report.md"))
                .unwrap()
                .contains("<ROOT>"),
            "report.md masking (tree '{}') must have replaced the scan root",
            tree.tag
        );

        if let Some(parent) = snapshot_parent {
            fs::remove_dir_all(parent).ok();
        }
    }
}

/// Comparator (default gate, always runs): per tree, fresh pipeline output
/// into a temp dir, SAME masking, compare each of the six artifacts against
/// the committed goldens. On mismatch the assertion names the tree and lists
/// every differing artifact plus the regeneration command.
#[test]
fn report_matches_committed_goldens() {
    for tree in &GOLDEN_TREES {
        let (fixture, snapshot_parent) = materialize_fixture(tree);
        let golden = manifest_path(tree.golden);
        let (out, _doc) = run_pipeline_and_parse(&format!("golden-{}", tree.tag), &fixture);

        let root_str = fixture.to_string_lossy().into_owned();
        let mut diffs: Vec<String> = Vec::new();
        for name in GOLDEN_ARTIFACTS {
            let expected_path = golden.join(name);
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
                // see common::mask_golden_semantic). Both maskers are no-ops on
                // trees that emit no resolved `PERF-C-ARCH-003`.
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
            "[{}] fresh output diverged from committed goldens in: {diffs:?}\n\
             regenerate with:\n  \
             PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens",
            tree.tag
        );

        if let Some(parent) = snapshot_parent {
            fs::remove_dir_all(parent).ok();
        }
    }
}
