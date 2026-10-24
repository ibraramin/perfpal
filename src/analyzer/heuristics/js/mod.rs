//! JavaScript/TypeScript (`PERF-JS-*`) heuristic families (plan `js-ts`
//! B2-W2), the frozen 11-rule disposition table in
//! docs/architecture/js-ts.md §8.
//!
//! JS and TS/TSX share one family because the runtime mechanism (V8 / Node)
//! is identical. Every rule is a pure IR view with no I/O, clock, global state
//! or sibling-order dependence (modules.md §4.1) and is only EVER invoked with
//! `language == "js" | "ts" | "tsx"` units (registry language gating,
//! `analyzer::analyze`). Findings inherit `resolver_tier` from their source IR
//! event; the JS backend is syntactic-only, so no rule may ever claim
//! [`crate::ir::ResolverTier::Semantic`].
//!
//! Every message cites the V8/Node mechanism, the concrete fix, and the
//! accepted false-positive class; each rule is silent when its mandatory gate
//! fails (loop-invariance for ALLOC-001, accumulator shape for ALLOC-003, tiny
//! bounded loops for STR-001/LOOP-007, the `async` gate for ASYNC-001/002, and
//! the visible-base-case predicate for CALL-001). `PERF-JS-LOOP-003` is CUT for
//! v1 and deliberately has no function here; no `PERF-TS-*` rule exists.
//!
//! Confidence ceiling: every rule is gated on syntactic inference
//! (`receiver_root`, loop spans, binding-free shape checks) and therefore caps
//! at [`crate::analyzer::Confidence::Medium`]. Traversals are sorted for
//! byte-determinism; no set iteration reaches output.

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::finding::{
    JS_ALLOC_001_RULE_ID, JS_ALLOC_003_RULE_ID, JS_ARCH_001_RULE_ID, JS_ASYNC_001_RULE_ID,
    JS_ASYNC_002_RULE_ID, JS_CALL_001_RULE_ID, JS_LOOP_001_RULE_ID, JS_LOOP_002_RULE_ID,
    JS_LOOP_007_RULE_ID, JS_STR_001_RULE_ID, JS_TYPE_001_RULE_ID,
};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::graph::break_cycles;
use crate::ir::{AllocKind, CalleeKind, LoopNode, ResolverTier, Shape, ShapeKind, Span};
use crate::scanner::{FileUnit, FunctionUnit};

/// Known synchronous Node APIs: one syscall/serialization per call, blocking
/// the single event-loop thread. Shared by LOOP-002 (in a loop) and ASYNC-002
/// (inside an `async` function).
const SYNC_NODE_APIS: [&str; 21] = [
    "readFileSync",
    "writeFileSync",
    "appendFileSync",
    "copyFileSync",
    "mkdirSync",
    "readdirSync",
    "statSync",
    "lstatSync",
    "existsSync",
    "unlinkSync",
    "renameSync",
    "rmSync",
    "cpSync",
    "execSync",
    "spawnSync",
    "execFileSync",
    "openSync",
    "closeSync",
    "readSync",
    "writeSync",
    "pbkdf2Sync",
];
// `scryptSync` and `randomFillSync` are the remaining two sync crypto calls
// (kept as a separate slice only to stay within a readable literal size).
const SYNC_CRYPTO_APIS: [&str; 2] = ["scryptSync", "randomFillSync"];

/// Linear membership/index scans whose per-iteration cost is O(n).
const MEMBERSHIP_SCANS: [&str; 7] = [
    "indexOf",
    "includes",
    "find",
    "findIndex",
    "some",
    "every",
    "filter",
];

/// STR-001 tiny-bounded-loop floor: a canonical loop with a known trip count
/// below this is the accepted false-positive class (the flattening cost is
/// negligible for a handful of parts).
const STR_001_MIN_TRIP: u64 = 8;

/// LOOP-007 small-collection floor: membership scans over fewer than this
/// many iterations do not build the O(n²) product worth surfacing.
const LOOP_007_MIN_TRIP: u64 = 16;

