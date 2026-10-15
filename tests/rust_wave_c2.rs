//! Rust heuristic wave C2 integration tests (plan `rust-backend` R3): the
//! API-name `PERF-RS-*` families (ALLOC-001/002/003, STR-002/003,
//! LOOP-002/003/006, MEM-001/002) that mirror the C library-name rules onto
//! Rust idioms.
//!
//! Parser-free IR fixtures under `tests/fixtures/ir/` (ADR-0001 guardrail 2)
//! drive positives AND negatives/suppressions, severity/confidence and
//! evidence keys; the LL-009 lesson is honored with REAL-PARSE locks for the
//! alloc/str/mem families (a small `.rs` tree through `scanner::scan` +
//! `analyzer::analyze`). A final lock asserts all 22 `PERF-RS-*` ids are
//! reachable and emitted in the report §4 total order.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::finding::{
    RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID, RS_ARCH_001_RULE_ID,
    RS_ARCH_002_RULE_ID, RS_ARCH_003_RULE_ID, RS_CALL_001_RULE_ID, RS_CALL_002_RULE_ID,
    RS_LOOP_001_RULE_ID, RS_LOOP_002_RULE_ID, RS_LOOP_003_RULE_ID, RS_LOOP_004_RULE_ID,
    RS_LOOP_005_RULE_ID, RS_LOOP_006_RULE_ID, RS_LOOP_007_RULE_ID, RS_LOOP_008_RULE_ID,
    RS_MEM_001_RULE_ID, RS_MEM_002_RULE_ID, RS_STR_002_RULE_ID, RS_STR_003_RULE_ID,
    RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, Finding, Severity, analyze};
use perfpal::scanner::{FileUnit, scan};

// ===========================================================================
// PERF-RS-ALLOC-001 — allocation inside loop
// ===========================================================================

#[test]
fn alloc_001_aggregates_one_finding_per_innermost_loop() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__alloc-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "two in-loop alloc sites in ONE loop aggregate to one finding (the \
         Vec::new zero-alloc ctor in the same loop stays silent since fix 3)"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Hot);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.evidence.get("alloc_count").map(String::as_str), Some("2"));
    assert_eq!(
        f.evidence.get("callees").map(String::as_str),
        Some("to_vec, vec"),
        "sorted-unique SHORT callee tokens (method receiver text is not leaked)"
    );
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("to_vec"),
        "evidence-key compatibility: first sorted token"
    );
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
    assert_eq!(
        (
            f.span.start_line,
            f.span.start_col,
            f.span.end_line,
            f.span.end_col
        ),
        (3, 5, 11, 6),
        "span anchors at the innermost enclosing loop, not a call site"
    );
    assert!(f.message.contains("candidate optimization"));
}

/// One finding per innermost enclosing loop — two sibling loops ⇒ two findings,
/// each carrying its own `alloc_count` and loop span.
#[test]
fn alloc_001_emits_one_finding_per_loop() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__two-loops.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(hits.len(), 2, "two innermost loops ⇒ two findings");
    let counts: Vec<&str> = hits
        .iter()
        .map(|f| f.evidence.get("alloc_count").map(String::as_str).unwrap())
        .collect();
    assert_eq!(counts, ["1", "1"], "groups ordered by loop span");
    assert_eq!(hits[0].span.start_line, 3, "first loop anchor");
    assert_eq!(hits[1].span.start_line, 8, "second loop anchor");
}

/// M1 literal-receiver suppression: `"…".to_string()` / `"…".to_owned()` build
/// a `String` from a literal (often foldable) and must stay silent.
#[test]
fn alloc_001_suppresses_literal_receiver_string_alloc() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__literal-receiver.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, RS_ALLOC_001_RULE_ID).is_empty(),
        "literal-receiver .to_string()/.to_owned() allocations stay silent"
    );
}

/// Precision FP-class fix 3: zero-alloc constructors never fire
/// RS-ALLOC-001 — qualified `String::new` / `Vec::new` per-call, and the
/// arg-less `vec![]` macro form via the scan-side dominant family
/// (`record.alloc_family == "empty_ctor"`). A `vec!` macro site in a
/// `real_alloc`-dominant function still fires (no over-suppression).
#[test]
fn alloc_001_suppresses_empty_ctors_but_not_real_vec_macro() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__empty-ctor.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "the empty-dominant loop stays silent; the real-dominant vec! loop fires"
    );
    let f = hits[0];
    assert_eq!(f.span.start_line, 16, "anchor is the real-dominant loop");
    assert_eq!(f.evidence.get("callees").map(String::as_str), Some("vec"));
}

