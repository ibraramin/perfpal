//! Parser-free unit tests for the JavaScript/TypeScript (`PERF-JS-*`) rules
//! (plan `js-ts` B2-W2). Every rule has at least one positive and one silent
//! control built directly from IR structs (ADR-0001 guardrail 2 — no
//! tree-sitter grammar dependency).

use super::*;
use crate::ir::{
    AllocEvent, AssignmentEvent, AwaitEvent, BinaryOpEvent, CallEvent, ComparisonEvent, EventMeta,
    FileRecord, FunctionRecord, IncludeEdge, IncludeForm, LocCounts, LoopKind, ScopeClass, Shape,
    StatementRole, UnaryEvent,
};
use crate::scanner::{FileUnit, FunctionUnit};

fn span(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Span {
    Span {
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

fn point(line: u32) -> Span {
    span(line, 1, line, 20)
}

fn loop_span() -> Span {
    span(3, 1, 8, 1)
}

fn meta(span: Span, depth: u32) -> EventMeta {
    EventMeta {
        span,
        in_loop_depth: depth,
        is_innermost: depth >= 1,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
    }
}

/// A minimal JS function; tests push event streams onto it directly.
fn func(name: &str) -> FunctionUnit {
    FunctionUnit {
        record: FunctionRecord {
            id: format!("m.js#{name}#1:1"),
            name: name.to_string(),
            path: "m.js".to_string(),
            span: span(1, 1, 100, 1),
            ..FunctionRecord::default()
        },
        ..FunctionUnit::default()
    }
}

fn unit(functions: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: "m.js".to_string(),
            language: "js".to_string(),
            loc: LocCounts::default(),
            function_count: functions.len() as u32,
            global_var_count: 0,
            include_counts: Default::default(),
        },
        functions,
        ..FileUnit::default()
    }
}

fn file_unit(path: &str, includes: Vec<IncludeEdge>, functions: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "js".to_string(),
            loc: LocCounts::default(),
            function_count: functions.len() as u32,
            global_var_count: 0,
            include_counts: Default::default(),
        },
        includes,
        functions,
        ..FileUnit::default()
    }
}

fn call(name: &str, root: Option<&str>, at: Span, depth: u32) -> CallEvent {
    CallEvent {
        meta: meta(at, depth),
        callee_name: name.to_string(),
        callee_kind: CalleeKind::Direct,
        callee_path: Some(match root {
            Some(root) => format!("{root}.{name}"),
            None => name.to_string(),
        }),
        is_macro: false,
        args: Vec::new(),
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
        semantic: None,
        receiver_root: root.map(str::to_string),
    }
}

fn alloc(kind: AllocKind, names: &[&str], at: Span, depth: u32) -> AllocEvent {
    AllocEvent {
        meta: meta(at, depth),
        kind,
        referenced_names: names.iter().map(|n| (*n).to_string()).collect(),
        has_spread: false,
    }
}

/// An array literal carrying a spread child (`[...acc, x]`).
fn spread_alloc(names: &[&str], at: Span, depth: u32) -> AllocEvent {
    AllocEvent {
        meta: meta(at, depth),
        kind: AllocKind::Array,
        referenced_names: names.iter().map(|n| (*n).to_string()).collect(),
        has_spread: true,
    }
}

fn assign(name: &str, at: Span, depth: u32) -> AssignmentEvent {
    AssignmentEvent {
        meta: meta(at, depth),
        target: Shape::identifier(name),
        scope_class: ScopeClass::Local,
        binding_kind: crate::ir::BindingKind::Unknown,
    }
}

fn binop(op: &str, left: &str, at: Span, depth: u32) -> BinaryOpEvent {
    binop_with_right(op, left, Shape::identifier("x"), at, depth)
}

fn binop_with_right(op: &str, left: &str, right: Shape, at: Span, depth: u32) -> BinaryOpEvent {
    BinaryOpEvent {
        meta: meta(at, depth),
        op: op.to_string(),
        left: Shape::identifier(left),
        right,
    }
}

fn comparison(at: Span, depth: u32) -> ComparisonEvent {
    ComparisonEvent {
        meta: meta(at, depth),
        operator: "<".to_string(),
        left: Shape::identifier("n"),
        right: Shape::literal("1"),
    }
}

