//! Python loop-family heuristics (plan `python-backend` W3):
//! PY-LOOP-001/002/003/007/009.
//!
//! CPython mechanism notes: iteration is interpreter-driven, so the cost a
//! loop rule surfaces is per-iteration work — attribute/dict lookups, object
//! materialisation, blocking I/O — not the loop itself. Messages keep the
//! "candidate optimization" framing.

use std::collections::BTreeSet;

use crate::analyzer::finding::{
    PY_LOOP_001_RULE_ID, PY_LOOP_002_RULE_ID, PY_LOOP_003_RULE_ID, PY_LOOP_007_RULE_ID,
    PY_LOOP_009_RULE_ID,
};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BindingKind, CalleeKind, LoopNode, Shape, ShapeKind, Span};
use crate::scanner::FileUnit;

use super::{enclosing_induction_vars, names_with_kind, path_final_segment, sort_findings};

/// Slow-I/O defaults in the REVIEW-FREE classes that actually cost: file,
/// network, subprocess and socket work. `print`/`input` are DELIBERATELY
/// absent (72–83% of the C rule's raw hits): a per-iteration print is at most
/// an Info-grade logging concern the project does not ship as a perf finding.
const DIRECT_IO_FAMILY: [&str; 14] = [
    "open",
    "urlopen",
    "Popen",
    "check_output",
    "check_call",
    "getoutput",
    "recv",
    "recvfrom",
    "send",
    "sendto",
    "accept",
    "readline",
    "readlines",
    "writelines",
];

/// Network/socket receiver roots (receiver-root gated).
const NET_ROOTS: [&str; 6] = ["requests", "session", "httpx", "urllib", "socket", "conn"];

/// Network/socket method names.
const NET_METHODS: [&str; 10] = [
    "get", "post", "put", "patch", "delete", "request", "connect", "send", "recv", "recvfrom",
];

/// File-object read/write methods (fired only when the receiver is proven a
/// filehandle by the binding lattice).
const FILE_METHODS: [&str; 4] = ["read", "write", "readline", "writelines"];

/// Handle/process-setup calls (LOOP-002 message split, fix 5): acquiring the
/// handle per iteration is hoistable setup. Every other call the rule fires
/// on (streaming reads/writes, network methods, filehandle transfers) is
/// element-wise transfer — batching is the fix, not hoisting.
const SETUP_IO_CALLS: [&str; 7] = [
    "open",
    "urlopen",
    "Popen",
    "check_output",
    "check_call",
    "getoutput",
    "accept",
];

