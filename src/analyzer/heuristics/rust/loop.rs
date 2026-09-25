//! Rust loop-family heuristics (plan `rust-backend` R3 workstreams C1 + C2):
//! PERF-RS-LOOP-001..008.
//!
//! Each mirrors the C rule named in its rustdoc onto Rust idioms. The
//! registry only ever hands this family `language == "rust"` units, so every
//! function below is a pure IR view over Rust records/events. Family layout
//! and evidence-key style deliberately match `heuristics::r#loop`.
//!
//! Wave C2 added the API-name rules LOOP-002 (expensive I/O/setup),
//! LOOP-003 (char/byte-wise processing) and LOOP-006 (indirect call in the
//! innermost loop). Their curated name lists are Rust-API-specific and live
//! as module consts below; they are NOT shared with the C family.

use crate::analyzer::finding::{
    RS_LOOP_001_RULE_ID, RS_LOOP_002_RULE_ID, RS_LOOP_003_RULE_ID, RS_LOOP_004_RULE_ID,
    RS_LOOP_005_RULE_ID, RS_LOOP_006_RULE_ID, RS_LOOP_007_RULE_ID, RS_LOOP_008_RULE_ID,
};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{
    BinaryOpEvent, CalleeKind, ComparisonEvent, LoopNode, ResolverTier, ScopeClass, Shape,
    ShapeKind, Span, StatementRole,
};
use crate::scanner::FileUnit;

use super::super::{
    LOOP_001_MIN_DEPTH, LOOP_007_MIN_DEPTH, LOOP_008_MIN_CHAIN_DEPTH, shape_kind_token,
    span_contains,
};
use super::{callee_token, innermost_enclosing_loop, qualified_match};

/// Rust expensive-I/O MACRO family (LOOP-002): matched on `callee_name` only
/// when the call event is a macro invocation (`is_macro`). `write`/`writeln`
/// are the formatting-to-writer macros.
const EXPENSIVE_IO_MACROS: [&str; 7] = [
    "println", "print", "eprintln", "eprint", "write", "writeln", "dbg",
];

/// Rust expensive-I/O METHOD family (LOOP-002): matched on `callee_name`.
const EXPENSIVE_IO_METHODS: [&str; 10] = [
    "read_to_string",
    "write_all",
    "read_to_end",
    "flush",
    "sync_all",
    "recv",
    "send",
    "recv_from",
    "send_to",
    "read_line",
];

/// Rust expensive SETUP family (LOOP-002): qualified paths matched as
/// `path == entry` or a `::entry` suffix, so `std::fs::read` matches
/// `fs::read`.
const EXPENSIVE_IO_QUALIFIED: [&str; 10] = [
    "File::open",
    "File::create",
    "fs::read",
    "fs::write",
    "fs::read_to_string",
    "Command::new",
    "TcpStream::connect",
    "UdpSocket::bind",
    "env::var",
    "env::args",
];

/// Char/byte-at-a-time method family (LOOP-003): matched on `callee_name`.
const CHARWISE_METHODS: [&str; 5] = [
    "chars",
    "bytes",
    "char_indices",
    "byte_indices",
    "read_byte",
];