fn loop_node(depth: u32, vars: &[&str], trip: Option<u64>, at: Span) -> LoopNode {
    LoopNode {
        kind: LoopKind::For,
        depth,
        induction_vars: vars.iter().map(|v| (*v).to_string()).collect(),
        span: at,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
        trip_count: trip,
        condition_is_zero: false,
    }
}

fn await_event(at: Span, depth: u32) -> AwaitEvent {
    AwaitEvent {
        meta: meta(at, depth),
    }
}

fn unary(op: &str, operand: Shape, at: Span) -> UnaryEvent {
    UnaryEvent {
        meta: meta(at, 0),
        op: op.to_string(),
        operand,
    }
}

fn edge(from: &str, to: &str) -> IncludeEdge {
    IncludeEdge {
        from_path: from.to_string(),
        to_path: Some(to.to_string()),
        form: IncludeForm::Quoted,
        resolved: true,
        is_mod: false,
        span: point(1),
    }
}

// ---------------------------------------------------------------------------
// ALLOC-001
// ---------------------------------------------------------------------------

#[test]
fn alloc_001_fires_on_loop_invariant_object_literal() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["i"], None, loop_span()));
    f.allocations
        .push(alloc(AllocKind::Object, &["cfg"], point(5), 1));

    let out = js_alloc_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_ALLOC_001_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
    assert_eq!(out[0].confidence, Confidence::Medium);
    assert!(!out[0].macro_suspect);
    assert!(!out[0].error_node_overlap);
}

#[test]
fn alloc_001_silent_on_loop_variant_or_mutated_literal() {
    // Depends on the induction variable.
    let mut induction = func("f");
    induction
        .loops
        .push(loop_node(1, &["i"], None, loop_span()));
    induction
        .allocations
        .push(alloc(AllocKind::Array, &["i"], point(5), 1));
    assert!(js_alloc_001(&[unit(vec![induction])]).is_empty());

    // A referenced name is mutated inside the loop (loop-local mutation).
    let mut mutated = func("f");
    mutated.loops.push(loop_node(1, &["i"], None, loop_span()));
    mutated
        .allocations
        .push(alloc(AllocKind::Object, &["cfg"], point(5), 1));
    mutated.assignments.push(assign("cfg", point(6), 1));
    assert!(js_alloc_001(&[unit(vec![mutated])]).is_empty());

    // No enclosing loop node contains the span (honest unknown).
    let mut orphan = func("f");
    orphan
        .allocations
        .push(alloc(AllocKind::Object, &["cfg"], point(5), 1));
    assert!(js_alloc_001(&[unit(vec![orphan])]).is_empty());
}

// ---------------------------------------------------------------------------
// ALLOC-003
// ---------------------------------------------------------------------------

#[test]
fn alloc_003_fires_on_accumulator_concat() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["x"], None, loop_span()));
    f.calls.push(call("concat", Some("acc"), point(5), 1));
    f.assignments.push(assign("acc", point(6), 1));

    let out = js_alloc_003(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_ALLOC_003_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
}

#[test]
fn alloc_003_fires_on_spread_accumulator() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["item"], None, loop_span()));
    f.allocations
        .push(spread_alloc(&["acc", "item"], point(5), 1));
    f.assignments.push(assign("acc", point(6), 1));

    let out = js_alloc_003(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_ALLOC_003_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
    assert_eq!(
        out[0].evidence.get("kind").map(String::as_str),
        Some("spread-accumulator")
    );
    assert_eq!(
        out[0].evidence.get("receiver_root").map(String::as_str),
        Some("acc")
    );
}

