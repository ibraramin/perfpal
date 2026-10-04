//! Python alloc-family heuristics (plan `python-backend` W3):
//! PY-ALLOC-001/003/004.
//!
//! CPython mechanism (why these are candidate optimizations, not bug
//! verdicts): every container/object construction allocates a fresh
//! `PyObject` header and the list/dict backing store; `list_resize` grows the
//! store by over-allocation and each deallocation returns the store to the
//! allocator, so a per-iteration construction is allocator churn. Accumulating
//! with non-in-place `+` rebuilds the whole list each pass (O(n²) copying);
//! `list(iterable)`/`dict(pairs)` force eager O(n) materialisation and defeat
//! laziness.

use std::collections::BTreeSet;

use crate::analyzer::finding::{PY_ALLOC_001_RULE_ID, PY_ALLOC_003_RULE_ID, PY_ALLOC_004_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BindingKind, CalleeKind, Shape, ShapeKind};
use crate::scanner::FileUnit;

use super::{
    assigned_within, enclosing_induction_vars, enclosing_loop, mentions_any, names_with_kind,
    shape_text, sort_findings,
};

/// Python object-construction family (the `alloc_site_count` contract too).
const CONSTRUCTOR_FAMILY: [&str; 13] = [
    "list",
    "dict",
    "set",
    "tuple",
    "str",
    "bytes",
    "bytearray",
    "frozenset",
    "open",
    "DataFrame",
    "Series",
    "array",
    "ndarray",
];

/// Containers whose one-argument form eagerly re-materialises an iterable.
const MATERIALISE_FAMILY: [&str; 4] = ["list", "dict", "set", "tuple"];

/// PY-ALLOC-001 — Loop-invariant construction (reframed mirror of
/// PERF-C-ALLOC-001), Info.
///
/// Ship ONLY as loop-invariant construction: a constructor call inside a loop
/// whose arguments are all literals (or absent), so the constructed value is
/// independent of the loop variable. An f-string in a loop is NOT a call and
/// never fires (the C rule's dominant 98–99.6% false-positive class); any call
/// or identifier argument is deferred to PY-ALLOC-004. Evidence `callee`,
/// `loop_depth`. Info/MEDIUM (loop-invariance is a syntactic approximation).
pub(crate) fn python_alloc_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !CONSTRUCTOR_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                // Loop-invariant only: every argument is a literal (or there
                // are none). Identifier/call arguments are materialisation
                // shapes owned by PY-ALLOC-004.
                if !call.args.iter().all(|a| a.kind == ShapeKind::Literal) {
                    continue;
                }
                let induced = enclosing_induction_vars(func, &call.meta.span);
                if call
                    .args
                    .iter()
                    .any(|a| mentions_any(&shape_text(a), &induced))
                {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_ALLOC_001_RULE_ID.to_string(),
                    title: "Loop-invariant construction inside loop".to_string(),
                    message: format!(
                        "'{}' constructs a fresh value on every iteration at \
                         loop depth {} — CPython allocates a new object header \
                         and backing store each pass (and deallocates it on \
                         rebind); hoisting the loop-invariant construction is \
                         a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
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

/// PY-ALLOC-003 — Superlinear accumulation in loop (transferred mirror of
/// PERF-C-ALLOC-003), flagship, Warn.
///
/// Two gated shapes:
/// 1. **non-in-place concatenation** — a `+` [`crate::ir::BinaryOpEvent`] at
///    `in_loop_depth ≥ 1` with a list-bound operand whose name is also
///    ASSIGNED inside the enclosing loop (`acc = acc + [x]`): each pass
///    rebuilds and copies the whole list, O(n²);
/// 2. **append-to-accumulator** — an `append`/`extend` call at
///    `in_loop_depth ≥ 1` whose receiver root is list-bound and is NOT
///    rebuilt inside any enclosing loop: the container accumulates across
///    iterations (`out = []; for x in xs: out.append(x)`), so a list that is
///    itself scanned/extended elsewhere grows superlinearly.
///
///    The polarity is the OPPOSITE of a naive "receiver reassigned in the
///    loop" gate: rebuilding the receiver inside the loop (`for x: tmp = [];
///    tmp.append(x)`) is the benign `CTL-APPEND-LOCAL` control, and a reset in
///    an OUTER frame is `CTL-APPEND-OUTER-RESET` — both stay silent
///    (`counter.py:358-368`); an accumulator carried across iterations is the
///    real hit.
///
/// Evidence `receiver_root`/`target`, `binding_kind="list"`, `growth`,
/// `loop_depth`. Warn/MEDIUM (binding kind is inference).
pub(crate) fn python_alloc_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let lists = names_with_kind(func, BindingKind::List);
            if lists.is_empty() {
                continue;
            }
            accumulate_concat(func, &lists, &unit.file.path, &mut findings);
            accumulate_append(func, &lists, &unit.file.path, &mut findings);
        }
    }
    sort_findings(&mut findings);
    findings
}

