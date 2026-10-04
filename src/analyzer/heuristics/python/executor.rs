//! Python executor heuristics (plan `python-backend` W3):
//! PY-EXECUTOR-001 (Python-native addition).

use crate::analyzer::finding::PY_EXECUTOR_001_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::CalleeKind;
use crate::scanner::FileUnit;

use super::sort_findings;

/// Executor / event-loop constructors.
const EXECUTOR_FAMILY: [&str; 3] = [
    "ThreadPoolExecutor",
    "ProcessPoolExecutor",
    "new_event_loop",
];

/// PY-EXECUTOR-001 — Executor/event-loop creation per iteration (Python-native
/// addition), Info.
///
/// `ThreadPoolExecutor(...)`, `ProcessPoolExecutor(...)` or
/// `asyncio.new_event_loop()` at `in_loop_depth ≥ 1`: each construction spawns
/// worker threads/processes or allocates a selector + self-pipe, so creating
/// one per iteration never amortises the setup. Evidence `callee`,
/// `loop_depth`. Info/MEDIUM.
pub(crate) fn python_executor_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !EXECUTOR_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_EXECUTOR_001_RULE_ID.to_string(),
                    title: "Executor/event-loop creation per iteration".to_string(),
                    message: format!(
                        "'{callee}' is constructed every iteration at loop depth \
                         {} — CPython spawns worker threads/processes or \
                         allocates a new event loop (selector + self-pipe) each \
                         pass, never amortising the setup; creating the executor \
                         once outside the loop is a candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
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
    use crate::analyzer::heuristics::python::test_support::{call, func, point, unit};
    use crate::ir::Span;

    #[test]
    fn executor_001_fires_in_loop_only() {
        let span = point(5);
        let in_loop = func(
            "f",
            Span::default(),
            vec![call("ThreadPoolExecutor", None, None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_executor_001(&[unit(vec![in_loop])]).len(), 1);

        let outside = func(
            "f",
            Span::default(),
            vec![call("ThreadPoolExecutor", None, None, span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_executor_001(&[unit(vec![outside])]).is_empty());
    }
}