#[test]
fn alloc_003_silent_without_accumulator_assignment() {
    // A snapshot call whose receiver is never reassigned in the loop.
    let mut snapshot = func("f");
    snapshot.loops.push(loop_node(1, &["x"], None, loop_span()));
    snapshot.calls.push(call("slice", Some("src"), point(5), 1));
    assert!(js_alloc_003(&[unit(vec![snapshot])]).is_empty());

    // No receiver root at all.
    let mut bare = func("f");
    bare.loops.push(loop_node(1, &["x"], None, loop_span()));
    bare.calls.push(call("concat", None, point(5), 1));
    assert!(js_alloc_003(&[unit(vec![bare])]).is_empty());

    // A spread literal whose referenced names are never reassigned in the
    // loop is an immutable rebuild, not the accumulator shape.
    let mut spread_snapshot = func("f");
    spread_snapshot
        .loops
        .push(loop_node(1, &["x"], None, loop_span()));
    spread_snapshot
        .allocations
        .push(spread_alloc(&["src", "x"], point(5), 1));
    assert!(js_alloc_003(&[unit(vec![spread_snapshot])]).is_empty());

    // No spread child (`has_spread == false`) never takes the spread arm.
    let mut no_spread = func("f");
    no_spread
        .loops
        .push(loop_node(1, &["x"], None, loop_span()));
    no_spread
        .allocations
        .push(alloc(AllocKind::Array, &["acc", "x"], point(5), 1));
    no_spread.assignments.push(assign("acc", point(6), 1));
    assert!(js_alloc_003(&[unit(vec![no_spread])]).is_empty());
}

// ---------------------------------------------------------------------------
// STR-001
// ---------------------------------------------------------------------------

#[test]
fn str_001_fires_on_string_and_template_plus_equals_in_loop() {
    // String literal RHS (`s += "a"`).
    let mut string_rhs = func("f");
    string_rhs
        .loops
        .push(loop_node(1, &["i"], None, loop_span()));
    string_rhs.binary_ops.push(binop_with_right(
        "+=",
        "s",
        Shape::literal("\"a\""),
        point(5),
        1,
    ));
    let out = js_str_001(&[unit(vec![string_rhs])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_STR_001_RULE_ID);
    assert_eq!(out[0].severity, Severity::Warn);
    assert_eq!(out[0].evidence.get("target").map(String::as_str), Some("s"));

    // Template literal RHS (`s += `a${b}``).
    let mut template_rhs = func("f");
    template_rhs
        .loops
        .push(loop_node(1, &["i"], None, loop_span()));
    template_rhs.binary_ops.push(binop_with_right(
        "+=",
        "s",
        Shape::literal("`a${b}`"),
        point(5),
        1,
    ));
    assert_eq!(js_str_001(&[unit(vec![template_rhs])]).len(), 1);
}

#[test]
fn str_001_fires_on_x_equals_x_plus_string_in_loop() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["i"], None, loop_span()));
    f.assignments.push(assign("s", point(5), 1));
    f.binary_ops.push(binop_with_right(
        "+",
        "s",
        Shape::literal("\"a\""),
        point(5),
        1,
    ));

    let out = js_str_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_STR_001_RULE_ID);
}