/// Stable per-rule finding order: `(start_line, start_col, path)`. Applied at
/// the end of every JS rule so direct unit-test calls are deterministic (the
/// analyzer re-sorts globally afterwards).
pub(super) fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        (a.span.start_line, a.span.start_col, a.path.as_str()).cmp(&(
            b.span.start_line,
            b.span.start_col,
            b.path.as_str(),
        ))
    });
}

/// Innermost loop (deepest span-containing, earliest start on ties) enclosing
/// `span`; the JS twin of the Python/Rust family helper.
pub(super) fn enclosing_loop<'a>(func: &'a FunctionUnit, span: &Span) -> Option<&'a LoopNode> {
    func.loops
        .iter()
        .filter(|l| super::span_contains(&l.span, span))
        .max_by_key(|l| {
            (
                l.depth,
                std::cmp::Reverse((l.span.start_line, l.span.start_col)),
            )
        })
}

/// Whether an assignment inside `span` targets the identifier `name`.
fn assigned_within(func: &FunctionUnit, name: &str, span: &Span) -> bool {
    func.assignments.iter().any(|a| {
        a.target.kind == ShapeKind::Identifier
            && a.target.name.as_deref() == Some(name)
            && super::span_contains(span, &a.meta.span)
    })
}

/// Token for an [`AllocKind`] used as deterministic evidence text.
fn alloc_kind_token(kind: AllocKind) -> &'static str {
    match kind {
        AllocKind::New => "new",
        AllocKind::Object => "object",
        AllocKind::Array => "array",
    }
}

