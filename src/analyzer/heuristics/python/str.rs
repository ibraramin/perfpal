//! Python string-family heuristics (plan `python-backend` W3): PY-STR-001.
//!
//! `STR-002` (`len` in loop) and `STR-003` (unbounded copy) are RETIRED for
//! Python: `len()` is O(1) on the object header and the runtime guarantees
//! memory safety.

use crate::analyzer::finding::PY_STR_001_RULE_ID;
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BindingKind, Shape, ShapeKind};
use crate::scanner::FileUnit;

use super::{assigned_within, enclosing_loop, names_with_kind, sort_findings};

/// PY-STR-001 — String accumulation in loop (transferred/reframed mirror of
/// PERF-C-STR-001), Warn.
///
/// A `+`/`+=` [`crate::ir::BinaryOpEvent`] at `in_loop_depth ≥ 1` where one
/// operand is an identifier bound to [`BindingKind::Str`] — the explicit
/// `s = s + piece` form (`op == "+"`) AND the named in-place form
/// `s += piece` (`op == "+="`, emitted by the Python backend from
/// `augmented_assignment`). CPython strings are immutable, so each pass
/// allocates a new string and copies BOTH operands, making the loop O(n²); the
/// `str.join`/list-accumulation idiom is the documented fix. Two gates are
/// load-bearing:
/// - the `kind` gate (only `str` bindings) excludes integer accumulators
///   (`CTL-INT-ACC`) and ambiguous/expression bindings (`CTL-AMBIG-ACC` /
///   `CTL-EXPR-ACC`);
/// - the **accumulator gate** — the str-bound name must be (re)assigned within
///   the enclosing loop span, i.e. it accumulates across iterations. This kills
///   the over-broad `msg = "p"; for x in xs: out = msg + x` false positive
///   (a loop-invariant string read, not an accumulation).
///
/// Evidence `identifier`, `binding_kind="str"`, `form`, `loop_depth`.
/// Warn/MEDIUM (binding kind is inference; the finding is honest about the
/// CPython refcount==1 amortisation that applies to in-place `+=`).
pub(crate) fn python_str_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let strings = names_with_kind(func, BindingKind::Str);
            if strings.is_empty() {
                continue;
            }
            for binop in &func.binary_ops {
                if binop.meta.in_loop_depth == 0 || (binop.op != "+" && binop.op != "+=") {
                    continue;
                }
                let Some(name) = string_operand_name(&binop.left, &binop.right, &strings) else {
                    continue;
                };
                let Some(loop_node) = enclosing_loop(func, &binop.meta.span) else {
                    continue;
                };
                // Accumulator gate: only an accumulation across iterations
                // (the name is rebound inside the loop) is the O(n²) shape.
                if !assigned_within(func, name, &loop_node.span) {
                    continue;
                }
                let form = if binop.op == "+=" {
                    "in_place"
                } else {
                    "concat"
                };
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_STR_001_RULE_ID.to_string(),
                    title: "String accumulation in loop".to_string(),
                    message: format!(
                        "string '{name}' is accumulated with '{}' inside a loop \
                         at depth {} — CPython strings are immutable, so each \
                         pass allocates a new string and copies both operands \
                         (O(n²)); accumulating pieces in a list and calling \
                         `str.join` once is a candidate optimization",
                        binop.op, binop.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: binop.meta.resolver_tier,
                    span: binop.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("identifier".to_string(), name.to_string()),
                        ("binding_kind".to_string(), "str".to_string()),
                        ("form".to_string(), form.to_string()),
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
    }
    sort_findings(&mut findings);
    findings
}

/// The `str`-bound identifier name participating in a `+`/`+=` binary op.
fn string_operand_name<'a>(
    left: &'a Shape,
    right: &'a Shape,
    strings: &std::collections::BTreeSet<String>,
) -> Option<&'a str> {
    let named = |shape: &'a Shape| -> Option<&'a str> {
        if shape.kind != ShapeKind::Identifier {
            return None;
        }
        shape.name.as_deref().filter(|n| strings.contains(*n))
    };
    named(left).or_else(|| named(right))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{
        assign, binop, func, loop_node, point, unit,
    };
    use crate::ir::Span;

    #[test]
    fn str_001_fires_on_str_concat_silent_for_int() {
        let span = point(5);
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 8,
            end_col: 1,
        };
        let concat = func(
            "f",
            Span::default(),
            Vec::new(),
            vec![
                assign("s", BindingKind::Str, point(2), 0),
                assign("s", BindingKind::Str, point(6), 1),
            ],
            vec![binop("+", Some("s"), None, span, 1)],
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["i"], loop_span)],
            Vec::new(),
            false,
        );
        let out = python_str_001(&[unit(vec![concat])]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule_id, PY_STR_001_RULE_ID);

        let int_acc = func(
            "f",
            Span::default(),
            Vec::new(),
            vec![assign("n", BindingKind::Unknown, point(2), 0)],
            vec![binop("+", Some("n"), None, span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_str_001(&[unit(vec![int_acc])]).is_empty());
    }

    /// The named in-place form `s += piece` (backend `augmented_assignment` →
    /// `BinaryOpEvent { op: "+=" }`) fires when `s` accumulates across the loop.
    #[test]
    fn str_001_fires_on_in_place_augmented_accumulation() {
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 8,
            end_col: 1,
        };
        let f = func(
            "f",
            Span::default(),
            Vec::new(),
            vec![
                assign("s", BindingKind::Str, point(2), 0),
                assign("s", BindingKind::Str, point(6), 1),
            ],
            vec![binop("+=", Some("s"), Some("x"), point(6), 1)],
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        let out = python_str_001(&[unit(vec![f])]);
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(
            out[0].evidence.get("form").map(String::as_str),
            Some("in_place")
        );
    }

    /// `msg = "p"; for x in xs: out = msg + x` is a loop-invariant read, not an
    /// accumulation — `msg` is never rebound inside the loop, so it stays silent.
    #[test]
    fn str_001_silent_when_str_not_rebuilt_in_loop() {
        let loop_span = Span {
            start_line: 3,
            start_col: 1,
            end_line: 8,
            end_col: 1,
        };
        let f = func(
            "f",
            Span::default(),
            Vec::new(),
            vec![assign("msg", BindingKind::Str, point(1), 0)],
            vec![binop("+", Some("msg"), Some("x"), point(5), 1)],
            Vec::new(),
            Vec::new(),
            vec![loop_node(1, &["x"], loop_span)],
            Vec::new(),
            false,
        );
        assert!(python_str_001(&[unit(vec![f])]).is_empty());
    }
}
