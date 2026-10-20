//! I1c Python entry-point detection + reachability sets (plan `map-build`,
//! founder-approved option 1: backend guard flag + aggregator mirror).
//!
//! Contract (birdseye-map.md F1 + ADR-0008 third layer, mirror of I1a
//! `tests/i1a_entry_c.rs` and I1b `tests/i1b_entry_rust.rs` — this completes
//! the I1 per-language sequence):
//! - per-language entry detection for Python: guarded `<module>` scripts
//!   (`has_main_guard == Some(true)` — the backend's exact
//!   `__name__ == "__main__"` idiom fact, HIGH) + handler heuristic
//!   `*handler*/*callback*/*dispatch*` case-insensitive (MEDIUM) +
//!   lib-roots (non-underscore-prefixed `def`s in guard-less files, MEDIUM;
//!   `_private` defs and the synthetic `<module>` record never qualify) +
//!   honest `unknown-entry` stub (LOW) when nothing qualifies;
//! - reachability BFS over RESOLVED (semantic-fact-backed, MEDIUM+) call
//!   edges only — intra-file INCLUDED — LOW-confidence name-based edges
//!   EXCLUDED. Python is syntactic-tier only in this milestone (the backend
//!   emits no semantic facts), so the resolved set is honestly EMPTY in
//!   production and reachability is entries-only by construction (option 3 —
//!   falling back to LOW edges — was explicitly rejected);
//! - additive IR/schema fields only, serde-defaulted, sorted/deterministic;
//! - C and Rust behavior byte-untouched (their assertions re-locked verbatim
//!   here).
//!
//! Strategy: synthetic [`FileUnit`]s through the REAL
//! [`aggregator::assemble`] (no libclang needed — semantic facts are
//! fabricated directly on the IR), plus one REAL-pipeline lock
//! (`cli::run_with` over a temp Python tree) proving end-to-end emission and
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
// Synthetic-IR builders (Python language unless stated otherwise)
// ---------------------------------------------------------------------------

fn py_def(path: &str, name: &str, line: u32) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        // Real `def`s never carry the backend-owned guard fact (I1c keeps it
        // on the synthetic `<module>` record only).
        has_main_guard: None,
        ..Default::default()
    }
}

fn py_module(path: &str, line: u32, guard: bool) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#<module>#{line}:1"),
        name: "<module>".to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        is_module_scope: true,
        has_main_guard: Some(guard),
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
        has_main_guard: None,
        ..Default::default()
    }
}

fn rust_record(path: &str, name: &str, line: u32) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        // The Rust backend never populates `is_static`, and the `__name__`
        // guard fact is Python-backend-owned — Rust fixtures leave both `None`.
        is_static: None,
        has_main_guard: None,
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

