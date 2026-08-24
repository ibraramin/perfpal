//! Parser-free Wave B fixture tests (ADR-0001 guardrail 2): ALLOC-003,
//! LOOP-002/004, and CALL-002 run against serialized IR under
//! `tests/fixtures/ir/` with NO tree-sitter binding. Covers detection,
//! negatives/suppression, and the ir-v1.md §5 macro-origin downgrade hook
//! on a wave-B rule (hooks are inherited centrally).

mod common;

use perfpal::analyzer::finding::{
    ALLOC_003_RULE_ID, CALL_002_RULE_ID, LOOP_002_RULE_ID, LOOP_004_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity, analyze};

// ===========================================================================
// PERF-C-ALLOC-003 — realloc linear growth in loop
// ===========================================================================

#[test]
fn additive_realloc_in_loop_fires_hot_medium() {
    let units = common::load_units_fixture("PERF-C-ALLOC-003__realloc-additive-in-loop.json");
    let findings = analyze(&units);
    let alloc003 = common::by_rule(&findings, ALLOC_003_RULE_ID);
    assert_eq!(alloc003.len(), 1, "the additive in-loop realloc fires");
    let f = alloc003[0];
    assert_eq!(f.severity, Severity::Hot);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("realloc")
    );
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
    assert_eq!(
        f.evidence.get("growth").map(String::as_str),
        Some("additive")
    );
    assert!(f.evidence.contains_key("alloc_site_index"));
    assert!(
        f.message.contains("doubling"),
        "message names doubling growth as the standard fix — {}",
        f.message
    );
}

#[test]
fn additive_realloc_outside_loop_stays_silent() {
    let units = common::load_units_fixture("PERF-C-ALLOC-003__realloc-additive-not-in-loop.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ALLOC_003_RULE_ID).is_empty(),
        "candidates require in_loop_depth ≥ 1"
    );
}

#[test]
fn multiplicative_realloc_growth_stays_silent() {
    // n * 2 is the documented FIX (amortized doubling), never a finding.
    let units = common::load_units_fixture("PERF-C-ALLOC-003__realloc-multiplicative-in-loop.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ALLOC_003_RULE_ID).is_empty(),
        "multiplicative growth must not fire ALLOC-003"
    );
}

#[test]
fn first_arg_pointer_arithmetic_with_plain_size_stays_silent() {
    // `realloc(p + 1, n)`: a '+' is span-contained in the call, but the
    // SIZE arg (last) is a plain identifier — zero size growth ⇒ silent.
    let units =
        common::load_units_fixture("PERF-C-ALLOC-003__realloc-pointer-arith-first-arg.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ALLOC_003_RULE_ID).is_empty(),
        "first-arg pointer arithmetic must not fire ALLOC-003"
    );
}

// ===========================================================================
// PERF-C-LOOP-002 — expensive setup / I-O call inside loop
// ===========================================================================

#[test]
fn unguarded_io_in_loop_is_hot_and_guarded_lowers_to_warn() {
    let units = common::load_units_fixture("PERF-C-LOOP-002__io-in-loop.json");
    let findings = analyze(&units);
    let loop002 = common::by_rule(&findings, LOOP_002_RULE_ID);
    assert_eq!(loop002.len(), 2, "printf + getenv call sites fire");

    let unguarded = loop002
        .iter()
        .find(|f| f.evidence.get("conditional_guarded").map(String::as_str) == Some("false"))
        .expect("unguarded printf finding");
    assert_eq!(unguarded.severity, Severity::Hot, "default severity");
    assert_eq!(
        unguarded.evidence.get("callee").map(String::as_str),
        Some("printf")
    );
    assert_eq!(
        unguarded.evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );

    let guarded = loop002
        .iter()
        .find(|f| f.evidence.get("conditional_guarded").map(String::as_str) == Some("true"))
        .expect("guarded getenv finding");
    assert_eq!(
        guarded.severity,
        Severity::Warn,
        "founder-approved lowering hook: conditional_guarded ⇒ Warn"
    );
    assert_eq!(
        guarded.evidence.get("callee").map(String::as_str),
        Some("getenv")
    );

    for f in &loop002 {
        assert_eq!(f.confidence, Confidence::Medium);
        assert!(f.message.contains("candidate optimization"));
    }
}

