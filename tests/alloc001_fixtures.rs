//! Parser-free ALLOC-001 fixture tests (ADR-0001 guardrail 2): the heuristic
//! suite runs against serialized IR under `tests/fixtures/ir/` with NO
//! tree-sitter binding. Covers detection, the not-in-loop negative, and both
//! ir-v1.md §5 downgrade hooks (macro origin, error-region overlap).

mod common;

use perfpal::analyzer::finding::ALLOC_001_RULE_ID;
use perfpal::analyzer::{Confidence, Severity, analyze};

#[test]
fn fires_for_malloc_in_for_loop() {
    let units = common::load_units_fixture("PERF-C-ALLOC-001__malloc-in-for.json");
    let findings = analyze(&units);
    assert_eq!(findings.len(), 1, "exactly one alloc-in-loop finding");
    let f = &findings[0];
    assert_eq!(f.rule_id, ALLOC_001_RULE_ID);
    assert_eq!(f.severity, Severity::Hot);
    assert_eq!(f.confidence, Confidence::High);
    assert!(!f.macro_suspect);
    assert!(!f.error_node_overlap);
    assert!(f.message.contains("per-iteration heap allocation"));
    assert_eq!(f.evidence.get("callee").map(String::as_str), Some("malloc"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
    assert_eq!(
        f.evidence.get("alloc_site_index").map(String::as_str),
        Some("1")
    );
}

#[test]
fn finding_id_uses_rule_path_line_col() {
    let units = common::load_units_fixture("PERF-C-ALLOC-001__malloc-in-for.json");
    let findings = analyze(&units);
    let f = &findings[0];
    assert_eq!(
        f.finding_id(),
        format!(
            "{}:{}:{}:{}",
            f.rule_id, f.path, f.span.start_line, f.span.start_col
        )
    );
}

#[test]
fn stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-C-ALLOC-001__not-in-loop.json");
    assert!(
        analyze(&units).is_empty(),
        "malloc outside any loop must not fire"
    );
}

#[test]
fn macro_wrapper_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-C-ALLOC-001__macro-wrapper.json");
    let findings = analyze(&units);
    assert_eq!(findings.len(), 1, "wrapper-macro use in loop still fires");
    let f = &findings[0];
    assert_eq!(f.severity, Severity::Hot, "severity band unchanged by hook");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin caps confidence"
    );
    assert!(f.macro_suspect);
    assert!(!f.error_node_overlap);
}

#[test]
fn error_region_overlap_downgrades_but_never_drops() {
    let units = common::load_units_fixture("PERF-C-ALLOC-001__error-region-overlap.json");
    let findings = analyze(&units);
    assert_eq!(
        findings.len(),
        1,
        "finding inside an error region is downgraded, never silently dropped"
    );
    let f = &findings[0];
    assert_eq!(f.confidence, Confidence::Medium);
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect);
}
