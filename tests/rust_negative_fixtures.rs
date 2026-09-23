//! Rust negative / suppression IR fixtures + tests (plan `rust-backend` R5
//! lane A).
//!
//! Parser-free IR documents under `tests/fixtures/ir/` (ADR-0001 guardrail 2)
//! lock the three negative classes that the C corpus pinned but the Rust
//! corpus did not yet:
//!
//! 1. **not-in-loop** — a family site outside every loop stays silent
//!    (`in_loop_depth == 0`);
//! 2. **downgrade hooks** — `macro_origin` and `error_node_overlap` findings
//!    still fire but are capped at MEDIUM with the matching flag (the shared
//!    `analyzer::apply_downgrade_hooks`, never per-rule);
//! 3. **suppression** — by-reference/small TYPE-001 params, STR-002
//!    reassignment, ARCH-003 file-local-first, and CALL-002 below-threshold.
//!
//! Every assertion is an EXACT count (`== 0` or `== 1`) — never a range — so a
//! regression that silently drops or duplicates a finding fails loudly.
//!
//! LL-009 pairing: the backend-derived `not-in-loop` and `macro-origin`
//! classes are additionally locked against the REAL `tree_sitter_rust`
//! backend end-to-end (`tests/fixtures/rust-src/`). The `error-region` class
//! has NO real-parse lock on purpose: tree-sitter ERROR-node recovery is not
//! deterministically controllable from source text, so both the region shape
//! and its downgrade are exercised only through the parser-free IR fixtures
//! (matching the C corpus, whose error-region fixtures are likewise
//! parser-free).

mod common;

use std::path::Path;

use perfpal::analyzer::Severity;
use perfpal::analyzer::finding::{
    RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID, RS_ARCH_003_RULE_ID,
    RS_CALL_002_RULE_ID, RS_LOOP_001_RULE_ID, RS_LOOP_002_RULE_ID, RS_LOOP_003_RULE_ID,
    RS_LOOP_005_RULE_ID, RS_LOOP_006_RULE_ID, RS_MEM_002_RULE_ID, RS_STR_001_RULE_ID,
    RS_STR_002_RULE_ID, RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, analyze};
use perfpal::scanner::scan;

// ===========================================================================
// 1. not-in-loop — the named rule must stay silent outside every loop.
// ===========================================================================

#[test]
fn alloc_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_ALLOC_001_RULE_ID).is_empty(),
        "a Vec::new outside any loop must not fire RS-ALLOC-001"
    );
}

#[test]
fn alloc_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_ALLOC_002_RULE_ID).is_empty(),
        "a drop outside any loop must not fire RS-ALLOC-002"
    );
}

#[test]
fn alloc_003_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-003__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_ALLOC_003_RULE_ID).is_empty(),
        "a reserve outside any loop must not fire RS-ALLOC-003"
    );
}

#[test]
fn str_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-STR-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_STR_001_RULE_ID).is_empty(),
        "a push_str outside any loop must not fire RS-STR-001"
    );
}

#[test]
fn loop_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-LOOP-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_LOOP_002_RULE_ID).is_empty(),
        "an expensive I/O macro outside any loop must not fire RS-LOOP-002"
    );
}

#[test]
fn loop_003_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-LOOP-003__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_LOOP_003_RULE_ID).is_empty(),
        "a charwise primary call outside any loop must not fire RS-LOOP-003"
    );
}

#[test]
fn loop_005_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-LOOP-005__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_LOOP_005_RULE_ID).is_empty(),
        "a global store outside any loop must not fire RS-LOOP-005"
    );
}

#[test]
fn loop_006_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-LOOP-006__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_LOOP_006_RULE_ID).is_empty(),
        "an indirect call outside any loop must not fire RS-LOOP-006"
    );
}

#[test]
fn mem_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-RS-MEM-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), RS_MEM_002_RULE_ID).is_empty(),
        "a block copy outside any loop must not fire RS-MEM-002"
    );
}

// ===========================================================================
// 2a. macro-origin downgrade — the rule STILL fires, capped + flagged.
// ===========================================================================

#[test]
fn alloc_001_macro_origin_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__macro-origin.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(hits.len(), 1, "macro-tainted allocation still fires");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Hot, "severity band unchanged by hook");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin caps HIGH at MEDIUM"
    );
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(f.evidence.get("callee").map(String::as_str), Some("vec"));
}

#[test]
fn loop_002_macro_origin_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-LOOP-002__macro-origin.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_002_RULE_ID);
    assert_eq!(hits.len(), 1, "macro-tainted I/O call still fires");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Hot, "severity band unchanged by hook");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin keeps the rule's MEDIUM ceiling"
    );
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("println")
    );
}

#[test]
fn loop_005_macro_origin_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-LOOP-005__macro-origin.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_005_RULE_ID);
    assert_eq!(hits.len(), 1, "macro-tainted global store still fires");
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin keeps the rule's MEDIUM ceiling"
    );
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(f.evidence.get("target").map(String::as_str), Some("G"));
}

