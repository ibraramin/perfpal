//! Python comprehension heuristics (plan `python-backend` W3):
//! PY-COMPREHENSION-001 (Python-native addition).

use crate::analyzer::finding::PY_COMPREHENSION_001_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BindingKind, CalleeKind, StatementRole};
use crate::scanner::FileUnit;

use super::{enclosing_loop, names_with_kind, sort_findings};

/// PY-COMPREHENSION-001 — Manual loop collapsible to a comprehension
/// (Python-native addition), Info.
///
/// A loop whose body is a SINGLE `list.append` statement
/// (`CTL-APPEND-MULTI` gate): the append is the loop's only Primary call and
/// the loop body performs no assignment, so it rewrites directly to a list
/// comprehension. A list comprehension compiles to a specialised
/// `LIST_APPEND` opcode with inline iteration, avoiding the loop branch and
/// per-iteration method lookup. Evidence `receiver_root`, `loop_depth`.
/// Info/MEDIUM.
pub(crate) fn python_comprehension_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let lists = names_with_kind(func, BindingKind::List);
            if lists.is_empty() {
                continue;
            }
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.callee_name != "append"
                    || call.meta.in_loop_depth == 0
                    || call.statement_role != StatementRole::Primary
                {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref() else {
                    continue;
                };
                if !lists.contains(root) {
                    continue;
                }
                let Some(loop_node) = enclosing_loop(func, &call.meta.span) else {
                    continue;
                };
                // Single-statement gate: exactly this one Primary call inside
                // the loop span and no in-loop assignment.
                let primary_calls = func
                    .calls
                    .iter()
                    .filter(|c| {
                        c.meta.in_loop_depth >= 1
                            && c.statement_role == StatementRole::Primary
                            && super::super::span_contains(&loop_node.span, &c.meta.span)
                    })
                    .count();
                let assignments = func
                    .assignments
                    .iter()
                    .filter(|a| {
                        a.meta.in_loop_depth >= 1
                            && super::super::span_contains(&loop_node.span, &a.meta.span)
                    })
                    .count();
                if primary_calls != 1 || assignments != 0 {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_COMPREHENSION_001_RULE_ID.to_string(),
                    title: "Manual loop collapsible to a comprehension".to_string(),
                    message: format!(
                        "loop body appends to list '{root}' as its only statement \
                         at loop depth {} — a list comprehension compiles to a \
                         specialised `LIST_APPEND` with inline iteration, \
                         avoiding the loop branch and per-element method lookup; \
                         rewriting it as a comprehension is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("receiver_root".to_string(), root.to_string()),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{
        assign, call, func, loop_node, point, unit,
    };
    use crate::ir::Span;

    #[test]
    fn comprehension_001_fires_on_single_append_statement() {
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 6,
            end_col: 1,
        };
        let single = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            vec![call("append", Some("out.append"), Some("out"), point(4), 1)],
            vec![assign("out", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        assert_eq!(python_comprehension_001(&[unit(vec![single])]).len(), 1);

        // An in-loop assignment makes the body multi-statement → silent.
        let multi = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            vec![call("append", Some("out.append"), Some("out"), point(4), 1)],
            vec![
                assign("out", BindingKind::List, point(2), 0),
                assign("y", BindingKind::Unknown, point(5), 1),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        assert!(python_comprehension_001(&[unit(vec![multi])]).is_empty());
    }
}
