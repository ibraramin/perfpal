//! Parser-free Wave B2 fixture tests (ADR-0001 guardrail 2): ALLOC-002
//! `fclose` on resolver-proven handles and STR-003 scanf-family calls with
//! resolver-resolved format identifiers run against serialized IR under
//! `tests/fixtures/ir/` with NO tree-sitter binding. The semantic facts in
//! these fixtures (`semantic.fopened_handle`, `semantic.literal_args`) stand
//! in for the libclang visitor's proofs; the visitor itself is locked behind
//! the env-gated `semantic` module in tests/resolver_health.rs.
//!
//! Runs in BOTH builds (no feature gate, no libclang needed).

mod common;

use perfpal::analyzer::finding::{ALLOC_002_RULE_ID, STR_003_RULE_ID};
use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::ir::ResolverTier;

// ===========================================================================
// PERF-C-ALLOC-002 — fclose on a proven fopen handle
// ===========================================================================

#[test]
fn fopened_fclose_in_loop_warns_medium_semantic() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__fclose-in-loop.json");
    let findings = analyze(&units);
    let alloc002 = common::by_rule(&findings, ALLOC_002_RULE_ID);
    assert_eq!(alloc002.len(), 2, "proven fclose sites fire");

    // Plain case: proven fclose in a loop with no fopen in the same
    // function — Warn/MEDIUM/semantic (founder verdict: the documented
    // per-iteration-pairing FP class caps the confidence at MEDIUM).
    let plain = alloc002
        .iter()
        .find(|f| f.span.start_line == 8)
        .expect("plain proven-fclose finding");
    assert_eq!(plain.severity, Severity::Warn);
    assert_eq!(plain.confidence, Confidence::Medium);
    assert_eq!(plain.resolver_tier, ResolverTier::Semantic);
    assert!(!plain.evidence.contains_key("alloc_cooccurrence"));
    assert_eq!(
        plain.evidence.get("callee").map(String::as_str),
        Some("fclose")
    );
    assert_eq!(
        plain.evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
    assert!(plain.message.contains("deallocation via 'fclose'"));
}

#[test]
fn fopen_pairing_never_escalates_proven_fclose() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__fclose-in-loop.json");
    let findings = analyze(&units);
    let cooc = common::by_rule(&findings, ALLOC_002_RULE_ID)
        .into_iter()
        .find(|f| f.span.start_line == 19)
        .expect("paired proven-fclose finding");
    // A balanced open-per-iteration + close-per-iteration is not hoistable,
    // so same-function fopen pairing never escalates: Warn/MEDIUM with no
    // `alloc_cooccurrence` key (the `callee` key keeps the fclose identity).
    assert_eq!(
        cooc.severity,
        Severity::Warn,
        "fopen pairing never escalates"
    );
    assert_eq!(cooc.confidence, Confidence::Medium);
    assert_eq!(cooc.resolver_tier, ResolverTier::Semantic);
    assert!(!cooc.evidence.contains_key("alloc_cooccurrence"));
    assert_eq!(
        cooc.evidence.get("callee").map(String::as_str),
        Some("fclose")
    );
}

#[test]
fn unproven_fclose_stays_silent() {
    // Param handle (no fact), unknown handle (no fact), and a proven
    // handle outside any loop (depth gate) — all silent: no fact ⇒ no
    // fire, and facts never waive the loop gate.
    let units = common::load_units_fixture("PERF-C-ALLOC-002__fclose-not-proven.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ALLOC_002_RULE_ID).is_empty(),
        "unproven/out-of-loop fclose must not fire ALLOC-002, got {findings:?}"
    );
    // NOTE: the in-loop `fclose` sites still trip the pre-existing LOOP-002
    // expensive-call rule (fclose is in its family) — B2 changes nothing
    // there; only ALLOC-002's proven-handle gate is under test here.
}

#[test]
fn fclose_finding_id_uses_rule_path_line_col() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__fclose-in-loop.json");
    let findings = analyze(&units);
    let f = common::by_rule(&findings, ALLOC_002_RULE_ID)[0];
    assert_eq!(
        f.finding_id(),
        format!(
            "{}:{}:{}:{}",
            f.rule_id, f.path, f.span.start_line, f.span.start_col
        )
    );
}

// ===========================================================================
// PERF-C-STR-003 — scanf-family format resolved through an identifier
// ===========================================================================

#[test]
fn identifier_format_resolved_to_pct_s_fires_semantic() {
    let units = common::load_units_fixture("PERF-C-STR-003__resolved-format.json");
    let findings = analyze(&units);
    let str003 = common::by_rule(&findings, STR_003_RULE_ID);
    assert_eq!(
        str003.len(),
        3,
        "scanf + fscanf via resolved identifiers, plus the untouched syntactic strcpy"
    );

    // scanf(fmt) with fmt → "%s": identical construction to the Literal
    // path (Warn/HIGH, format evidence, width-specifier message) at the
    // semantic tier with additive resolved_format evidence.
    let scanf = str003
        .iter()
        .find(|f| f.evidence.get("callee").map(String::as_str) == Some("scanf"))
        .expect("resolved scanf finding");
    assert_eq!(scanf.severity, Severity::Warn);
    assert_eq!(scanf.confidence, Confidence::High);
    assert_eq!(scanf.resolver_tier, ResolverTier::Semantic);
    assert_eq!(scanf.evidence.get("format").map(String::as_str), Some("%s"));
    assert_eq!(
        scanf.evidence.get("resolved_format").map(String::as_str),
        Some("true")
    );
    assert!(scanf.message.contains("width specifier"));

    // fscanf(stream, row_fmt) with row_fmt → "name:%s": the arg-1 format
    // position replays through the same family/position contract.
    let fscanf = str003
        .iter()
        .find(|f| f.evidence.get("callee").map(String::as_str) == Some("fscanf"))
        .expect("resolved fscanf finding");
    assert_eq!(fscanf.resolver_tier, ResolverTier::Semantic);
    assert_eq!(
        fscanf.evidence.get("format").map(String::as_str),
        Some("name:%s")
    );
    assert_eq!(
        fscanf.evidence.get("resolved_format").map(String::as_str),
        Some("true")
    );

    // Non-scanf rules untouched: strcpy stays syntactic without the
    // additive key.
    let strcpy = str003
        .iter()
        .find(|f| f.evidence.get("callee").map(String::as_str) == Some("strcpy"))
        .expect("syntactic strcpy finding");
    assert_eq!(strcpy.resolver_tier, ResolverTier::Syntactic);
    assert!(!strcpy.evidence.contains_key("resolved_format"));
    assert!(!strcpy.evidence.contains_key("format"));
}

#[test]
fn identifier_format_without_pct_s_or_proof_stays_silent() {
    // fmt → "%d" (proven, but no unbounded conversion) and fmt with no
    // proof at all (never invents content) — both silent.
    let units = common::load_units_fixture("PERF-C-STR-003__resolved-format-negative.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, STR_003_RULE_ID).is_empty(),
        "no-%s and unproven identifiers must stay silent, got {findings:?}"
    );
    assert!(
        findings.is_empty(),
        "the negative fixture trips no rule at all, got {findings:?}"
    );
}