#[test]
fn mem_002_macro_origin_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-MEM-002__macro-origin.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_MEM_002_RULE_ID);
    assert_eq!(hits.len(), 1, "macro-tainted block copy still fires");
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Warn,
        "innermost escalation survives the hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin keeps the rule's MEDIUM ceiling"
    );
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("buf.clone")
    );
}

// ===========================================================================
// 2b. error-region overlap — downgraded, never silently dropped.
// ===========================================================================

#[test]
fn alloc_001_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-ALLOC-001__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "finding inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Hot, "severity band unchanged by hook");
    assert_eq!(f.confidence, Confidence::Medium);
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
}

#[test]
fn loop_001_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-LOOP-001__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "finding inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Warn,
        "severity band unchanged by hook"
    );
    assert_eq!(f.confidence, Confidence::Medium);
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
}

#[test]
fn loop_006_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-LOOP-006__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_006_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "finding inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps LOW at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
}

#[test]
fn type_002_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-RS-TYPE-002__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_TYPE_002_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "finding inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps LOW at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
    assert_eq!(f.evidence.get("struct").map(String::as_str), Some("Loose"));
}

// ===========================================================================
// 3. Suppression negatives.
// ===========================================================================

#[test]
fn type_001_by_reference_and_small_structs_stay_silent() {
    let units = common::load_units_fixture("PERF-RS-TYPE-001__by-reference.json");
    assert!(
        common::by_rule(&analyze(&units), RS_TYPE_001_RULE_ID).is_empty(),
        "`&Big` passes by reference and `Small` is below the 32-byte floor"
    );
}

#[test]
fn str_002_reassignment_suppression_stays_silent() {
    let units = common::load_units_fixture("PERF-RS-STR-002__reassignment.json");
    assert!(
        common::by_rule(&analyze(&units), RS_STR_002_RULE_ID).is_empty(),
        "a length query on a receiver reassigned inside its loop is suppressed"
    );
}

#[test]
fn arch_003_file_local_definition_suppresses_cross_module_edge() {
    let units = common::load_units_fixture("PERF-RS-ARCH-003__file-local-first.json");
    assert!(
        common::by_rule(&analyze(&units), RS_ARCH_003_RULE_ID).is_empty(),
        "a same-file definition suppresses the cross-module hypothesis"
    );
}

#[test]
fn call_002_below_threshold_stays_silent() {
    let units = common::load_units_fixture("PERF-RS-CALL-002__below-threshold.json");
    assert!(
        common::by_rule(&analyze(&units), RS_CALL_002_RULE_ID).is_empty(),
        "cyclomatic complexity 14 is below the threshold of 15"
    );
}

// ===========================================================================
// 4. Real-parse locks (LL-009 pairing) over committed `.rs` sources.
// ===========================================================================

/// Parse a committed real-source tree through the REAL backend end-to-end
/// (`scanner::scan` dispatch + grammar + `analyzer::analyze`).
fn scan_rust_src(tag: &str) -> Vec<perfpal::scanner::FileUnit> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rust-src")
        .join(tag);
    scan(&dir).unwrap_or_else(|e| panic!("scan of {} failed: {e}", dir.display()))
}

#[test]
fn real_parse_lock_not_in_loop_family() {
    let units = scan_rust_src("not_in_loop");
    assert!(
        !units.is_empty(),
        "real-parse lock source must be discovered"
    );
    let findings = analyze(&units);

    // ALLOC-001: only the in-loop `Vec::new` fires; the out-of-loop one and
    // the out-of-loop `Vec::with_capacity` stay silent.
    let alloc = common::by_rule(&findings, RS_ALLOC_001_RULE_ID);
    assert_eq!(
        alloc.len(),
        1,
        "real parse: exactly the in-loop allocation fires"
    );
    assert_eq!(
        alloc[0].evidence.get("callee").map(String::as_str),
        Some("Vec::new")
    );
    assert_eq!(
        alloc[0].evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );

    // LOOP-002: only the in-loop `println!` fires; the trailing out-of-loop
    // log stays silent.
    let io = common::by_rule(&findings, RS_LOOP_002_RULE_ID);
    assert_eq!(
        io.len(),
        1,
        "real parse: exactly the in-loop I/O macro fires"
    );
    assert_eq!(
        io[0].evidence.get("callee").map(String::as_str),
        Some("println")
    );
    assert_eq!(
        io[0].evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
}

#[test]
fn real_parse_lock_macro_origin_downgrade() {
    let units = scan_rust_src("macro_origin");
    assert!(
        !units.is_empty(),
        "real-parse lock source must be discovered"
    );
    let findings = analyze(&units);

    // The depth-3 nest fires LOOP-001; the local `tick!` macro call inside it
    // is minted macro-origin by the REAL backend, so the central downgrade
    // hook must cap HIGH -> MEDIUM and set the flag.
    let hits = common::by_rule(&findings, RS_LOOP_001_RULE_ID);
    assert_eq!(hits.len(), 1, "real parse: one depth-3 nest finding");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Warn, "severity band unchanged");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro-origin event caps HIGH at MEDIUM"
    );
    assert!(
        f.macro_suspect,
        "real backend-derived macro_origin event must flag the finding"
    );
    assert!(!f.error_node_overlap, "no error region in a clean parse");
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
}
