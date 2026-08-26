//! Call-graph-family heuristics (catalog-v1 §3 Tier 1):
//! PERF-C-CALL-001/002.

use crate::analyzer::finding::{CALL_001_RULE_ID, CALL_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ResolverTier};
use crate::scanner::FileUnit;

use super::CALL_002_MIN_COMPLEXITY;

/// PERF-C-CALL-001 — Direct recursion (catalog-v1 §3 Tier 1).
///
/// Per function: DIRECT calls whose callee names the enclosing function
/// itself. Indirect calls never count (name-based, presence-only — depth
/// analysis is deferred per catalog). One finding per function at the
/// earliest self-call site by `(start_line, start_col)`.
pub(crate) fn call_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let self_calls: Vec<&CallEvent> = func
                .calls
                .iter()
                .filter(|c| {
                    c.callee_kind == CalleeKind::Direct && c.callee_name == func.record.name
                })
                .collect();
            if self_calls.is_empty() {
                continue;
            }
            let earliest = self_calls
                .iter()
                .min_by_key(|c| (c.meta.span.start_line, c.meta.span.start_col))
                .expect("non-empty by the guard above");
            findings.push(Finding {
                rule_id: CALL_001_RULE_ID.to_string(),
                title: "Direct recursion".to_string(),
                message: format!(
                    "'{}' calls itself directly at {} site(s) — \
                     recursion keeps stack frames alive; converting to \
                     iteration or memoizing is a candidate optimization",
                    func.record.name,
                    self_calls.len()
                ),
                severity: Severity::Info,
                confidence: Confidence::High,
                resolver_tier: earliest.meta.resolver_tier,
                span: earliest.meta.span,
                path: unit.file.path.clone(),
                evidence: [
                    ("function".to_string(), func.record.name.clone()),
                    ("self_call_sites".to_string(), self_calls.len().to_string()),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            });
        }
    }
    findings
}

/// PERF-C-CALL-002 — High cyclomatic complexity (catalog-v1 §3 Tier 1).
///
/// One finding per [`FunctionRecord`] whose branch-count complexity
/// reaches [`CALL_002_MIN_COMPLEXITY`] (catalog default ≥ 15), anchored at
/// the function's record span. Pillar-1a stat promoted to a finding when
/// extreme — the stat itself stays in `stats.functions[]` (promotion, not
/// a move).
///
/// `FunctionRecord` carries no resolver tier of its own; v1 emits only
/// [`ResolverTier::Syntactic`] (ir-v1.md §2), so that tier is used.
pub(crate) fn call_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if func.record.cyclomatic_complexity < CALL_002_MIN_COMPLEXITY {
                continue;
            }
            findings.push(Finding {
                rule_id: CALL_002_RULE_ID.to_string(),
                title: "High cyclomatic complexity".to_string(),
                message: format!(
                    "'{}' branches {} ways (threshold {}) — complex \
                     control flow hides hot paths and resists testing; \
                     extracting helpers or table-driving the dispatch \
                     is a candidate optimization",
                    func.record.name, func.record.cyclomatic_complexity, CALL_002_MIN_COMPLEXITY
                ),
                severity: Severity::Warn,
                confidence: Confidence::High,
                resolver_tier: ResolverTier::Syntactic,
                span: func.record.span,
                path: unit.file.path.clone(),
                evidence: [
                    ("function".to_string(), func.record.name.clone()),
                    (
                        "complexity".to_string(),
                        func.record.cyclomatic_complexity.to_string(),
                    ),
                    ("threshold".to_string(), CALL_002_MIN_COMPLEXITY.to_string()),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            });
        }
    }
    findings
}
