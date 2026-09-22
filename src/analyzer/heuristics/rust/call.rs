//! Rust call-graph-family heuristics (plan `rust-backend` R3 workstream C1):
//! PERF-RS-CALL-001/002, mirroring PERF-C-CALL-001/002.

use crate::analyzer::finding::{RS_CALL_001_RULE_ID, RS_CALL_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ResolverTier};
use crate::scanner::FileUnit;

use super::super::CALL_002_MIN_COMPLEXITY;

/// PERF-RS-CALL-001 — Direct recursion (mirrors PERF-C-CALL-001).
///
/// Intent: a function that calls its own name keeps stack frames alive and
/// can blow up exponentially. Detection: per function, DIRECT [`CallEvent`]s
/// whose callee names the enclosing function; ONE finding at the earliest
/// self-call site by `(start_line, start_col)`; evidence `function`,
/// `self_call_sites`. Info/HIGH.
///
/// Accepted limitations: role/method resolution is name-based (a same-named
/// method on another type in scope can false-fire); recursion DEPTH is not
/// bounded (presence-only, catalog deferral); mutual recursion is a later
/// cross-file wave. Indirect calls never count.
pub(crate) fn rust_call_001(units: &[FileUnit]) -> Vec<Finding> {
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
                threshold: None,
                rule_id: RS_CALL_001_RULE_ID.to_string(),
                title: "Direct recursion (Rust)".to_string(),
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

/// PERF-RS-CALL-002 — High cyclomatic complexity (mirrors PERF-C-CALL-002).
///
/// Intent: promote the pillar-1a complexity stat to a finding when extreme.
/// Detection: [`crate::ir::FunctionRecord::cyclomatic_complexity`] ≥
/// [`CALL_002_MIN_COMPLEXITY`] (15); threshold `("min_complexity", 15)`;
/// evidence `function`, `complexity`, `threshold`. Warn/HIGH. Rust
/// complexity counts `if_expression`, each `match_arm`, loops, `&&`/`||`,
/// and `try_expression` (backend formula, rust-backend.md §8) — accepted
/// approximation, `match`-heavy code legitimately scores high.
pub(crate) fn rust_call_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if func.record.cyclomatic_complexity < CALL_002_MIN_COMPLEXITY {
                continue;
            }
            findings.push(Finding {
                threshold: Some(("min_complexity", CALL_002_MIN_COMPLEXITY)),
                rule_id: RS_CALL_002_RULE_ID.to_string(),
                title: "High cyclomatic complexity (Rust)".to_string(),
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