/// PERF-RS-LOOP-002 — Expensive I/O / setup call inside loop
/// (mirrors PERF-C-LOOP-002).
///
/// Intent: per-iteration I/O or resource setup dominates runtime; hoisting or
/// batching is the candidate. Detection — a Rust call event at
/// `in_loop_depth ≥ 1` in one of three curated groups:
/// - MACRO ([`EXPENSIVE_IO_MACROS`], matched only when `is_macro`):
///   `println!`/`print!`/`eprintln!`/`eprint!`/`write!`/`writeln!`/`dbg!`;
/// - METHOD ([`EXPENSIVE_IO_METHODS`], matched on `callee_name`):
///   `read_to_string`/`write_all`/`read_to_end`/`flush`/`sync_all`/`recv`/
///   `send`/`recv_from`/`send_to`/`read_line`;
/// - QUALIFIED ([`EXPENSIVE_IO_QUALIFIED`], matched on `callee_path` with a
///   `::`-suffix rule): `File::open`/`File::create`/`fs::read`/`fs::write`/
///   `fs::read_to_string`/`Command::new`/`TcpStream::connect`/
///   `UdpSocket::bind`/`env::var`/`env::args`.
///
/// Severity: Hot/MEDIUM by default; lowered to Warn/MEDIUM when the call sits
/// inside a conditional branch within the loop (`conditional_guarded`,
/// mirroring the C rule's intentional-logging FP note). Evidence `callee`,
/// `loop_depth`, `conditional_guarded`.
///
/// Accepted FP classes: an intentionally interleaved debug/log print stays
/// candidate-grade (Warn when guarded); `send`/`recv` on a non-blocking
/// channel also fire. Windows of `token_tree`-opaque macros cannot be
/// inspected beyond the invocation event.
pub(crate) fn rust_loop_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.meta.in_loop_depth == 0 || !is_expensive_io_call(call) {
                    continue;
                }
                let guarded = call.conditional_guarded;
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_LOOP_002_RULE_ID.to_string(),
                    title: "Expensive setup or I/O call inside loop (Rust)".to_string(),
                    message: format!(
                        "'{callee}' runs every iteration at loop depth {} — \
                         hoisting the call out of the loop or batching its \
                         work is a candidate optimization",
                        call.meta.in_loop_depth
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
                        ("callee".to_string(), callee),
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

/// LOOP-002 group gate: macro name (macro only), method name, or qualified
/// setup path — the three curated Rust families.
fn is_expensive_io_call(call: &crate::ir::CallEvent) -> bool {
    if call.is_macro {
        return EXPENSIVE_IO_MACROS.contains(&call.callee_name.as_str());
    }
    if call.callee_kind != CalleeKind::Direct {
        return false;
    }
    EXPENSIVE_IO_METHODS.contains(&call.callee_name.as_str())
        || EXPENSIVE_IO_QUALIFIED
            .iter()
            .any(|entry| qualified_match(call.callee_path.as_deref(), entry))
}

/// PERF-RS-LOOP-003 — Char/byte-at-a-time processing in loop
/// (REFRAMED mirror of PERF-C-LOOP-003).
///
/// REFRAMING (mandatory, documented): the C rule caught byte-at-a-time stdio
/// (`fgetc`/`putc`) as the loop body's primary statement. The Rust analogue is
/// per-character/per-byte iterator or reader work: a DIRECT method call
/// `chars`/`bytes`/`char_indices`/`byte_indices`/`read_byte` at
/// `in_loop_depth ≥ 1` whose `statement_role == Primary` (the call IS the
/// loop body's primary statement). Info/MEDIUM; evidence `callee`,
/// `loop_depth`.
///
/// Accepted FP classes / honest limitations: buffering is often already
/// provided by the reader (`BufReader`), so a single-element read per
/// iteration can be deliberate; the primary-statement gate mirrors the C
/// rule exactly. `chars()` in a `for` header is NOT in the loop body, so it
/// never fires (correct: iteration itself is the intended pattern).
pub(crate) fn rust_loop_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || !CHARWISE_METHODS.contains(&call.callee_name.as_str())
                    || call.statement_role != StatementRole::Primary
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_LOOP_003_RULE_ID.to_string(),
                    title: "Character-wise processing in loop (Rust)".to_string(),
                    message: format!(
                        "character/byte-wise processing via '{callee}' as the \
                         loop body's primary statement — processing the input \
                         in buffered blocks or chunks is a candidate \
                         optimization",
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
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

/// PERF-RS-LOOP-006 — Indirect call in innermost loop
/// (mirrors PERF-C-LOOP-006).
///
/// Intent: an indirect (function-pointer/trait-object) call inhibits inlining
/// and branch prediction worst where it runs every iteration. Detection: a
/// [`crate::ir::CallEvent`] with `callee_kind == Indirect` at
/// `in_loop_depth ≥ 1` AND `is_innermost == true`. Info/LOW (no type info
/// confirming the target). Evidence `callee`, `loop_depth`.
///
/// HONEST LIMITATION (mandated): the Rust backend classifies a closure or
/// method called through a struct field (`self.callback()`,
/// `ctx.handler(field)`) as DIRECT (`field_expression` callee), so the
/// Rust analogue of that dispatch class does NOT fire here — the existing D2
/// direct-classification FP class.
pub(crate) fn rust_loop_006(units: &[FileUnit]) -> Vec<Finding> {
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
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_LOOP_006_RULE_ID.to_string(),
                    title: "Indirect call in innermost loop (Rust)".to_string(),
                    message: format!(
                        "indirect call through '{callee}' runs in the \
                         innermost loop at depth {} — resolving the target \
                         once outside the loop is a candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Low,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
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

/// PERF-RS-LOOP-001 — Deeply nested loops (mirrors PERF-C-LOOP-001).
///
/// Intent: surface loop nests deep enough to imply an O(n^k) candidate zone.
/// Detection: every Rust [`LoopNode`] whose `depth` is EXACTLY
/// [`LOOP_001_MIN_DEPTH`] (3) — one finding per maximal nest entry point (a
/// depth-5 chain yields one finding at its depth-3 ancestor). Evidence
/// `nest_depth` / `max_depth`; threshold `("min_nesting_depth", 3)`.
///
/// Accepted FP classes / limitations: a deep nest over tiny fixed bounds is
/// legitimate; the syntactic tier cannot bound trip counts. Macro-generated
/// loops are invisible (`token_tree` blind spot, rust-backend.md §5).
pub(crate) fn rust_loop_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for loop_node in &func.loops {
                if loop_node.depth != LOOP_001_MIN_DEPTH {
                    continue;
                }
                // Max depth among this loop's inclusive descendants; the loop
                // contains itself, so the floor is its own depth.
                let max_depth = func
                    .loops
                    .iter()
                    .filter(|l| span_contains(&loop_node.span, &l.span))
                    .map(|l: &LoopNode| l.depth)
                    .max()
                    .unwrap_or(LOOP_001_MIN_DEPTH)
                    .max(LOOP_001_MIN_DEPTH);
                findings.push(Finding {
                    threshold: Some(("min_nesting_depth", LOOP_001_MIN_DEPTH)),
                    rule_id: RS_LOOP_001_RULE_ID.to_string(),
                    title: "Deeply nested loops (Rust)".to_string(),
                    message: format!(
                        "loops nest {} deep here (chain reaches depth \
                         {max_depth}) — flattening the nesting or using \
                         iterator combinators with early exits is a \
                         candidate optimization",
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

/// PERF-RS-LOOP-004 — Variable-divisor integer div/mod in innermost loop
/// (mirrors PERF-C-LOOP-004).
///
/// Intent: `/` or `%` by a non-constant divisor inside the innermost loop
/// pays real integer division per iteration (constant divisors are
/// strength-reduced by the compiler; Rust's `const`/literal values count).
/// Detection: [`BinaryOpEvent`] op `/` or `%`, `meta.is_innermost`,
/// `right.kind != Literal`. Info/Medium; evidence `op`, `divisor_kind` and
/// the divisor name when it is an identifier.
///
/// Accepted FP classes: constant-foldable non-literal divisors (a `const`
/// identifier, `size_of::<T>()`) fire by design — the syntactic tier cannot
/// prove constness. INFO severity keeps them candidate-grade.
pub(crate) fn rust_loop_004(units: &[FileUnit]) -> Vec<Finding> {
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
                    threshold: None,
                    rule_id: RS_LOOP_004_RULE_ID.to_string(),
                    title: "Variable-divisor division in innermost loop (Rust)".to_string(),
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

/// RS-LOOP-004 gate: `/` or `%`, innermost only, non-literal divisor.
fn is_variable_division(binop: &BinaryOpEvent) -> bool {
    (binop.op == "/" || binop.op == "%")
        && binop.meta.is_innermost
        && binop.right.kind != ShapeKind::Literal
}

/// PERF-RS-LOOP-005 — Global (`static`/`const`) variable written inside loop
/// (mirrors PERF-C-LOOP-005).
///
/// Intent: a store to a Rust `static`/`const`-backed target from inside a
/// loop is a potential loop-invariant store or shared-state/contention
/// signal. Detection: [`crate::ir::AssignmentEvent`] with
/// `scope_class == ScopeClass::Global` at `in_loop_depth ≥ 1` and an
/// `Identifier` target.
///
/// DEDUP: one finding per `(target identifier, innermost enclosing loop
/// span)` pair, anchored at the EARLIEST qualifying assignment span.
/// Confidence MEDIUM is mandatory (scope classification is name-based against
/// collected `static`/`const` items). Accepted FP class: accumulator updates
/// that legitimately belong in the loop stay flagged (Info band). Framing
/// rule: optimization candidate, never bug/unsafe.
pub(crate) fn rust_loop_005(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
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
                    threshold: None,
                    rule_id: RS_LOOP_005_RULE_ID.to_string(),
                    title: "Global variable written inside loop (Rust)".to_string(),
                    message: format!(
                        "static/const-backed '{}' is stored every iteration at \
                         loop depth {} — hoisting the write out of the loop or \
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

/// PERF-RS-LOOP-007 — Linear membership scan nested in loop
/// (mirrors PERF-C-LOOP-007).
///
/// Intent: the hand-rolled O(n²) lookup shape — an element compared against
/// a non-literal key inside a nested loop. Detection: [`ComparisonEvent`] at
/// `in_loop_depth ≥ LOOP_007_MIN_DEPTH` (2) where ONE side is a `Subscript`
/// whose `index_ident` matches an induction variable of a span-containing
/// [`LoopNode`] (Rust `for` patterns populate `induction_vars`) AND the other
/// side is NOT a literal. Info/LOW; evidence `root` (when present),
/// `index_ident`, `loop_depth`.
///
/// Accepted FP class: comparing against a known-sentinel variable is not
/// distinguishable syntactically from comparing against a live key — the
/// candidate stays Info/LOW. Framing rule: optimization candidate.
pub(crate) fn rust_loop_007(units: &[FileUnit]) -> Vec<Finding> {
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
                    threshold: None,
                    rule_id: RS_LOOP_007_RULE_ID.to_string(),
                    title: "Linear membership scan nested in loop (Rust)".to_string(),
                    message: format!(
                        "element lookup compared against a non-literal at \
                         loop depth {} re-scans linearly on every outer \
                         iteration — using a HashSet/HashMap lookup table or \
                         sorting plus binary search is a candidate \
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

/// RS-LOOP-007 gate: at [`LOOP_007_MIN_DEPTH`] or deeper, one side a
/// `Subscript` indexed by an induction var of an enclosing loop, the other
/// side not a literal. Returns `(subscript_side, index_ident)` with the LEFT
/// side preferred when both qualify.
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

/// PERF-RS-LOOP-008 — Pointer/lifetime-chasing chain in innermost loop
/// (mirrors PERF-C-LOOP-008).
///
/// Intent: repeated member/subscript dereferences per iteration are
/// cache-miss candidates. Detection: [`crate::ir::AccessChainEvent`] with
/// chain depth ≥ [`LOOP_008_MIN_CHAIN_DEPTH`] (3) at `in_loop_depth ≥ 1` AND
/// `is_innermost` AND at least one chain element is a
/// [`ShapeKind::Subscript`] (an INDEX step). Info/Medium; evidence `root`,
/// `chain_depth`, `loop_depth`.
///
/// M1 Precision Gate (plan `evidence-beta`): the Subscript (index)
/// requirement was added because pure member chains (`ctx.ops.fun`) are
/// idiomatic trait-object/vtable-style dispatch, not a cache-miss chase —
/// the dominant false-positive class on the Rust self-scan. Without an index
/// step the chain no longer fires.
///
/// Accepted limitation (documented): the backend does NOT model explicit
/// derefs as chain elements, so a pure deref chain such as `(*p).a.b` yields
/// `MemberAccess`-only elements and therefore stays SILENT — the gate cannot
/// see its pointer dereference. This is an accepted false negative, not a
/// "pointer chase" the rule claims to catch.
///
/// Accepted FP class (catalog-mandated): trait-object/vtable-style dispatch
/// (`ctx.ops.fun`) is idiomatic Rust design, not a defect — the finding stays
/// Info/MEDIUM with the class documented, never suppressed. Framing rule:
/// optimization candidate.
pub(crate) fn rust_loop_008(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for chain in &func.access_chains {
                if chain.elements.len() < LOOP_008_MIN_CHAIN_DEPTH
                    || !chain.meta.is_innermost
                    || chain.meta.in_loop_depth == 0
                    || !chain
                        .elements
                        .iter()
                        .any(|e| e.kind == ShapeKind::Subscript)
                {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_LOOP_008_RULE_ID.to_string(),
                    title: "Member-chain dereference in innermost loop (Rust)".to_string(),
                    message: format!(
                        "'{}' field/index chain of depth {} executes in \
                         the innermost loop — caching intermediate \
                         references or flattening the structure is a \
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

/// RS-LOOP-005 dedup group: one finding per `(target identifier,
/// enclosing-loop span)` pair, anchored at the earliest qualifying span.
struct Loop005Group {
    target: String,
    loop_span: Span,
    anchor: Span,
    loop_depth: u32,
    resolver_tier: ResolverTier,
}