fn py_unit(path: &str, funcs: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "python".to_string(),
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

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-10-17T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

/// Path-qualified stat lookup (a mixed corpus can hold same-named defs).
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
// Lock 1 — guard-script HIGH; plain defs in the guarded file are non-entries
// ---------------------------------------------------------------------------

#[test]
fn python_guard_script_detected_with_high_confidence() {
    let units = vec![py_unit(
        "app.py",
        vec![
            func(py_module("app.py", 1, true), vec![]),
            func(py_def("app.py", "helper", 5), vec![]),
            // Handler-named defs keep their own entry even in guarded files
            // (script precedence claims only the module record itself).
            func(py_def("app.py", "on_callback", 10), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    let module = stat_at(&doc, "app.py", "<module>");
    assert_eq!(module.entry_kind.as_deref(), Some("script"));
    assert_eq!(module.entry_confidence.as_deref(), Some("HIGH"));
    assert_eq!(module.reachable_from_entry, Some(true));

    // A plain def in the guarded file is NOT a lib-root — the file's surface
    // is the script entry, not its defs.
    let helper = stat_at(&doc, "app.py", "helper");
    assert_eq!(helper.entry_kind, None);
    assert_eq!(helper.entry_confidence, None);
    // No semantic facts in this corpus: the def is NOT reachable —
    // reachability is entries-only here.
    assert_eq!(helper.reachable_from_entry, Some(false));

    let handler = stat_at(&doc, "app.py", "on_callback");
    assert_eq!(handler.entry_kind.as_deref(), Some("handler"));
    assert_eq!(handler.entry_confidence.as_deref(), Some("MEDIUM"));
    assert_eq!(handler.reachable_from_entry, Some(true));
}

// ---------------------------------------------------------------------------
// Lock 2 — handler heuristic (MEDIUM) in guard-less files
// ---------------------------------------------------------------------------

#[test]
fn python_handler_heuristic_in_guard_less_files() {
    let units = vec![py_unit(
        "svc.py",
        vec![
            func(py_module("svc.py", 1, false), vec![]),
            func(py_def("svc.py", "request_Handler", 5), vec![]),
            func(py_def("svc.py", "dispatch_queue", 10), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    for name in ["request_Handler", "dispatch_queue"] {
        let entry = stat_at(&doc, "svc.py", name);
        assert_eq!(entry.entry_kind.as_deref(), Some("handler"), "{name}");
        assert_eq!(entry.entry_confidence.as_deref(), Some("MEDIUM"), "{name}");
        assert_eq!(entry.reachable_from_entry, Some(true), "{name}");
    }
    // The guard-less `<module>` record itself is not a `def` — never an entry.
    assert_eq!(stat_at(&doc, "svc.py", "<module>").entry_kind, None);
}

// ---------------------------------------------------------------------------
// Lock 3 — lib-root fallback (MEDIUM); underscore-prefixed defs excluded
// ---------------------------------------------------------------------------

#[test]
fn python_lib_root_fallback_excludes_underscore_defs() {
    let units = vec![py_unit(
        "lib.py",
        vec![
            func(py_module("lib.py", 1, false), vec![]),
            func(py_def("lib.py", "api_init", 5), vec![]),
            func(py_def("lib.py", "_private", 10), vec![]),
            func(py_def("lib.py", "__dunder__", 15), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    let api = stat_at(&doc, "lib.py", "api_init");
    assert_eq!(api.entry_kind.as_deref(), Some("lib-root"));
    assert_eq!(api.entry_confidence.as_deref(), Some("MEDIUM"));
    assert_eq!(api.reachable_from_entry, Some(true));

    // Underscore-prefixed defs NEVER qualify — not even as lib-roots.
    for name in ["_private", "__dunder__"] {
        let hidden = stat_at(&doc, "lib.py", name);
        assert_eq!(hidden.entry_kind, None, "{name}");
        assert_eq!(hidden.entry_confidence, None, "{name}");
        assert_eq!(hidden.reachable_from_entry, Some(false), "{name}");
    }
    // Guard-less `<module>` is not a `def` — no lib-root for it either.
    let module = stat_at(&doc, "lib.py", "<module>");
    assert_eq!(module.entry_kind, None);
    assert_eq!(module.reachable_from_entry, Some(false));
}

// ---------------------------------------------------------------------------
// Lock 4 — honest unknown-entry stub (LOW) when nothing qualifies
// ---------------------------------------------------------------------------

#[test]
fn python_unknown_entry_stub_when_nothing_qualifies() {
    // A guard-less file holding ONLY an underscore-prefixed def: no script,
    // no handler, no lib-root — the stub fires so reachability stays total
    // instead of silently empty.
    let units = vec![py_unit(
        "quiet.py",
        vec![
            func(py_module("quiet.py", 1, false), vec![]),
            func(py_def("quiet.py", "_private", 5), vec![]),
        ],
    )];
    let doc = assemble(&units, &[], &prov());

    for name in ["<module>", "_private"] {
        let stub = stat_at(&doc, "quiet.py", name);
        assert_eq!(stub.entry_kind.as_deref(), Some("unknown-entry"), "{name}");
        assert_eq!(stub.entry_confidence.as_deref(), Some("LOW"), "{name}");
        assert_eq!(stub.reachable_from_entry, Some(true), "{name}");
    }
}

// ---------------------------------------------------------------------------
// Lock 5 — LOW-confidence name-based edges EXCLUDED from reachability;
// resolved (MEDIUM+) edges INCLUDED, intra-file included (positive control
// via fabricated fact). In production the resolved set is honestly EMPTY:
// the Python backend emits no semantic facts (syntactic tier only), so
// reachability == entries by construction.
// ---------------------------------------------------------------------------

#[test]
fn python_low_confidence_name_edges_excluded_from_reachability() {
    // `unique_helper`: called from the script module with a globally-unique
    // name — the name-based fallback WOULD resolve it (LOW), but
    // reachability must not follow it. `sibling_work`: semantic-fact-backed
    // INTRA-file call (MEDIUM+) — followed, proving resolved edges include
    // same-file pairs (unlike `cross_file_call_edges`, which excludes them).
    let sibling_span = Span::point(20, 1);
    let units = vec![py_unit(
        "app.py",
        vec![
            func(
                py_module("app.py", 1, true),
                vec![
                    direct_call("unique_helper", 2, None),
                    direct_call(
                        "sibling_work",
                        3,
                        Some(ResolvedSite {
                            path: "app.py".to_string(),
                            span: sibling_span,
                        }),
                    ),
                ],
            ),
            func(py_def("app.py", "unique_helper", 10), vec![]),
            func(
                FunctionRecord {
                    span: sibling_span,
                    ..py_def("app.py", "sibling_work", 20)
                },
                vec![],
            ),
        ],
    )];
    // The guarded module is a script entry; the fabricated fact must resolve
    // even though the callee def is a non-entry.
    let doc = assemble(&units, &[], &prov());

    // LOW name-based edge excluded: reachable stays false despite the
    // unambiguous unique-name call from the script entry.
    let helper = stat_at(&doc, "app.py", "unique_helper");
    assert_eq!(
        helper.reachable_from_entry,
        Some(false),
        "LOW-confidence name-based edge must not confer reachability"
    );
    // Resolved MEDIUM+ edge included — intra-file (positive control).
    let sibling = stat_at(&doc, "app.py", "sibling_work");
    assert_eq!(sibling.reachable_from_entry, Some(true));
    assert_eq!(sibling.entry_kind, None, "reached, not an entry");
    // Entries-only otherwise: the script entry reaches itself (distance 0).
    assert_eq!(
        stat_at(&doc, "app.py", "<module>").reachable_from_entry,
        Some(true)
    );
}

// ---------------------------------------------------------------------------
// Lock 6 — C + Rust gates untouched: their outputs are identical with Python
// units present, and each language's entries are claimed by its own detector
// ---------------------------------------------------------------------------

#[test]
fn c_and_rust_gates_untouched_by_python_pass() {
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
        // Rust side mirrors I1b lock 2: a bin `main` plus a lib-root. Neither
        // perturbs C or Python.
        rust_unit(
            "src/main.rs",
            vec![func(rust_record("src/main.rs", "main", 1), vec![])],
        ),
        rust_unit(
            "src/lib.rs",
            vec![func(rust_record("src/lib.rs", "api_init", 1), vec![])],
        ),
        // Python side: a guarded script plus a guard-less lib file.
        py_unit(
            "app.py",
            vec![
                func(py_module("app.py", 1, true), vec![]),
                func(py_def("app.py", "helper", 5), vec![]),
            ],
        ),
        py_unit(
            "lib.py",
            vec![
                func(py_module("lib.py", 1, false), vec![]),
                func(py_def("lib.py", "api_init", 5), vec![]),
            ],
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
    // Each language's entries claimed by its own detector.
    assert_eq!(
        stat_at(&doc, "src/main.rs", "main").entry_kind.as_deref(),
        Some("main")
    );
    assert_eq!(
        stat_at(&doc, "src/lib.rs", "api_init")
            .entry_kind
            .as_deref(),
        Some("lib-root")
    );
    assert_eq!(
        stat_at(&doc, "app.py", "<module>").entry_kind.as_deref(),
        Some("script")
    );
    assert_eq!(
        stat_at(&doc, "lib.py", "api_init").entry_kind.as_deref(),
        Some("lib-root")
    );
}

// ---------------------------------------------------------------------------
// Lock 7 — determinism: shuffled input ⇒ identical `stats.functions[]`
// ---------------------------------------------------------------------------

#[test]
fn python_entry_determinism_under_shuffled_input() {
    let guarded = py_unit(
        "app.py",
        vec![
            func(py_module("app.py", 1, true), vec![]),
            func(py_def("app.py", "helper", 5), vec![]),
        ],
    );
    let lib = py_unit(
        "lib.py",
        vec![
            func(py_module("lib.py", 1, false), vec![]),
            func(py_def("lib.py", "api_init", 5), vec![]),
        ],
    );
    let c = c_unit(
        "app.c",
        vec![func(c_record("app.c", "main", 1, Some(false)), vec![])],
    );

    let fwd = assemble(&[guarded.clone(), lib.clone(), c.clone()], &[], &prov());
    let rev = assemble(&[c, lib, guarded], &[], &prov());
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
        std::env::temp_dir().join(format!("perfpal-i1c-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

#[test]
fn python_entry_pipeline_emits_and_double_scan_is_byte_identical() {
    let root = temp_dir("tree");
    fs::write(
        root.join("app.py"),
        "def helper(x):\n    return x + 1\n\n\nif __name__ == \"__main__\":\n    print(helper(1))\n",
    )
    .expect("write app.py");
    fs::write(
        root.join("lib.py"),
        "def api_init():\n    return 0\n\n\ndef _private():\n    return 1\n",
    )
    .expect("write lib.py");

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
    // emptiness of `resolved_python_edges`: Python is syntactic-tier only).
    assert_eq!(doc1["schema_version"], "1.6");
    let by_path_name = |path: &str, name: &str| {
        doc1["stats"]["functions"]
            .as_array()
            .expect("functions[]")
            .iter()
            .find(|f| f["path"] == path && f["name"] == name)
            .unwrap_or_else(|| panic!("function {path}#{name} present"))
            .clone()
    };
    let module = by_path_name("app.py", "<module>");
    assert_eq!(module["entry_kind"], "script");
    assert_eq!(module["entry_confidence"], "HIGH");
    assert_eq!(module["reachable_from_entry"], true);
    // A guarded file's plain defs are non-entries and unreachable without
    // semantic facts.
    let helper = by_path_name("app.py", "helper");
    assert!(helper.get("entry_kind").is_none());
    assert_eq!(helper["reachable_from_entry"], false);
    // Guard-less public defs are lib-roots; underscore-prefixed never entries.
    let api = by_path_name("lib.py", "api_init");
    assert_eq!(api["entry_kind"], "lib-root");
    assert_eq!(api["entry_confidence"], "MEDIUM");
    assert_eq!(api["reachable_from_entry"], true);
    let private = by_path_name("lib.py", "_private");
    assert!(private.get("entry_kind").is_none());
    assert_eq!(private["reachable_from_entry"], false);

    fs::remove_dir_all(&root).ok();
}