/// M1 `+N more` cap: a single innermost loop with more than 8 distinct
/// alloc-family callees still yields ONE finding whose `callees` evidence is
/// the 8-token prefix plus an EXACT `+N more` suffix, and whose
/// `alloc_count` is the exact site total.
#[test]
fn alloc_001_caps_callee_list_with_more_suffix() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__callee-cap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "ten distinct sites in one loop aggregate to one finding"
    );
    let f = hits[0];
    assert_eq!(
        f.evidence.get("alloc_count").map(String::as_str),
        Some("10"),
        "alloc_count is the exact site total, not the displayed token count"
    );
    assert_eq!(
        f.evidence.get("callees").map(String::as_str),
        Some(
            "Arc::new, Box::new, Rc::new, String::with_capacity, \
              VecDeque::with_capacity, collect, format, to_owned, +2 more"
        ),
        "the 8-token cap keeps the smallest sorted-unique tokens with an exact remainder"
    );
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("Arc::new"),
        "evidence-key compatibility: first sorted token"
    );
}

/// M1 defensive singleton: an alloc-family call flagged `in_loop_depth ≥ 1` +
/// `is_innermost` but with NO modelled enclosing `loops[]` entry still emits
/// exactly one finding anchored at the CALL span, with the call-derived
/// `loop_depth`.
#[test]
fn alloc_001_defensive_singleton_anchors_at_call_span() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__defensive-singleton.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "an unmodelled enclosing loop still fires as a singleton group"
    );
    let f = hits[0];
    assert_eq!(
        (
            f.span.start_line,
            f.span.start_col,
            f.span.end_line,
            f.span.end_col
        ),
        (4, 9, 4, 20),
        "the singleton anchors at the call span"
    );
    assert_eq!(f.evidence.get("alloc_count").map(String::as_str), Some("1"));
    assert_eq!(
        f.evidence.get("loop_depth").map(String::as_str),
        Some("1"),
        "the fallback keeps the call-derived in_loop_depth"
    );
    assert_eq!(
        f.evidence.get("callees").map(String::as_str),
        Some("Vec::with_capacity")
    );
}

/// M1 short-token qualified-name rule: a generic qualified name is retained
/// (`HashMap::<K, V>::with_capacity`), while a method call carrying a receiver
/// expression in `callee_path` (`Mod::make().collect`) collapses to its bare
/// `callee_name` instead of leaking the receiver text.
#[test]
fn alloc_001_short_token_keeps_qualified_and_drops_receiver_text() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__qualified-token.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(hits.len(), 1);
    let f = hits[0];
    assert_eq!(
        f.evidence.get("callees").map(String::as_str),
        Some("HashMap::<K, V>::with_capacity, collect"),
        "qualified constructor kept; receiver text from a method call not leaked"
    );
}

// ===========================================================================
// PERF-RS-ALLOC-002 — drop inside loop (+ co-occurrence escalation)
// ===========================================================================

#[test]
fn alloc_002_warns_alone_and_escalates_on_alloc_cooccurrence() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-002__drop-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_002_RULE_ID);
    assert_eq!(hits.len(), 2, "the depth-0 drop stays silent");

    let warn = hits
        .iter()
        .find(|f| f.path == "src/drops.rs" && f.span.start_line == 5)
        .expect("lone in-loop drop");
    assert_eq!(warn.severity, Severity::Warn);
    assert_eq!(warn.confidence, Confidence::High);
    assert!(
        !warn.evidence.contains_key("alloc_cooccurrence"),
        "no dead evidence key without a paired allocation"
    );

    let hot = hits
        .iter()
        .find(|f| f.span.start_line == 16)
        .expect("paired in-loop drop");
    assert_eq!(hot.severity, Severity::Hot, "alloc co-occurrence escalates");
    assert_eq!(
        hot.evidence.get("alloc_cooccurrence").map(String::as_str),
        Some("true")
    );
}

// ===========================================================================
// PERF-RS-ALLOC-003 — additive reserve growth in loop
// ===========================================================================

