//! P6 hardening gates (modules.md §7): finding-id uniqueness (composite-id
//! contract, modules.md §1 analyzer row) and golden-report locking.
//!
//! Golden flow (six committed trees, see [`GOLDEN_TREES`]): each tree is a
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
//! round-trip, report.md as line-masked text, report.html as
//! absolute-root-masked text (no provenance surfaces by construction),
//! schema copy + SVGs byte-exact.
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
#[cfg_attr(feature = "semantic-clang", allow(dead_code))]
struct GoldenTree {
    /// Stable tag for temp-dir names + failure messages.
    tag: &'static str,
    /// Fixture repo relative to `CARGO_MANIFEST_DIR` (scan input).
    fixture: &'static str,
    /// Committed golden report tree relative to `CARGO_MANIFEST_DIR`.
    golden: &'static str,
    /// When `Some(dir)`, the fixture is snapshotted into a tempdir EXCLUDING
    /// `dir` at every depth (see [`common::copy_tree_excluding`]); when
    /// `None`, the fixture is scanned in place. A descriptor field — never a
    /// `tag`-string match — drives the dispatch.
    snapshot_exclude: Option<&'static str>,
}

/// The six locked trees. `c` is the original (T1 uniqueness stays pointed
/// here); `mixed` (`.c` + `.h` + `.rs`) and `rust` (`examples-rust/`) are the
/// R5 Wave 2 additions; `python` (`examples-python/`) is the plan
/// `python-backend` W4 Lane 5 addition; `js` (`examples-js/`) is the plan
/// `js-ts` B2-W2 addition; `ts` (`examples-ts/`) is the plan `js-ts` B3-WP2
/// addition.
const GOLDEN_TREES: [GoldenTree; 6] = [
    GoldenTree {
        tag: "c",
        fixture: "tests/fixtures/e2e",
        golden: "tests/golden/report",
        snapshot_exclude: None,
    },
    GoldenTree {
        tag: "mixed",
        fixture: "tests/fixtures/e2e-mixed",
        golden: "tests/golden/report-mixed",
        snapshot_exclude: None,
    },
    GoldenTree {
        tag: "rust",
        fixture: "examples-rust",
        golden: "tests/golden/report-rust",
        snapshot_exclude: Some("target"),
    },
    GoldenTree {
        tag: "python",
        fixture: "examples-python",
        golden: "tests/golden/report-python",
        snapshot_exclude: None,
    },
    GoldenTree {
        tag: "js",
        fixture: "examples-js",
        golden: "tests/golden/report-js",
        snapshot_exclude: None,
    },
    GoldenTree {
        tag: "ts",
        fixture: "examples-ts",
        golden: "tests/golden/report-ts",
        snapshot_exclude: None,
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

/// Run-varying-masked artifacts under comparison. metrics.json, report.md,
/// findings-detail.md, report.html and findings-detail.html are compared as
/// masked text; schema + SVGs + graph.dot carry no provenance and are
/// compared byte-exact (verified by `regenerate_goldens`). The two Atlas
/// pages (`atlas-q2.html`, `atlas-weight.html`) are in the fixed set too and
/// are compared byte-exact: regeneration asserts they carry no provenance
/// (no year-like RFC3339 fragment, no absolute fixture path). report.html and
/// findings-detail.html carry no provenance by construction (schema/file/
/// finding counts only), so their masks are the absolute-root substitution
/// plus the Wave B1 resolved-ARCH-003 row neutralizer (see
/// common::mask_golden_html_semantic). findings-detail.md masks exactly like
/// report.md (provenance surfaces + absolute root + ARCH-003 block strip).
///
/// Default-build only: committed goldens are produced by the syntactic
/// (default) build; under `semantic-clang` the resolved-edge tier changes
/// chart topology and ARCH-003 confidence, which cannot be masked. The
/// semantic tier is covered by `tests/resolver_health.rs` and the
/// double-run determinism suites instead.
#[cfg(not(feature = "semantic-clang"))]
const GOLDEN_ARTIFACTS: [&str; 19] = [
    "metrics.json",
    "metrics.schema.v1.8.json",
    "report.md",
    "report.html",
    "findings-detail.md",
    "findings-detail.html",
    "atlas-q2.html",
    "atlas-weight.html",
    "profile-queue.md",
    "profile-queue.html",
    "charts/include-graph.svg",
    "charts/layer-file.svg",
    "charts/layer-entry.svg",
    "charts/component-coupling.svg",
    "charts/hot-components.svg",
    "charts/layer-workspace.svg",
    "charts/layer-bands.svg",
    "charts/layer-findings.svg",
    "charts/graph.dot",
];

/// Sorted `charts/component-*.svg` page names under a report tree,
/// excluding the `component-coupling.svg` overview (it is a fixed golden
/// above, not a per-component page). The filter mirrors the hardened
/// reporter registry exactly: `component-<one-or-more-ASCII-digits>-*`
/// (a colliding `component-<word>` or extension-less name is an overview,
/// never a detail page).
#[cfg(not(feature = "semantic-clang"))]
fn is_component_page_name(n: &str) -> bool {
    let Some(rest) = n.strip_prefix("component-") else {
        return false;
    };
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && rest.as_bytes().get(digits) == Some(&b'-') && n.ends_with(".svg")
}

#[cfg(not(feature = "semantic-clang"))]
fn component_page_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join("charts"))
        .expect("charts dir present")
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| is_component_page_name(n) && n != "component-coupling.svg")
        .map(|n| format!("charts/{n}"))
        .collect();
    names.sort();
    names
}

