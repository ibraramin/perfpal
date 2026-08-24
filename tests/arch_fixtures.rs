//! Parser-free Wave 2a fixture tests (ADR-0001 guardrail 2): ARCH-001/002/003
//! run against serialized IR under `tests/fixtures/ir/` with NO tree-sitter
//! binding. Covers detection, negatives (below-floor component, file-local-
//! first resolution), and anchor/evidence contracts for every sub-kind.

mod common;

use perfpal::analyzer::finding::{ARCH_001_RULE_ID, ARCH_002_RULE_ID, ARCH_003_RULE_ID};
use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::ir::{FileRecord, IncludeEdge, IncludeForm, LocCounts, ParseHealth, Span};
use perfpal::scanner::FileUnit;

/// Synthetic unit with only include directives (no functions needed for
/// ARCH-001 graph analysis).
fn include_only_unit(path: &str, includes: Vec<IncludeEdge>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 0,
            global_var_count: 0,
            include_counts: Default::default(),
        },
        parse_health: ParseHealth::default(),
        includes,
        ..FileUnit::default()
    }
}

/// A resolved internal include directive at `(line, col 1)`.
fn resolved_include(from: &str, to: &str, line: u32) -> IncludeEdge {
    IncludeEdge {
        from_path: from.to_string(),
        to_path: Some(to.to_string()),
        form: IncludeForm::Quoted,
        resolved: true,
        span: Span {
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 20,
        },
    }
}

// ===========================================================================
// PERF-C-ARCH-001 — include-graph fan-out / cycles
// ===========================================================================

#[test]
fn two_way_include_cycle_fires_one_finding_on_the_broken_edge() {
    // Sorted pairs: ("a.h","b.h") < ("b.h","a.h"); the second closes the
    // directed cycle and is dropped (visible as THE finding).
    let units = common::load_units_fixture("PERF-C-ARCH-001__include-cycle.json");
    let findings = analyze(&units);
    let arch001 = common::by_rule(&findings, ARCH_001_RULE_ID);
    assert_eq!(arch001.len(), 1, "exactly one broken edge ⇒ one finding");
    let f = arch001[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.path, "b.h", "anchored on the edge that closed the cycle");
    assert_eq!(f.span.start_line, 1, "retained earliest directive span");
    assert_eq!(f.evidence.get("kind").map(String::as_str), Some("cycle"));
    assert_eq!(
        f.evidence.get("coupled_with").map(String::as_str),
        Some("a.h")
    );
    assert!(f.message.contains("candidate optimization"));
}

#[test]
fn thirteen_target_hub_fires_once_at_first_directive() {
    let units = common::load_units_fixture("PERF-C-ARCH-001__fanout-hub.json");
    let findings = analyze(&units);
    let arch001 = common::by_rule(&findings, ARCH_001_RULE_ID);
    assert_eq!(arch001.len(), 1, "hub fires once; targets stay silent");
    let f = arch001[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.path, "hub.c");
    assert_eq!(
        f.span.start_line, 1,
        "fan-out trigger anchors at the FIRST include directive"
    );
    assert_eq!(f.evidence.get("kind").map(String::as_str), Some("hub"));
    assert_eq!(f.evidence.get("fan_out").map(String::as_str), Some("13"));
    assert_eq!(f.evidence.get("fan_in").map(String::as_str), Some("0"));
}

#[test]
fn eight_deep_include_chain_fires_at_the_chain_head() {
    // n0 → n1 → … → n8: depth(n0) = 8 ≥ threshold; every other node ≤ 7.
    let units = common::load_units_fixture("PERF-C-ARCH-001__deep-chain.json");
    let findings = analyze(&units);
    let arch001 = common::by_rule(&findings, ARCH_001_RULE_ID);
    assert_eq!(arch001.len(), 1, "only the depth-8 head fires");
    let f = arch001[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.path, "chain/n0.c");
    assert_eq!(
        f.span.start_line, 1,
        "anchor = include edge toward the max-depth successor"
    );
    assert_eq!(
        f.evidence.get("kind").map(String::as_str),
        Some("deep_chain")
    );
    assert_eq!(
        f.evidence.get("include_depth").map(String::as_str),
        Some("8")
    );
}

#[test]
fn unresolved_quoted_include_is_info_not_error() {
    let units = common::load_units_fixture("PERF-C-ARCH-001__unresolved-quoted.json");
    let findings = analyze(&units);
    let arch001 = common::by_rule(&findings, ARCH_001_RULE_ID);
    assert_eq!(arch001.len(), 1);
    let f = arch001[0];
    assert_eq!(f.severity, Severity::Info, "data, not error (Principle 4)");
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.path, "src/orphan.c");
    assert_eq!(f.span.start_col, 1);
    assert_eq!(
        f.evidence.get("kind").map(String::as_str),
        Some("unresolved_include")
    );
    assert_eq!(f.evidence.get("form").map(String::as_str), Some("quoted"));
}