#[test]
fn str_001_assignment_arm_requires_the_binary_to_be_the_whole_rhs() {
    // `s = s + "x"`: the `+` ends exactly where the assignment ends.
    let mut direct = func("f");
    direct.loops.push(loop_node(1, &["i"], None, loop_span()));
    direct.assignments.push(assign("s", span(5, 1, 5, 20), 1));
    direct.binary_ops.push(binop_with_right(
        "+",
        "s",
        Shape::literal("\"x\""),
        span(5, 1, 5, 20),
        1,
    ));
    let out = js_str_001(&[unit(vec![direct])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_STR_001_RULE_ID);

    // `s = f(s + "!")`: the `+` is nested in a call, so it is merely
    // CONTAINED in the assignment and must not fire (canary predicate).
    let mut nested = func("f");
    nested.loops.push(loop_node(1, &["i"], None, loop_span()));
    nested.assignments.push(assign("s", span(5, 1, 5, 24), 1));
    nested.binary_ops.push(binop_with_right(
        "+",
        "s",
        Shape::literal("\"!\""),
        span(5, 1, 5, 23),
        1,
    ));
    assert!(js_str_001(&[unit(vec![nested])]).is_empty());
}

#[test]
fn str_001_silent_on_numeric_or_identifier_plus_equals() {
    // Numeric `+=` inside a loop is the B2-W0 silent control.
    let mut numeric = func("f");
    numeric.loops.push(loop_node(1, &["i"], None, loop_span()));
    numeric.binary_ops.push(binop_with_right(
        "+=",
        "n",
        Shape::literal("1"),
        point(5),
        1,
    ));
    assert!(js_str_001(&[unit(vec![numeric])]).is_empty());

    // Identifier `+=` (unknown runtime type) stays silent.
    let mut identifier = func("f");
    identifier
        .loops
        .push(loop_node(1, &["i"], None, loop_span()));
    identifier.binary_ops.push(binop("+=", "s", point(5), 1));
    assert!(js_str_001(&[unit(vec![identifier])]).is_empty());

    // `x = x + <numeric identifier>` is not a string accumulation either.
    let mut non_string_assign = func("f");
    non_string_assign
        .loops
        .push(loop_node(1, &["i"], None, loop_span()));
    non_string_assign.assignments.push(assign("s", point(5), 1));
    non_string_assign
        .binary_ops
        .push(binop("+", "s", point(5), 1));
    assert!(js_str_001(&[unit(vec![non_string_assign])]).is_empty());
}

#[test]
fn str_001_silent_on_tiny_bounded_loop() {
    let mut tiny = func("f");
    tiny.loops.push(loop_node(1, &["i"], Some(5), loop_span()));
    tiny.binary_ops.push(binop_with_right(
        "+=",
        "s",
        Shape::literal("\"a\""),
        point(5),
        1,
    ));
    assert!(js_str_001(&[unit(vec![tiny])]).is_empty());

    // The assignment form is tiny-gated too.
    let mut tiny_assign = func("f");
    tiny_assign
        .loops
        .push(loop_node(1, &["i"], Some(5), loop_span()));
    tiny_assign.assignments.push(assign("s", point(5), 1));
    tiny_assign.binary_ops.push(binop_with_right(
        "+",
        "s",
        Shape::literal("`a`"),
        point(5),
        1,
    ));
    assert!(js_str_001(&[unit(vec![tiny_assign])]).is_empty());
}

#[test]
fn str_001_silent_outside_any_loop() {
    let mut outside = func("f");
    outside.binary_ops.push(binop_with_right(
        "+=",
        "s",
        Shape::literal("\"a\""),
        point(5),
        0,
    ));
    assert!(js_str_001(&[unit(vec![outside])]).is_empty());

    let mut outside_assign = func("f");
    outside_assign.assignments.push(assign("s", point(5), 0));
    outside_assign.binary_ops.push(binop_with_right(
        "+",
        "s",
        Shape::literal("`a`"),
        point(5),
        0,
    ));
    assert!(js_str_001(&[unit(vec![outside_assign])]).is_empty());
}

// ---------------------------------------------------------------------------
// LOOP-001
// ---------------------------------------------------------------------------

#[test]
fn loop_001_fires_once_at_depth_two() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["i"], None, span(2, 1, 9, 1)));
    f.loops.push(loop_node(2, &["j"], None, span(3, 1, 8, 1)));

    let out = js_loop_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_LOOP_001_RULE_ID);
    assert_eq!(out[0].span.start_line, 3);
    assert!(out[0].message.contains('2'));
}

#[test]
fn loop_001_silent_at_depth_one() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["i"], None, loop_span()));
    assert!(js_loop_001(&[unit(vec![f])]).is_empty());
}

#[test]
fn loop_001_max_depth_is_scoped_to_the_anchored_nest() {
    let mut f = func("f");
    // Earliest depth-2 nest (anchored): lines 2-9.
    f.loops.push(loop_node(1, &["i"], None, span(2, 1, 9, 1)));
    f.loops.push(loop_node(2, &["j"], None, span(3, 1, 8, 1)));
    // A separate, deeper nest later in the same function (lines 20-30) must
    // NOT inflate the anchored nest's reported max depth.
    f.loops.push(loop_node(1, &["a"], None, span(20, 1, 30, 1)));
    f.loops.push(loop_node(2, &["b"], None, span(21, 1, 29, 1)));
    f.loops.push(loop_node(3, &["c"], None, span(22, 1, 28, 1)));

    let out = js_loop_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(
        out[0].span.start_line, 3,
        "anchor is the earliest depth-2 loop"
    );
    assert_eq!(
        out[0].evidence.get("max_depth").map(String::as_str),
        Some("2"),
        "max_depth counts only loops contained in the anchored loop span"
    );
}

