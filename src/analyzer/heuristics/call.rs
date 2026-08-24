//! Call-graph-family heuristics (catalog-v1 §3 Tier 1): PERF-C-CALL-001.

use crate::analyzer::finding::CALL_001_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind};
use crate::scanner::FileUnit;

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