/// PINNED per-tree detail-page sets (B6 lock): the exact `component-<rank>-`
/// page names each golden tree must emit. Any add/remove/rename fails the
/// gate — even a regeneration must update this table explicitly, never
/// auto-adopt a changed set. `component-coupling.svg` is an overview chart
/// and is NOT listed here.
#[cfg(not(feature = "semantic-clang"))]
const EXPECTED_COMPONENT_PAGES: &[(&str, &[&str])] = &[
    ("c", &["charts/component-01-src.svg"]),
    ("mixed", &["charts/component-01-src.svg"]),
    ("rust", &["charts/component-01-src.svg"]),
    (
        "python",
        &[
            "charts/component-01-root.svg",
            "charts/component-02-pkg.svg",
        ],
    ),
    // `examples-js` is flat (all sources at the package root), so its single
    // component is the root component.
    ("js", &["charts/component-01-root.svg"]),
    // `examples-ts` is likewise flat; one root component.
    ("ts", &["charts/component-01-root.svg"]),
];

/// Assert a tree's page set is EXACTLY the pinned expectation (no
/// auto-adopt: extras and missing pages both fail with the regen command).
#[cfg(not(feature = "semantic-clang"))]
fn assert_pinned_page_set(tag: &str, pages: &[String]) {
    let expected: Vec<String> = EXPECTED_COMPONENT_PAGES
        .iter()
        .find(|(t, _)| *t == tag)
        .unwrap_or_else(|| panic!("no pinned page set for tree '{tag}'"))
        .1
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert_eq!(
        *pages, expected,
        "[{tag}] per-component page set changed (add/remove/rename) — \
         update EXPECTED_COMPONENT_PAGES explicitly, then regenerate with:\n  \
         PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens"
    );
}

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

