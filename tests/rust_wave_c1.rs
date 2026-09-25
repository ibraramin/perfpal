//! Rust heuristic wave C1 integration tests (plan `rust-backend` R3):
//! language gating + the STRUCTURAL `PERF-RS-*` family.
//!
//! Parser-free IR fixtures under `tests/fixtures/ir/` (ADR-0001 guardrail 2)
//! drive detection, negatives, severity/confidence and evidence keys; the
//! LL-009 lesson is honored by pairing them with REAL-PARSE locks (one per
//! family group) that run a small `.rs` tree through `scanner::scan` +
//! `analyzer::analyze`. Gating is locked with a synthetic Rust unit that WOULD
//! fire the C ALLOC rule if rules were not language-scoped.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::finding::{
    RS_ARCH_001_RULE_ID, RS_ARCH_002_RULE_ID, RS_ARCH_003_RULE_ID, RS_CALL_001_RULE_ID,
    RS_CALL_002_RULE_ID, RS_LOOP_001_RULE_ID, RS_LOOP_004_RULE_ID, RS_LOOP_005_RULE_ID,
    RS_LOOP_007_RULE_ID, RS_LOOP_008_RULE_ID, RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::scanner::{FileUnit, scan};

// ===========================================================================
// PERF-RS-LOOP-001 — deeply nested loops
// ===========================================================================

#[test]
fn loop_001_depth_three_fires_once_at_the_ancestor() {
    let units = common::load_units_fixture("PERF-RS-LOOP-001__deep-nest.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_001_RULE_ID);
    assert_eq!(hits.len(), 1, "one finding per maximal nest entry");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        (f.span.start_line, f.span.start_col),
        (7, 13),
        "anchors at the depth-3 ancestor"
    );
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
    assert_eq!(f.evidence.get("max_depth").map(String::as_str), Some("4"));
    assert_eq!(
        f.threshold,
        Some(("min_nesting_depth", 3)),
        "threshold travels into the schema lift"
    );
    assert!(f.message.contains("candidate optimization"));
}

// ===========================================================================
// PERF-RS-LOOP-004 — variable-divisor division in the innermost loop
// ===========================================================================

