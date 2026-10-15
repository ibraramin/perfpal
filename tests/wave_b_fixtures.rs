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

// ---------------------------------------------------------------------------
// Fixture-derived expectation helpers
//
// Expected spans are DERIVED from the raw fixture JSON (via the untyped
// loader) instead of hardcoded literals — editing a fixture's span flips the
// test instead of leaving a stale magic number.
// ---------------------------------------------------------------------------

/// First document of a fixture array (the corpus files are single-document).
fn fixture_doc(fixture: &serde_json::Value) -> &serde_json::Value {
    fixture
        .as_array()
        .and_then(|docs| docs.first())
        .expect("fixture holds at least one IR document")
}

/// `start_line` of the FIRST function-level binary_op matching `pred`.
fn binop_start_line(fixture: &serde_json::Value, pred: impl Fn(&serde_json::Value) -> bool) -> u32 {
    fixture_doc(fixture)["functions"]
        .as_array()
        .and_then(|fns| fns.first())
        .and_then(|f| f["binary_ops"].as_array())
        .and_then(|ops| ops.iter().find(|b| pred(b)))
        .and_then(|b| b["span"]["start_line"].as_u64())
        .map(|l| u32::try_from(l).expect("line fits u32"))
        .expect("fixture must contain the referenced binary_op")
}

/// `start_line` of the function record named `name`.
fn record_start_line(fixture: &serde_json::Value, name: &str) -> u32 {
    fixture_doc(fixture)["functions"]
        .as_array()
        .and_then(|fns| fns.iter().find(|f| f["record"]["name"] == name))
        .and_then(|f| f["record"]["span"]["start_line"].as_u64())
        .map(|l| u32::try_from(l).expect("line fits u32"))
        .expect("fixture must contain the referenced function record")
}

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

#[test]
fn capacity_guarded_realloc_stays_silent() {
    // Precision FP-class fix 4: a `realloc` inside a conditional branch
    // within the loop (`if (len >= cap)` — guarded growth that reallocates
    // only when needed) stays silent even though the size arg is additive.
    let units = common::load_units_fixture("PERF-C-ALLOC-003__realloc-guarded-in-loop.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ALLOC_003_RULE_ID).is_empty(),
        "capacity-guarded growth must not fire ALLOC-003"
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

    // Expectations derived from the fixture itself (op `/`, identifier
    // divisor `count`) — no magic line literal.
    let fixture = common::load_ir_fixture("PERF-C-LOOP-004__var-divisor-innermost.json");
    let div_line = binop_start_line(&fixture, |b| {
        b["op"] == "/" && b["right"]["name"] == "count"
    });

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
    assert_eq!(
        div.span.start_line, div_line,
        "anchored at the binop span (derived from fixture)"
    );

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
    // Expectations derived from the fixture itself: the literal-divisor
    // `/` and the non-innermost `%` lines come from the raw JSON.
    let fixture = common::load_ir_fixture("PERF-C-LOOP-004__var-divisor-innermost.json");
    let literal_line = binop_start_line(&fixture, |b| {
        b["op"] == "/" && b["right"]["kind"] == "literal"
    });
    let non_innermost_line = binop_start_line(&fixture, |b| b["is_innermost"] == false);
    assert!(
        findings
            .iter()
            .all(|f| { f.rule_id != LOOP_004_RULE_ID || f.span.start_line != literal_line }),
        "literal divisor `x / 2` skipped by design (fixture line {literal_line})"
    );
    assert!(
        findings
            .iter()
            .all(|f| { f.rule_id != LOOP_004_RULE_ID || f.span.start_line != non_innermost_line }),
        "non-innermost `%` skipped (fixture line {non_innermost_line})"
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
    // Expected anchor derived from the fixture's own record span — no
    // magic line literal.
    let fixture = common::load_ir_fixture("PERF-C-CALL-002__high-complexity.json");
    let record_line = record_start_line(&fixture, "mega_dispatch");
    assert_eq!(
        f.span.start_line, record_line,
        "anchored at the record span (derived from fixture)"
    );
}
