//! Python call-graph-family heuristics (plan `python-backend` W3):
//! PY-CALL-001/002.

use crate::analyzer::finding::{PY_CALL_001_RULE_ID, PY_CALL_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ResolverTier};
use crate::scanner::FileUnit;

use super::sort_findings;

/// PY-CALL-001 — Direct recursion (transferred mirror of PERF-C-CALL-001),
/// Info.
///
/// Per function: a DIRECT call whose callee names the enclosing function
/// itself. Framed as the recursion-depth / `RecursionError` mechanism, not C
/// stack cost. One finding per function at the earliest self-call.
/// Info/HIGH.
pub(crate) fn python_call_001(units: &[FileUnit]) -> Vec<Finding> {
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
                rule_id: PY_CALL_001_RULE_ID.to_string(),
                title: "Direct recursion".to_string(),
                message: format!(
                    "'{}' calls itself directly at {} site(s) — each recursive \
                     frame costs a CPython call and risks `RecursionError` at \
                     the interpreter recursion limit; converting to iteration \
                     or memoizing is a candidate optimization",
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
    sort_findings(&mut findings);
    findings
}

/// PY-CALL-002 — High cyclomatic complexity (transferred mirror of
/// PERF-C-CALL-002), Info.
///
/// One finding per [`crate::ir::FunctionRecord`] whose branch count reaches
/// the shared threshold (default ≥ 15). Evidence `function`, `complexity`,
/// `threshold`. Info/HIGH (the metric is exact; only the threshold is a
/// heuristic).
pub(crate) fn python_call_002(units: &[FileUnit]) -> Vec<Finding> {
    use super::super::CALL_002_MIN_COMPLEXITY;
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if func.record.cyclomatic_complexity < CALL_002_MIN_COMPLEXITY {
                continue;
            }
            findings.push(Finding {
                threshold: Some(("min_complexity", CALL_002_MIN_COMPLEXITY)),
                rule_id: PY_CALL_002_RULE_ID.to_string(),
                title: "High cyclomatic complexity".to_string(),
                message: format!(
                    "'{}' branches {} ways (threshold {}) — complex control \
                     flow hides hot paths and resists testing; extracting \
                     helpers or table-driving the dispatch is a candidate \
                     optimization",
                    func.record.name, func.record.cyclomatic_complexity, CALL_002_MIN_COMPLEXITY
                ),
                severity: Severity::Info,
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
    sort_findings(&mut findings);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{call, func, point, unit};
    use crate::ir::Span;

    #[test]
    fn call_001_fires_on_self_call_only() {
        let span = point(3);
        let recursive = func(
            "walk",
            Span::default(),
            vec![call("walk", None, None, span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_call_001(&[unit(vec![recursive])]).len(), 1);

        let other = func(
            "walk",
            Span::default(),
            vec![call("step", None, None, span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_call_001(&[unit(vec![other])]).is_empty());
    }

    #[test]
    fn call_002_fires_at_threshold() {
        let mut complex = func(
            "branchy",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        complex.record.cyclomatic_complexity = 15;
        assert_eq!(python_call_002(&[unit(vec![complex])]).len(), 1);

        let simple = func(
            "tiny",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_call_002(&[unit(vec![simple])]).is_empty());
    }
}
