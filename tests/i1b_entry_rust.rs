//! I1b Rust entry-point detection + reachability sets (plan `map-build`,
//! founder-approved option 1: entries + honest-empty reachability).
//!
//! Contract (birdseye-map.md F1 + ADR-0008 third layer, mirror of I1a
//! `tests/i1a_entry_c.rs`):
//! - per-language entry detection for Rust: `main` (HIGH) + handler
//!   heuristic `*handler*/*callback*/*dispatch*` case-insensitive (MEDIUM) +
//!   bin/lib-roots (exported-looking functions in main-less, non-bin files —
//!   `src/lib.rs` or any other non-`main.rs` file — MEDIUM; bin-owned
//!   `src/main.rs` files contribute no lib-roots) + honest `unknown-entry`
//!   stub (LOW) when nothing qualifies;
//! - reachability BFS over RESOLVED (semantic-fact-backed, MEDIUM+) call
//!   edges only — intra-file INCLUDED — LOW-confidence name-based edges
//!   EXCLUDED. Rust is syntactic-tier only in this milestone, so the
//!   resolved set is honestly EMPTY in production and reachability is
//!   entries-only by construction (option 3 — falling back to LOW edges —
//!   was explicitly rejected);
//! - additive IR/schema fields only, serde-defaulted, sorted/deterministic;
//! - C behavior byte-untouched (C assertions re-locked verbatim here).
//!
//! Strategy: synthetic [`FileUnit`]s through the REAL
//! [`aggregator::assemble`] (no libclang needed — semantic facts are
//! fabricated directly on the IR), plus one REAL-pipeline lock
//! (`cli::run_with` over a temp Rust tree) proving end-to-end emission and
//! double-scan byte-identity.

use std::ffi::OsString;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::cli::run_with;
use perfpal::ir::{
    CallEvent, CallSemantic, CalleeKind, EventMeta, FileRecord, FunctionRecord, ParseHealth,
    ParseStatus, ResolvedSite, Span, StatementRole,
};
use perfpal::scanner::{FileUnit, FunctionUnit};

// ---------------------------------------------------------------------------
// Synthetic-IR builders (Rust language unless stated otherwise)
// ---------------------------------------------------------------------------

fn rust_record(path: &str, name: &str, line: u32) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        // The Rust backend never populates `is_static` (no surfaced
        // static-linkage concept) — every Rust function counts as
        // exported-looking for lib-root purposes.
        is_static: None,
        ..Default::default()
    }
}

fn c_record(path: &str, name: &str, line: u32, is_static: Option<bool>) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        is_static,
        ..Default::default()
    }
}

fn direct_call(callee: &str, at_line: u32, resolved: Option<ResolvedSite>) -> CallEvent {
    CallEvent {
        meta: EventMeta {
            span: Span::point(at_line, 5),
            ..EventMeta::default()
        },
        callee_name: callee.to_string(),
        callee_kind: CalleeKind::Direct,
        callee_path: None,
        is_macro: false,
        args: vec![],
        statement_role: StatementRole::Unknown,
        conditional_guarded: false,
        semantic: resolved.map(|site| CallSemantic {
            resolved_definition: Some(site),
            fopened_handle: false,
            literal_args: vec![],
        }),
        receiver_root: None,
    }
}

fn func(record: FunctionRecord, calls: Vec<CallEvent>) -> FunctionUnit {
    FunctionUnit {
        record,
        calls,
        ..Default::default()
    }
}

fn rust_unit(path: &str, funcs: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "rust".to_string(),
            ..Default::default()
        },
        parse_health: ParseHealth {
            status: ParseStatus::Ok,
            ..Default::default()
        },
        functions: funcs,
        ..Default::default()
    }
}

fn c_unit(path: &str, funcs: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            ..Default::default()
        },
        parse_health: ParseHealth {
            status: ParseStatus::Ok,
            ..Default::default()
        },
        functions: funcs,
        ..Default::default()
    }
}

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-10-17T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

/// Path-qualified stat lookup (a mixed C+Rust corpus can hold two `main`s).
fn stat_at<'d>(
    doc: &'d perfpal::schema::MetricsDocument,
    path: &str,
    name: &str,
) -> &'d perfpal::schema::FunctionStats {
    doc.stats
        .functions
        .iter()
        .find(|f| f.path == path && f.name == name)
        .unwrap_or_else(|| panic!("function {path}#{name} present in stats"))
}