#[test]
fn alloc_003_reserve_methods_fire_at_any_loop_depth() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-003__reserve-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_003_RULE_ID);
    assert_eq!(
        hits.len(),
        2,
        "out-of-loop reserve and the non-family method stay silent"
    );
    let callees: Vec<&str> = hits
        .iter()
        .map(|f| f.evidence.get("callee").map(String::as_str).unwrap())
        .collect();
    assert_eq!(callees, ["v.reserve", "v.reserve_exact"]);
    for f in &hits {
        assert_eq!(f.severity, Severity::Hot);
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(
            f.evidence.get("growth").map(String::as_str),
            Some("additive")
        );
    }
}

// ===========================================================================
// PERF-RS-STR-002 — length query recomputed in loop (suppression)
// ===========================================================================

#[test]
fn str_002_suppresses_reassigned_identifier_in_loop() {
    let units = common::load_units_fixture("PERF-RS-STR-002__len-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_STR_002_RULE_ID);
    let identifiers: Vec<&str> = hits
        .iter()
        .map(|f| f.evidence.get("identifier").map(String::as_str).unwrap())
        .collect();
    assert_eq!(
        identifiers,
        ["s", "t"],
        "`u` reassigned inside its loop and the expression-receiver len stay silent"
    );
    for f in &hits {
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.confidence, Confidence::High);
    }
}

// ===========================================================================
// PERF-RS-STR-003 — unbounded unsafe ops (anywhere)
// ===========================================================================

#[test]
fn str_003_unbounded_unsafe_ops_fire_outside_loops() {
    let units = common::load_units_fixture("PERF-RS-STR-003__unbounded-ops.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_STR_003_RULE_ID);
    let callees: BTreeSet<&str> = hits
        .iter()
        .map(|f| f.evidence.get("callee").map(String::as_str).unwrap())
        .collect();
    assert_eq!(
        callees,
        BTreeSet::from([
            "ptr.copy_nonoverlapping",
            "std::slice::from_raw_parts",
            "v.set_len"
        ]),
        "len() and the copy_of name collision stay silent"
    );
    for f in &hits {
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.confidence, Confidence::High);
    }
}

// ===========================================================================
// PERF-RS-LOOP-002 — expensive I/O / setup in loop
// ===========================================================================

