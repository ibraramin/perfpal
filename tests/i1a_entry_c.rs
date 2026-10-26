//! I1a C entry-point detection + reachability sets (plan `map-build`,
//! founder-approved lib-roots variant — THIS BURST IS C ONLY).
//!
//! Contract (birdseye-map.md F1 + ADR-0008 third layer):
//! - per-language entry detection for C: `main` (HIGH) + handler heuristic
//!   `*handler*/*callback*/*dispatch*` case-insensitive (MEDIUM) + lib-roots
//!   (exported non-static functions in main-less files, MEDIUM) + honest
//!   `unknown-entry` stub (LOW) when nothing qualifies;
//! - reachability BFS over RESOLVED (semantic-fact-backed, MEDIUM+) call
//!   edges only — LOW-confidence name-based edges EXCLUDED;
//! - additive IR/schema fields only, serde-defaulted, sorted/deterministic.
//!
//! Strategy: synthetic [`FileUnit`]s through the REAL
//! [`aggregator::assemble`] (no libclang needed — semantic facts are
//! fabricated directly on the IR), plus one REAL-pipeline lock
//! (`cli::run_with` over temp C trees) proving end-to-end emission and
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
// Synthetic-IR builders (C language unless stated otherwise)
// ---------------------------------------------------------------------------

fn c_record(path: &str, name: &str, line: u32, is_static: Option<bool>) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        is_static,
        // v1.5 I1c: synthetic-fixture records never carry the backend-owned
        // `__name__` guard fact.
        has_main_guard: None,
        ..Default::default()
    }
}

fn direct_call(callee: &str, at_line: u32, resolved: Option<ResolvedSite>) -> CallEvent {
    CallEvent {
        meta: EventMeta {
            span: Span::point(at_line, 5),
            ..Default::default()
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

fn rust_unit(path: &str, name: &str) -> FileUnit {
    let mut unit = c_unit(path, vec![func(c_record(path, name, 1, None), vec![])]);
    unit.file.language = "rust".to_string();
    unit
}

fn python_unit(path: &str, name: &str) -> FileUnit {
    let mut unit = c_unit(path, vec![func(c_record(path, name, 1, None), vec![])]);
    unit.file.language = "python".to_string();
    unit
}

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-10-17T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

fn stat<'d>(
    doc: &'d perfpal::schema::MetricsDocument,
    name: &str,
) -> &'d perfpal::schema::FunctionStats {
    doc.stats
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("function {name} present in stats"))
}

// ---------------------------------------------------------------------------
// Lock 1 — main found (HIGH); non-entry C functions carry no entry kind
// ---------------------------------------------------------------------------