/// JS-ALLOC-001 — Loop-invariant object/array literal in a loop, Info.
///
/// An `object`/`array` [`crate::ir::AllocEvent`] at `in_loop_depth >= 1` is
/// young-generation allocation churn: V8 allocates the literal per iteration
/// and escape analysis rarely elides it. The reframed rule ships ONLY behind a
/// mandatory loop-invariance/escape gate:
/// - skip when any `referenced_names` entry is an induction variable of the
///   nearest enclosing loop (the literal depends on the loop variable);
/// - skip when an [`crate::ir::AssignmentEvent`] inside that same loop targets
///   one of those names (the literal's inputs are mutated per iteration, so it
///   is not loop-invariant);
/// - skip when no enclosing loop node contains the span (honest unknown).
///
/// Evidence `kind`, `referenced_names`, `loop_depth`. Info/MEDIUM.
pub(crate) fn js_alloc_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for alloc in &func.allocations {
                if !matches!(alloc.kind, AllocKind::Object | AllocKind::Array)
                    || alloc.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(loop_node) = enclosing_loop(func, &alloc.meta.span) else {
                    continue;
                };
                let names: BTreeSet<&str> =
                    alloc.referenced_names.iter().map(String::as_str).collect();
                // Loop-variant by induction variable.
                if names
                    .iter()
                    .any(|name| loop_node.induction_vars.iter().any(|v| v == name))
                {
                    continue;
                }
                // Loop-variant by mutation of a referenced name inside the loop.
                if names
                    .iter()
                    .any(|name| assigned_within(func, name, &loop_node.span))
                {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_ALLOC_001_RULE_ID.to_string(),
                    title: "Loop-invariant object/array literal inside loop".to_string(),
                    message: format!(
                        "an {} literal is constructed on every iteration at loop \
                         depth {} — V8 allocates it in the young generation and \
                         escape analysis rarely elides a per-iteration literal; \
                         hoisting the loop-invariant construction (or reusing one \
                         object) is a candidate optimization. Accepted FP class: a \
                         literal created once for a different reason but mutated \
                         per iteration",
                        alloc_kind_token(alloc.kind),
                        alloc.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: alloc.meta.resolver_tier,
                    span: alloc.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("kind".to_string(), alloc_kind_token(alloc.kind).to_string()),
                        (
                            "referenced_names".to_string(),
                            alloc.referenced_names.join(","),
                        ),
                        (
                            "loop_depth".to_string(),
                            alloc.meta.in_loop_depth.to_string(),
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

/// JS-ALLOC-003 — `concat`/`slice`/spread accumulation in a loop, Info.
///
/// Two accumulator shapes copy n elements per iteration, so accumulating
/// across iterations is O(n²):
///
/// - a `concat`/`slice` [`crate::ir::CallEvent`] whose non-empty
///   `receiver_root` an [`crate::ir::AssignmentEvent`] inside the same loop
///   reassigns (`acc = acc.concat(x)`);
/// - an array [`crate::ir::AllocEvent`] with a spread child
///   (`acc = [...acc, x]`, `has_spread`) whose `referenced_names` include a
///   name the nearest enclosing loop reassigns (the accumulator root).
///
/// A snapshot call/literal whose root is never reassigned in the loop stays
/// silent (the deliberate immutable-snapshot accepted FP class). Evidence
/// `receiver_root`, `callee` OR `kind="spread-accumulator"`, `loop_depth`.
/// Info/MEDIUM.
pub(crate) fn js_alloc_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0
                    || (call.callee_name != "concat" && call.callee_name != "slice")
                {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref().filter(|r| !r.is_empty()) else {
                    continue;
                };
                let Some(loop_node) = enclosing_loop(func, &call.meta.span) else {
                    continue;
                };
                // Accumulator pattern only: the same root is reassigned in
                // the loop (`acc = acc.concat(x)`).
                if !assigned_within(func, root, &loop_node.span) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_ALLOC_003_RULE_ID.to_string(),
                    title: "Superlinear `concat`/`slice` accumulation in loop".to_string(),
                    message: format!(
                        "'{root} = {root}.{}' copies n elements per iteration at \
                         loop depth {} — V8 allocates and copies a fresh array \
                         each pass, making the loop O(n²); pushing into one array \
                         (or a single concat after the loop) is a candidate \
                         optimization. Accepted FP class: a deliberate immutable \
                         snapshot per iteration",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("receiver_root".to_string(), root.to_string()),
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
            // Spread accumulation: `acc = [...acc, x]` in a loop. The literal
            // is an array `AllocEvent` carrying `has_spread`; the accumulator
            // root is the one referenced name the same loop reassigns.
            for alloc in &func.allocations {
                if alloc.kind != AllocKind::Array
                    || alloc.meta.in_loop_depth == 0
                    || !alloc.has_spread
                {
                    continue;
                }
                let Some(loop_node) = enclosing_loop(func, &alloc.meta.span) else {
                    continue;
                };
                let Some(root) = alloc
                    .referenced_names
                    .iter()
                    .find(|name| assigned_within(func, name, &loop_node.span))
                else {
                    continue;
                };
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_ALLOC_003_RULE_ID.to_string(),
                    title: "Superlinear spread accumulation in loop".to_string(),
                    message: format!(
                        "'{root} = [...{root}, ...]' copies n elements per \
                         iteration at loop depth {} — the spread materialises a \
                         fresh array each pass, making the loop O(n²); pushing \
                         into one array (`{root}.push(x)`) or spreading once \
                         after the loop is a candidate optimization. Accepted FP \
                         class: a deliberate immutable snapshot per iteration",
                        alloc.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: alloc.meta.resolver_tier,
                    span: alloc.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("kind".to_string(), "spread-accumulator".to_string()),
                        ("receiver_root".to_string(), root.clone()),
                        (
                            "loop_depth".to_string(),
                            alloc.meta.in_loop_depth.to_string(),
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

/// Whether `shape` is a string/template literal — i.e. a [`ShapeKind::Literal`]
/// whose raw text starts with a quote or a backtick. Numeric/boolean/regex
/// literals fail the predicate, which is the B2-W0 canary's string gate.
fn is_string_literal_shape(shape: &Shape) -> bool {
    shape.kind == ShapeKind::Literal
        && shape.value.as_deref().is_some_and(|value| {
            value.starts_with('"') || value.starts_with('\'') || value.starts_with('`')
        })
}

/// The single STR-001 finding shape shared by the `+=` and `x = x + …` arms.
fn string_accumulation_finding(
    unit: &FileUnit,
    target: &str,
    span: Span,
    loop_depth: u32,
    resolver_tier: ResolverTier,
) -> Finding {
    Finding {
        threshold: None,
        rule_id: JS_STR_001_RULE_ID.to_string(),
        title: "String accumulation with `+=` inside loop".to_string(),
        message: format!(
            "'{target}' accumulates a string with '+=' or \
             '{target} = {target} + ...' inside a loop at depth {loop_depth} — \
             this builds a cons-string chain; V8 ropes mitigate short chains, \
             but flattening the rope at first use is linear and repeated \
             concatenation is quadratic; collecting parts into an array and \
             joining once (`parts.join(\"\")`) is a candidate optimization. \
             Accepted FP class: a tiny bounded loop (< 8 iterations)"
        ),
        severity: Severity::Warn,
        confidence: Confidence::Medium,
        resolver_tier,
        span,
        path: unit.file.path.clone(),
        evidence: [
            ("target".to_string(), target.to_string()),
            ("loop_depth".to_string(), loop_depth.to_string()),
        ]
        .into_iter()
        .collect(),
        macro_suspect: false,
        error_node_overlap: false,
    }
}

/// JS-STR-001 — string accumulation in a loop, Warn.
///
/// Two syntactic shapes are admitted, both gated on a STRING/template literal
/// RHS (the B2-W0 canary predicate — numeric `+=` is the silent control):
///
/// 1. a [`crate::ir::BinaryOpEvent`] with `op == "+="`, an identifier LEFT
///    operand, and a string/template literal RIGHT operand;
/// 2. an [`crate::ir::AssignmentEvent`] whose identifier target span contains
///    a [`crate::ir::BinaryOpEvent`] with `op == "+"`, the same identifier on
///    the left, a string/template literal on the right, and the SAME END
///    position as the assignment — i.e. the binary expression is the whole
///    right-hand side (`x = x + "…"`, not `x = f(x + "…")`).
///
/// Both build a cons-string chain; V8 ropes mitigate the per-pass cost, but
/// flattening at first use is linear and repeated concatenation is quadratic.
/// The tiny-bounded-loop accepted FP class is excluded: a canonical loop with
/// `trip_count < 8` stays silent. The two arms cannot double-fire because the
/// augmented-assignment arm admits only `op == "+="` and the assignment arm
/// only `op == "+"`. Evidence `target`, `loop_depth`. Warn/MEDIUM.
pub(crate) fn js_str_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for binop in &func.binary_ops {
                if binop.op != "+="
                    || binop.meta.in_loop_depth == 0
                    || binop.left.kind != ShapeKind::Identifier
                    || !is_string_literal_shape(&binop.right)
                {
                    continue;
                }
                let Some(loop_node) = enclosing_loop(func, &binop.meta.span) else {
                    continue;
                };
                if loop_node.trip_count.is_some_and(|n| n < STR_001_MIN_TRIP) {
                    continue;
                }
                let target = binop.left.name.clone().unwrap_or_default();
                findings.push(string_accumulation_finding(
                    unit,
                    &target,
                    binop.meta.span,
                    binop.meta.in_loop_depth,
                    binop.meta.resolver_tier,
                ));
            }
            // `x = x + <string|template literal>`: the assignment target is
            // the accumulator and the binary `+` must BE the whole RHS —
            // contained in the assignment span AND ending at the same
            // line/column, so `x = f(x + "…")` stays silent.
            for assignment in &func.assignments {
                if assignment.target.kind != ShapeKind::Identifier
                    || assignment.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(target) = assignment.target.name.as_deref() else {
                    continue;
                };
                let Some(concat) = func.binary_ops.iter().find(|binop| {
                    binop.op == "+"
                        && binop.left.kind == ShapeKind::Identifier
                        && binop.left.name.as_deref() == Some(target)
                        && is_string_literal_shape(&binop.right)
                        && super::span_contains(&assignment.meta.span, &binop.meta.span)
                        && binop.meta.span.end_line == assignment.meta.span.end_line
                        && binop.meta.span.end_col == assignment.meta.span.end_col
                }) else {
                    continue;
                };
                if concat.meta.in_loop_depth == 0 {
                    continue;
                }
                let Some(loop_node) = enclosing_loop(func, &concat.meta.span) else {
                    continue;
                };
                if loop_node.trip_count.is_some_and(|n| n < STR_001_MIN_TRIP) {
                    continue;
                }
                findings.push(string_accumulation_finding(
                    unit,
                    target,
                    concat.meta.span,
                    concat.meta.in_loop_depth,
                    concat.meta.resolver_tier,
                ));
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// JS-LOOP-001 — Nested loop depth ≥ 2, Info.
///
/// JS uses a depth floor of **2** (not the C catalog's 3): a two-deep nest
/// already multiplies iteration superlinearly. At most one finding per
/// function, anchored at the earliest-span loop whose `depth >= 2`; the
/// message names the depth. Evidence `nest_depth`, `max_depth`. Info/MEDIUM.
pub(crate) fn js_loop_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let Some(loop_node) = func.loops.iter().filter(|l| l.depth >= 2).min_by_key(|l| {
                (
                    l.span.start_line,
                    l.span.start_col,
                    l.span.end_line,
                    l.span.end_col,
                )
            }) else {
                continue;
            };
            let max_depth = func
                .loops
                .iter()
                .filter(|l| super::span_contains(&loop_node.span, &l.span))
                .map(|l| l.depth)
                .max()
                .unwrap_or(loop_node.depth);
            findings.push(Finding {
                threshold: None,
                rule_id: JS_LOOP_001_RULE_ID.to_string(),
                title: "Nested loop depth ≥ 2".to_string(),
                message: format!(
                    "loops nest {} deep here (chain reaches depth {max_depth}) — \
                     every extra level multiplies the iteration count, so the \
                     body runs superlinearly; flattening the nesting, hoisting \
                     invariant work, or adding an early exit is a candidate \
                     optimization. Accepted FP class: matrix work where n² is \
                     inherent",
                    loop_node.depth
                ),
                severity: Severity::Info,
                confidence: Confidence::Medium,
                resolver_tier: loop_node.resolver_tier,
                span: loop_node.span,
                path: unit.file.path.clone(),
                evidence: [
                    ("nest_depth".to_string(), loop_node.depth.to_string()),
                    ("max_depth".to_string(), max_depth.to_string()),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            });
        }
    }
    sort_findings(&mut findings);
    findings
}

/// Whether `callee_name` is one of the known synchronous Node APIs.
fn is_sync_node_api(callee_name: &str) -> bool {
    SYNC_NODE_APIS.contains(&callee_name) || SYNC_CRYPTO_APIS.contains(&callee_name)
}

/// JS-LOOP-002 — Synchronous blocking I/O in a loop, Warn.
///
/// A [`crate::ir::CallEvent`] at `in_loop_depth >= 1` whose callee is a known
/// synchronous Node API (`readFileSync`, `execSync`, sync `crypto`, …) pays one
/// syscall/serialization per iteration and blocks the event loop. Silent
/// outside loops. Evidence `callee`, `loop_depth`. Warn/MEDIUM.
pub(crate) fn js_loop_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0 || !is_sync_node_api(&call.callee_name) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_LOOP_002_RULE_ID.to_string(),
                    title: "Synchronous blocking I/O inside loop".to_string(),
                    message: format!(
                        "synchronous '{}' runs inside a loop at depth {} — each \
                         iteration performs a blocking syscall/serialization on \
                         the single event-loop thread; batching the work, using \
                         the async API (`fs/promises`, async `exec`), or making \
                         one call outside the loop is a candidate optimization. \
                         Accepted FP class: a startup-only loop",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
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

/// JS-LOOP-007 — Linear membership scan inside a loop, Warn (flagship).
///
/// An `indexOf`/`includes`/`find`/`findIndex`/`some`/`every`/`filter` call at
/// `in_loop_depth >= 1` with a non-empty `receiver_root` scans a collection
/// linearly per iteration → O(n²). The small-collection accepted FP class is
/// excluded: a canonical loop with `trip_count < 16` stays silent. Evidence
/// `callee`, `receiver_root`, `loop_depth`. Warn/MEDIUM.
pub(crate) fn js_loop_007(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0
                    || !MEMBERSHIP_SCANS.contains(&call.callee_name.as_str())
                {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref().filter(|r| !r.is_empty()) else {
                    continue;
                };
                let Some(loop_node) = enclosing_loop(func, &call.meta.span) else {
                    continue;
                };
                if loop_node.trip_count.is_some_and(|n| n < LOOP_007_MIN_TRIP) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_LOOP_007_RULE_ID.to_string(),
                    title: "Linear membership scan inside loop".to_string(),
                    message: format!(
                        "'{root}.{}' scans the collection linearly on every \
                         iteration at loop depth {} — the nested linear scan \
                         makes the loop O(n²); building a `Set`/`Map` once and \
                         doing a keyed lookup is a candidate optimization. \
                         Accepted FP class: small (< 16) collections",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
                        ("receiver_root".to_string(), root.to_string()),
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

/// JS-ASYNC-001 — `await` inside a loop, Warn (flagship).
///
/// An [`crate::ir::AwaitEvent`] at `in_loop_depth >= 1` inside an `async`
/// function serialises each coroutine's latency: one microtask round-trip per
/// item. The `record.is_async` gate is mandatory. One finding per await event.
/// Evidence `loop_depth`. Warn/MEDIUM.
pub(crate) fn js_async_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if !func.record.is_async {
                continue;
            }
            for await_event in &func.awaits {
                if await_event.meta.in_loop_depth == 0 {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_ASYNC_001_RULE_ID.to_string(),
                    title: "`await` inside a loop".to_string(),
                    message: format!(
                        "an `await` runs inside a loop at depth {} — awaiting \
                         each item sequentially serialises one microtask \
                         round-trip per iteration; collecting the promises and \
                         awaiting `Promise.all(items.map(...))` is a candidate \
                         optimization. Accepted FP class: deliberately \
                         sequential work (rate limiting, ordering)",
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

/// JS-ASYNC-002 — Synchronous blocking call inside an `async` function, Warn
/// (flagship).
///
/// A known synchronous Node API called anywhere in an `async` function blocks
/// the event loop for every concurrent task. The `record.is_async` gate is
/// mandatory; there is NO loop requirement (any depth fires). Evidence
/// `callee`, `loop_depth`. Warn/MEDIUM.
pub(crate) fn js_async_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            if !func.record.is_async {
                continue;
            }
            for call in &func.calls {
                if !is_sync_node_api(&call.callee_name) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_ASYNC_002_RULE_ID.to_string(),
                    title: "Synchronous blocking call inside `async` function".to_string(),
                    message: format!(
                        "synchronous '{}' is called inside an `async` function — \
                         the blocking call stalls the single event-loop thread \
                         for every concurrent task; using the async equivalent \
                         (`fs/promises`, async `exec`/`spawn`) is a candidate \
                         optimization. Accepted FP class: a CLI single-shot path",
                        call.callee_name
                    ),
                    severity: Severity::Warn,
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

/// JS-CALL-001 — Unguarded direct recursion, Info.
///
/// A DIRECT call whose `callee_name` equals the enclosing function name and
/// whose `receiver_root` is `None` (a bare self-call; `this.f()`/member calls
/// carry a receiver and are excluded). Guard-gated: the rule fires ONLY when NO
/// [`crate::ir::ComparisonEvent`] in the same function has a span strictly
/// before the recursive call's span — i.e. no visible base-case check precedes
/// it. Evidence `function`, `loop_depth`. Info/MEDIUM.
pub(crate) fn js_call_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.callee_name != func.record.name
                    || call.receiver_root.is_some()
                {
                    continue;
                }
                let call_start = (call.meta.span.start_line, call.meta.span.start_col);
                let guarded = func
                    .comparisons
                    .iter()
                    .any(|c| (c.meta.span.start_line, c.meta.span.start_col) < call_start);
                if guarded {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_CALL_001_RULE_ID.to_string(),
                    title: "Direct recursion without a visible base case".to_string(),
                    message: format!(
                        "'{}' calls itself directly with no comparison guarding \
                         the call — each recursive frame grows the stack and \
                         repeats frame-setup cost; converting to a loop with an \
                         accumulator, or adding an explicit base-case guard, is \
                         a candidate optimization. Accepted FP class: a recursive \
                         tree walk with small depth",
                        func.record.name
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("function".to_string(), func.record.name.clone()),
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

/// JS-ARCH-001 — Import cycle, Info.
///
/// Over RESOLVED relative-import edges only (`edge.resolved` with `to_path`),
/// one finding per [`break_cycles`]-broken edge, anchored at that pair's
/// earliest retained directive span. A module cycle is a module-init-ordering
/// hazard and blocks clean tree-shaking. Evidence `kind="cycle"`,
/// `coupled_with`. Info/MEDIUM.
pub(crate) fn js_arch_001(units: &[FileUnit]) -> Vec<Finding> {
    // (from, to) → earliest span.
    let mut earliest: BTreeMap<(String, String), Span> = BTreeMap::new();
    for unit in units {
        for edge in &unit.includes {
            if !edge.resolved {
                continue;
            }
            let Some(to) = &edge.to_path else {
                continue;
            };
            let key = (edge.from_path.clone(), to.clone());
            match earliest.get(&key) {
                Some(span) if *span <= edge.span => {}
                _ => {
                    earliest.insert(key, edge.span);
                }
            }
        }
    }
    let pairs: Vec<(String, String)> = earliest.keys().cloned().collect();
    let result = break_cycles(&pairs);
    let mut findings = Vec::new();
    for (from, to) in &result.broken {
        let span = earliest
            .get(&(from.clone(), to.clone()))
            .copied()
            .unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: JS_ARCH_001_RULE_ID.to_string(),
            title: "Import cycle".to_string(),
            message: format!(
                "'{from}' and '{to}' import each other, forming a cycle — module \
                 initialisation order becomes observable and bundlers cannot \
                 tree-shake the cycle cleanly; breaking the lowest-sorted edge \
                 into a shared leaf module is a candidate optimization. Accepted \
                 FP class: type-only import cycles (TypeScript erases them)"
            ),
            severity: Severity::Info,
            confidence: Confidence::Medium,
            resolver_tier: ResolverTier::Syntactic,
            span,
            path: from.clone(),
            evidence: [
                ("kind".to_string(), "cycle".to_string()),
                ("coupled_with".to_string(), to.clone()),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
    }
    sort_findings(&mut findings);
    findings
}

/// JS-TYPE-001 — `delete obj.prop`, Info.
///
/// A [`crate::ir::UnaryEvent`] with `op == "delete"` forces a hidden-class →
/// dictionary-mode transition on the object. The subscript form
/// (`delete obj["k"]`) is EXCLUDED per the frozen disposition: it is a
/// different (keyed) operation and stays silent. Evidence `property`.
/// Info/MEDIUM.
pub(crate) fn js_type_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for unary in &func.unaries {
                if unary.op != "delete" || unary.operand.kind != ShapeKind::MemberAccess {
                    continue;
                }
                let property = unary
                    .operand
                    .name
                    .clone()
                    .or_else(|| unary.operand.value.clone())
                    .unwrap_or_default();
                findings.push(Finding {
                    threshold: None,
                    rule_id: JS_TYPE_001_RULE_ID.to_string(),
                    title: "`delete` on an object property".to_string(),
                    message: format!(
                        "'delete' removes a property from an object ({}), forcing \
                         a V8 hidden-class → dictionary-mode transition that slows \
                         every later access; setting the property to `undefined` \
                         or rebuilding the object is a candidate optimization. \
                         Accepted FP class: intentional cache eviction (rare)",
                        if property.is_empty() {
                            "object property".to_string()
                        } else {
                            format!("'{property}'")
                        }
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: unary.meta.resolver_tier,
                    span: unary.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [("property".to_string(), property)].into_iter().collect(),
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
mod tests;