/// PY-LOOP-001 — Deep loop nesting (transferred mirror of PERF-C-LOOP-001),
/// Info.
///
/// One finding per loop whose `depth` is exactly 3 (a depth-5 chain yields one
/// finding at its depth-3 ancestor). Evidence `nest_depth`, `max_depth`;
/// threshold `("min_nesting_depth", 3)`.
pub(crate) fn python_loop_001(units: &[FileUnit]) -> Vec<Finding> {
    use super::super::LOOP_001_MIN_DEPTH;
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for loop_node in &func.loops {
                if loop_node.depth != LOOP_001_MIN_DEPTH {
                    continue;
                }
                let max_depth = func
                    .loops
                    .iter()
                    .filter(|l| super::super::span_contains(&loop_node.span, &l.span))
                    .map(|l: &LoopNode| l.depth)
                    .max()
                    .unwrap_or(LOOP_001_MIN_DEPTH)
                    .max(LOOP_001_MIN_DEPTH);
                findings.push(Finding {
                    threshold: Some(("min_nesting_depth", LOOP_001_MIN_DEPTH)),
                    rule_id: PY_LOOP_001_RULE_ID.to_string(),
                    title: "Deep loop nesting".to_string(),
                    message: format!(
                        "loops nest {} deep here (chain reaches depth {max_depth}) \
                         — CPython runs an interpreted loop body per element, so \
                         an O(n^k) nest is the primary hotspot zone; flattening \
                         the nesting or using itertools early-exit helpers is a \
                         candidate optimization",
                        LOOP_001_MIN_DEPTH
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::High,
                    resolver_tier: loop_node.resolver_tier,
                    span: loop_node.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("nest_depth".to_string(), LOOP_001_MIN_DEPTH.to_string()),
                        ("max_depth".to_string(), max_depth.to_string()),
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

/// PY-LOOP-002 — Expensive I/O call inside loop (reframed mirror of
/// PERF-C-LOOP-002), Warn.
///
/// Keeps file/network/subprocess/socket I/O, drops `print`/`input` entirely.
/// A call at `in_loop_depth ≥ 1` whose callee is a direct I/O name
/// ([`DIRECT_IO_FAMILY`]), a network-rooted method ([`NET_ROOTS`] ×
/// [`NET_METHODS`]), or a filehandle read/write method proven by the binding
/// lattice. Evidence `callee`, `receiver_root` (when present), `loop_depth`,
/// `io_kind`. Warn/MEDIUM (receiver-root/filehandle gates are inference).
///
/// Divergence-study message split (fix 5): acquiring a handle per iteration
/// ([`SETUP_IO_CALLS`]) is hoistable setup, while element-wise transfer
/// (streaming reads/writes, network methods, filehandle transfers) cannot be
/// hoisted — batching the transfer is the candidate.
///
/// Precision FP-class fix 1: Python has no `do-while(0)` shape (the producer
/// always emits `condition_is_zero: false`), so the zero-guard gate below is
/// a no-op today — kept for uniformity so a future backend emitting the flag
/// is gated without a heuristic change.
pub(crate) fn python_loop_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let filehandles: BTreeSet<String> = names_with_kind(func, BindingKind::Filehandle);
            for call in &func.calls {
                if call.meta.in_loop_depth == 0
                    || call.callee_kind != CalleeKind::Direct
                    || in_zero_guard_loop(&func.loops, &call.meta.span)
                {
                    continue;
                }
                let root = call.receiver_root.as_deref();
                let direct = DIRECT_IO_FAMILY.contains(&call.callee_name.as_str());
                let net = root.is_some_and(|r| NET_ROOTS.contains(&r))
                    && NET_METHODS.contains(&call.callee_name.as_str());
                let file = root.is_some_and(|r| filehandles.contains(r))
                    && FILE_METHODS.contains(&call.callee_name.as_str());
                if !(direct || net || file) {
                    continue;
                }
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                // Only a DIRECT handle-setup call is hoistable; network and
                // filehandle transfers are element-wise streaming.
                let hoistable = direct && SETUP_IO_CALLS.contains(&call.callee_name.as_str());
                let message = if hoistable {
                    format!(
                        "'{}' acquires a file/network/subprocess handle every \
                         iteration at loop depth {} — setup dominates the \
                         loop; hoisting the handle out of the loop is a \
                         candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    )
                } else {
                    format!(
                        "'{}' transfers element-wise every iteration at loop \
                         depth {} — the blocking transfer dominates the loop; \
                         batching into block operations is a candidate \
                         optimization",
                        call.callee_name, call.meta.in_loop_depth
                    )
                };
                let mut evidence = vec![
                    ("callee".to_string(), callee),
                    (
                        "loop_depth".to_string(),
                        call.meta.in_loop_depth.to_string(),
                    ),
                    (
                        "io_kind".to_string(),
                        if hoistable { "hoistable" } else { "streaming" }.to_string(),
                    ),
                ];
                if let Some(root) = root {
                    evidence.push(("receiver_root".to_string(), root.to_string()));
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_002_RULE_ID.to_string(),
                    title: "Expensive I/O call inside loop".to_string(),
                    message,
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

/// PY-LOOP-003 — `range(len(seq))` iteration (transferred mirror of
/// PERF-C-LOOP-003), Info.
///
/// A `range(...)` call at `in_loop_depth ≥ 1` whose first argument is a `len`
/// call — indexing a sequence by position instead of iterating it. The `len()`
/// gate keeps this false-positive-free (`range(n)` over a counter stays
/// silent). Evidence `callee`, `loop_depth`. Info/HIGH.
pub(crate) fn python_loop_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.callee_name != "range"
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let range_len = call.args.first().is_some_and(|a| a.kind == ShapeKind::Call)
                    && call
                        .args
                        .first()
                        .and_then(|a| a.name.as_deref())
                        .is_some_and(|n| path_final_segment(n) == "len");
                if !range_len {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_003_RULE_ID.to_string(),
                    title: "`range(len(seq))` iteration".to_string(),
                    message: format!(
                        "`range(len(...))` at loop depth {} indexes the sequence \
                         by position and re-runs `len` each pass — iterating the \
                         sequence directly or using `enumerate` is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), "range".to_string()),
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

/// PY-LOOP-007 — Linear scan of a list inside a loop (transferred mirror of
/// PERF-C-LOOP-007), flagship, Warn.
///
/// Two loop-scoped, list-kind-gated shapes (the kind gate excludes `set`/`dict`,
/// killing the canary's `CTL-IN-SET` false-positive class):
/// 1. **membership scan** — a [`crate::ir::ComparisonEvent`] with
///    `operator ∈ {"in", "not in"}` at `in_loop_depth ≥ 1` where one operand
///    resolves to a list-bound name (`x in seq`): CPython's list membership is
///    an O(n) linear scan;
/// 2. **index scan** — an [`crate::ir::AccessChainEvent`] whose ROOT is
///    list-bound and whose subscript index IS an enclosing loop's induction
///    variable (`seq[i]`, including the LONE-subscript form now emitted by the
///    backend, and `seq[i].field` chains): positional re-scan of the whole list.
/// 3. **`.index()` scan** — a DIRECT [`crate::ir::CallEvent`] named `index` whose
///    `receiver_root` is list-bound at `in_loop_depth ≥ 1` (`seq.index(x)`):
///    CPython's list `.index()` is an O(n) linear scan (plan `python-backend`
///    W4, R-pf-093).
///
/// Evidence `root`, (`method` | `index_ident` | `operator`), `binding_kind`,
/// `loop_depth`. Warn/MEDIUM (binding kind is inference).
pub(crate) fn python_loop_007(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let lists = names_with_kind(func, BindingKind::List);
            if lists.is_empty() {
                continue;
            }
            // Membership shape: `x in seq` / `x not in seq` with a list-bound
            // container, loop-scoped (plan `python-backend` W3).
            for cmp in &func.comparisons {
                if cmp.meta.in_loop_depth == 0 || (cmp.operator != "in" && cmp.operator != "not in")
                {
                    continue;
                }
                let Some(root) = list_member_name(&cmp.left, &cmp.right, &lists) else {
                    continue;
                };
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_007_RULE_ID.to_string(),
                    title: "Linear list scan inside loop".to_string(),
                    message: format!(
                        "list '{root}' is tested with '{}' inside a loop at \
                         depth {} — CPython's list membership is an O(n) linear \
                         scan, so re-testing per outer iteration is O(n²); \
                         building a set/dict lookup or iterating directly is a \
                         candidate optimization",
                        cmp.operator, cmp.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: cmp.meta.resolver_tier,
                    span: cmp.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("root".to_string(), root.to_string()),
                        ("operator".to_string(), cmp.operator.clone()),
                        ("binding_kind".to_string(), "list".to_string()),
                        ("loop_depth".to_string(), cmp.meta.in_loop_depth.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
            // Index shape: `seq[i]` (lone subscript) or `seq[i].field` chains
            // whose index is an enclosing induction variable.
            for chain in &func.access_chains {
                if chain.meta.in_loop_depth == 0 || !lists.contains(&chain.root) {
                    continue;
                }
                let induced = enclosing_induction_vars(func, &chain.meta.span);
                let Some(index_ident) = chain
                    .elements
                    .iter()
                    .filter(|e| e.kind == ShapeKind::Subscript)
                    .filter_map(|e| e.index_ident.as_deref())
                    .find(|idx| induced.contains(*idx))
                else {
                    continue;
                };
                let list_depth = chain.meta.in_loop_depth;
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_007_RULE_ID.to_string(),
                    title: "Linear index scan of a list inside loop".to_string(),
                    message: format!(
                        "list '{}' is indexed by loop variable '{}' at depth {} — \
                         CPython's list subscript is O(1) per access, but a scan \
                         over the whole list re-walks it per outer iteration \
                         (O(n²)); iterating directly, using `enumerate`, or \
                         building a set/dict lookup is a candidate optimization",
                        chain.root, index_ident, list_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: chain.meta.resolver_tier,
                    span: chain.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("root".to_string(), chain.root.clone()),
                        ("index_ident".to_string(), index_ident.to_string()),
                        ("binding_kind".to_string(), "list".to_string()),
                        ("loop_depth".to_string(), list_depth.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
            // `.index()` shape: a direct call `seq.index(x)` whose receiver root
            // is list-bound at loop depth ≥ 1. The Python backend emits an
            // `attribute` callee as `Direct` and populates `receiver_root` with
            // the leftmost chain name (so `seq.index` → `receiver_root = "seq"`).
            // CPython's list `.index()` is an O(n) linear scan (plan
            // `python-backend` W4, R-pf-093).
            for call in &func.calls {
                if call.meta.in_loop_depth == 0
                    || call.callee_kind != CalleeKind::Direct
                    || call.callee_name != "index"
                {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref().filter(|r| lists.contains(*r))
                else {
                    continue;
                };
                let call_depth = call.meta.in_loop_depth;
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_007_RULE_ID.to_string(),
                    title: "Linear list scan inside loop".to_string(),
                    message: format!(
                        "list '{root}' is scanned with `.index()` inside a loop \
                         at depth {call_depth} — CPython's list `.index()` is an \
                         O(n) linear scan, so re-scanning per outer iteration is \
                         O(n²); building a set/dict lookup or a positional map is \
                         a candidate optimization"
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("root".to_string(), root.to_string()),
                        ("method".to_string(), "index".to_string()),
                        ("binding_kind".to_string(), "list".to_string()),
                        ("loop_depth".to_string(), call_depth.to_string()),
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

/// The list-bound identifier name used as the membership container in an `in` /
/// `not in` comparison (`x in seq` → `seq`), if either operand is list-bound.
fn list_member_name<'a>(
    left: &'a Shape,
    right: &'a Shape,
    lists: &BTreeSet<String>,
) -> Option<&'a str> {
    let named = |shape: &'a Shape| -> Option<&'a str> {
        if shape.kind != ShapeKind::Identifier {
            return None;
        }
        shape.name.as_deref().filter(|n| lists.contains(*n))
    };
    named(left).or_else(|| named(right))
}

/// PY-LOOP-009 — `try`/`except` inside a loop (Python-native addition), Info.
///
/// One finding per [`crate::ir::TryEvent`] whose `meta.in_loop_depth ≥ 1`: a
/// `try_statement` in a loop body makes CPython set up exception handling each
/// iteration. The finding is honest that the zero-cost-`try` claim is
/// version-dependent — CPython ≥ 3.11 moved to zero-cost exceptions, so this is
/// a readability/robustness-flavoured optimization (the Ruff `PERF203`
/// analogue), not a guaranteed hot spot. Evidence `loop_depth`. Info/MEDIUM.
pub(crate) fn python_loop_009(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for try_event in &func.try_blocks {
                if try_event.meta.in_loop_depth == 0 {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_LOOP_009_RULE_ID.to_string(),
                    title: "`try`/`except` inside a loop".to_string(),
                    message: format!(
                        "a `try`/`except` sits inside a loop body at depth {} — \
                         CPython sets up exception-handling state for the block, \
                         so per-iteration `try` adds work on the hot path (the \
                         zero-cost-`try` optimization is version-dependent: \
                         CPython ≥ 3.11). Hoisting the `try` out of the loop or \
                         narrowing it is a candidate optimization",
                        try_event.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: try_event.meta.resolver_tier,
                    span: try_event.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [(
                        "loop_depth".to_string(),
                        try_event.meta.in_loop_depth.to_string(),
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

/// Precision FP-class fix 1 shared gate: true when `span`'s innermost
/// enclosing loop carries the producer-set `(0)`-condition flag. The Python
/// backend never sets the flag (no do-while shape); the gate is uniformity
/// with the C rule.
fn in_zero_guard_loop(loops: &[LoopNode], span: &Span) -> bool {
    loops
        .iter()
        .filter(|l| super::super::span_contains(&l.span, span))
        .min_by_key(|l| {
            (
                std::cmp::Reverse(l.depth),
                l.span.start_line,
                l.span.start_col,
            )
        })
        .is_some_and(|l| l.condition_is_zero)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{
        assign, call, comparison, func, lone_subscript, loop_node, point, subscript_chain,
        try_event, unit, with_call_arg,
    };
    use crate::ir::Span;

    fn outer(line_end: u32) -> LoopNode {
        loop_node(
            1,
            &["i"],
            Span {
                start_line: 1,
                start_col: 1,
                end_line: line_end,
                end_col: 1,
            },
        )
    }

    #[test]
    fn loop_001_fires_only_at_depth_three() {
        let deep = func(
            "f",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![
                loop_node(1, &["i"], point(2)),
                loop_node(2, &["j"], point(3)),
                loop_node(3, &["k"], point(4)),
            ],
            Vec::new(),
            false,
        );
        let out = python_loop_001(&[unit(vec![deep])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id, PY_LOOP_001_RULE_ID);

        let shallow = func(
            "f",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(2, &["j"], point(3))],
            Vec::new(),
            false,
        );
        assert!(python_loop_001(&[unit(vec![shallow])]).is_empty());
    }

    #[test]
    fn loop_002_fires_on_network_io_but_not_print() {
        let span = point(4);
        let net = func(
            "f",
            Span::default(),
            vec![call("get", Some("requests.get"), Some("requests"), span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let hits = python_loop_002(&[unit(vec![net])]);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].evidence.get("io_kind").map(String::as_str),
            Some("streaming"),
            "network transfer is element-wise streaming"
        );

        let logging = func(
            "f",
            Span::default(),
            vec![call("print", None, None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_loop_002(&[unit(vec![logging])]).is_empty());
    }

    #[test]
    fn loop_002_splits_setup_from_streaming() {
        let span = point(4);
        let setup = func(
            "f",
            Span::default(),
            vec![call("open", None, None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let hits = python_loop_002(&[unit(vec![setup])]);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].evidence.get("io_kind").map(String::as_str),
            Some("hoistable")
        );
        assert!(
            hits[0].message.contains("hoisting the handle"),
            "setup message names hoisting: {}",
            hits[0].message
        );

        let streaming = func(
            "f",
            Span::default(),
            vec![call("recv", None, None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let hits = python_loop_002(&[unit(vec![streaming])]);
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].evidence.get("io_kind").map(String::as_str),
            Some("streaming")
        );
        assert!(
            hits[0].message.contains("batching into block operations"),
            "streaming message names batching: {}",
            hits[0].message
        );
    }

    #[test]
    fn loop_003_fires_on_range_len_only() {
        let span = point(4);
        let rl = func(
            "f",
            Span::default(),
            vec![with_call_arg(call("range", None, None, span, 1), "len")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_loop_003(&[unit(vec![rl])]).len(), 1);

        let plain = func(
            "f",
            Span::default(),
            vec![call("range", None, None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_loop_003(&[unit(vec![plain])]).is_empty());
    }

    #[test]
    fn loop_007_fires_on_list_index_scan_silent_for_set() {
        let lspan = Span {
            start_line: 1,
            start_col: 1,
            end_line: 12,
            end_col: 1,
        };
        let list_scan = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seq", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            vec![subscript_chain("seq", "i", point(5), 1)],
            vec![outer(12)],
            Vec::new(),
            false,
        );
        let out = python_loop_007(&[unit(vec![list_scan])]);
        assert_eq!(out.len(), 1);

        let set_scan = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seen", BindingKind::Set, point(2), 0)],
            Vec::new(),
            Vec::new(),
            vec![subscript_chain("seen", "i", point(5), 1)],
            vec![outer(12)],
            Vec::new(),
            false,
        );
        assert!(python_loop_007(&[unit(vec![set_scan])]).is_empty());
    }

    #[test]
    fn loop_009_fires_on_try_inside_loop_silent_outside() {
        let mut inside = func(
            "f",
            Span::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![outer(9)],
            Vec::new(),
            false,
        );
        inside.try_blocks = vec![try_event(point(5), 1)];
        let out = python_loop_009(&[unit(vec![inside])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule_id, PY_LOOP_009_RULE_ID);
        assert_eq!(out[0].severity, Severity::Info);

        let mut outside = func(
            "f",
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
        outside.try_blocks = vec![try_event(point(5), 0)];
        assert!(python_loop_009(&[unit(vec![outside])]).is_empty());
    }

    #[test]
    fn loop_007_fires_on_list_membership_silent_for_set() {
        let lspan = Span {
            start_line: 1,
            start_col: 1,
            end_line: 12,
            end_col: 1,
        };
        let list_member = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seq", BindingKind::List, point(2), 0)],
            Vec::new(),
            vec![comparison("in", Some("x"), Some("seq"), point(5), 1)],
            Vec::new(),
            vec![outer(12)],
            Vec::new(),
            false,
        );
        let out = python_loop_007(&[unit(vec![list_member])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(
            out[0].evidence.get("operator").map(String::as_str),
            Some("in")
        );

        let set_member = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seen", BindingKind::Set, point(2), 0)],
            Vec::new(),
            vec![comparison("in", Some("x"), Some("seen"), point(5), 1)],
            Vec::new(),
            vec![outer(12)],
            Vec::new(),
            false,
        );
        assert!(python_loop_007(&[unit(vec![set_member])]).is_empty());
    }

    #[test]
    fn loop_007_fires_on_lone_subscript_list_scan() {
        let lspan = Span {
            start_line: 1,
            start_col: 1,
            end_line: 12,
            end_col: 1,
        };
        let lone = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seq", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            vec![lone_subscript("seq", "i", point(5), 1)],
            vec![outer(12)],
            Vec::new(),
            false,
        );
        let out = python_loop_007(&[unit(vec![lone])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(
            out[0].evidence.get("index_ident").map(String::as_str),
            Some("i")
        );

        // A lone subscript into a set stays silent (kind gate).
        let set_lone = func(
            "f",
            lspan,
            Vec::new(),
            vec![assign("seen", BindingKind::Set, point(2), 0)],
            Vec::new(),
            Vec::new(),
            vec![lone_subscript("seen", "i", point(5), 1)],
            vec![outer(12)],
            Vec::new(),
            false,
        );
        assert!(python_loop_007(&[unit(vec![set_lone])]).is_empty());
    }

    #[test]
    fn loop_007_fires_on_list_index_method_call_silent_for_set() {
        let lspan = Span {
            start_line: 1,
            start_col: 1,
            end_line: 12,
            end_col: 1,
        };
        let list_index = func(
            "f",
            lspan,
            vec![call("index", Some("seq.index"), Some("seq"), point(5), 1)],
            vec![assign("seq", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![outer(12)],
            Vec::new(),
            false,
        );
        let out = python_loop_007(&[unit(vec![list_index])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(
            out[0].evidence.get("method").map(String::as_str),
            Some("index")
        );

        // Outside a loop nothing fires.
        let outside = func(
            "f",
            lspan,
            vec![call("index", Some("seq.index"), Some("seq"), point(5), 0)],
            vec![assign("seq", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_loop_007(&[unit(vec![outside])]).is_empty());

        // A set-bound receiver stays silent (kind gate).
        let set_index = func(
            "f",
            lspan,
            vec![call("index", Some("seen.index"), Some("seen"), point(5), 1)],
            vec![assign("seen", BindingKind::Set, point(2), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![outer(12)],
            Vec::new(),
            false,
        );
        assert!(python_loop_007(&[unit(vec![set_index])]).is_empty());
    }
}