// ---------------------------------------------------------------------------
// LOOP-002
// ---------------------------------------------------------------------------

#[test]
fn loop_002_fires_on_sync_io_in_loop() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["p"], None, loop_span()));
    f.calls.push(call("readFileSync", Some("fs"), point(5), 1));

    let out = js_loop_002(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_LOOP_002_RULE_ID);
    assert_eq!(out[0].severity, Severity::Warn);
}

#[test]
fn loop_002_silent_outside_loop() {
    let mut f = func("f");
    f.calls.push(call("readFileSync", Some("fs"), point(5), 0));
    assert!(js_loop_002(&[unit(vec![f])]).is_empty());
}

// ---------------------------------------------------------------------------
// LOOP-007
// ---------------------------------------------------------------------------

#[test]
fn loop_007_fires_on_membership_scan_in_loop() {
    let mut f = func("f");
    f.loops.push(loop_node(1, &["x"], None, loop_span()));
    f.calls.push(call("includes", Some("seen"), point(5), 1));

    let out = js_loop_007(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_LOOP_007_RULE_ID);
    assert_eq!(out[0].severity, Severity::Warn);
}

#[test]
fn loop_007_silent_on_small_loop_or_missing_receiver() {
    // Small collection: trip_count < 16.
    let mut small = func("f");
    small
        .loops
        .push(loop_node(1, &["i"], Some(10), loop_span()));
    small.calls.push(call("indexOf", Some("xs"), point(5), 1));
    assert!(js_loop_007(&[unit(vec![small])]).is_empty());

    // No receiver root.
    let mut bare = func("f");
    bare.loops.push(loop_node(1, &["x"], None, loop_span()));
    bare.calls.push(call("find", None, point(5), 1));
    assert!(js_loop_007(&[unit(vec![bare])]).is_empty());
}

// ---------------------------------------------------------------------------
// ASYNC-001 / ASYNC-002
// ---------------------------------------------------------------------------

#[test]
fn async_001_fires_on_await_in_loop_only_in_async_fn() {
    let mut f = func("f");
    f.record.is_async = true;
    f.loops.push(loop_node(1, &["x"], None, loop_span()));
    f.awaits.push(await_event(point(5), 1));
    assert_eq!(js_async_001(&[unit(vec![f])]).len(), 1);

    let mut sync_fn = func("f");
    sync_fn.loops.push(loop_node(1, &["x"], None, loop_span()));
    sync_fn.awaits.push(await_event(point(5), 1));
    assert!(js_async_001(&[unit(vec![sync_fn])]).is_empty());

    let mut outside = func("f");
    outside.record.is_async = true;
    outside.awaits.push(await_event(point(5), 0));
    assert!(js_async_001(&[unit(vec![outside])]).is_empty());
}

#[test]
fn async_002_fires_on_sync_call_in_async_fn_without_loop() {
    let mut f = func("f");
    f.record.is_async = true;
    f.calls
        .push(call("execSync", Some("child_process"), point(5), 0));
    assert_eq!(js_async_002(&[unit(vec![f])]).len(), 1);

    let mut sync_fn = func("f");
    sync_fn
        .calls
        .push(call("execSync", Some("child_process"), point(5), 0));
    assert!(js_async_002(&[unit(vec![sync_fn])]).is_empty());
}

// ---------------------------------------------------------------------------
// CALL-001
// ---------------------------------------------------------------------------

#[test]
fn call_001_fires_on_unguarded_direct_recursion() {
    let mut f = func("walk");
    f.calls.push(call("walk", None, point(5), 0));

    let out = js_call_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_CALL_001_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
}

