//! Loop-family heuristics (catalog-v1 §3 Tier 1 + §4 Tier 2):
//! PERF-C-LOOP-001..008.

use crate::analyzer::finding::{
    LOOP_001_RULE_ID, LOOP_002_RULE_ID, LOOP_003_RULE_ID, LOOP_004_RULE_ID, LOOP_005_RULE_ID,
    LOOP_006_RULE_ID, LOOP_007_RULE_ID, LOOP_008_RULE_ID,
};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{
    BinaryOpEvent, CalleeKind, ComparisonEvent, LoopNode, ResolverTier, ScopeClass, Shape,
    ShapeKind, Span, StatementRole,
};
use crate::scanner::FileUnit;

use super::{
    LOOP_001_MIN_DEPTH, LOOP_007_MIN_DEPTH, LOOP_008_MIN_CHAIN_DEPTH, shape_kind_token,
    span_contains,
};

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

/// PERF-C-LOOP-005 — Global variable written inside loop
/// (catalog-v1 §4 Tier 2).
///
/// Fires per [`AssignmentEvent`] whose target is classified
/// [`ScopeClass::Global`] (name-based file-scope classification, MEDIUM by
/// construction) at `in_loop_depth ≥ 1` — a potential loop-invariant store
/// or shared-state contention signal.
///
/// DEDUP: one finding per `(target identifier, enclosing-loop span)` pair,
/// anchored at the EARLIEST qualifying assignment span — a global written N
/// times inside the SAME loop body yields ONE finding; a second global or a
/// different enclosing loop yields its own. Documented FP class: accumulator
/// updates that legitimately belong in the loop stay flagged (Info band
/// keeps them candidate-grade). Framing rule: optimization candidate, never
/// bug/unsafe.
pub(crate) fn loop_005(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            // Group qualifying assignments by (target ident, innermost
            // enclosing-loop span), keeping the EARLIEST member span, its
            // depth, and its resolver tier. Insertion order preserves
            // document determinism.
            let mut groups: Vec<Loop005Group> = Vec::new();
            for assignment in &func.assignments {
                if assignment.scope_class != ScopeClass::Global
                    || assignment.meta.in_loop_depth == 0
                    || assignment.target.kind != ShapeKind::Identifier
                {
                    continue;
                }
                let Some(target) = assignment.target.name.clone() else {
                    continue;
                };
                let Some(enclosing) = innermost_enclosing_loop(&func.loops, &assignment.meta.span)
                else {
                    continue;
                };
                if let Some(group) = groups
                    .iter_mut()
                    .find(|g| g.target == target && g.loop_span == enclosing.span)
                {
                    if (
                        assignment.meta.span.start_line,
                        assignment.meta.span.start_col,
                    ) < (group.anchor.start_line, group.anchor.start_col)
                    {
                        group.anchor = assignment.meta.span;
                        group.loop_depth = assignment.meta.in_loop_depth;
                        group.resolver_tier = assignment.meta.resolver_tier;
                    }
                } else {
                    groups.push(Loop005Group {
                        target,
                        loop_span: enclosing.span,
                        anchor: assignment.meta.span,
                        loop_depth: assignment.meta.in_loop_depth,
                        resolver_tier: assignment.meta.resolver_tier,
                    });
                }
            }
            for group in groups {
                findings.push(Finding {
                    rule_id: LOOP_005_RULE_ID.to_string(),
                    title: "Global variable written inside loop".to_string(),
                    message: format!(
                        "global '{}' is stored every iteration at loop \
                         depth {} — hoisting the write out of the loop or \
                         caching it in a local is a candidate optimization",
                        group.target, group.loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: group.resolver_tier,
                    span: group.anchor,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("target".to_string(), group.target),
                        ("loop_depth".to_string(), group.loop_depth.to_string()),
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

/// PERF-C-LOOP-006 — Indirect call via function pointer in innermost loop
/// (catalog-v1 §4 Tier 2).
///
/// Fires per [`CallEvent`] with `callee_kind == Indirect` at
/// `in_loop_depth ≥ 1` AND `is_innermost == true` — indirect calls inhibit
/// inlining and hurt branch prediction worst where they run every iteration.
/// One finding per call event. Confidence LOW is mandatory (no type info
/// confirming the callee really is a function pointer); documented FP class:
/// dispatch-table patterns are idiomatic and often intentional. Framing
/// rule: optimization candidate, never bug/unsafe.
pub(crate) fn loop_006(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Indirect
                    || !call.meta.is_innermost
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                findings.push(Finding {
                    rule_id: LOOP_006_RULE_ID.to_string(),
                    title: "Indirect call via function pointer in innermost loop".to_string(),
                    message: format!(
                        "indirect call through '{}' runs in the innermost \
                         loop at depth {} — resolving the target once \
                         outside the loop or replacing the pointer \
                         dispatch with a direct call is a candidate \
                         optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Low,
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

/// PERF-C-LOOP-007 — Linear membership scan nested in loop
/// (catalog-v1 §4 Tier 2).
///
/// Fires per [`ComparisonEvent`] at `in_loop_depth ≥ LOOP_007_MIN_DEPTH`
/// where ONE side is a Subscript shape whose
/// `index_ident` matches ANY induction var of a loop span-containing the
/// comparison AND the OTHER side is NOT a literal shape — the hand-rolled
/// O(n²) lookup shape (`arr[j] == key` inside a doubly nested loop).
///
/// The non-literal guard excludes sentinel/bounds idioms (`arr[j] == '\0'`)
/// BY DESIGN. Candidate-grade only: comparing against another array element
/// is legitimate, but comparing against a known-sentinel VARIABLE is not
/// distinguishable syntactically — that residual FP class stands (catalog
/// note). One finding per comparison span. Confidence LOW mandatory.
pub(crate) fn loop_007(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for cmp in &func.comparisons {
                let Some((sub, index_ident)) = membership_sides(cmp, &func.loops) else {
                    continue;
                };
                let mut evidence = Vec::new();
                if let Some(root) = sub.name.clone() {
                    evidence.push(("root".to_string(), root));
                }
                evidence.push(("index_ident".to_string(), index_ident.to_string()));
                evidence.push(("loop_depth".to_string(), cmp.meta.in_loop_depth.to_string()));
                findings.push(Finding {
                    rule_id: LOOP_007_RULE_ID.to_string(),
                    title: "Linear membership scan nested in loop".to_string(),
                    message: format!(
                        "element lookup compared against a non-literal at \
                         loop depth {} re-scans linearly on every outer \
                         iteration — precomputing a hash set / lookup \
                         table or sorting plus bisecting is a candidate \
                         optimization",
                        cmp.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Low,
                    resolver_tier: cmp.meta.resolver_tier,
                    span: cmp.meta.span,
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

/// LOOP-007 gate: at [`LOOP_007_MIN_DEPTH`] or deeper, one side a Subscript
/// indexed by an induction var of an enclosing loop, the other side not a
/// literal. Returns `(subscript_side, index_ident)` with the LEFT side
/// preferred when both qualify — the index identifier is part of the gate
/// proof, so callers never see a missing index.
fn membership_sides<'a>(
    cmp: &'a ComparisonEvent,
    loops: &'a [LoopNode],
) -> Option<(&'a Shape, &'a str)> {
    if cmp.meta.in_loop_depth < LOOP_007_MIN_DEPTH {
        return None;
    }
    let induced = |sub: &'a Shape| -> Option<&'a str> {
        if sub.kind != ShapeKind::Subscript {
            return None;
        }
        let idx = sub.index_ident.as_deref()?;
        loops
            .iter()
            .filter(|l| span_contains(&l.span, &cmp.meta.span))
            .any(|l| l.induction_vars.iter().any(|v| v == idx))
            .then_some(idx)
    };
    match (induced(&cmp.left), induced(&cmp.right)) {
        (Some(idx), _) if cmp.right.kind != ShapeKind::Literal => Some((&cmp.left, idx)),
        (_, Some(idx)) if cmp.left.kind != ShapeKind::Literal => Some((&cmp.right, idx)),
        _ => None,
    }
}

/// PERF-C-LOOP-008 — Pointer-chasing chain in innermost loop
/// (catalog-v1 §4 Tier 2).
///
/// Fires per [`AccessChainEvent`] with chain depth ≥
/// [`LOOP_008_MIN_CHAIN_DEPTH`] at `in_loop_depth ≥ 1` AND
/// `is_innermost == true` — repeated pointer dereferences per iteration are
/// cache-miss candidates. One finding per chain span. Documented FP class
/// (catalog-mandated): dispatch through ops/vtable-style tables
/// (`ctx->ops->fn`) is idiomatic C design, not a defect — severity stays
/// Info with MEDIUM confidence and this class documented. Framing rule:
/// optimization candidate, never bug/unsafe.
pub(crate) fn loop_008(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for chain in &func.access_chains {
                if chain.elements.len() < LOOP_008_MIN_CHAIN_DEPTH
                    || !chain.meta.is_innermost
                    || chain.meta.in_loop_depth == 0
                {
                    continue;
                }
                findings.push(Finding {
                    rule_id: LOOP_008_RULE_ID.to_string(),
                    title: "Pointer-chasing chain in innermost loop".to_string(),
                    message: format!(
                        "'{}' dereference chain of depth {} executes in \
                         the innermost loop — caching intermediate \
                         pointers or flattening the structure is a \
                         candidate optimization",
                        chain.root,
                        chain.elements.len()
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: chain.meta.resolver_tier,
                    span: chain.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("root".to_string(), chain.root.clone()),
                        ("chain_depth".to_string(), chain.elements.len().to_string()),
                        (
                            "loop_depth".to_string(),
                            chain.meta.in_loop_depth.to_string(),
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

/// LOOP-005 dedup group: one finding per `(target identifier, enclosing-loop
/// span)` pair, anchored at the earliest qualifying assignment span.
struct Loop005Group {
    target: String,
    loop_span: Span,
    anchor: Span,
    loop_depth: u32,
    resolver_tier: ResolverTier,
}

/// Innermost (deepest) loop whose span contains `span`; ties on depth break
/// to the earlier start position for determinism.
fn innermost_enclosing_loop<'a>(loops: &'a [LoopNode], span: &Span) -> Option<&'a LoopNode> {
    loops
        .iter()
        .filter(|l| span_contains(&l.span, span))
        .min_by_key(|l| {
            (
                std::cmp::Reverse(l.depth),
                l.span.start_line,
                l.span.start_col,
            )
        })
}
