//! Rust string-family heuristics (plan `rust-backend` R3 workstream C2):
//! PERF-RS-STR-002/003 — the API-name mirror of the C string family
//! (`heuristics::str`) onto Rust idioms. PERF-RS-STR-001 was RETIRED by the
//! M1 Precision Gate (plan `evidence-beta`; `Vec::push` collision noise).
//!
//! Rust string methods carry the receiver in the callee `field_expression`
//! (`s.push_str` ⇒ `callee_path`, receiver `s`), so the receiver gate reuses
//! [`super::method_receiver`] rather than the C rule's first-argument
//! identifier.
//!
//! Purity (modules.md §4.1): pure IR views, no I/O/clock/globals. Messages use
//! the mandatory "candidate optimization" framing.

use crate::analyzer::finding::{RS_STR_002_RULE_ID, RS_STR_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ShapeKind, Span};
use crate::scanner::{FileUnit, FunctionUnit};

use super::super::span_contains;
use super::{callee_token, method_receiver};

/// Length-query method family (STR-002): recomputed per iteration.
const LENGTH_METHODS: [&str; 3] = ["len", "is_empty", "capacity"];

/// Unsafe unbounded memory-op family (STR-003), fires anywhere: these are the
/// real "unbounded operation" surfaces in Rust (safe Rust bounds most
/// operations, so the C STR-003 analogue is the `unsafe` raw-memory family).
const UNBOUNDED_OP_METHODS: [&str; 5] = [
    "copy_nonoverlapping",
    "copy",
    "from_raw_parts",
    "from_utf8_unchecked",
    "set_len",
];

/// Qualified spellings of the STR-003 family (matched as `path == entry` or a
/// `::entry` suffix, e.g. `std::slice::from_raw_parts`).
const UNBOUNDED_OP_QUALIFIED: [&str; 2] = ["slice::from_raw_parts", "str::from_utf8_unchecked"];

/// PERF-RS-STR-002 — Length query recomputed in loop
/// (mirrors PERF-C-STR-002).
///
/// Intent: recomputing a container/string length per iteration is wasted work
/// when the receiver is not mutated inside the loop. Detection: a DIRECT
/// `len`/`is_empty`/`capacity` method call at `in_loop_depth ≥ 1` whose
/// receiver is a simple identifier or field chain, SUPPRESSED when an
/// assignment to that same identifier lies inside the innermost enclosing
/// loop (the C rule's
/// reassignment suppression: max-depth span-containing loop, earliest-span
/// tie-break; function-span fallback when no enclosing loop is modeled).
/// Warn/HIGH. Evidence `identifier`, `loop_depth`.
///
/// Accepted FP classes: `Vec::len` is O(1) and may be deliberately queried
/// per iteration, and a receiver mutated through a method (not an assignment)
/// is not detectable — the candidate stays candidate-grade.
pub(crate) fn rust_str_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || !LENGTH_METHODS.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(ident) = method_receiver(call) else {
                    continue;
                };
                if reassigned_in_innermost_loop(func, call, ident) {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_STR_002_RULE_ID.to_string(),
                    title: "Length query recomputed in loop (Rust)".to_string(),
                    message: format!(
                        "'{callee}' recomputes the length of '{ident}' at loop \
                         depth {} without an assignment to it inside the loop; \
                         hoisting the length before the loop is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("identifier".to_string(), ident.to_string()),
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
    findings
}

/// PERF-RS-STR-003 — Unbounded/unsafe memory operation
/// (REFRAMED mirror of PERF-C-STR-003).
///
/// REFRAMING (mandatory, documented): Rust is memory-safe by default, so the
/// C rule's "unbounded string op" (`strcpy`/`sprintf`/`gets`) has no direct
/// analogue. The real unbounded operations in Rust live behind `unsafe`:
/// `copy_nonoverlapping`, `copy`, `from_raw_parts`, `from_utf8_unchecked`,
/// `set_len` — length unchecked by the compiler and easy to get wrong.
/// Detection: any DIRECT call (method or qualified, matched on `callee_name`
/// or a `::`-suffix of `callee_path`) to that family, at ANY loop depth
/// (mirroring the C rule's anywhere-firing behavior). Warn/HIGH; evidence
/// `callee`.
///
/// Accepted FP classes / honest limitations: a correctly size-checked
/// `unsafe` block still fires (the rule surfaces the unchecked-op surface,
/// not a proven defect); a user-defined method that happens to be named
/// `copy` also fires (documented name-collision FP class).
pub(crate) fn rust_str_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct || call.is_macro {
                    continue;
                }
                let by_name = UNBOUNDED_OP_METHODS.contains(&call.callee_name.as_str());
                let by_path = UNBOUNDED_OP_QUALIFIED
                    .iter()
                    .any(|entry| super::qualified_match(call.callee_path.as_deref(), entry));
                if !by_name && !by_path {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_STR_003_RULE_ID.to_string(),
                    title: "Unbounded unsafe memory operation (Rust)".to_string(),
                    message: format!(
                        "'{callee}' performs an unchecked (unbounded) memory \
                         operation — prefer a safe/bounded variant or add an \
                         explicit length check; that is a candidate \
                         optimization",
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [("callee".to_string(), callee)].into_iter().collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    findings
}

/// STR-002 reassignment suppression (mirrors the C `strlen` check): find the
/// innermost enclosing loop (max depth among loops whose span contains the
/// call span, earliest-span tie-break) and suppress iff an assignment to
/// `ident` lands inside that loop. No enclosing loop (defensive) ⇒ fall back
/// to any assignment to `ident` within the same function span.
fn reassigned_in_innermost_loop(func: &FunctionUnit, call: &CallEvent, ident: &str) -> bool {
    let enclosing = func
        .loops
        .iter()
        .filter(|l| span_contains(&l.span, &call.meta.span))
        .max_by_key(|l| (l.depth, std::cmp::Reverse(l.span)));
    let target_assignments = |filter: &dyn Fn(&Span) -> bool| {
        func.assignments.iter().any(|a| {
            a.target.kind == ShapeKind::Identifier
                && a.target.name.as_deref() == Some(ident)
                && filter(&a.meta.span)
        })
    };
    match enclosing {
        Some(loop_node) => target_assignments(&|s| span_contains(&loop_node.span, s)),
        None => target_assignments(&|s| span_contains(&func.record.span, s)),
    }
}
