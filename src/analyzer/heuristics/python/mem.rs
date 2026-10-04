//! Python mem-family heuristics (plan `python-backend` W3): PY-MEM-002.
//!
//! `MEM-001` (`sizeof`) and `TYPE-001/002` are RETIRED for Python — there is
//! no `sizeof` and no by-value struct layout to reason about.

use crate::analyzer::finding::PY_MEM_002_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::CalleeKind;
use crate::scanner::FileUnit;

use super::{path_final_segment, sort_findings};

/// Container copy family: module function `copy`/`deepcopy` and the method
/// `.copy()`.
const COPY_FAMILY: [&str; 2] = ["copy", "deepcopy"];

/// PY-MEM-002 — Container copy inside loop (transferred mirror of
/// PERF-C-MEM-002), Info.
///
/// A `copy`/`deepcopy` call (module function or `.copy()` method) at
/// `in_loop_depth ≥ 1` materialises a whole new container per iteration;
/// `deepcopy` additionally walks the object graph with a memo dict. Evidence
/// `callee`, `receiver_root` (when present), `loop_depth`. Info/MEDIUM.
pub(crate) fn python_mem_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0 || call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let by_name = COPY_FAMILY.contains(&call.callee_name.as_str());
                let by_method = call.receiver_root.is_some() && call.callee_name == "copy";
                let by_path = call
                    .callee_path
                    .as_deref()
                    .is_some_and(|p| COPY_FAMILY.contains(&path_final_segment(p)));
                if !(by_name || by_method || by_path) {
                    continue;
                }
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                let mut evidence = vec![
                    ("callee".to_string(), callee),
                    (
                        "loop_depth".to_string(),
                        call.meta.in_loop_depth.to_string(),
                    ),
                ];
                if let Some(root) = call.receiver_root.as_deref() {
                    evidence.push(("receiver_root".to_string(), root.to_string()));
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_MEM_002_RULE_ID.to_string(),
                    title: "Container copy inside loop".to_string(),
                    message: format!(
                        "'{}' copies a whole container every iteration at loop \
                         depth {} — CPython allocates and fills a new object \
                         (`deepcopy` also walks the graph with a memo dict); \
                         copying once outside the loop or reusing the buffer \
                         is a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: evidence.into_iter().collect(),
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
    fn mem_002_fires_in_loop_only() {
        let span = point(4);
        let in_loop = func(
            "f",
            Span::default(),
            vec![call(
                "deepcopy",
                Some("copy.deepcopy"),
                Some("copy"),
                span,
                1,
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_mem_002(&[unit(vec![in_loop])]).len(), 1);

        let outside = func(
            "f",
            Span::default(),
            vec![call(
                "deepcopy",
                Some("copy.deepcopy"),
                Some("copy"),
                span,
                0,
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_mem_002(&[unit(vec![outside])]).is_empty());
    }
}