#[test]
fn loop_002_macros_methods_and_qualified_calls_fire_with_guard_downgrade() {
    let units = common::load_units_fixture("PERF-RS-LOOP-002__expensive-io-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_002_RULE_ID);
    assert_eq!(
        hits.len(),
        5,
        "the out-of-loop macro and helper stay silent"
    );
    let hot = hits.iter().filter(|f| f.severity == Severity::Hot).count();
    let warn = hits.iter().filter(|f| f.severity == Severity::Warn).count();
    assert_eq!(
        (hot, warn),
        (4, 1),
        "conditional_guarded lowers Hot to Warn"
    );
    let guarded = hits
        .iter()
        .find(|f| f.severity == Severity::Warn)
        .expect("guarded println");
    assert_eq!(
        guarded
            .evidence
            .get("conditional_guarded")
            .map(String::as_str),
        Some("true")
    );
    for f in &hits {
        assert_eq!(f.confidence, Confidence::Medium);
    }
    for f in &hits {
        assert_eq!(
            f.evidence.get("io_kind").map(String::as_str),
            Some("hoistable"),
            "macros, whole-buffer methods and setup paths are hoistable"
        );
    }
}

/// Divergence-study message split (fix 5): element-wise streaming transfer
/// (`recv`) cannot be hoisted — batching is the candidate — while a setup
/// macro (`println!`) is hoistable.
#[test]
fn loop_002_splits_streaming_from_hoistable() {
    let units = common::load_units_fixture("PERF-RS-LOOP-002__streaming-split.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_002_RULE_ID);
    assert_eq!(
        hits.len(),
        2,
        "streaming recv and hoistable println both fire"
    );

    let stream = hits
        .iter()
        .find(|f| f.span.start_line == 5)
        .expect("streaming recv");
    assert_eq!(
        stream.evidence.get("io_kind").map(String::as_str),
        Some("streaming")
    );
    assert!(
        stream.message.contains("batching"),
        "streaming message names batching: {}",
        stream.message
    );
    assert!(
        !stream.message.contains("hoisting the call"),
        "streaming message must not advise hoisting: {}",
        stream.message
    );

    let hoist = hits
        .iter()
        .find(|f| f.span.start_line == 15)
        .expect("hoistable println");
    assert_eq!(
        hoist.evidence.get("io_kind").map(String::as_str),
        Some("hoistable")
    );
    assert!(
        hoist.message.contains("hoisting the call"),
        "hoistable message names hoisting: {}",
        hoist.message
    );
}

// ===========================================================================
// PERF-RS-LOOP-003 — char/byte-wise processing in loop
// ===========================================================================

#[test]
fn loop_003_only_primary_statement_charwise_calls_fire() {
    let units = common::load_units_fixture("PERF-RS-LOOP-003__charwise-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_003_RULE_ID);
    assert_eq!(
        hits.len(),
        2,
        "nested-role and out-of-loop charwise calls stay silent"
    );
    let callees: Vec<&str> = hits
        .iter()
        .map(|f| f.evidence.get("callee").map(String::as_str).unwrap())
        .collect();
    assert_eq!(callees, ["s.chars", "s.bytes"]);
    for f in &hits {
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.confidence, Confidence::Medium);
    }
}

// ===========================================================================
// PERF-RS-LOOP-006 — indirect call in innermost loop
// ===========================================================================

#[test]
fn loop_006_only_innermost_indirect_calls_fire() {
    let units = common::load_units_fixture("PERF-RS-LOOP-006__indirect-call-in-innermost.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_006_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "non-innermost indirect, direct, and out-of-loop calls stay silent"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Low);
    assert_eq!(f.evidence.get("callee").map(String::as_str), Some("cb"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
}

// ===========================================================================
// PERF-RS-MEM-001 — size_of passed as an element count
// ===========================================================================

#[test]
fn mem_001_flags_size_of_arguments_to_with_capacity() {
    let units = common::load_units_fixture("PERF-RS-MEM-001__size-of-in-capacity.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_MEM_001_RULE_ID);
    assert_eq!(
        hits.len(),
        2,
        "an identifier count and a plain Vec::new stay silent"
    );
    let callees: Vec<&str> = hits
        .iter()
        .map(|f| f.evidence.get("callee").map(String::as_str).unwrap())
        .collect();
    assert_eq!(callees, ["Vec::with_capacity", "String::with_capacity"]);
    for f in &hits {
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(
            f.evidence.get("misuse").map(String::as_str),
            Some("size_of_as_element_count")
        );
    }
}

// ===========================================================================
// PERF-RS-MEM-002 — block copy/clone in loop
// ===========================================================================

#[test]
fn mem_002_block_copies_warn_innermost_and_clone_stays_silent() {
    let units = common::load_units_fixture("PERF-RS-MEM-002__block-copy-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_MEM_002_RULE_ID);
    assert_eq!(
        hits.len(),
        2,
        "only the two block-copy methods fire; clone/to_vec were dropped from the family"
    );
    let callees: BTreeSet<&str> = hits
        .iter()
        .map(|f| f.evidence.get("callee").map(String::as_str).unwrap())
        .collect();
    assert_eq!(
        callees,
        BTreeSet::from(["dst.copy_from_slice", "dst.extend_from_slice"])
    );
    for f in &hits {
        assert_eq!(f.severity, Severity::Warn, "innermost copies escalate");
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(
            f.evidence.get("innermost").map(String::as_str),
            Some("true")
        );
    }
}

// ===========================================================================
// All-22 reachability + total order
// ===========================================================================

/// Report §4 rank comparison (spec-derived, independent of the enum order):
/// severity hot→warn→info, then confidence HIGH→MEDIUM→LOW, then rule_id,
/// path, start_line, start_col.
fn adjacently_in_total_order(a: &Finding, b: &Finding) -> bool {
    let severity = |s: Severity| match s {
        Severity::Hot => 0u8,
        Severity::Warn => 1,
        Severity::Info => 2,
    };
    let confidence = |c: Confidence| match c {
        Confidence::High => 0u8,
        Confidence::Medium => 1,
        Confidence::Low => 2,
    };
    let key = |f: &Finding| {
        (
            severity(f.severity),
            confidence(f.confidence),
            f.rule_id.clone(),
            f.path.clone(),
            f.span.start_line,
            f.span.start_col,
        )
    };
    key(a) <= key(b)
}

#[test]
fn all_22_rust_rules_are_reachable_and_total_ordered() {
    // Load EVERY `PERF-RS-*` fixture in the corpus (parser-free) and analyze
    // them together: the union of emitted rule ids must be the full 22-rule
    // Rust catalog, and the document must be in report §4 total order.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ir");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .expect("fixtures dir readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("PERF-RS-") && n.ends_with(".json"))
        .collect();
    names.sort();
    assert!(names.len() >= 22, "expected the full R3 fixture corpus");
    let mut units: Vec<FileUnit> = Vec::new();
    for name in &names {
        units.extend(common::load_units_fixture(name));
    }

    let findings = analyze(&units);
    let expected: BTreeSet<&str> = [
        RS_ALLOC_001_RULE_ID,
        RS_ALLOC_002_RULE_ID,
        RS_ALLOC_003_RULE_ID,
        RS_ARCH_001_RULE_ID,
        RS_ARCH_002_RULE_ID,
        RS_ARCH_003_RULE_ID,
        RS_CALL_001_RULE_ID,
        RS_CALL_002_RULE_ID,
        RS_LOOP_001_RULE_ID,
        RS_LOOP_002_RULE_ID,
        RS_LOOP_003_RULE_ID,
        RS_LOOP_004_RULE_ID,
        RS_LOOP_005_RULE_ID,
        RS_LOOP_006_RULE_ID,
        RS_LOOP_007_RULE_ID,
        RS_LOOP_008_RULE_ID,
        RS_MEM_001_RULE_ID,
        RS_MEM_002_RULE_ID,
        RS_STR_002_RULE_ID,
        RS_STR_003_RULE_ID,
        RS_TYPE_001_RULE_ID,
        RS_TYPE_002_RULE_ID,
    ]
    .into_iter()
    .collect();
    let seen: BTreeSet<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert_eq!(seen, expected, "every PERF-RS-* id must be reachable");

    for pair in findings.windows(2) {
        assert!(
            adjacently_in_total_order(&pair[0], &pair[1]),
            "findings must be emitted in the report §4 total order at {:?} -> {:?}",
            pair[0].rule_id,
            pair[1].rule_id
        );
    }
    assert!(
        findings.iter().all(|f| f.path.ends_with(".rs")),
        "language gating: every fixture path is a Rust file"
    );
}

// ===========================================================================
// Real-parse locks (LL-009): the alloc / str / mem families end-to-end.
// ===========================================================================

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-rust-wc2-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

fn write_file(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture file");
}

#[test]
fn real_parse_lock_alloc_family() {
    let root = temp_root("alloc");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn fill(n: usize) -> Vec<u8> {\n    let mut out = Vec::with_capacity(n);\n    for i in 0..n {\n        let tmp = vec![i as u8; 2];\n        out.reserve(1);\n        out.push(tmp.len() as u8);\n        let _ = i;\n    }\n    out\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    assert!(
        !common::by_rule(&findings, RS_ALLOC_001_RULE_ID).is_empty(),
        "real parse: per-iteration vec![] fires RS-ALLOC-001 (a Vec::new zero-alloc ctor here would stay silent since fix 3)"
    );
    assert!(
        !common::by_rule(&findings, RS_ALLOC_003_RULE_ID).is_empty(),
        "real parse: per-iteration reserve fires RS-ALLOC-003"
    );
    assert!(
        findings.iter().all(|f| f.rule_id.starts_with("PERF-RS-")),
        "real parse: gating holds on a Rust-only tree"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn real_parse_lock_str_family() {
    let root = temp_root("str");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn build(parts: &[&str], v: &mut Vec<u8>, n: usize) -> String {\n    let mut s = String::new();\n    for p in parts {\n        s.push_str(p);\n        let _ = s.len();\n    }\n    unsafe { v.set_len(n); }\n    s\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    assert!(
        !common::by_rule(&findings, RS_STR_002_RULE_ID).is_empty(),
        "real parse: unassigned .len() in loop fires RS-STR-002"
    );
    assert!(
        !common::by_rule(&findings, RS_STR_003_RULE_ID).is_empty(),
        "real parse: set_len fires RS-STR-003"
    );
    assert!(
        findings.iter().all(|f| f.rule_id != "PERF-RS-STR-001"),
        "real parse: retired RS-STR-001 must stay silent on push_str"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn real_parse_lock_mem_family() {
    let root = temp_root("mem");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn copy(dst: &mut [u8], src: &[u8], n: usize) -> Vec<u8> {\n    for i in 0..n {\n        dst.copy_from_slice(src);\n        let _ = i;\n    }\n    let v: Vec<u8> = Vec::with_capacity(std::mem::size_of::<u64>());\n    v\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    let block = common::by_rule(&findings, RS_MEM_002_RULE_ID);
    assert_eq!(block.len(), 1, "real parse: in-loop copy_from_slice fires");
    assert_eq!(block[0].severity, Severity::Warn, "innermost escalates");
    let capacity = common::by_rule(&findings, RS_MEM_001_RULE_ID);
    assert_eq!(
        capacity.len(),
        1,
        "real parse: Vec::with_capacity(size_of::<u64>()) fires RS-MEM-001"
    );
    assert_eq!(
        capacity[0].evidence.get("misuse").map(String::as_str),
        Some("size_of_as_element_count")
    );
    fs::remove_dir_all(&root).ok();
}

/// LL-009 real-parse locks for the paths that were previously fixture-only:
/// `PERF-RS-ALLOC-002` (`drop`), `PERF-RS-LOOP-002`, `PERF-RS-LOOP-003` and
/// `PERF-RS-LOOP-006`. `Acc::accum` keeps the m1 receiver shape
/// (`self.buf.push_str(p)`) as a retired-RS-STR-001 silence probe.
#[test]
fn real_parse_lock_drop_loop_api_and_receiver_evidence() {
    let root = temp_root("drop-loop-recv");
    write_file(
        &root,
        "src/lib.rs",
        concat!(
            "pub struct Acc { pub buf: String }\n",
            "impl Acc {\n",
            "    pub fn accum(&mut self, parts: &[&str]) {\n",
            "        for p in parts {\n",
            "            self.buf.push_str(p);\n",
            "        }\n",
            "    }\n",
            "}\n",
            "\n",
            "pub fn churn(v: Vec<u8>) {\n",
            "    for _ in 0..3 {\n",
            "        drop(v);\n",
            "    }\n",
            "}\n",
            "\n",
            "pub fn log_loop(n: usize) {\n",
            "    for i in 0..n {\n",
            "        println!(\"{i}\");\n",
            "    }\n",
            "}\n",
            "\n",
            "pub fn charwise(s: &str, n: usize) {\n",
            "    for _ in 0..n {\n",
            "        s.chars();\n",
            "    }\n",
            "}\n",
            "\n",
            "pub fn indirect(arr: &[fn(i32) -> i32], n: usize) {\n",
            "    for i in 0..n {\n",
            "        arr[i](1);\n",
            "    }\n",
            "}\n",
        ),
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);

    // Retired RS-STR-001: `self.buf.push_str(p)` must emit NOTHING.
    assert!(
        findings.iter().all(|f| f.rule_id != "PERF-RS-STR-001"),
        "real parse: retired RS-STR-001 stays silent on self.buf.push_str"
    );

    // ALLOC-002: a per-iteration `drop` fires (Warn, no paired in-loop alloc).
    let drops = common::by_rule(&findings, RS_ALLOC_002_RULE_ID);
    assert_eq!(
        drops.len(),
        1,
        "real parse: in-loop drop fires RS-ALLOC-002"
    );
    assert_eq!(drops[0].severity, Severity::Warn);

    let io = common::by_rule(&findings, RS_LOOP_002_RULE_ID);
    assert_eq!(
        io.len(),
        1,
        "real parse: in-loop println! fires RS-LOOP-002"
    );

    let charwise = common::by_rule(&findings, RS_LOOP_003_RULE_ID);
    assert_eq!(
        charwise.len(),
        1,
        "real parse: primary s.chars() fires RS-LOOP-003"
    );

    let indirect = common::by_rule(&findings, RS_LOOP_006_RULE_ID);
    assert_eq!(
        indirect.len(),
        1,
        "real parse: innermost arr[i](1) indirect call fires RS-LOOP-006"
    );

    fs::remove_dir_all(&root).ok();
}
