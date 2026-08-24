//! Loop-family heuristics (catalog-v1 §3 Tier 1):
//! PERF-C-LOOP-001/002/003/004.

use crate::analyzer::finding::{
    LOOP_001_RULE_ID, LOOP_002_RULE_ID, LOOP_003_RULE_ID, LOOP_004_RULE_ID,
};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BinaryOpEvent, CalleeKind, LoopNode, ShapeKind, StatementRole};
use crate::scanner::FileUnit;

use super::{LOOP_001_MIN_DEPTH, shape_kind_token, span_contains};

/// Character-wise stdio family (LOOP-003).
const CHAR_IO_FAMILY: [&str; 5] = ["fgetc", "getc", "getchar", "fputc", "putc"];

/// Expensive setup / I-O family (LOOP-002, catalog-v1 §3 detail block):
/// printf-family formatting churn, file/syscall setup-teardown per
/// iteration, environment/process spawns, and regex compilation.
const EXPENSIVE_CALL_FAMILY: [&str; 18] = [
    // printf-family
    "printf",
    "fprintf",
    "sprintf",
    "snprintf",
    "vprintf",
    "vfprintf",
    "vsprintf",
    "vsnprintf",
    // file / syscall
    "fopen",
    "freopen",
    "fclose",
    "open",
    "close",
    "read",
    "write",
    // env / process
    "getenv",
    "system",
    // regex
    "regcomp",
];

/// PERF-C-LOOP-001 — Deeply nested loops (catalog-v1 §3 Tier 1).
///
/// Fires on every LoopNode whose depth is EXACTLY [`LOOP_001_MIN_DEPTH`] —
/// one finding per maximal nest entry point (a depth-5 chain yields one
/// finding at its depth-3 ancestor, not three). `max_depth` evidence
/// reports how deep the chain under this entry point actually reaches.
pub(crate) fn loop_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for loop_node in &func.loops {
                if loop_node.depth != LOOP_001_MIN_DEPTH {
                    continue;
                }
                // Max depth among this loop's inclusive descendants; the
                // loop contains itself, so the floor is its own depth.
                let max_depth = func
                    .loops
                    .iter()
                    .filter(|l| span_contains(&loop_node.span, &l.span))
                    .map(|l: &LoopNode| l.depth)
                    .max()
                    .unwrap_or(LOOP_001_MIN_DEPTH)
                    .max(LOOP_001_MIN_DEPTH);
                findings.push(Finding {
                    rule_id: LOOP_001_RULE_ID.to_string(),
                    title: "Deeply nested loops".to_string(),
                    message: format!(
                        "loops nest {} deep here (chain reaches depth \
                         {max_depth}) — flattening the nesting or adding \
                         early exits is a candidate optimization",
                        LOOP_001_MIN_DEPTH
                    ),
                    severity: Severity::Warn,
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
    findings
}

/// PERF-C-LOOP-003 — Character-wise stdio as primary loop-body statement
/// (catalog-v1 §3 Tier 1). Byte-at-a-time stdio calls that ARE the loop
/// body's primary statement signal a per-character processing pattern;
/// buffered block reads are the optimization candidate.
pub(crate) fn loop_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !CHAR_IO_FAMILY.contains(&call.callee_name.as_str())
                    || call.statement_role != StatementRole::Primary
                {
                    continue;
                }
                findings.push(Finding {
                    rule_id: LOOP_003_RULE_ID.to_string(),
                    title: "Character-wise stdio in loop".to_string(),
                    message: format!(
                        "character-wise I/O via '{}' as the loop body's \
                         primary statement — buffered block reads or \
                         writes are a candidate optimization",
                        call.callee_name
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
    findings
}

/// PERF-C-LOOP-002 — Expensive setup / I-O call inside loop
/// (catalog-v1 §3 Tier 1).
///
/// Fires per DIRECT call in [`EXPENSIVE_CALL_FAMILY`] at
/// `in_loop_depth ≥ 1` — one finding per call site, anchored at the call
/// span. No `statement_role` gate (unlike [`loop_003`]): any in-loop
/// occurrence of these calls is setup/teardown churn worth surfacing.
///
/// Severity: Hot by default; lowered to Warn when the call sits inside a
/// conditional branch within the loop (`conditional_guarded`, catalog FP
/// note — intentionally interleaved logging is a weaker signal).
pub(crate) fn loop_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !EXPENSIVE_CALL_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let guarded = call.conditional_guarded;
                findings.push(Finding {
                    rule_id: LOOP_002_RULE_ID.to_string(),
                    title: "Expensive setup or I/O call inside loop".to_string(),
                    message: format!(
                        "'{}' runs every iteration at loop depth {} — \
                         hoisting the call out of the loop or batching \
                         its work is a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: if guarded {
                        Severity::Warn
                    } else {
                        Severity::Hot
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
                        ("conditional_guarded".to_string(), guarded.to_string()),
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

/// PERF-C-LOOP-004 — Variable-divisor integer div/mod in innermost loop
/// (catalog-v1 §3 Tier 1).
///
/// Fires per `/` or `%` BinaryOpEvent with `is_innermost = true` whose
/// divisor (the right operand) is NOT a literal shape. Literal divisors
/// are skipped BY DESIGN (catalog detail block): compilers strength-reduce
/// constant divisors via magic-number multiply-shift (powers of two via
/// shift), so only variable divisors pay real division per iteration.
pub(crate) fn loop_004(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for binop in &func.binary_ops {
                if !is_variable_division(binop) {
                    continue;
                }
                let divisor_name = binop.right.name.clone();
                let mut evidence = vec![
                    ("op".to_string(), binop.op.clone()),
                    (
                        "divisor_kind".to_string(),
                        shape_kind_token(binop.right.kind).to_string(),
                    ),
                ];
                if let Some(name) = divisor_name {
                    evidence.push(("divisor".to_string(), name));
                }
                findings.push(Finding {
                    rule_id: LOOP_004_RULE_ID.to_string(),
                    title: "Variable-divisor division in innermost loop".to_string(),
                    message: format!(
                        "'{}' by a non-constant divisor executes in the \
                         innermost loop — real integer division runs per \
                         iteration; hoisting loop-invariant divisors or \
                         precomputing reciprocals is a candidate \
                         optimization",
                        binop.op
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: binop.meta.resolver_tier,
                    span: binop.meta.span,
                    path: unit.file.path.clone(),
                    evidence: evidence.into_iter().collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    findings
}

/// LOOP-004 gate: `/` or `%`, innermost only, non-literal divisor.
fn is_variable_division(binop: &BinaryOpEvent) -> bool {
    (binop.op == "/" || binop.op == "%")
        && binop.meta.is_innermost
        && binop.right.kind != ShapeKind::Literal
}