fn accumulate_concat(
    func: &crate::scanner::FunctionUnit,
    lists: &BTreeSet<String>,
    path: &str,
    findings: &mut Vec<Finding>,
) {
    for binop in &func.binary_ops {
        if binop.op != "+" || binop.meta.in_loop_depth == 0 {
            continue;
        }
        let Some(name) = list_operand_name(&binop.left, &binop.right, lists) else {
            continue;
        };
        let Some(loop_node) = enclosing_loop(func, &binop.meta.span) else {
            continue;
        };
        if !assigned_within(func, name, &loop_node.span) {
            continue;
        }
        findings.push(Finding {
            threshold: None,
            rule_id: PY_ALLOC_003_RULE_ID.to_string(),
            title: "Superlinear list accumulation in loop".to_string(),
            message: format!(
                "'{name}' is grown with non-in-place '+' inside a loop at \
                 depth {} — CPython rebuilds and copies the whole list each \
                 pass (list_resize + memcpy), making the loop O(n²); using \
                 'append'/'extend' (or a list comprehension) is a candidate \
                 optimization",
                binop.meta.in_loop_depth
            ),
            severity: Severity::Warn,
            confidence: Confidence::Medium,
            resolver_tier: binop.meta.resolver_tier,
            span: binop.meta.span,
            path: path.to_string(),
            evidence: [
                ("target".to_string(), name.to_string()),
                ("binding_kind".to_string(), "list".to_string()),
                ("growth".to_string(), "concat".to_string()),
                (
                    "loop_depth".to_string(),
                    binop.meta.in_loop_depth.to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
    }
}

fn accumulate_append(
    func: &crate::scanner::FunctionUnit,
    lists: &BTreeSet<String>,
    path: &str,
    findings: &mut Vec<Finding>,
) {
    for call in &func.calls {
        if call.meta.in_loop_depth == 0
            || (call.callee_name != "append" && call.callee_name != "extend")
        {
            continue;
        }
        let Some(root) = call.receiver_root.as_deref() else {
            continue;
        };
        if !lists.contains(root) {
            continue;
        }
        if enclosing_loop(func, &call.meta.span).is_none() {
            continue;
        }
        // POLARITY (plan `python-backend` W3; canary `CTL-APPEND-LOCAL` /
        // `CTL-APPEND-OUTER-RESET`, counter.py:358-368): a receiver rebuilt
        // inside ANY enclosing loop is benign — the container never accumulates
        // across iterations. Fire ONLY for a receiver that is never reassigned
        // within an enclosing loop span.
        let rebuilt_in_a_loop = func
            .loops
            .iter()
            .filter(|l| super::super::span_contains(&l.span, &call.meta.span))
            .any(|l| assigned_within(func, root, &l.span));
        if rebuilt_in_a_loop {
            continue;
        }
        findings.push(Finding {
            threshold: None,
            rule_id: PY_ALLOC_003_RULE_ID.to_string(),
            title: "Superlinear list accumulation in loop".to_string(),
            message: format!(
                "list '{root}' is grown with '{}' inside the loop at depth {} \
                 without being rebuilt there — the container accumulates across \
                 iterations, so list_resize growth plus any scan over it makes \
                 the loop superlinear; accumulating into a local or using a \
                 comprehension is a candidate optimization",
                call.callee_name, call.meta.in_loop_depth
            ),
            severity: Severity::Warn,
            confidence: Confidence::Medium,
            resolver_tier: call.meta.resolver_tier,
            span: call.meta.span,
            path: path.to_string(),
            evidence: [
                ("receiver_root".to_string(), root.to_string()),
                ("binding_kind".to_string(), "list".to_string()),
                ("growth".to_string(), call.callee_name.clone()),
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

/// The list-bound identifier name participating in a `+` binary op, LEFT
/// side preferred.
fn list_operand_name<'a>(
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

/// PY-ALLOC-004 — Eager iterable re-materialisation inside loop (Python-native
/// addition), Info.
///
/// A `list()`/`dict()`/`set()`/`tuple()` call inside a loop whose first
/// argument is a non-literal iterable forces an eager O(n) allocation and copy
/// (`list(gen)` / `dict(pairs)`), defeating laziness. Argument-free
/// constructors and literal arguments belong to PY-ALLOC-001. Evidence
/// `callee`, `arg_kind`, `loop_depth`. Info/MEDIUM.
pub(crate) fn python_alloc_004(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !MATERIALISE_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(first) = call.args.first() else {
                    continue;
                };
                if first.kind == ShapeKind::Literal {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_ALLOC_004_RULE_ID.to_string(),
                    title: "Eager iterable materialisation inside loop".to_string(),
                    message: format!(
                        "'{}' called with a non-literal iterable every \
                         iteration at loop depth {} — CPython eagerly \
                         allocates and copies the whole sequence, defeating \
                         lazy iteration; keeping the lazy generator (or \
                         hoisting) is a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
                        (
                            "arg_kind".to_string(),
                            super::super::shape_kind_token(first.kind).to_string(),
                        ),
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
        assign, binop, call, func, loop_node, point, unit, with_ident_arg, with_literal_arg,
    };
    use crate::ir::{BindingKind, Span};

    #[test]
    fn alloc_001_fires_on_loop_invariant_literal_construction() {
        let span = point(3);
        let f = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            vec![with_literal_arg(call("open", None, None, span, 1), "f")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(
                1,
                &["i"],
                Span {
                    start_line: 2,
                    start_col: 1,
                    end_line: 8,
                    end_col: 1,
                },
            )],
            Vec::new(),
            false,
        );
        let out = python_alloc_001(&[unit(vec![f])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id, PY_ALLOC_001_RULE_ID);
    }

    #[test]
    fn alloc_001_silent_outside_loop_and_on_iterable_args() {
        let span = point(3);
        // depth 0 → silent.
        let outside = func(
            "f",
            Span::default(),
            vec![call("list", None, None, span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_alloc_001(&[unit(vec![outside])]).is_empty());
        // identifier arg → owned by ALLOC-004, silent here.
        let iterable = func(
            "f",
            Span::default(),
            vec![with_ident_arg(call("list", None, None, span, 1), "xs")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_alloc_001(&[unit(vec![iterable])]).is_empty());
    }

    #[test]
    fn alloc_003_fires_on_non_in_place_concat() {
        let span = point(5);
        let f = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            Vec::new(),
            vec![
                assign("acc", BindingKind::List, point(2), 0),
                assign("acc", BindingKind::List, point(6), 1),
            ],
            vec![binop("+", Some("acc"), None, span, 1)],
            Vec::new(),
            Vec::new(),
            vec![loop_node(
                1,
                &["i"],
                Span {
                    start_line: 3,
                    start_col: 1,
                    end_line: 8,
                    end_col: 1,
                },
            )],
            Vec::new(),
            false,
        );
        let out = python_alloc_003(&[unit(vec![f])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].severity, Severity::Warn);
        assert_eq!(out[0].confidence, Confidence::Medium);
    }

    #[test]
    fn alloc_003_silent_for_set_accumulation() {
        let span = point(5);
        let f = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            Vec::new(),
            vec![
                assign("seen", BindingKind::Set, point(2), 0),
                assign("seen", BindingKind::Set, point(6), 1),
            ],
            vec![binop("+", Some("seen"), None, span, 1)],
            Vec::new(),
            Vec::new(),
            vec![loop_node(
                1,
                &["i"],
                Span {
                    start_line: 3,
                    start_col: 1,
                    end_line: 8,
                    end_col: 1,
                },
            )],
            Vec::new(),
            false,
        );
        assert!(python_alloc_003(&[unit(vec![f])]).is_empty());
    }

    #[test]
    fn alloc_004_fires_on_iterable_materialisation_in_loop() {
        let span = point(4);
        let f = func(
            "f",
            Span::default(),
            vec![with_ident_arg(call("list", None, None, span, 1), "xs")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let out = python_alloc_004(&[unit(vec![f])]);
        assert_eq!(out.len(), 1);
        // literal arg → not materialisation.
        let lit = func(
            "f",
            Span::default(),
            vec![with_literal_arg(call("list", None, None, span, 1), "abc")],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_alloc_004(&[unit(vec![lit])]).is_empty());
    }

    /// ALLOC-003 append polarity: an accumulator built OUTSIDE the loop and
    /// appended to inside fires (`out=[]; for x in xs: out.append(x)`).
    #[test]
    fn alloc_003_append_fires_when_receiver_not_rebuilt_in_loop() {
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 8,
            end_col: 1,
        };
        let f = func(
            "f",
            Span::default(),
            vec![call("append", None, Some("out"), point(5), 1)],
            vec![assign("out", BindingKind::List, point(1), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        let out = python_alloc_003(&[unit(vec![f])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].rule_id, PY_ALLOC_003_RULE_ID);
    }

    /// ALLOC-003 append polarity: a receiver rebuilt INSIDE the loop is the
    /// benign `CTL-APPEND-LOCAL` control (`for x in xs: tmp=[]; tmp.append(x)`).
    #[test]
    fn alloc_003_append_silent_when_receiver_rebuilt_in_loop() {
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 8,
            end_col: 1,
        };
        let f = func(
            "f",
            Span::default(),
            vec![call("append", None, Some("tmp"), point(6), 1)],
            vec![assign("tmp", BindingKind::List, point(5), 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        assert!(python_alloc_003(&[unit(vec![f])]).is_empty());
    }
}