#[test]
fn expensive_call_outside_loop_stays_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-002__not-in-loop.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, LOOP_002_RULE_ID).is_empty(),
        "fopen outside any loop must not fire LOOP-002"
    );
}

#[test]
fn macro_origin_downgrades_loop_002_confidence_and_flags() {
    let units = common::load_units_fixture("PERF-C-LOOP-002__macro-origin.json");
    let findings = analyze(&units);
    let loop002 = common::by_rule(&findings, LOOP_002_RULE_ID);
    assert_eq!(loop002.len(), 1, "macro-tainted call still fires");
    let f = loop002[0];
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin caps confidence at MEDIUM"
    );
    assert_eq!(f.severity, Severity::Hot, "severity band unchanged by hook");
}

// ===========================================================================
// PERF-C-LOOP-004 — variable-divisor div/mod in innermost loop
// ===========================================================================

#[test]
fn variable_divisor_innermost_div_and_mod_fire_info() {
    let units = common::load_units_fixture("PERF-C-LOOP-004__var-divisor-innermost.json");
    let findings = analyze(&units);
    let loop004 = common::by_rule(&findings, LOOP_004_RULE_ID);
    assert_eq!(
        loop004.len(),
        2,
        "variable-divisor / and % fire; literal divisor and \
         non-innermost stay silent in the same fixture"
    );

    let div = loop004
        .iter()
        .find(|f| f.evidence.get("op").map(String::as_str) == Some("/"))
        .expect("division finding");
    assert_eq!(div.severity, Severity::Info);
    assert_eq!(div.confidence, Confidence::Medium);
    assert_eq!(
        div.evidence.get("divisor_kind").map(String::as_str),
        Some("identifier")
    );
    assert_eq!(
        div.evidence.get("divisor").map(String::as_str),
        Some("count")
    );
    assert_eq!(div.span.start_line, 8, "anchored at the binop span");

    let modulo = loop004
        .iter()
        .find(|f| f.evidence.get("op").map(String::as_str) == Some("%"))
        .expect("modulo finding");
    assert_eq!(
        modulo.evidence.get("divisor").map(String::as_str),
        Some("stride")
    );
}

#[test]
fn literal_divisor_and_non_innermost_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-004__var-divisor-innermost.json");
    let findings = analyze(&units);
    assert!(
        findings
            .iter()
            .all(|f| { f.rule_id != LOOP_004_RULE_ID || f.span.start_line != 12 }),
        "literal divisor `x / 2` skipped by design (line 12)"
    );
    assert!(
        findings
            .iter()
            .all(|f| { f.rule_id != LOOP_004_RULE_ID || f.span.start_line != 15 }),
        "non-innermost `%` skipped (line 15)"
    );
}

// ===========================================================================
// PERF-C-CALL-002 — high cyclomatic complexity
// ===========================================================================

#[test]
fn complexity_at_threshold_fires_warn_high_once_per_function() {
    let units = common::load_units_fixture("PERF-C-CALL-002__high-complexity.json");
    let findings = analyze(&units);
    let call002 = common::by_rule(&findings, CALL_002_RULE_ID);
    assert_eq!(
        call002.len(),
        1,
        "complexity-16 function fires; complexity-3 stays silent"
    );
    let f = call002[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        f.evidence.get("function").map(String::as_str),
        Some("mega_dispatch")
    );
    assert_eq!(f.evidence.get("complexity").map(String::as_str), Some("16"));
    assert_eq!(f.evidence.get("threshold").map(String::as_str), Some("15"));
    assert_eq!(f.span.start_line, 4, "anchored at the record span");
}
