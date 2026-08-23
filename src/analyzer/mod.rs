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

/// The v1 registry. Adding a heuristic = appending a new entry whose rule_id
/// sorts into place (open pattern, minor-bump schema policy).
const REGISTRY: &[HeuristicFn] = &[heuristics::alloc_001];

mod heuristics {
    //! Catalog heuristics (catalog-v1.md). One module-level function each,
    //! pure over `[FileUnit]`.

    use super::finding::ALLOC_001_RULE_ID;
    use super::{Confidence, Finding, Severity};
    use crate::ir::{ALLOC_WRAPPER_PATTERN_ID, CallEvent, CalleeKind, MacroEventKind, ShapeKind};
    use crate::scanner::FileUnit;

    /// PERF-C-ALLOC-001 — Heap allocation inside loop `(syntactic)`
    /// (catalog-v1 §3 detail block).
    ///
    /// Detects: `malloc`/`calloc`/`realloc`/`aligned_alloc`/`strdup` calls
    /// nested anywhere inside a loop body (`in_loop_depth ≥ 1`). Default
    /// HIGH/hot; macro-origin capping is applied centrally (ir-v1.md §5).
    ///
    /// Macro policy (catalog detail block): a wrapper-macro USE inside a loop
    /// IS the alloc site under tree-sitter's no-expansion parse, so calls
    /// whose callee names a recognized alloc-wrapper macro (`pattern_id =
    /// [`ALLOC_WRAPPER_PATTERN_ID`] in the same file's MacroEvents) also
    /// fire — the central hook then caps them at MEDIUM + `macro_suspect`.
    pub(super) fn alloc_001(units: &[FileUnit]) -> Vec<Finding> {
        let mut findings = Vec::new();
        for f in alloc_candidates(units) {
            let message = format!(
                "per-iteration heap allocation via '{}' at loop depth {} — \
                 allocator churn dominates runtime; hoisting or buffer reuse \
                 is a candidate optimization",
                f.callee, f.loop_depth
            );
            findings.push(Finding {
                rule_id: ALLOC_001_RULE_ID.to_string(),
                title: "Heap allocation inside loop".to_string(),
                message,
                severity: Severity::Hot,
                confidence: Confidence::High,
                resolver_tier: crate::ir::ResolverTier::Syntactic,
                span: f.span,
                path: f.path.to_string(),
                evidence: [
                    ("callee".to_string(), f.callee.to_string()),
                    ("loop_depth".to_string(), f.loop_depth.to_string()),
                    (
                        "alloc_site_index".to_string(),
                        f.alloc_site_index.to_string(),
                    ),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            });
        }
        findings
    }

    /// Internal candidate shape shared by alloc-family heuristics.
    pub(super) struct AllocCandidate<'a> {
        pub path: &'a str,
        pub span: crate::ir::Span,
        pub callee: &'a str,
        pub loop_depth: u32,
        /// 1-based index of this call among the function's alloc-family
        /// calls (evidence for ALLOC-001/002 pairing downstream).
        pub alloc_site_index: u32,
    }

    /// Walk every CallEvent that is an alloc site: a DIRECT alloc-family
    /// call, or a call whose callee names a recognized alloc-wrapper macro in
    /// the same file (see [`alloc_001`] doc). Only sites with
    /// `in_loop_depth ≥ 1` become candidates.
    pub(super) fn alloc_candidates(units: &[FileUnit]) -> Vec<AllocCandidate<'_>> {
        let mut out = Vec::new();
        for unit in units {
            let wrapper_names: Vec<&str> = unit
                .macros
                .iter()
                .filter(|m| {
                    m.kind == MacroEventKind::RecognizedPattern
                        && m.pattern_id.as_deref() == Some(ALLOC_WRAPPER_PATTERN_ID)
                })
                .map(|m| m.name.as_str())
                .collect();
            for func in &unit.functions {
                let mut site_index = 0u32;
                for call in &func.calls {
                    if !is_alloc_call(call, &wrapper_names) {
                        continue;
                    }
                    site_index += 1;
                    if call.meta.in_loop_depth >= 1 {
                        out.push(AllocCandidate {
                            path: &unit.file.path,
                            span: call.meta.span,
                            callee: &call.callee_name,
                            loop_depth: call.meta.in_loop_depth,
                            alloc_site_index: site_index,
                        });
                    }
                }
            }
        }
        out
    }

    const ALLOC_FAMILY: [&str; 5] = ["malloc", "calloc", "realloc", "aligned_alloc", "strdup"];

    fn is_alloc_call(call: &CallEvent, wrapper_names: &[&str]) -> bool {
        if call.callee_kind != CalleeKind::Direct {
            return false;
        }
        ALLOC_FAMILY.contains(&call.callee_name.as_str())
            || wrapper_names.contains(&call.callee_name.as_str())
    }

    // Silence unused-import lint until more alloc heuristics land in P2;
    // ShapeKind is referenced by upcoming MEM-001 arg-shape matching.
    const _: Option<ShapeKind> = None;
}

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
