//! Python async heuristics (plan `python-backend` W3):
//! PY-ASYNC-001/002 (Python-native additions).
//!
//! `ASYNC-001` (blocking call inside `async def`) is the highest-value native
//! rule: the `async def` gate is proven necessary (the W0 canary found 166
//! production blocking calls in sync functions vs 0 real hits). `ASYNC-002`
//! (await in a loop) consumes the additive `FunctionUnit.awaits` stream so it
//! never reads engine node kinds (docs/architecture/python-backend.md §6).

use crate::analyzer::finding::{PY_ASYNC_001_RULE_ID, PY_ASYNC_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::CalleeKind;
use crate::scanner::FileUnit;

use super::sort_findings;

/// Blocking client roots (HTTP + sync DB drivers).
const BLOCKING_ROOTS: [&str; 8] = [
    "requests",
    "httpx",
    "urllib",
    "session",
    "conn",
    "connection",
    "cursor",
    "engine",
];

/// Blocking method names.
const BLOCKING_METHODS: [&str; 14] = [
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "request",
    "execute",
    "executemany",
    "query",
    "fetchone",
    "fetchall",
    "commit",
    "recv",
    "send",
];

/// PY-ASYNC-001 — Blocking call inside `async def` (Python-native),
/// high-value, Warn.
///
/// A DIRECT call inside an `async def` body to `time.sleep`, a blocking HTTP
/// client (`requests`/`httpx`/`urllib`), a synchronous DB driver method, or the
/// BUILTIN `open` (no receiver root — so the non-blocking `aiofiles.open` is not
/// flagged) — each blocks the single event loop thread, stalling every other
/// task. Evidence `callee`, `receiver_root` (when present), `loop_depth`.
/// Warn/MEDIUM (receiver-root inference).
pub(crate) fn python_async_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if !func.record.is_async {
                continue;
            }
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let root = call.receiver_root.as_deref();
                let is_sleep = call.callee_name == "sleep" && root == Some("time");
                // Only the BUILTIN `open` blocks: a module-qualified `.open`
                // (e.g. `aiofiles.open`) is a non-blocking async client and
                // must not fire (plan `python-backend` W4, R-pf-093).
                let is_open = call.callee_name == "open" && root.is_none();
                let is_blocking_client = root.is_some_and(|r| BLOCKING_ROOTS.contains(&r))
                    && BLOCKING_METHODS.contains(&call.callee_name.as_str());
                if !(is_sleep || is_open || is_blocking_client) {
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
                if let Some(root) = root {
                    evidence.push(("receiver_root".to_string(), root.to_string()));
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_ASYNC_001_RULE_ID.to_string(),
                    title: "Blocking call inside `async def`".to_string(),
                    message: format!(
                        "'{}' blocks inside an `async def` — the synchronous \
                         call stalls the single event-loop thread and every \
                         other task; using the async equivalent (`asyncio.sleep`, \
                         an async HTTP/DB client) is a candidate optimization",
                        call.callee_name
                    ),
                    severity: Severity::Warn,
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

/// PY-ASYNC-002 — `await` inside a loop (Python-native), Warn.
///
/// Uses the additive [`crate::ir::AwaitEvent`] stream: one finding per await
/// whose `meta.in_loop_depth ≥ 1`. Sequential awaits in a loop serialise
/// latency that `asyncio.gather` could overlap. Evidence `loop_depth`.
/// Warn/MEDIUM.
pub(crate) fn python_async_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for await_event in &func.awaits {
                if await_event.meta.in_loop_depth == 0 {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_ASYNC_002_RULE_ID.to_string(),
                    title: "`await` inside a loop".to_string(),
                    message: format!(
                        "an `await` runs inside a loop at depth {} — awaiting \
                         sequentially serialises each coroutine's latency; \
                         collecting the coroutines and awaiting \
                         `asyncio.gather(...)` is a candidate optimization",
                        await_event.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: await_event.meta.resolver_tier,
                    span: await_event.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [(
                        "loop_depth".to_string(),
                        await_event.meta.in_loop_depth.to_string(),
                    )]
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
    use crate::analyzer::heuristics::python::test_support::{await_event, call, func, point, unit};
    use crate::ir::Span;

    #[test]
    fn async_001_fires_in_async_def_only() {
        let span = point(4);
        let in_async = func(
            "fetch",
            Span::default(),
            vec![call("get", Some("requests.get"), Some("requests"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        );
        assert_eq!(python_async_001(&[unit(vec![in_async])]).len(), 1);

        let sync_fn = func(
            "fetch",
            Span::default(),
            vec![call("get", Some("requests.get"), Some("requests"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_async_001(&[unit(vec![sync_fn])]).is_empty());
    }

    #[test]
    fn async_001_fires_on_builtin_open_only() {
        let span = point(4);
        // Bare builtin `open(...)` inside `async def` still fires.
        let builtin = func(
            "read",
            Span::default(),
            vec![call("open", None, None, span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        );
        assert_eq!(python_async_001(&[unit(vec![builtin])]).len(), 1);

        // A module-qualified `aiofiles.open(...)` is a non-blocking async
        // client and must NOT fire (R-pf-093 false positive).
        let aiofiles = func(
            "read",
            Span::default(),
            vec![call(
                "open",
                Some("aiofiles.open"),
                Some("aiofiles"),
                span,
                0,
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true,
        );
        assert!(python_async_001(&[unit(vec![aiofiles])]).is_empty());
    }

    #[test]
    fn async_002_fires_on_await_in_loop_only() {
        let f = func(
            "f",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![await_event(point(4), 1)],
            true,
        );
        assert_eq!(python_async_002(&[unit(vec![f])]).len(), 1);

        let outside = func(
            "f",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![await_event(point(4), 0)],
            true,
        );
        assert!(python_async_002(&[unit(vec![outside])]).is_empty());
    }
}