#[test]
fn hub_anchor_advances_when_first_directive_collides_with_deep_chain() {
    // hub.c is BOTH a depth-8 deep-chain head AND a 13-target fan-out hub.
    // Its EARLIEST directive (line 1) targets chain/c1.c — its max-depth
    // successor — so the deep-chain anchor and the historical hub anchor
    // coincide on (hub.c, line 1). The duplicate-collapse key
    // (rule_id, path, line, col) is sub-kind-blind and would silently drop
    // whichever candidate pushes later; the hub must instead ADVANCE to its
    // next directive span (line 2) so BOTH findings survive.
    let mut hub_includes = vec![resolved_include("hub.c", "chain/c1.c", 1)];
    for i in 1u32..=12 {
        hub_includes.push(resolved_include("hub.c", &format!("t{i:02}.h"), i + 1));
    }
    let mut units = vec![include_only_unit("hub.c", hub_includes)];
    for i in 1u32..=7 {
        let path = format!("chain/c{i}.c");
        let next = format!("chain/c{}.c", i + 1);
        units.push(include_only_unit(
            &path,
            vec![resolved_include(&path, &next, 1)],
        ));
    }
    units.push(include_only_unit("chain/c8.c", vec![]));

    let findings = analyze(&units);
    let arch001 = common::by_rule(&findings, ARCH_001_RULE_ID);
    assert_eq!(
        arch001.len(),
        2,
        "deep chain AND hub must BOTH be present — got {arch001:?}"
    );
    let deep_chain = arch001
        .iter()
        .find(|f| f.evidence.get("kind").map(String::as_str) == Some("deep_chain"))
        .expect("deep-chain finding survives");
    let hub = arch001
        .iter()
        .find(|f| f.evidence.get("kind").map(String::as_str) == Some("hub"))
        .expect("hub finding survives the anchor collision");
    assert_eq!(deep_chain.span.start_line, 1, "deep chain keeps its anchor");
    assert_eq!(
        hub.span.start_line, 2,
        "hub anchor ADVANCED past the colliding first directive"
    );
    assert_ne!(
        (deep_chain.span.start_line, deep_chain.span.start_col),
        (hub.span.start_line, hub.span.start_col),
        "anchors must be distinct or dedup would drop one finding"
    );
}

// ===========================================================================
// PERF-C-ARCH-002 — hot-component aggregation
// ===========================================================================

#[test]
fn hottest_component_fires_hot_high_at_its_most_complex_function() {
    // src/hot: complexity_sum 18 ≥ 15, density 2 alloc sites / 0.1 KLOC =
    // 20; src/cold: sum 3. Clear winner: src/hot, anchored at parse_big.
    let units = common::load_units_fixture("PERF-C-ARCH-002__hot-component.json");
    let findings = analyze(&units);
    let arch002 = common::by_rule(&findings, ARCH_002_RULE_ID);
    assert_eq!(arch002.len(), 1, "rank-1 component only");
    let f = arch002[0];
    assert_eq!(f.severity, Severity::Hot);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.path, "src/hot/parse.c");
    assert_eq!(f.span.start_line, 4, "highest-complexity function's span");
    assert_eq!(
        f.evidence.get("path_prefix").map(String::as_str),
        Some("src/hot")
    );
    assert_eq!(
        f.evidence.get("complexity_sum").map(String::as_str),
        Some("18")
    );
    assert_eq!(
        f.evidence.get("max_loop_depth").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        f.evidence.get("alloc_site_density").map(String::as_str),
        Some("20"),
        "2 alloc sites over 100 code LOC = 20 per KLOC"
    );
    assert_eq!(
        f.evidence.get("function_count").map(String::as_str),
        Some("2")
    );
}

#[test]
fn below_floor_complexity_keeps_arch_002_silent() {
    let units = common::load_units_fixture("PERF-C-ARCH-002__below-floor.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ARCH_002_RULE_ID).is_empty(),
        "complexity_sum 2 < floor 15 must not fire"
    );
    assert!(
        findings.is_empty(),
        "hello-world scans stay entirely clean — got {findings:?}"
    );
}

// ===========================================================================
// PERF-C-ARCH-003 — cross-file call coupling
// ===========================================================================

#[test]
fn name_matched_cross_file_call_fires_info_low_locked() {
    let units = common::load_units_fixture("PERF-C-ARCH-003__cross-file-match.json");
    let findings = analyze(&units);
    let arch003 = common::by_rule(&findings, ARCH_003_RULE_ID);
    assert_eq!(arch003.len(), 1);
    let f = arch003[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(
        f.confidence,
        Confidence::Low,
        "schema locks this rule at LOW"
    );
    assert_eq!(f.path, "main.c");
    assert_eq!(f.span.start_line, 4, "anchored at the call site");
    assert_eq!(
        f.evidence.get("callee_name").map(String::as_str),
        Some("util_init")
    );
    assert_eq!(
        f.evidence.get("matched_path").map(String::as_str),
        Some("util.c")
    );
    assert!(
        f.message.contains("LOW-confidence hypothesis"),
        "name-based framing required — {}",
        f.message
    );
}

#[test]
fn file_local_definition_wins_over_cross_file_match() {
    // helper is defined in main.c AND other.c; file-local-first (ADR-0001)
    // means the same-file hit suppresses the hypothesis entirely.
    let units = common::load_units_fixture("PERF-C-ARCH-003__file-local-first.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, ARCH_003_RULE_ID).is_empty(),
        "same-file definition must suppress the cross-file hypothesis"
    );
}

#[test]
fn ambiguous_name_resolves_to_smallest_definition_path() {
    let units = common::load_units_fixture("PERF-C-ARCH-003__ambiguous-smallest-path.json");
    let findings = analyze(&units);
    let arch003 = common::by_rule(&findings, ARCH_003_RULE_ID);
    assert_eq!(arch003.len(), 1);
    assert_eq!(
        arch003[0].evidence.get("matched_path").map(String::as_str),
        Some("a.c"),
        "ambiguity resolves to the lexicographically smallest path"
    );
}