#[test]
fn c_main_detected_with_high_confidence() {
    let units = vec![c_unit(
        "app.c",
        vec![
            func(
                c_record("app.c", "main", 1, Some(false)),
                vec![direct_call("helper", 2, None)],
            ),
            func(c_record("app.c", "helper", 10, Some(false)), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    let main = stat(&doc, "main");
    assert_eq!(main.entry_kind.as_deref(), Some("main"));
    assert_eq!(main.entry_confidence.as_deref(), Some("HIGH"));
    assert_eq!(main.reachable_from_entry, Some(true));

    // A plain exported callee in a WITH-main file is NOT a lib-root.
    let helper = stat(&doc, "helper");
    assert_eq!(helper.entry_kind, None);
    assert_eq!(helper.entry_confidence, None);
}

// ---------------------------------------------------------------------------
// Lock 2 — handler heuristic (case-insensitive, MEDIUM); static excluded
// ---------------------------------------------------------------------------

#[test]
fn c_handler_heuristic_is_case_insensitive_medium() {
    let units = vec![c_unit(
        "net.c",
        vec![
            func(c_record("net.c", "NetDispatch", 1, Some(false)), vec![]),
            func(
                c_record("net.c", "on_PACKET_callback", 10, Some(false)),
                vec![],
            ),
            func(c_record("net.c", "myHandler", 20, Some(false)), vec![]),
            // Static + no handler fragment: not an entry of any kind.
            func(c_record("net.c", "poll_static", 30, Some(true)), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    for name in ["NetDispatch", "on_PACKET_callback", "myHandler"] {
        let entry = stat(&doc, name);
        assert_eq!(entry.entry_kind.as_deref(), Some("handler"), "{name}");
        assert_eq!(entry.entry_confidence.as_deref(), Some("MEDIUM"), "{name}");
        assert_eq!(entry.reachable_from_entry, Some(true), "{name}");
    }
    let plain = stat(&doc, "poll_static");
    assert_eq!(plain.entry_kind, None);
    assert_eq!(plain.reachable_from_entry, Some(false));
}

// ---------------------------------------------------------------------------
// Lock 3 — lib-root fallback (exported, main-less files only, MEDIUM)
// ---------------------------------------------------------------------------

#[test]
fn c_lib_root_fallback_for_main_less_files() {
    let units = vec![
        c_unit(
            "app.c",
            vec![
                func(c_record("app.c", "main", 1, Some(false)), vec![]),
                // Exported BUT in a with-main file: not a lib-root.
                func(c_record("app.c", "app_helper", 10, Some(false)), vec![]),
            ],
        ),
        c_unit(
            "lib.c",
            vec![
                func(c_record("lib.c", "api_init", 1, Some(false)), vec![]),
                // Static: never a lib-root even in a main-less file.
                func(c_record("lib.c", "lib_util", 10, Some(true)), vec![]),
            ],
        ),
    ];
    let doc = assemble(&units, &[], &prov());

    let api = stat(&doc, "api_init");
    assert_eq!(api.entry_kind.as_deref(), Some("lib-root"));
    assert_eq!(api.entry_confidence.as_deref(), Some("MEDIUM"));
    assert_eq!(api.reachable_from_entry, Some(true));

    assert_eq!(stat(&doc, "app_helper").entry_kind, None);
    assert_eq!(stat(&doc, "lib_util").entry_kind, None);
    assert_eq!(stat(&doc, "main").entry_kind.as_deref(), Some("main"));
}

// ---------------------------------------------------------------------------
// Lock 4 — honest unknown-entry stub (LOW) when nothing qualifies
// ---------------------------------------------------------------------------

#[test]
fn c_unknown_entry_stub_when_nothing_qualifies() {
    // One static-only function: no main, no handler name, no exports — the
    // stub fires so reachability stays total instead of silently empty.
    let units = vec![c_unit(
        "lonely.c",
        vec![func(c_record("lonely.c", "work", 1, Some(true)), vec![])],
    )];
    let doc = assemble(&units, &[], &prov());

    let work = stat(&doc, "work");
    assert_eq!(work.entry_kind.as_deref(), Some("unknown-entry"));
    assert_eq!(work.entry_confidence.as_deref(), Some("LOW"));
    assert_eq!(work.reachable_from_entry, Some(true));
}

// ---------------------------------------------------------------------------
// Lock 5 — LOW-confidence name-based edges EXCLUDED from reachability;
// resolved (MEDIUM+) edges INCLUDED (positive control via fabricated fact)
// ---------------------------------------------------------------------------

#[test]
fn c_low_confidence_name_edges_excluded_from_reachability() {
    // `unique_helper`: called by main with a globally-unique name — the
    // name-based fallback WOULD resolve it (LOW), but reachability must not
    // follow it. `remote_work`: semantic-fact-backed (MEDIUM+) — followed.
    let remote_span = Span::point(1, 1);
    let units = vec![
        c_unit(
            "app.c",
            vec![
                func(
                    c_record("app.c", "main", 1, Some(false)),
                    vec![
                        direct_call("unique_helper", 2, None),
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
                func(c_record("app.c", "unique_helper", 10, Some(false)), vec![]),
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
    ];
    let doc = assemble(&units, &[], &prov());

    // LOW name-based edge excluded: reachable stays false despite the
    // unambiguous unique-name call from main.
    let helper = stat(&doc, "unique_helper");
    assert_eq!(
        helper.reachable_from_entry,
        Some(false),
        "LOW-confidence name-based edge must not confer reachability"
    );
    // Resolved MEDIUM+ edge included (positive control).
    let remote = stat(&doc, "remote_work");
    assert_eq!(remote.reachable_from_entry, Some(true));
    assert_eq!(remote.entry_kind.as_deref(), Some("lib-root"));
}

// ---------------------------------------------------------------------------
// Language gates — every language's entries are claimed by its own pass
// (C since I1a, Rust since I1b, Python since I1c — see
// `tests/i1c_entry_python.rs` for the Python-side locks).
// ---------------------------------------------------------------------------

#[test]
fn language_gates_claim_all_three_languages() {
    let units = vec![
        c_unit(
            "app.c",
            vec![func(c_record("app.c", "main", 1, Some(false)), vec![])],
        ),
        // A Rust `main` IS claimed by the Rust entry pass since I1b.
        rust_unit("src/main.rs", "main"),
        // A Python `main`-named def IS claimed by the Python entry pass
        // since I1c — as a guard-less, non-underscore lib-root (Python has
        // no `main`-HIGH rule; HIGH is reserved for guarded scripts).
        python_unit("app.py", "main"),
    ];
    let doc = assemble(&units, &[], &prov());

    assert_eq!(stat(&doc, "main").entry_kind.as_deref(), Some("main"));
    let rs = doc
        .stats
        .functions
        .iter()
        .find(|f| f.path == "src/main.rs")
        .expect("rust function present");
    assert_eq!(rs.entry_kind.as_deref(), Some("main"));
    assert_eq!(rs.entry_confidence.as_deref(), Some("HIGH"));
    assert_eq!(rs.reachable_from_entry, Some(true));
    let py = doc
        .stats
        .functions
        .iter()
        .find(|f| f.path == "app.py")
        .expect("python function present");
    assert_eq!(py.entry_kind.as_deref(), Some("lib-root"));
    assert_eq!(py.entry_confidence.as_deref(), Some("MEDIUM"));
    assert_eq!(py.reachable_from_entry, Some(true));
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
        std::env::temp_dir().join(format!("perfpal-i1a-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

#[test]
fn c_entry_pipeline_emits_and_double_scan_is_byte_identical() {
    let root = temp_dir("tree");
    fs::write(
        root.join("main.c"),
        "int helper(int x) { return x + 1; }\nint main(void) { return helper(1); }\n",
    )
    .expect("write main.c");
    fs::write(
        root.join("lib.c"),
        "int api_init(void) { return 0; }\nstatic int lib_util(void) { return 1; }\n",
    )
    .expect("write lib.c");

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
    // a default build, so reachability is entries-only here).
    assert_eq!(doc1["schema_version"], "1.8");
    let by_name = |name: &str| {
        doc1["stats"]["functions"]
            .as_array()
            .expect("functions[]")
            .iter()
            .find(|f| f["name"] == name)
            .unwrap_or_else(|| panic!("function {name} present"))
            .clone()
    };
    let main = by_name("main");
    assert_eq!(main["entry_kind"], "main");
    assert_eq!(main["entry_confidence"], "HIGH");
    assert_eq!(main["reachable_from_entry"], true);
    let api = by_name("api_init");
    assert_eq!(api["entry_kind"], "lib-root");
    assert_eq!(api["entry_confidence"], "MEDIUM");
    // No semantic facts without libclang: the uniquely-named callee is NOT
    // reachable — the LOW-excluded caption holds end to end.
    let helper = by_name("helper");
    assert!(helper.get("entry_kind").is_none());
    assert_eq!(helper["reachable_from_entry"], false);
    let util = by_name("lib_util");
    assert!(util.get("entry_kind").is_none());

    fs::remove_dir_all(&root).ok();
}