#[test]
fn call_001_silent_when_guarded_or_member_call() {
    // A comparison strictly before the recursive call is a visible base case.
    let mut guarded = func("walk");
    guarded.comparisons.push(comparison(point(3), 0));
    guarded.calls.push(call("walk", None, point(5), 0));
    assert!(js_call_001(&[unit(vec![guarded])]).is_empty());

    // `this.walk()` / member self-calls carry a receiver and are excluded.
    let mut member = func("walk");
    member.calls.push(call("walk", Some("this"), point(5), 0));
    assert!(js_call_001(&[unit(vec![member])]).is_empty());
}

// ---------------------------------------------------------------------------
// ARCH-001
// ---------------------------------------------------------------------------

#[test]
fn arch_001_fires_on_import_cycle() {
    let a = file_unit("a.js", vec![edge("a.js", "b.js")], Vec::new());
    let b = file_unit("b.js", vec![edge("b.js", "a.js")], Vec::new());

    let out = js_arch_001(&[a, b]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_ARCH_001_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
}

#[test]
fn arch_001_silent_without_cycle_or_when_unresolved() {
    let a = file_unit("a.js", vec![edge("a.js", "b.js")], Vec::new());
    let b = file_unit("b.js", Vec::new(), Vec::new());
    assert!(js_arch_001(&[a, b]).is_empty());

    let mut unresolved = edge("a.js", "b.js");
    unresolved.resolved = false;
    let a = file_unit("a.js", vec![unresolved.clone()], Vec::new());
    let mut back = edge("b.js", "a.js");
    back.resolved = false;
    let b = file_unit("b.js", vec![back], Vec::new());
    assert!(js_arch_001(&[a, b]).is_empty());
}

// ---------------------------------------------------------------------------
// TYPE-001
// ---------------------------------------------------------------------------

#[test]
fn type_001_fires_on_member_delete() {
    let mut f = func("f");
    f.unaries.push(unary(
        "delete",
        Shape {
            kind: ShapeKind::MemberAccess,
            name: Some("prop".to_string()),
            ..Shape::default()
        },
        point(4),
    ));

    let out = js_type_001(&[unit(vec![f])]);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].rule_id, JS_TYPE_001_RULE_ID);
    assert_eq!(out[0].severity, Severity::Info);
    assert_eq!(
        out[0].evidence.get("property").map(String::as_str),
        Some("prop"),
        "TYPE-001 must carry the deleted property name as evidence"
    );
}

#[test]
fn type_001_silent_on_subscript_or_identifier_delete() {
    // Keyed form (`delete obj["k"]`) is a different operation.
    let mut subscript = func("f");
    subscript.unaries.push(unary(
        "delete",
        Shape {
            kind: ShapeKind::Subscript,
            index_ident: Some("k".to_string()),
            ..Shape::default()
        },
        point(4),
    ));
    assert!(js_type_001(&[unit(vec![subscript])]).is_empty());

    // A bare identifier (`delete x`) has no property to shadow; only a member
    // access is the hidden-class shape.
    let mut identifier = func("f");
    identifier
        .unaries
        .push(unary("delete", Shape::identifier("x"), point(4)));
    assert!(js_type_001(&[unit(vec![identifier])]).is_empty());
}

// ---------------------------------------------------------------------------
// Frozen catalog absence
// ---------------------------------------------------------------------------

#[test]
fn loop_003_and_ts_rules_are_not_registered() {
    let ids: Vec<&str> = crate::analyzer::registered_rule_ids().collect();
    assert!(
        !ids.contains(&"PERF-JS-LOOP-003"),
        "PERF-JS-LOOP-003 is CUT for v1"
    );
    assert!(
        !ids.iter().any(|id| id.starts_with("PERF-TS-")),
        "v1 ships zero PERF-TS-* rules"
    );
    for id in [
        JS_ALLOC_001_RULE_ID,
        JS_ALLOC_003_RULE_ID,
        JS_ARCH_001_RULE_ID,
        JS_ASYNC_001_RULE_ID,
        JS_ASYNC_002_RULE_ID,
        JS_CALL_001_RULE_ID,
        JS_LOOP_001_RULE_ID,
        JS_LOOP_002_RULE_ID,
        JS_LOOP_007_RULE_ID,
        JS_STR_001_RULE_ID,
        JS_TYPE_001_RULE_ID,
    ] {
        assert!(ids.contains(&id), "{id} must be registered");
    }
}
