//! Python N+1 heuristics (plan `python-backend` W3): PY-N1-001, flagship.
//!
//! A DB/HTTP call inside a loop is the classic N+1: one round-trip per
//! iteration instead of a single batched/`IN (...)`/`JOIN` query. The
//! receiver-root + call-name family gate is load-bearing — an ungated
//! `execute`/`get` match would fire on every in-memory mapping too.

use crate::analyzer::finding::PY_N1_001_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::CalleeKind;
use crate::scanner::FileUnit;

use super::sort_findings;

/// HTTP client receiver roots.
const HTTP_ROOTS: [&str; 5] = ["requests", "session", "httpx", "urllib", "client"];

/// HTTP method names.
const HTTP_METHODS: [&str; 8] = [
    "get", "post", "put", "patch", "delete", "head", "options", "request",
];

/// DB/driver receiver roots.
const DB_ROOTS: [&str; 9] = [
    "conn",
    "connection",
    "cursor",
    "cur",
    "db",
    "engine",
    "collection",
    "session",
    "client",
];

/// DB driver method names (DB-API + common ORMs).
const DB_METHODS: [&str; 18] = [
    "execute",
    "executemany",
    "query",
    "fetchone",
    "fetchall",
    "fetchmany",
    "commit",
    "insert",
    "insert_one",
    "insert_many",
    "find",
    "find_one",
    "update",
    "update_one",
    "delete",
    "delete_one",
    "aggregate",
    "save",
];

/// PY-N1-001 — DB/HTTP call inside loop (Python-native), flagship, Warn.
///
/// A DIRECT call at `in_loop_depth ≥ 1` whose receiver root is in the HTTP or
/// DB root family AND whose callee name is in the matching method family. One
/// network/database round-trip per iteration (N+1). Evidence `callee`,
/// `receiver_root`, `family`, `loop_depth`. Warn/MEDIUM (receiver-root is
/// name-based inference; the call can be an in-memory object with the same
/// method names — an accepted FP class).
pub(crate) fn python_n1_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0 || call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref() else {
                    continue;
                };
                let family = if HTTP_ROOTS.contains(&root)
                    && HTTP_METHODS.contains(&call.callee_name.as_str())
                {
                    "http"
                } else if DB_ROOTS.contains(&root)
                    && DB_METHODS.contains(&call.callee_name.as_str())
                {
                    "db"
                } else {
                    continue;
                };
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_N1_001_RULE_ID.to_string(),
                    title: "DB/HTTP call inside loop (N+1)".to_string(),
                    message: format!(
                        "'{callee}' issues a {family} round-trip every iteration \
                         at loop depth {} — the N+1 pattern (one query/request \
                         per element); batching into a single `IN (...)`, JOIN, \
                         or bulk request is a candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        ("receiver_root".to_string(), root.to_string()),
                        ("family".to_string(), family.to_string()),
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
    fn n1_001_fires_on_db_call_in_loop_only() {
        let span = point(4);
        let in_loop = func(
            "f",
            Span::default(),
            vec![call("execute", Some("conn.execute"), Some("conn"), span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let out = python_n1_001(&[unit(vec![in_loop])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id, PY_N1_001_RULE_ID);

        let outside = func(
            "f",
            Span::default(),
            vec![call("execute", Some("conn.execute"), Some("conn"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_n1_001(&[unit(vec![outside])]).is_empty());

        let unrelated = func(
            "f",
            Span::default(),
            vec![call("execute", None, Some("mapping"), span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_n1_001(&[unit(vec![unrelated])]).is_empty());
    }
}
