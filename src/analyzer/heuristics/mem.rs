//! Memory-op-family heuristics (catalog-v1 §4 Tier 2):
//! PERF-C-MEM-001/002.

use std::collections::BTreeSet;

use crate::analyzer::finding::{MEM_001_RULE_ID, MEM_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CalleeKind, Shape, ShapeKind};
use crate::scanner::FileUnit;

use super::{ALLOC_FAMILY, shape_kind_token, span_contains};

/// Block copy/set family (MEM-002): repeated large block churn per
/// iteration.
const BLOCK_COPY_FAMILY: [&str; 3] = ["memcpy", "memmove", "memset"];

/// PERF-C-MEM-002 — Block copy/set family in loop (catalog-v1 §4 Tier 2).
///
/// Detects DIRECT `memcpy`/`memmove`/`memset` calls at `in_loop_depth ≥ 1`
/// whose LAST (size) argument is NOT a literal shape — repeated block
/// copy/set churn with a runtime byte count. Literal-size calls stay
/// silent (fixed small copies are idiomatic and compiler-friendly).
///
/// Severity: Info by default; Warn when the call sits in an INNERMOST loop
/// (catalog FP class: per-element small-struct copies are idiomatic and
/// cheap — the outer-loop Info band keeps them candidate-grade). Framing
/// rule: optimization candidate, never bug/unsafe.
pub(crate) fn mem_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !BLOCK_COPY_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(size_arg) = call.args.last() else {
                    continue;
                };
                if size_arg.kind == ShapeKind::Literal {
                    continue;
                }
                let innermost = call.meta.is_innermost;
                findings.push(Finding {
                    rule_id: MEM_002_RULE_ID.to_string(),
                    title: "Block copy/set family in loop".to_string(),
                    message: format!(
                        "'{}' moves a non-literal byte count every \
                         iteration at loop depth {} — hoisting the call, \
                         batching the copies, or tracking dirty regions \
                         is a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: if innermost {
                        Severity::Warn
                    } else {
                        Severity::Info
                    },
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
                        (
                            "size_arg_kind".to_string(),
                            shape_kind_token(size_arg.kind).to_string(),
                        ),
                        ("innermost".to_string(), innermost.to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    findings
}

/// PERF-C-MEM-001 — sizeof-misuse shapes in allocation args
/// (catalog-v1 §4 Tier 2).
///
/// Fires per DIRECT alloc-family call (any loop depth — wrong-size
/// allocations hurt wherever they run) whose size expression carries one
/// of the catalog misuse classes:
/// - `pointer_param_sizeof`: a sizeof operand that is a BARE identifier
///   resolving to a pointer-typed parameter (`malloc(n * sizeof(p))` with
///   `p` declared `T *p`) — sizes the POINTER, not the pointee;
/// - `multiply_sizeof`: a `*` BinaryOpEvent with BOTH operands Sizeof,
///   span-contained in the call (`sizeof(T) * sizeof(U)`);
/// - `sizeof_of_sizeof`: operand text starts with `sizeof`;
/// - `divide_by_sizeof`: a `/` BinaryOpEvent with a Sizeof operand,
///   span-contained in the call.
///
/// Cross-event proof follows the ALLOC-003 containment precedent: operand
/// binops are matched by span containment against the call span. One
/// finding per call; multiple classes join into sorted evidence.
pub(crate) fn mem_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !ALLOC_FAMILY.contains(&call.callee_name.as_str())
                {
                    continue;
                }
                let mut misuses = BTreeSet::new();
                // Direct sizeof arguments (`malloc(sizeof(p))`).
                for arg in &call.args {
                    classify_sizeof(arg, func, &mut misuses);
                }
                // Sizeof shapes riding inside multiplicative/divisive size
                // expressions (`n * sizeof(p)`, `sizeof(T) * sizeof(U)`,
                // `n / sizeof(T)`).
                //
                // Bound note: O(calls × binary_ops) per function, matching
                // the ALLOC-003 span-containment precedent — acceptable at
                // MVP corpus sizes; a span-sorted prefilter is the obvious
                // upgrade if ever needed. Re-classifying operands already
                // caught via direct args is harmless: the BTreeSet makes
                // class insertion idempotent.
                for binop in &func.binary_ops {
                    if !span_contains(&call.meta.span, &binop.meta.span) {
                        continue;
                    }
                    if binop.op == "*"
                        && binop.left.kind == ShapeKind::Sizeof
                        && binop.right.kind == ShapeKind::Sizeof
                    {
                        misuses.insert("multiply_sizeof");
                    }
                    if binop.op == "/"
                        && (binop.left.kind == ShapeKind::Sizeof
                            || binop.right.kind == ShapeKind::Sizeof)
                    {
                        misuses.insert("divide_by_sizeof");
                    }
                    if binop.op == "*" {
                        classify_sizeof(&binop.left, func, &mut misuses);
                        classify_sizeof(&binop.right, func, &mut misuses);
                    }
                }
                if misuses.is_empty() {
                    continue;
                }
                findings.push(Finding {
                    rule_id: MEM_001_RULE_ID.to_string(),
                    title: "sizeof misuse in allocation argument".to_string(),
                    message: format!(
                        "'{}' size argument carries a sizeof misuse shape \
                         ({}) — the allocated byte count likely does not \
                         match the element type; sizing by the pointee \
                         type is a candidate fix",
                        call.callee_name,
                        misuses.iter().copied().collect::<Vec<_>>().join("+")
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
                        (
                            "misuse".to_string(),
                            misuses.iter().copied().collect::<Vec<_>>().join("+"),
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
    findings
}

/// Classify one Sizeof shape against the enclosing function's parameter
/// types (catalog MEM-001 classes i and iii).
fn classify_sizeof(
    shape: &Shape,
    func: &crate::scanner::FunctionUnit,
    misuses: &mut BTreeSet<&'static str>,
) {
    if shape.kind != ShapeKind::Sizeof {
        return;
    }
    let Some(operand) = shape.sizeof_operand.as_deref() else {
        return;
    };
    let operand = operand.trim();
    if operand.starts_with("sizeof") {
        misuses.insert("sizeof_of_sizeof");
        return;
    }
    // Flagship class: a bare identifier naming a pointer-typed parameter —
    // `sizeof(p)` where `p` is `T *p` measures the pointer, not the pointee.
    if is_bare_identifier(operand)
        && func
            .record
            .param_types
            .iter()
            .any(|param| param_is_pointer_named(param, operand))
    {
        misuses.insert("pointer_param_sizeof");
    }
}

/// A bare C identifier: non-empty, alphanumeric/underscore, no leading digit.
fn is_bare_identifier(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !text.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// True when raw `param_types` text names the pointer-typed parameter
/// `ident` (`char *buf`, `node **pp`). Raw text INCLUDES the param name
/// (ir-v1.md §3.2), so the last whitespace token minus its leading stars
/// is the name candidate.
fn param_is_pointer_named(param: &str, ident: &str) -> bool {
    param.contains('*')
        && param
            .split_whitespace()
            .last()
            .is_some_and(|last| last.trim_start_matches('*') == ident)
}