#[test]
fn loop_004_only_variable_innermost_division_fires() {
    let units = common::load_units_fixture("PERF-RS-LOOP-004__var-divisor-innermost.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_004_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "non-innermost and literal divisors stay silent by design"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.evidence.get("op").map(String::as_str), Some("/"));
    assert_eq!(f.evidence.get("divisor").map(String::as_str), Some("n"));
    assert_eq!(
        f.evidence.get("divisor_kind").map(String::as_str),
        Some("identifier")
    );
}

// ===========================================================================
// PERF-RS-LOOP-005 — global (static/const) write inside loop
// ===========================================================================

#[test]
fn loop_005_dedups_per_target_and_loop_and_ignores_locals() {
    let units = common::load_units_fixture("PERF-RS-LOOP-005__global-write-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_005_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "two same-loop stores dedup to one; local and out-of-loop stores stay silent"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.span.start_line, 5, "earliest qualifying span anchors");
    assert_eq!(f.evidence.get("target").map(String::as_str), Some("G"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
}

// ===========================================================================
// PERF-RS-LOOP-007 — linear membership scan nested in loop
// ===========================================================================

#[test]
fn loop_007_induced_subscript_vs_non_literal_fires_info_low() {
    let units = common::load_units_fixture("PERF-RS-LOOP-007__membership-scan.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_007_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "non-induction index and depth-1 comparison stay silent"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Low);
    assert_eq!(f.evidence.get("index_ident").map(String::as_str), Some("j"));
    assert_eq!(f.evidence.get("root").map(String::as_str), Some("arr"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("2"));
}

// ===========================================================================
// PERF-RS-LOOP-008 — member-chain dereference in the innermost loop
// ===========================================================================

#[test]
fn loop_008_only_deep_innermost_chains_fire() {
    let units = common::load_units_fixture("PERF-RS-LOOP-008__chain-in-innermost.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_LOOP_008_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "pure member chains, shallow chains, and non-innermost chains stay silent"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Medium);
    assert_eq!(f.evidence.get("root").map(String::as_str), Some("ctx"));
    assert_eq!(f.evidence.get("chain_depth").map(String::as_str), Some("3"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
}

// ===========================================================================
// PERF-RS-CALL-001 — direct recursion
// ===========================================================================

#[test]
fn call_001_one_finding_per_function_at_earliest_site() {
    let units = common::load_units_fixture("PERF-RS-CALL-001__self-call.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_CALL_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "indirect-only and non-recursive functions stay silent"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.span.start_line, 5, "earliest self-call anchors");
    assert_eq!(f.evidence.get("function").map(String::as_str), Some("walk"));
    assert_eq!(
        f.evidence.get("self_call_sites").map(String::as_str),
        Some("2")
    );
}

// ===========================================================================
// PERF-RS-CALL-002 — high cyclomatic complexity
// ===========================================================================

#[test]
fn call_002_fires_at_threshold_and_not_below() {
    let units = common::load_units_fixture("PERF-RS-CALL-002__high-complexity.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_CALL_002_RULE_ID);
    assert_eq!(hits.len(), 1, "complexity 15 fires, 14 does not");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        f.evidence.get("function").map(String::as_str),
        Some("branchy")
    );
    assert_eq!(f.evidence.get("complexity").map(String::as_str), Some("15"));
    assert_eq!(f.evidence.get("threshold").map(String::as_str), Some("15"));
    assert_eq!(f.threshold, Some(("min_complexity", 15)));
}

// ===========================================================================
// PERF-RS-ARCH-001 — module-graph cycles / unresolved internal edges
// ===========================================================================

#[test]
fn arch_001_cycle_fires_warn_high_on_lowest_sorted_closing_edge() {
    let units = common::load_units_fixture("PERF-RS-ARCH-001__cycle.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ARCH_001_RULE_ID);
    assert_eq!(hits.len(), 1);
    let f = hits[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(f.evidence.get("kind").map(String::as_str), Some("cycle"));
    assert_eq!(f.path, "src/b.rs", "the closing edge is the lowest-sorted");
    assert_eq!(
        f.evidence.get("coupled_with").map(String::as_str),
        Some("src/a.rs")
    );
}

// ===========================================================================
// PERF-RS-ARCH-002 — hot-component aggregation
// ===========================================================================

#[test]
fn arch_002_rank1_component_fires_hot_high() {
    let units = common::load_units_fixture("PERF-RS-ARCH-002__hot-component.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ARCH_002_RULE_ID);
    assert_eq!(hits.len(), 1);
    let f = hits[0];
    assert_eq!(f.severity, Severity::Hot);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        f.evidence.get("path_prefix").map(String::as_str),
        Some("src")
    );
    assert_eq!(
        f.evidence.get("complexity_sum").map(String::as_str),
        Some("22"),
        "10 + 8 + 4 across the src component"
    );
    assert_eq!(f.path, "src/app.rs", "anchored at the hottest function");
    assert_eq!(f.span.start_line, 1);
}

#[test]
fn arch_002_below_floor_stays_silent() {
    let units = common::load_units_fixture("PERF-RS-ARCH-002__below-floor.json");
    assert!(
        common::by_rule(&analyze(&units), RS_ARCH_002_RULE_ID).is_empty(),
        "a component below the complexity floor must not fire"
    );
}

// ===========================================================================
// PERF-RS-ARCH-003 — cross-module call coupling
// ===========================================================================

#[test]
fn arch_003_cross_module_name_match_fires_and_local_is_suppressed() {
    let units = common::load_units_fixture("PERF-RS-ARCH-003__cross-module.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_ARCH_003_RULE_ID);
    assert_eq!(hits.len(), 1, "only the cross-module call couples");
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Low);
    assert_eq!(
        f.evidence.get("callee_name").map(String::as_str),
        Some("helper_fn")
    );
    assert_eq!(
        f.evidence.get("matched_path").map(String::as_str),
        Some("src/util.rs")
    );
    assert!(!f.evidence.contains_key("resolved"), "name-based fallback");
}

/// M1 Precision Gate uniqueness: an ambiguous name (defined in TWO Rust files)
/// must emit NO name-based edge. Mirrors the C uniqueness negative.
#[test]
fn arch_003_ambiguous_name_emits_no_edge() {
    let units = common::load_units_fixture("PERF-RS-ARCH-003__ambiguous.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, RS_ARCH_003_RULE_ID).is_empty(),
        "an ambiguous cross-module name must not emit an ARCH-003 edge — got {:?}",
        common::by_rule(&findings, RS_ARCH_003_RULE_ID)
    );
}

// ===========================================================================
// PERF-RS-TYPE-001 — large struct passed by value
// ===========================================================================

#[test]
fn type_001_aggregates_by_value_params_and_ignores_references() {
    let units = common::load_units_fixture("PERF-RS-TYPE-001__large-struct-by-value.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_TYPE_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "&Big and Small parameters stay silent; one aggregated finding"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::Low);
    assert_eq!(f.evidence.get("structs").map(String::as_str), Some("Big"));
    assert_eq!(
        f.evidence.get("estimated_sizes").map(String::as_str),
        Some("32")
    );
    assert_eq!(f.evidence.get("params").map(String::as_str), Some("Big"));
    assert_eq!(f.evidence.get("threshold").map(String::as_str), Some("32"));
    assert_eq!(f.path, "src/geom.rs");
}

// ===========================================================================
// PERF-RS-TYPE-002 — declared-order padding candidate
// ===========================================================================

#[test]
fn type_002_padding_fires_for_structs_only() {
    let units = common::load_units_fixture("PERF-RS-TYPE-002__padding.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_TYPE_002_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "union/enum are not layout aggregates; packed struct has no waste"
    );
    let f = hits[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Low);
    assert_eq!(f.evidence.get("struct").map(String::as_str), Some("Loose"));
    assert_eq!(
        f.evidence.get("padding_bytes").map(String::as_str),
        Some("14")
    );
    assert!(
        f.message.contains("repr(Rust)"),
        "the honest repr(Rust) reorder caveat is mandatory"
    );
}

// ===========================================================================
// Language gating
// ===========================================================================

/// A synthetic Rust unit that WOULD trip the C ALLOC-001 and LOOP-001 rules
/// if rules were not language-scoped: a `malloc` call and a depth-3 loop nest
/// inside a `language == "rust"` file.
fn rust_hazard_unit() -> Vec<FileUnit> {
    let raw = r#"[
      {
        "file": {"path": "src/hazard.rs", "language": "rust", "loc": {"code": 12, "comment": 0, "blank": 0}, "function_count": 1, "global_var_count": 0, "include_counts": {"quoted": 0, "angle": 0}},
        "parse_health": {"status": "ok", "error_regions": [], "error_node_count": 0},
        "includes": [],
        "global_vars": [],
        "structs": [],
        "macros": [],
        "functions": [
          {
            "record": {"id": "src/hazard.rs#f#1:1", "name": "f", "path": "src/hazard.rs", "span": {"start_line": 1, "start_col": 1, "end_line": 12, "end_col": 2}, "param_count": 0, "param_types": [], "cyclomatic_complexity": 4, "cognitive_complexity": 4, "max_loop_depth": 3, "alloc_site_count": 1, "length_lines": 12},
            "calls": [
              {"span": {"start_line": 5, "start_col": 9, "end_line": 5, "end_col": 20}, "in_loop_depth": 3, "is_innermost": true, "resolver_tier": "syntactic", "macro_origin": false, "callee_name": "malloc", "callee_kind": "direct", "args": [], "statement_role": "nested", "conditional_guarded": false}
            ],
            "assignments": [], "binary_ops": [], "comparisons": [], "access_chains": [],
            "loops": [
              {"kind": "for", "depth": 1, "induction_vars": ["i"], "span": {"start_line": 2, "start_col": 5, "end_line": 11, "end_col": 6}, "resolver_tier": "syntactic", "macro_origin": false},
              {"kind": "for", "depth": 2, "induction_vars": ["j"], "span": {"start_line": 3, "start_col": 9, "end_line": 10, "end_col": 10}, "resolver_tier": "syntactic", "macro_origin": false},
              {"kind": "for", "depth": 3, "induction_vars": ["k"], "span": {"start_line": 4, "start_col": 13, "end_line": 9, "end_col": 14}, "resolver_tier": "syntactic", "macro_origin": false}
            ]
          }
        ]
      }
    ]"#;
    serde_json::from_str(raw).expect("hazard IR fixture parses")
}

#[test]
fn c_rules_never_run_on_rust_files() {
    let units = rust_hazard_unit();
    let findings = analyze(&units);
    assert!(
        findings.iter().all(|f| f.rule_id.starts_with("PERF-RS-")),
        "no PERF-C-* rule may fire on a Rust file, got {:?}",
        findings
            .iter()
            .map(|f| f.rule_id.as_str())
            .collect::<Vec<_>>()
    );
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != "PERF-C-ALLOC-001" && f.rule_id != "PERF-C-LOOP-001"),
        "the C alloc/loop hazard must be gated off"
    );
    // The Rust structural rules still see their own file.
    assert_eq!(
        common::by_rule(&findings, RS_LOOP_001_RULE_ID).len(),
        1,
        "the Rust LOOP-001 fires on the same hazard unit"
    );
}

#[test]
fn mixed_scan_keeps_each_language_on_its_own_rules() {
    let mut units = common::kitchen_units();
    units.push(common::helper_unit());
    units.extend(common::load_units_fixture(
        "PERF-RS-LOOP-001__deep-nest.json",
    ));

    let findings = analyze(&units);
    assert!(
        findings
            .iter()
            .filter(|f| f.path.ends_with(".rs"))
            .all(|f| f.rule_id.starts_with("PERF-RS-")),
        "Rust files only carry PERF-RS-* findings"
    );
    assert!(
        findings
            .iter()
            .filter(|f| f.path.ends_with(".c"))
            .all(|f| f.rule_id.starts_with("PERF-C-")),
        "C files only carry PERF-C-* findings"
    );
    assert_eq!(common::by_rule(&findings, RS_LOOP_001_RULE_ID).len(), 1);
}

// ===========================================================================
// Real-parse locks (LL-009): one per family group, over the dominant shape.
// ===========================================================================

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-rust-wc1-{tag}-{}-{nanos}",
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
fn real_parse_lock_loop_family() {
    let root = temp_root("loop");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn deep(a: &[i32]) {\n    for i in 0..3 {\n        for j in 0..3 {\n            for k in 0..3 {\n                let _ = (i, j, k, a);\n            }\n        }\n    }\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    assert!(
        !common::by_rule(&findings, RS_LOOP_001_RULE_ID).is_empty(),
        "real parse: a triple-nested Rust for loop fires RS-LOOP-001"
    );
    assert!(
        findings.iter().all(|f| f.rule_id.starts_with("PERF-RS-")),
        "real parse: gating holds on a Rust-only tree"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn real_parse_lock_call_family() {
    let root = temp_root("call");
    write_file(
        &root,
        "src/lib.rs",
        "pub fn rec(n: i64) -> i64 {\n    if n <= 0 {\n        0\n    } else {\n        rec(n - 1)\n    }\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    assert!(
        !common::by_rule(&findings, RS_CALL_001_RULE_ID).is_empty(),
        "real parse: a directly recursive Rust fn fires RS-CALL-001"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn real_parse_lock_arch_family() {
    let root = temp_root("arch");
    write_file(
        &root,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    write_file(&root, "src/lib.rs", "mod a;\nmod b;\n");
    write_file(&root, "src/a.rs", "use crate::b::B;\npub struct A;\n");
    write_file(&root, "src/b.rs", "use crate::a::A;\npub struct B;\n");
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    assert!(
        findings.iter().any(|f| {
            f.rule_id == RS_ARCH_001_RULE_ID
                && f.evidence.get("kind").map(String::as_str) == Some("cycle")
        }),
        "real parse: a two-module use cycle fires RS-ARCH-001"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn real_parse_lock_type_family() {
    let root = temp_root("type");
    write_file(
        &root,
        "src/lib.rs",
        "pub struct Big {\n    pub a: i64,\n    pub b: i64,\n    pub c: i64,\n    pub d: i64,\n}\n\npub fn take(b: Big) -> i64 {\n    b.a\n}\n",
    );
    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, RS_TYPE_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "real parse: a 32-byte struct passed by value fires RS-TYPE-001"
    );
    assert_eq!(
        hits[0].evidence.get("structs").map(String::as_str),
        Some("Big")
    );
    fs::remove_dir_all(&root).ok();
}