// ---------------------------------------------------------------------------
// Lock 1 — main found (HIGH); bin-owned non-entries carry no entry kind
// ---------------------------------------------------------------------------

#[test]
fn rust_main_detected_with_high_confidence() {
    let units = vec![rust_unit(
        "src/main.rs",
        vec![
            func(
                rust_record("src/main.rs", "main", 1),
                vec![direct_call("helper", 2, None)],
            ),
            func(rust_record("src/main.rs", "helper", 10), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    let main = stat_at(&doc, "src/main.rs", "main");
    assert_eq!(main.entry_kind.as_deref(), Some("main"));
    assert_eq!(main.entry_confidence.as_deref(), Some("HIGH"));
    assert_eq!(main.reachable_from_entry, Some(true));

    // A plain helper in the bin-owned WITH-main file is NOT a lib-root —
    // the bin surface is reachable from `main`, not an entry itself.
    let helper = stat_at(&doc, "src/main.rs", "helper");
    assert_eq!(helper.entry_kind, None);
    assert_eq!(helper.entry_confidence, None);
    // No semantic facts in this corpus: the uniquely-named callee is NOT
    // reachable — reachability is entries-only here.
    assert_eq!(helper.reachable_from_entry, Some(false));
}

// ---------------------------------------------------------------------------
// Lock 2 — bin/lib-root classification + handler precedence
// ---------------------------------------------------------------------------

#[test]
fn rust_bin_and_lib_root_classification() {
    let units = vec![
        rust_unit(
            "src/main.rs",
            vec![
                func(rust_record("src/main.rs", "main", 1), vec![]),
                // Exported BUT in a bin-owned, with-main file: not a lib-root.
                func(rust_record("src/main.rs", "bin_helper", 10), vec![]),
            ],
        ),
        rust_unit(
            "src/lib.rs",
            vec![
                func(rust_record("src/lib.rs", "api_init", 1), vec![]),
                // Handler fragment wins over the lib-root fallback.
                func(rust_record("src/lib.rs", "on_callback", 10), vec![]),
            ],
        ),
        // Non-main, non-bin, main-less file: lib-root territory too.
        rust_unit(
            "src/util.rs",
            vec![func(rust_record("src/util.rs", "quote_price", 1), vec![])],
        ),
    ];
    let doc = assemble(&units, &[], &prov());

    for (path, name) in [("src/lib.rs", "api_init"), ("src/util.rs", "quote_price")] {
        let entry = stat_at(&doc, path, name);
        assert_eq!(
            entry.entry_kind.as_deref(),
            Some("lib-root"),
            "{path}#{name}"
        );
        assert_eq!(
            entry.entry_confidence.as_deref(),
            Some("MEDIUM"),
            "{path}#{name}"
        );
        assert_eq!(entry.reachable_from_entry, Some(true), "{path}#{name}");
    }
    let handler = stat_at(&doc, "src/lib.rs", "on_callback");
    assert_eq!(handler.entry_kind.as_deref(), Some("handler"));
    assert_eq!(handler.entry_confidence.as_deref(), Some("MEDIUM"));
    assert_eq!(handler.reachable_from_entry, Some(true));

    assert_eq!(stat_at(&doc, "src/main.rs", "bin_helper").entry_kind, None);
    assert_eq!(
        stat_at(&doc, "src/main.rs", "main").entry_kind.as_deref(),
        Some("main")
    );
}

// ---------------------------------------------------------------------------
// Lock 3 — honest unknown-entry stub (LOW) when nothing qualifies
// ---------------------------------------------------------------------------

#[test]
fn rust_unknown_entry_stub_when_nothing_qualifies() {
    // A bin-owned file with NO `main` and no handler-named function: no
    // lib-roots (bin exclusion), no main, no handler — the stub fires so
    // reachability stays total instead of silently empty.
    let units = vec![rust_unit(
        "src/main.rs",
        vec![func(rust_record("src/main.rs", "work", 1), vec![])],
    )];
    let doc = assemble(&units, &[], &prov());

    let work = stat_at(&doc, "src/main.rs", "work");
    assert_eq!(work.entry_kind.as_deref(), Some("unknown-entry"));
    assert_eq!(work.entry_confidence.as_deref(), Some("LOW"));
    assert_eq!(work.reachable_from_entry, Some(true));
}

// ---------------------------------------------------------------------------
// Lock 4 — LOW-confidence name-based edges EXCLUDED from reachability;
// resolved (MEDIUM+) edges INCLUDED, intra-file included (positive control
// via fabricated fact). In production the resolved set is honestly EMPTY:
// Rust is syntactic-tier only (rust/arch.rs:490-493 — every Rust edge is
// the LOW name-based fallback), so reachability == entries by construction.
// ---------------------------------------------------------------------------

#[test]
fn rust_low_confidence_name_edges_excluded_from_reachability() {
    // `unique_helper`: called by main with a globally-unique name — the
    // name-based fallback WOULD resolve it (LOW), but reachability must not
    // follow it. `sibling_work`: semantic-fact-backed INTRA-file call
    // (MEDIUM+) — followed, proving resolved edges include same-file pairs
    // (unlike `cross_file_call_edges`, which excludes them).
    let sibling_span = Span::point(20, 1);
    let units = vec![rust_unit(
        "src/main.rs",
        vec![
            func(
                rust_record("src/main.rs", "main", 1),
                vec![
                    direct_call("unique_helper", 2, None),
                    direct_call(
                        "sibling_work",
                        3,
                        Some(ResolvedSite {
                            path: "src/main.rs".to_string(),
                            span: sibling_span,
                        }),
                    ),
                ],
            ),
            func(rust_record("src/main.rs", "unique_helper", 10), vec![]),
            func(
                FunctionRecord {
                    span: sibling_span,
                    ..rust_record("src/main.rs", "sibling_work", 20)
                },
                vec![],
            ),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    // LOW name-based edge excluded: reachable stays false despite the
    // unambiguous unique-name call from main.
    let helper = stat_at(&doc, "src/main.rs", "unique_helper");
    assert_eq!(
        helper.reachable_from_entry,
        Some(false),
        "LOW-confidence name-based edge must not confer reachability"
    );
    // Resolved MEDIUM+ edge included — intra-file (positive control).
    let sibling = stat_at(&doc, "src/main.rs", "sibling_work");
    assert_eq!(sibling.reachable_from_entry, Some(true));
    assert_eq!(sibling.entry_kind, None, "reached, not an entry");
}

// ---------------------------------------------------------------------------
// Lock 5 — C gate untouched: C output identical with Rust units present,
// and each language's `main` is claimed by its own detector
// ---------------------------------------------------------------------------

#[test]
fn c_gate_untouched_by_rust_pass() {
    // C side mirrors I1a lock 5 verbatim (unique-name LOW call excluded,
    // semantic-fact MEDIUM+ call followed).
    let remote_span = Span::point(1, 1);
    let units = vec![
        c_unit(
            "app.c",
            vec![
                func(
                    c_record("app.c", "main", 1, Some(false)),
                    vec![
                        direct_call("unique_c_helper", 2, None),
                        direct_call(
                            "remote_work",
                            3,
                            Some(ResolvedSite {
                                path: "other.c".to_string(),
                                span: remote_span,
                            }),
                        ),
                    ],
                ),
                func(
                    c_record("app.c", "unique_c_helper", 10, Some(false)),
                    vec![],
                ),
            ],
        ),
        c_unit(
            "other.c",
            vec![func(
                FunctionRecord {
                    span: remote_span,
                    ..c_record("other.c", "remote_work", 1, Some(false))
                },
                vec![],
            )],
        ),
        // Rust side: a bin `main` plus a lib-root. Neither perturbs C.
        rust_unit(
            "src/main.rs",
            vec![func(rust_record("src/main.rs", "main", 1), vec![])],
        ),
        rust_unit(
            "src/lib.rs",
            vec![func(rust_record("src/lib.rs", "api_init", 1), vec![])],
        ),
    ];
    let doc = assemble(&units, &[], &prov());

    // C output identical to the I1a-only corpus.
    let c_helper = stat_at(&doc, "app.c", "unique_c_helper");
    assert_eq!(c_helper.reachable_from_entry, Some(false));
    let remote = stat_at(&doc, "other.c", "remote_work");
    assert_eq!(remote.reachable_from_entry, Some(true));
    assert_eq!(remote.entry_kind.as_deref(), Some("lib-root"));
    assert_eq!(
        stat_at(&doc, "app.c", "main").entry_kind.as_deref(),
        Some("main")
    );
    // Each language's `main` is claimed by its own detector.
    assert_eq!(
        stat_at(&doc, "src/main.rs", "main").entry_kind.as_deref(),
        Some("main")
    );
    assert_eq!(
        stat_at(&doc, "src/main.rs", "main")
            .entry_confidence
            .as_deref(),
        Some("HIGH")
    );
    // Rust lib-root coexists with the C pass.
    assert_eq!(
        stat_at(&doc, "src/lib.rs", "api_init")
            .entry_kind
            .as_deref(),
        Some("lib-root")
    );
}

// ---------------------------------------------------------------------------
// Lock 6 — determinism: shuffled input ⇒ identical `stats.functions[]`
// ---------------------------------------------------------------------------

#[test]
fn rust_entry_determinism_under_shuffled_input() {
    let bin = rust_unit(
        "src/main.rs",
        vec![
            func(rust_record("src/main.rs", "main", 1), vec![]),
            func(rust_record("src/main.rs", "bin_helper", 10), vec![]),
        ],
    );
    let lib = rust_unit(
        "src/lib.rs",
        vec![func(rust_record("src/lib.rs", "api_init", 1), vec![])],
    );
    let c = c_unit(
        "app.c",
        vec![func(c_record("app.c", "main", 1, Some(false)), vec![])],
    );

    let fwd = assemble(&[bin.clone(), lib.clone(), c.clone()], &[], &prov());
    let rev = assemble(&[c, lib, bin], &[], &prov());
    assert_eq!(fwd.stats.functions, rev.stats.functions);
}

// ---------------------------------------------------------------------------
// Real-pipeline lock — end-to-end emission + double-scan byte-identity
// ---------------------------------------------------------------------------

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("perfpal-i1b-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

#[test]
fn rust_entry_pipeline_emits_and_double_scan_is_byte_identical() {
    let root = temp_dir("tree");
    let src = root.join("src");
    fs::create_dir_all(&src).expect("create src dir");
    fs::write(
        src.join("main.rs"),
        "fn helper() -> i32 { 1 }\nfn main() { println!(\"{}\", helper()); }\n",
    )
    .expect("write main.rs");
    fs::write(src.join("lib.rs"), "pub fn api_init() -> i32 { 0 }\n").expect("write lib.rs");

    let run = |tag: &str| {
        let out = temp_dir(tag).join("perfpal-report");
        let code = run_with(argv(&[
            "--output",
            out.to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
        ]));
        assert_eq!(code, 0, "pipeline must exit 0");
        fs::read_to_string(out.join("metrics.json")).expect("metrics.json present")
    };
    let raw1 = run("scan1");
    let raw2 = run("scan2");

    // Mask run-varying provenance, then require byte-identity of the
    // canonical form (the double-scan determinism contract, A3).
    let mut doc1: serde_json::Value = serde_json::from_str(&raw1).expect("scan 1 parses");
    let mut doc2: serde_json::Value = serde_json::from_str(&raw2).expect("scan 2 parses");
    for doc in [&mut doc1, &mut doc2] {
        doc["scan"]["started_at"] = serde_json::Value::String("<MASKED>".to_string());
        doc["scan"]["duration_ms"] = serde_json::Value::Null;
    }
    assert_eq!(doc1, doc2, "double-scan documents must be identical");

    // End-to-end entry assertions on the real pipeline (no resolver facts in
    // a default build, so reachability is entries-only here — the honest
    // emptiness of `resolved_rust_edges`: Rust is syntactic-tier only,
    // rust/arch.rs:490-493).
    assert_eq!(doc1["schema_version"], "1.5");
    let by_path_name = |path: &str, name: &str| {
        doc1["stats"]["functions"]
            .as_array()
            .expect("functions[]")
            .iter()
            .find(|f| f["path"] == path && f["name"] == name)
            .unwrap_or_else(|| panic!("function {path}#{name} present"))
            .clone()
    };
    let main = by_path_name("src/main.rs", "main");
    assert_eq!(main["entry_kind"], "main");
    assert_eq!(main["entry_confidence"], "HIGH");
    assert_eq!(main["reachable_from_entry"], true);
    let api = by_path_name("src/lib.rs", "api_init");
    assert_eq!(api["entry_kind"], "lib-root");
    assert_eq!(api["entry_confidence"], "MEDIUM");
    assert_eq!(api["reachable_from_entry"], true);
    // No semantic facts without libclang: the uniquely-named callee is NOT
    // reachable — the LOW-excluded caption holds end to end.
    let helper = by_path_name("src/main.rs", "helper");
    assert!(helper.get("entry_kind").is_none());
    assert_eq!(helper["reachable_from_entry"], false);

    fs::remove_dir_all(&root).ok();
}
