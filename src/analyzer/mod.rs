//! `analyzer` — heuristic registry execution (modules.md §3.3).
//!
//! Consumes: IR stream (read-only). Produces: [`Finding`]s in the report §4
//! total order (severity → confidence → rule_id → path → start_line →
//! start_col) so document order equals report order.
//!
//! Responsibilities: execute every registered heuristic over the IR; enforce
//! ParseHealth gating and the downgrade hooks (macro origin, error-region
//! overlap — ir-v1.md §5; cross-file name-based edges are ARCH-003's own LOW
//! tier and arrive with that rule); construct `finding_id` values; enforce
//! uniqueness — identical `(rule_id, path, start_line, start_col)` MUST NOT
//! exist; the registry collapses duplicates deterministically (first in
//! execution order; heuristics run in ascending rule_id).
//!
//! Purity rules (modules.md §4.1): heuristics are pure IR-view functions —
//! no I/O, no clock, no global state, no sibling-order dependence. ParseHealth
//! gating is applied CENTRALLY after candidate collection so no heuristic can
//! forget it: findings overlapping an error region are capped at MEDIUM and
//! flagged `error_node_overlap` — downgraded, never silently dropped
//! (modules.md Principle 3). Parser-free testing against tests/fixtures/ir/
//! per ADR-0001 guardrail 2.
//!
//! Hard seam rule (ADR-0001 guardrail 1): this module imports ONLY
//! `crate::ir` (+ `crate::scanner`'s IR-stream container types) — zero
//! tree-sitter types.
//!
//! Non-goals: graph/component building; stats assembly; rendering; runtime
//! threshold configuration.

use std::collections::BTreeMap;

use crate::ir::{MacroEventKind, ParseStatus};
use crate::scanner::FileUnit;

pub mod finding;
pub use finding::{Confidence, Finding, Severity};

/// Registry of heuristics, executed in ascending `rule_id` order
/// (modules.md §3.3). Each entry is a pure function over read-only IR views.
type HeuristicFn = fn(&[FileUnit]) -> Vec<Finding>;

/// The v1 registry, DECLARED IN ASCENDING `rule_id` ORDER — execution
/// order equals declaration order and duplicate collapse is first-wins, so
/// declaration order is load-bearing (modules.md §3.3). Adding a heuristic
/// = inserting a new entry whose rule_id sorts into place (open pattern,
/// minor-bump schema policy):
/// alloc_001, alloc_002, call_001, loop_001, loop_003, str_001, str_002, str_003.
const REGISTRY: &[HeuristicFn] = &[
    heuristics::alloc_001,
    heuristics::alloc_002,
    heuristics::call_001,
    heuristics::loop_001,
    heuristics::loop_003,
    heuristics::str_001,
    heuristics::str_002,
    heuristics::str_003,
];

mod heuristics;

/// Run the full registry over the IR stream and return findings in the
/// report §4 total order (modules.md §3.3 / report §4):
/// severity → confidence → rule_id → path → start_line → start_col.
#[must_use]
pub fn analyze(units: &[FileUnit]) -> Vec<Finding> {
    // Heuristics run in ascending rule_id = REGISTRY declaration order;
    // candidates keep emission order for deterministic duplicate collapse.
    let mut findings: Vec<Finding> = REGISTRY
        .iter()
        .flat_map(|heuristic| heuristic(units))
        .collect();

    apply_downgrade_hooks(units, &mut findings);
    collapse_duplicates(&mut findings);
    findings.sort_by(total_order);
    findings
}

/// Central downgrade hooks (ir-v1.md §5), applied uniformly:
/// 1. macro origin — finding span intersects a `recognized_pattern`
///    MacroEvent ⇒ cap MEDIUM + `macro_suspect=true`;
/// 2. ParseHealth gating — finding span overlaps an error region ⇒ cap
///    MEDIUM + `error_node_overlap=true` (never silently dropped).
fn apply_downgrade_hooks(units: &[FileUnit], findings: &mut [Finding]) {
    for finding in findings.iter_mut() {
        let Some(unit) = units.iter().find(|u| u.file.path == finding.path) else {
            continue;
        };
        let macro_hit = unit.macros.iter().any(|m| {
            m.kind == MacroEventKind::RecognizedPattern && m.span.intersects(&finding.span)
        }) || event_macro_origin(unit, &finding.span);
        if macro_hit {
            finding.macro_suspect = true;
            finding.confidence = Confidence::Medium;
        }
        if unit.parse_health.status != ParseStatus::Ok
            && unit
                .parse_health
                .error_regions
                .iter()
                .any(|region| region.intersects(&finding.span))
        {
            finding.error_node_overlap = true;
            finding.confidence = Confidence::Medium;
        }
    }
}

/// True when any event overlapping `span` carries `macro_origin=true`
/// (wrapper USE sites — the recognizer's definition-span intersection alone
/// cannot see them, since the pattern event anchors at the `#define`).
fn event_macro_origin(unit: &FileUnit, span: &crate::ir::Span) -> bool {
    unit.functions.iter().any(|f| {
        f.calls
            .iter()
            .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.assignments
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.binary_ops
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.comparisons
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.access_chains
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
    })
}

/// Uniqueness enforcement (modules.md §3.3): collapse identical
/// `(rule_id, path, start_line, start_col)` keeping the FIRST in execution
/// order (registry order + emission order within a heuristic).
fn collapse_duplicates(findings: &mut Vec<Finding>) {
    let mut seen: BTreeMap<(String, String, u32, u32), usize> = BTreeMap::new();
    let mut kept = Vec::with_capacity(findings.len());
    for finding in findings.drain(..) {
        let key = (
            finding.rule_id.clone(),
            finding.path.clone(),
            finding.span.start_line,
            finding.span.start_col,
        );
        match seen.get(&key) {
            Some(_) => {} // duplicate: first-in-execution-order wins
            None => {
                seen.insert(key, kept.len());
                kept.push(finding);
            }
        }
    }
    *findings = kept;
}

/// Report §4 total order comparator.
fn total_order(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    b.severity
        .cmp(&a.severity)
        .then(b.confidence.cmp(&a.confidence))
        .then(a.rule_id.cmp(&b.rule_id))
        .then(a.path.cmp(&b.path))
        .then(a.span.start_line.cmp(&b.span.start_line))
        .then(a.span.start_col.cmp(&b.span.start_col))
}