/// Materialize the scan target for a tree. A tree carrying
/// [`GoldenTree::snapshot_exclude`] is snapshotted into a tempdir with that
/// directory excluded at every depth: `scanner::discover` does not skip it,
/// so a future `cargo build` in the corpus would silently inflate the scan.
/// Relative paths are preserved, so the snapshot is byte-identical to the
/// live tree modulo the (masked) absolute root. All other fixtures are
/// scanned in place.
///
/// Returns the scan root plus, for snapshotted trees, the RAII temp parent:
/// dropping the guard removes the snapshot — even on panic — so no manual
/// `remove_dir_all` (which a panic would skip) is needed at the call sites.
#[cfg(not(feature = "semantic-clang"))]
fn materialize_fixture(tree: &GoldenTree) -> (PathBuf, Option<common::TempDir>) {
    if let Some(excluded) = tree.snapshot_exclude {
        let parent = common::TempDir::new(&format!("corpus-{}", tree.tag));
        let snapshot = parent.path().join("corpus");
        common::copy_tree_excluding(&manifest_path(tree.fixture), &snapshot, excluded);
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
#[cfg(not(feature = "semantic-clang"))]
#[test]
fn regenerate_goldens() {
    if std::env::var("PERFPAL_REGEN_GOLDEN").ok().as_deref() != Some("1") {
        // Default gate: a no-op. Run with PERFPAL_REGEN_GOLDEN=1 to refresh
        // the committed trees (see the module docs for the exact command).
        return;
    }

    for tree in &GOLDEN_TREES {
        let (fixture, _snapshot_guard) = materialize_fixture(tree);
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

        // Mask report.html in place (absolute-root substitution only — the
        // template carries no started_at/duration_ms/root_path surfaces, so
        // this is a defensive no-op on today's trees).
        let html_raw = fs::read_to_string(golden.join("report.html")).expect("report.html");
        fs::write(
            golden.join("report.html"),
            common::mask_golden_html(&html_raw, &fixture.to_string_lossy()),
        )
        .expect("write masked report.html");

        // Mask findings-detail.md in place (same masker as report.md:
        // provenance surfaces + every root-path occurrence + the Wave B1
        // ARCH-003 full-detail block strip, which lives HERE post-split).
        let detail_md_raw =
            fs::read_to_string(golden.join("findings-detail.md")).expect("findings-detail.md");
        fs::write(
            golden.join("findings-detail.md"),
            common::mask_golden_report(&detail_md_raw, &fixture.to_string_lossy()),
        )
        .expect("write masked findings-detail.md");

        // Mask findings-detail.html in place (absolute-root substitution
        // only — the template carries no provenance surfaces, same as
        // report.html).
        let detail_html_raw =
            fs::read_to_string(golden.join("findings-detail.html")).expect("findings-detail.html");
        fs::write(
            golden.join("findings-detail.html"),
            common::mask_golden_html(&detail_html_raw, &fixture.to_string_lossy()),
        )
        .expect("write masked findings-detail.html");

        // SVGs + DOT + schema + the two Atlas pages: committed byte-exact;
        // assert they carry no provenance (no year-like RFC3339 fragments, no
        // absolute fixture path). The per-component detail pages join the
        // fixed set here — discovered from the fresh output so the lock
        // tracks the emission rule instead of a hardcoded count.
        let root_str = fixture.to_string_lossy().into_owned();
        let mut provenance_free = vec![
            "atlas-q2.html".to_string(),
            "atlas-weight.html".to_string(),
            "charts/include-graph.svg".to_string(),
            "charts/layer-file.svg".to_string(),
            "charts/layer-entry.svg".to_string(),
            "charts/component-coupling.svg".to_string(),
            "charts/hot-components.svg".to_string(),
            "charts/layer-workspace.svg".to_string(),
            "charts/layer-bands.svg".to_string(),
            "charts/layer-findings.svg".to_string(),
            "charts/graph.dot".to_string(),
            "metrics.schema.v1.8.json".to_string(),
        ];
        let mut component_pages: Vec<String> = fs::read_dir(golden.join("charts"))
            .expect("charts dir")
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| is_component_page_name(n) && n != "component-coupling.svg")
            .map(|n| format!("charts/{n}"))
            .collect();
        component_pages.sort();
        // Pinned-set gate (B6): regeneration adopts nothing implicitly —
        // a changed emission set fails here until EXPECTED_COMPONENT_PAGES
        // is updated explicitly.
        assert_pinned_page_set(tree.tag, &component_pages);
        provenance_free.extend(component_pages);
        for name in &provenance_free {
            let text = fs::read_to_string(golden.join(name)).expect(name);
            assert!(
                !text.contains("2026-") && !text.contains(&root_str),
                "{name} (tree '{}') must be provenance-free to be committed byte-exact",
                tree.tag
            );
        }

        // Sanity: the masked text artifacts must contain the <ROOT> sentinel
        // (report.html is exempt: it renders no root_path, so its mask is a
        // no-op — instead assert it is provenance-free outright).
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
        let html_text = fs::read_to_string(golden.join("report.html")).expect("report.html");
        assert!(
            !html_text.contains("2026-") && !html_text.contains(&root_str),
            "report.html (tree '{}') must be provenance-free to be committed masked-only",
            tree.tag
        );
        assert!(
            html_text.matches("<script>").count() == 1
                && html_text.matches("</script>").count() == 1,
            "report.html (tree '{}') must contain exactly one fixed enhancement script block",
            tree.tag
        );
        // findings-detail.md carries the per-finding Location/Message lines
        // that embed the scan root on trees with absolute-path findings —
        // the masker must have replaced it (same sentinel as report.md).
        // findings-detail.html is provenance-free outright (same bar as
        // report.html) plus exactly one fixed enhancement script block.
        let detail_md_text =
            fs::read_to_string(golden.join("findings-detail.md")).expect("findings-detail.md");
        assert!(
            !detail_md_text.contains(&root_str),
            "findings-detail.md (tree '{}') must not leak the absolute scan root",
            tree.tag
        );
        let detail_html_text =
            fs::read_to_string(golden.join("findings-detail.html")).expect("findings-detail.html");
        assert!(
            !detail_html_text.contains("2026-") && !detail_html_text.contains(&root_str),
            "findings-detail.html (tree '{}') must be provenance-free to be committed masked-only",
            tree.tag
        );
        assert!(
            detail_html_text.matches("<script>").count() == 1
                && detail_html_text.matches("</script>").count() == 1,
            "findings-detail.html (tree '{}') must contain exactly one fixed enhancement script block",
            tree.tag
        );
        // The snapshot tempdir (when any) is removed by `_snapshot_guard`'s
        // drop — even on panic — so no manual cleanup belongs here.
    }
}

/// Comparator (default gate, always runs): per tree, fresh pipeline output
/// into a temp dir, SAME masking, compare each of the fixed artifacts
/// PLUS every committed per-component detail page against the committed
/// goldens. On mismatch the assertion names the tree and lists
/// every differing artifact plus the regeneration command.
#[cfg(not(feature = "semantic-clang"))]
#[test]
fn report_matches_committed_goldens() {
    for tree in &GOLDEN_TREES {
        let (fixture, _snapshot_guard) = materialize_fixture(tree);
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
                "report.html" => {
                    let actual = String::from_utf8(actual).expect("report.html is UTF-8");
                    let expected = String::from_utf8(expected).expect("golden is UTF-8");
                    let actual = common::mask_golden_html(&actual, &root_str);
                    let expected = common::mask_golden_html(&expected, &root_str);
                    // Wave B1 resolved-edge delta (MEDIUM resolved ARCH-003 row
                    // under `semantic-clang` vs the goldens' default-build LOW
                    // row — see common::mask_golden_html_semantic), no-op on
                    // trees that emit no resolved `PERF-C-ARCH-003`.
                    common::mask_golden_html_semantic(&actual)
                        == common::mask_golden_html_semantic(&expected)
                }
                "findings-detail.md" => {
                    let actual = String::from_utf8(actual).expect("findings-detail.md is UTF-8");
                    let expected = String::from_utf8(expected).expect("golden is UTF-8");
                    common::mask_golden_report(&actual, &root_str)
                        == common::mask_golden_report(&expected, &root_str)
                }
                "findings-detail.html" => {
                    let actual = String::from_utf8(actual).expect("findings-detail.html is UTF-8");
                    let expected = String::from_utf8(expected).expect("golden is UTF-8");
                    let actual = common::mask_golden_html(&actual, &root_str);
                    let expected = common::mask_golden_html(&expected, &root_str);
                    common::mask_golden_html_semantic(&actual)
                        == common::mask_golden_html_semantic(&expected)
                }
                // Byte-exact for schema + SVGs (no provenance inside).
                _ => actual == expected,
            };
            if !equal {
                diffs.push(name.to_string());
            }
        }

        // Per-component detail pages: the PINNED set per tree (B6 — no
        // auto-adopt), compared byte-exact like the overview SVGs; a fresh
        // page with no golden twin (or vice versa) is a diff either way.
        let golden_pages = component_page_names(&golden);
        assert_pinned_page_set(tree.tag, &golden_pages);
        let fresh_pages = component_page_names(&out);
        assert_pinned_page_set(tree.tag, &fresh_pages);
        for name in &golden_pages {
            if !fresh_pages.contains(name) {
                diffs.push(format!("{name}: MISSING from fresh output"));
                continue;
            }
            if fs::read(golden.join(name)).expect("golden page readable")
                != fs::read(out.join(name)).expect("fresh page readable")
            {
                diffs.push(name.clone());
            }
        }
        for name in &fresh_pages {
            if !golden_pages.contains(name) {
                diffs.push(format!("{name}: no committed golden — regenerate the tree"));
            }
        }

        assert!(
            diffs.is_empty(),
            "[{}] fresh output diverged from committed goldens in: {diffs:?}\n\
             regenerate with:\n  \
             PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens",
            tree.tag
        );
        // The snapshot tempdir (when any) is removed by `_snapshot_guard`'s
        // drop — even on panic — so no manual cleanup belongs here.
    }
}
