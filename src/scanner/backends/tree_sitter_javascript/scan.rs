//! JavaScript backend event engine (plan `js-ts`, wave B2).
//!
//! One body walk produces events and complexity metrics with shared traversal
//! state (loop depth, nesting), mirroring the Python backend's `FuncScan`
//! strategy for the `tree-sitter-javascript` node kinds:
//!
//! - `for_statement` / `for_in_statement` / `while_statement` / `do_statement`
//!   ⇒ [`LoopNode`]; both `for...of` and `for...in` map to `LoopKind::For` and
//!   the grammar's `of`/`in` operator token is NOT stored (`PERF-JS-LOOP-003`
//!   was cut in the B2-W0 canary, so the distinction has no consumer);
//! - loop headers (init/condition/increment/iterable) are walked at the
//!   ENCLOSING depth — they are not body statements (same convention as
//!   `tools/js_canary.rs` and the Python backend's `for`-iterable fix);
//! - function and class nodes are scope boundaries: their bodies belong to
//!   their own records, never to the enclosing function's event stream.

use tree_sitter::Node;

use super::infer;
use super::{ir_span, text};
use crate::ir::{
    AccessChainEvent, AllocEvent, AllocKind, AssignmentEvent, AwaitEvent, BinaryOpEvent,
    BindingKind, CallEvent, CalleeKind, ComparisonEvent, EventMeta, LoopKind, LoopNode,
    ResolverTier, ScopeClass, Shape, ShapeKind, StatementRole, TryEvent, UnaryEvent,
};

/// Loop-bearing node kinds (ancestor climbing for `is_innermost`, and the
/// visitor's loop arms).
const LOOP_KINDS: [&str; 4] = [
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
];

/// Comparison operators that produce a [`ComparisonEvent`] (plan `js-ts`).
const COMPARISON_OPERATORS: [&str; 10] = [
    "==",
    "!=",
    "===",
    "!==",
    "<",
    "<=",
    ">",
    ">=",
    "in",
    "instanceof",
];

/// Everything one function/module body yields.
pub(super) struct ScanResult {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub max_loop_depth: u32,
    pub alloc_site_count: u32,
    pub calls: Vec<CallEvent>,
    pub assignments: Vec<AssignmentEvent>,
    pub binary_ops: Vec<BinaryOpEvent>,
    pub comparisons: Vec<ComparisonEvent>,
    pub access_chains: Vec<AccessChainEvent>,
    pub loops: Vec<LoopNode>,
    pub awaits: Vec<AwaitEvent>,
    pub try_blocks: Vec<TryEvent>,
    pub allocations: Vec<AllocEvent>,
    pub unaries: Vec<UnaryEvent>,
}

/// Scan-local scope facts (built once in `mod.rs`).
pub(super) struct ScanInputs<'a> {
    pub source: &'a [u8],
    pub param_names: &'a [String],
    pub local_names: &'a [String],
}

/// Walk `start` (a function body or the `program` root) emitting events and
/// metrics. Never panics: every node access is optional.
pub(super) fn scan(start: Node<'_>, inputs: ScanInputs<'_>) -> ScanResult {
    let mut state = FuncScan {
        source: inputs.source,
        param_names: inputs.param_names,
        local_names: inputs.local_names,
        loop_depth: 0,
        max_loop_depth: 0,
        nesting: 0,
        cyclomatic: 1,
        cognitive: 0,
        alloc_site_count: 0,
        calls: Vec::new(),
        assignments: Vec::new(),
        binary_ops: Vec::new(),
        comparisons: Vec::new(),
        access_chains: Vec::new(),
        loops: Vec::new(),
        awaits: Vec::new(),
        try_blocks: Vec::new(),
        allocations: Vec::new(),
        unaries: Vec::new(),
    };
    state.visit(start);
    ScanResult {
        cyclomatic: state.cyclomatic,
        cognitive: state.cognitive,
        max_loop_depth: state.max_loop_depth,
        alloc_site_count: state.alloc_site_count,
        calls: state.calls,
        assignments: state.assignments,
        binary_ops: state.binary_ops,
        comparisons: state.comparisons,
        access_chains: state.access_chains,
        loops: state.loops,
        awaits: state.awaits,
        try_blocks: state.try_blocks,
        allocations: state.allocations,
        unaries: state.unaries,
    }
}

struct FuncScan<'a> {
    source: &'a [u8],
    param_names: &'a [String],
    local_names: &'a [String],
    loop_depth: u32,
    max_loop_depth: u32,
    nesting: u32,
    cyclomatic: u32,
    cognitive: u32,
    alloc_site_count: u32,
    calls: Vec<CallEvent>,
    assignments: Vec<AssignmentEvent>,
    binary_ops: Vec<BinaryOpEvent>,
    comparisons: Vec<ComparisonEvent>,
    access_chains: Vec<AccessChainEvent>,
    loops: Vec<LoopNode>,
    awaits: Vec<AwaitEvent>,
    try_blocks: Vec<TryEvent>,
    allocations: Vec<AllocEvent>,
    unaries: Vec<UnaryEvent>,
}

impl FuncScan<'_> {
    fn visit(&mut self, node: Node<'_>) {
        match node.kind() {
            // Branch nodes mirror the Python backend's decision-point policy:
            // `if`/`catch`/`switch_case` add nesting-weighted cognitive load;
            // `conditional_expression` and logical `&&`/`||` add flat load.
            "if_statement" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "catch_clause" | "switch_case" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "conditional_expression" => {
                self.cyclomatic += 1;
                self.cognitive += 1;
                self.visit_children(node);
            }
            "binary_expression" => self.visit_binary(node),
            "for_statement" => self.enter_loop(node, LoopKind::For),
            "for_in_statement" => self.enter_loop(node, LoopKind::For),
            "while_statement" => self.enter_loop(node, LoopKind::While),
            "do_statement" => self.enter_loop(node, LoopKind::Do),
            "call_expression" => {
                self.emit_call(node);
                self.visit_children(node);
            }
            "new_expression" => {
                self.emit_new(node);
                self.visit_children(node);
            }
            // One additive `AwaitEvent` per `await_expression`, then descend so
            // the awaited expression's own events are emitted normally.
            "await_expression" => {
                self.awaits.push(AwaitEvent {
                    meta: self.meta_for(node),
                });
                self.visit_children(node);
            }
            // One additive `TryEvent` per `try_statement`; `catch_clause`
            // already contributes the branch complexity above.
            "try_statement" => {
                self.try_blocks.push(TryEvent {
                    meta: self.meta_for(node),
                });
                self.visit_children(node);
            }
            "assignment_expression" => {
                self.emit_assignment(node);
                self.visit_children(node);
            }
            "augmented_assignment_expression" => {
                self.emit_augmented_binop(node);
                self.visit_children(node);
            }
            "unary_expression" => {
                self.maybe_emit_delete(node);
                self.visit_children(node);
            }
            "member_expression" | "subscript_expression" => {
                self.maybe_emit_access_chain(node);
                self.visit_children(node);
            }
            "object" | "array" => {
                let is_array = node.kind() == "array";
                let kind = if is_array {
                    AllocKind::Array
                } else {
                    AllocKind::Object
                };
                self.allocations.push(AllocEvent {
                    meta: self.meta_for(node),
                    kind,
                    referenced_names: infer::collect_identifier_names(node, self.source),
                    has_spread: is_array && array_has_spread(node),
                });
                self.alloc_site_count += 1;
                self.visit_children(node);
            }
            // Scope boundaries: nested functions/classes are separate records
            // (or file-level records) — their bodies are never attributed to
            // the enclosing scope.
            k if infer::is_function_kind(k) || infer::is_class_kind(k) => {}
            _ => self.visit_children(node),
        }
    }

    fn visit_children(&mut self, node: Node<'_>) {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn enter_loop(&mut self, node: Node<'_>, kind: LoopKind) {
        let depth = self.loop_depth + 1;
        self.max_loop_depth = self.max_loop_depth.max(depth);
        self.cyclomatic += 1;
        self.cognitive += 1 + self.nesting;

        let span = ir_span(node, self.source);
        let induction_vars = match node.kind() {
            "for_statement" => for_init_names(node, self.source),
            "for_in_statement" => node
                .child_by_field_name("left")
                .map(|left| infer::pattern_names(left, self.source))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        // Only the canonical counted `for` exposes a static trip count; every
        // `for...of`/`for...in`/while/do shape is honestly unknown.
        let trip_count = if node.kind() == "for_statement" {
            canonical_for_trip_count(node, self.source)
        } else {
            None
        };
        self.loops.push(LoopNode {
            kind,
            depth,
            induction_vars,
            span,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
            trip_count,
            // `do { ... } while (0)` is not a JS idiom; the C-only wrapper
            // gate never fires.
            condition_is_zero: false,
        });

        // Headers run in the ENCLOSING context (not once per body iteration
        // for the events we model; the `for` initializer is evaluated once).
        // A `do ... while (cond)` condition is walked AFTER the body below so
        // every event vector stays in source order.
        match node.kind() {
            "for_statement" => {
                for field in ["initializer", "condition", "increment"] {
                    if let Some(child) = node.child_by_field_name(field) {
                        self.visit(child);
                    }
                }
            }
            "for_in_statement" => {
                if let Some(left) = node.child_by_field_name("left") {
                    self.visit(left);
                }
                if let Some(right) = node.child_by_field_name("right") {
                    self.visit(right);
                }
            }
            "while_statement" => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    self.visit(condition);
                }
            }
            _ => {}
        }

        self.loop_depth = depth;
        self.nesting += 1;
        if let Some(body) = node.child_by_field_name("body") {
            self.visit(body);
        }
        self.nesting -= 1;
        self.loop_depth -= 1;

        // `do` conditions come after the body in source order.
        if node.kind() == "do_statement"
            && let Some(condition) = node.child_by_field_name("condition")
        {
            self.visit(condition);
        }
    }

    fn visit_binary(&mut self, node: Node<'_>) {
        let op = node
            .child_by_field_name("operator")
            .map(|o| text(o, self.source))
            .unwrap_or("");
        if matches!(op, "&&" | "||") {
            self.cyclomatic += 1;
            self.cognitive += 1;
        }
        let left = node.child_by_field_name("left");
        let right = node.child_by_field_name("right");
        if let (Some(left), Some(right)) = (left, right) {
            let meta = self.meta_for(node);
            if COMPARISON_OPERATORS.contains(&op) {
                self.comparisons.push(ComparisonEvent {
                    meta,
                    operator: op.to_string(),
                    left: infer::shape_of_node(left, self.source),
                    right: infer::shape_of_node(right, self.source),
                });
            } else {
                self.binary_ops.push(BinaryOpEvent {
                    meta,
                    op: op.to_string(),
                    left: infer::shape_of_node(left, self.source),
                    right: infer::shape_of_node(right, self.source),
                });
            }
        }
        self.visit_children(node);
    }

    fn emit_call(&mut self, node: Node<'_>) {
        let (callee_name, callee_kind, callee_path, receiver_root) = self.callee_parts(node);
        let args = self.call_args(node);
        let (statement_role, conditional_guarded) = self.call_position(node);
        self.calls.push(CallEvent {
            meta: self.meta_for(node),
            callee_name,
            callee_kind,
            callee_path,
            is_macro: false,
            args,
            statement_role,
            conditional_guarded,
            semantic: None,
            receiver_root,
        });
    }

    /// `new` emits BOTH a constructor [`CallEvent`] and an [`AllocEvent`], and
    /// counts once toward `alloc_site_count`.
    fn emit_new(&mut self, node: Node<'_>) {
        let (callee_name, callee_kind, callee_path, receiver_root) = self.callee_parts(node);
        let args = self.call_args(node);
        let (statement_role, conditional_guarded) = self.call_position(node);
        self.calls.push(CallEvent {
            meta: self.meta_for(node),
            callee_name,
            callee_kind,
            callee_path,
            is_macro: false,
            args,
            statement_role,
            conditional_guarded,
            semantic: None,
            receiver_root,
        });
        let referenced_names = node
            .child_by_field_name("arguments")
            .map(|arguments| infer::collect_identifier_names(arguments, self.source))
            .unwrap_or_default();
        self.allocations.push(AllocEvent {
            meta: self.meta_for(node),
            kind: AllocKind::New,
            referenced_names,
            has_spread: false,
        });
        self.alloc_site_count += 1;
    }

    /// Shared `call_expression`/`new_expression` callee decomposition.
    fn callee_parts(&self, node: Node<'_>) -> (String, CalleeKind, Option<String>, Option<String>) {
        let field = if node.kind() == "new_expression" {
            "constructor"
        } else {
            "function"
        };
        let Some(callee) = node.child_by_field_name(field) else {
            return (String::new(), CalleeKind::Unknown, None, None);
        };
        let kind = match callee.kind() {
            "identifier" | "member_expression" => CalleeKind::Direct,
            "subscript_expression" => CalleeKind::Indirect,
            _ => CalleeKind::Unknown,
        };
        let name = infer::final_segment_of(callee, self.source);
        let path = text(callee, self.source).trim().to_string();
        let receiver_root = match callee.kind() {
            "member_expression" | "subscript_expression" => {
                infer::receiver_root_of(callee, self.source)
            }
            _ => None,
        };
        (name, kind, Some(path), receiver_root)
    }

    fn call_args(&self, node: Node<'_>) -> Vec<Shape> {
        let Some(arguments) = node.child_by_field_name("arguments") else {
            return Vec::new();
        };
        // Tagged templates (`tag\`...\``) are the only non-`arguments` shape.
        if arguments.kind() == "template_string" {
            return vec![infer::shape_of_node(arguments, self.source)];
        }
        let mut shapes = Vec::new();
        let mut cursor = arguments.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    shapes.push(infer::shape_of_node(child, self.source));
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        shapes
    }

    /// A call is `Primary` only when it IS the entire expression statement;
    /// every other position is `Nested`.
    fn call_position(&self, node: Node<'_>) -> (StatementRole, bool) {
        let role = if node.parent().is_some_and(|parent| {
            parent.kind() == "expression_statement"
                && parent.named_child_count() == 1
                && parent
                    .named_child(0)
                    .is_some_and(|child| child.id() == node.id())
        }) {
            StatementRole::Primary
        } else {
            StatementRole::Nested
        };
        (role, conditional_guarded(node))
    }

    fn emit_assignment(&mut self, node: Node<'_>) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let target = infer::shape_of_node(left, self.source);
        let scope_class = self.classify_target(&target);
        self.assignments.push(AssignmentEvent {
            meta: self.meta_for(node),
            target,
            scope_class,
            binding_kind: BindingKind::Unknown,
        });
    }

    fn classify_target(&self, target: &Shape) -> ScopeClass {
        match (&target.kind, target.name.as_deref()) {
            (ShapeKind::Identifier, Some(name)) if self.param_names.iter().any(|p| p == name) => {
                ScopeClass::Param
            }
            (ShapeKind::Identifier, Some(name)) if self.local_names.iter().any(|l| l == name) => {
                ScopeClass::Local
            }
            _ => ScopeClass::Unknown,
        }
    }

    /// `augmented_assignment_expression` (`s += x`) → a [`BinaryOpEvent`] with
    /// the operator text and both operand shapes (STR-001's in-place signal).
    fn emit_augmented_binop(&mut self, node: Node<'_>) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let Some(right) = node.child_by_field_name("right") else {
            return;
        };
        let op = node
            .child_by_field_name("operator")
            .map(|o| text(o, self.source))
            .unwrap_or("")
            .to_string();
        self.binary_ops.push(BinaryOpEvent {
            meta: self.meta_for(node),
            op,
            left: infer::shape_of_node(left, self.source),
            right: infer::shape_of_node(right, self.source),
        });
    }

    /// `delete obj.prop` → `UnaryEvent { op: "delete", .. }`. No other unary
    /// operator is a v1 input.
    fn maybe_emit_delete(&mut self, node: Node<'_>) {
        let operator = node
            .child_by_field_name("operator")
            .map(|o| text(o, self.source))
            .unwrap_or("");
        if operator != "delete" {
            return;
        }
        let Some(argument) = node.child_by_field_name("argument") else {
            return;
        };
        self.unaries.push(UnaryEvent {
            meta: self.meta_for(node),
            op: "delete".to_string(),
            operand: infer::shape_of_node(argument, self.source),
        });
    }

    fn maybe_emit_access_chain(&mut self, node: Node<'_>) {
        // Only the OUTERMOST chain node of a member/subscript nest emits, so
        // `a.b.c` produces one event, never one per intermediate chain.
        if let Some(parent) = node.parent()
            && matches!(parent.kind(), "member_expression" | "subscript_expression")
            && parent
                .child_by_field_name("object")
                .is_some_and(|object| object.id() == node.id())
        {
            return;
        }
        let Some((root, elements)) = self.access_chain(node) else {
            return;
        };
        if elements.is_empty() {
            return;
        }
        self.access_chains.push(AccessChainEvent {
            meta: self.meta_for(node),
            root,
            elements,
        });
    }

    /// Decompose `a.b[i].c` into (root identifier, element shapes).
    fn access_chain(&self, node: Node<'_>) -> Option<(String, Vec<Shape>)> {
        match node.kind() {
            "subscript_expression" => {
                let base = node.child_by_field_name("object")?;
                let (root, mut elements) = self.access_chain(base)?;
                let index_ident = node
                    .child_by_field_name("index")
                    .and_then(|index| infer::leftmost_identifier(index, self.source));
                elements.push(Shape {
                    kind: ShapeKind::Subscript,
                    index_ident,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            "member_expression" => {
                let base = node.child_by_field_name("object")?;
                let (root, mut elements) = self.access_chain(base)?;
                let name = node
                    .child_by_field_name("property")
                    .map(|property| text(property, self.source).to_string());
                elements.push(Shape {
                    kind: ShapeKind::MemberAccess,
                    name,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            _ => infer::leftmost_identifier(node, self.source).map(|root| (root, Vec::new())),
        }
    }

    fn meta_for(&self, node: Node<'_>) -> EventMeta {
        EventMeta {
            span: ir_span(node, self.source),
            in_loop_depth: self.loop_depth,
            is_innermost: !in_loop_header(node) && is_innermost_event(node),
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        }
    }
}

/// Whether an `array` literal has any `spread_element` child (`[...acc, x]`).
/// Object literals never carry a spread element in this grammar, so callers
/// only ask about arrays.
fn array_has_spread(node: Node<'_>) -> bool {
    (0..node.named_child_count() as u32).any(|i| {
        node.named_child(i)
            .is_some_and(|child| child.kind() == "spread_element")
    })
}

/// Names bound by a `for_statement` initializer (`for (let i = 0; ...)`,
/// `for (i = 0; ...)`).
fn for_init_names(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(initializer) = node.child_by_field_name("initializer") else {
        return Vec::new();
    };
    match initializer.kind() {
        "lexical_declaration" | "variable_declaration" => {
            let mut names = Vec::new();
            let mut cursor = initializer.walk();
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.kind() == "variable_declarator"
                        && let Some(name) = child.child_by_field_name("name")
                    {
                        names.extend(infer::pattern_names(name, source));
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            names
        }
        "assignment_expression" => initializer
            .child_by_field_name("left")
            .map(|left| infer::pattern_names(left, source))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Canonical counted-`for` trip count (plan `js-ts`).
///
/// Admitted shapes ONLY:
/// - initializer: one `variable_declarator` (or assignment) binding a single
///   identifier to the integer literal `0`;
/// - condition: `<identifier> < N` or `<identifier> <= N`, same identifier, `N`
///   a non-negative integer literal;
/// - increment: `i++` on the same identifier.
///
/// `for…of`/`for…in`, non-zero starts, non-`i++` increments, multi-declarator
/// initializers, variable bounds, and wrapped operands all report `None` —
/// honest unknown, never a guess. Clamped to [`crate::ir::MAX_TRIP`].
fn canonical_for_trip_count(node: Node<'_>, source: &[u8]) -> Option<u64> {
    let initializer = node.child_by_field_name("initializer")?;
    let induction = canonical_zero_init_identifier(initializer, source)?;

    let condition = node.child_by_field_name("condition")?;
    if condition.kind() != "binary_expression" {
        return None;
    }
    let operator = condition
        .child_by_field_name("operator")
        .map(|o| text(o, source))
        .unwrap_or("");
    if operator != "<" && operator != "<=" {
        return None;
    }
    let left = condition.child_by_field_name("left")?;
    if left.kind() != "identifier" || text(left, source) != induction {
        return None;
    }
    let right = condition.child_by_field_name("right")?;
    let bound = js_int_literal_value(right, source)?;
    if bound < 0 {
        return None;
    }

    let increment = node.child_by_field_name("increment")?;
    if increment.kind() != "update_expression" {
        return None;
    }
    if text(
        increment
            .child_by_field_name("operator")
            .unwrap_or(increment),
        source,
    ) != "++"
    {
        return None;
    }
    let argument = increment.child_by_field_name("argument")?;
    if argument.kind() != "identifier" || text(argument, source) != induction {
        return None;
    }

    let trips = if operator == "<" {
        bound
    } else {
        bound.saturating_add(1)
    };
    let trips = u64::try_from(trips).ok()?;
    Some(trips.min(crate::ir::MAX_TRIP))
}

/// The identifier a canonical zero-initializer binds.
fn canonical_zero_init_identifier(initializer: Node<'_>, source: &[u8]) -> Option<String> {
    match initializer.kind() {
        "lexical_declaration" | "variable_declaration" => {
            let mut declarators = Vec::new();
            let mut cursor = initializer.walk();
            if cursor.goto_first_child() {
                loop {
                    if cursor.node().kind() == "variable_declarator" {
                        declarators.push(cursor.node());
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            // Multi-declarator initializers are not the canonical single-var shape.
            let [declarator] = declarators.as_slice() else {
                return None;
            };
            let name = declarator.child_by_field_name("name")?;
            if name.kind() != "identifier" {
                return None;
            }
            let value = declarator.child_by_field_name("value")?;
            if js_int_literal_value(value, source)? != 0 {
                return None;
            }
            Some(text(name, source).to_string())
        }
        "assignment_expression" => {
            let left = initializer.child_by_field_name("left")?;
            if left.kind() != "identifier" {
                return None;
            }
            let right = initializer.child_by_field_name("right")?;
            if js_int_literal_value(right, source)? != 0 {
                return None;
            }
            Some(text(left, source).to_string())
        }
        _ => None,
    }
}

/// Integer value of a `number` node, if it is an integer literal.
fn js_int_literal_value(node: Node<'_>, source: &[u8]) -> Option<i128> {
    if node.kind() != "number" {
        return None;
    }
    parse_js_int(text(node, source))
}

/// Parse a JavaScript numeric literal as an integer. Accepts decimal and
/// `0x`/`0o`/`0b` radixes with `_` separators and an optional BigInt `n`
/// suffix; rejects floats, exponents, legacy-octal forms (`010`), and any
/// other expression (radix-aware, so a hex `e` digit is valid while a decimal
/// `e` exponent is not). A bare `0` (and `0n`) stays `0`.
fn parse_js_int(raw: &str) -> Option<i128> {
    let trimmed = raw.trim();
    let lower = trimmed
        .strip_suffix('n')
        .unwrap_or(trimmed)
        .to_ascii_lowercase();
    let has_radix_prefix =
        lower.starts_with("0x") || lower.starts_with("0o") || lower.starts_with("0b");
    if !has_radix_prefix {
        let body = trimmed.strip_suffix('n').unwrap_or(trimmed);
        // Legacy octal (`010`) is a different runtime value; never guess it as
        // decimal. `0` itself (length 1) is the only leading-zero literal kept.
        if body.len() > 1 && body.starts_with('0') {
            return None;
        }
    }
    let (radix, digits) = if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        (16u32, rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        (8u32, rest)
    } else if let Some(rest) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        (2u32, rest)
    } else {
        (10u32, trimmed)
    };
    let digits = digits.strip_suffix('n').unwrap_or(digits);
    if digits.is_empty() {
        return None;
    }
    let mut value: i128 = 0;
    let mut seen_digit = false;
    for ch in digits.chars() {
        if ch == '_' {
            continue;
        }
        let digit = ch.to_digit(radix)?;
        value = value
            .saturating_mul(i128::from(radix))
            .saturating_add(i128::from(digit));
        seen_digit = true;
    }
    if seen_digit { Some(value) } else { None }
}

/// A call is conditionally guarded when an enclosing `if`/ternary/`catch` or
/// logical `&&`/`||` sits between it and the function boundary.
fn conditional_guarded(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        let kind = parent.kind();
        if infer::is_function_kind(kind) || infer::is_class_kind(kind) {
            return false;
        }
        match kind {
            "if_statement" | "conditional_expression" | "catch_clause" => return true,
            "binary_expression" => {
                // The operator token's kind IS its source text in this grammar.
                let operator = parent
                    .child_by_field_name("operator")
                    .map(|o| o.kind())
                    .unwrap_or("");
                if matches!(operator, "&&" | "||") {
                    return true;
                }
            }
            _ => {}
        }
        current = parent.parent();
    }
    false
}

/// Nearest loop-bearing ancestor of `event` within the SAME function/module
/// scope. Nested function and class bodies are scope boundaries: a loop that
/// encloses the function declaration does not enclose the function's own
/// execution (same rule as [`contains_loop`]).
fn nearest_enclosing_loop(event: Node<'_>) -> Option<Node<'_>> {
    let mut current = event.parent();
    while let Some(node) = current {
        if infer::is_function_kind(node.kind()) || infer::is_class_kind(node.kind()) {
            return None;
        }
        if LOOP_KINDS.contains(&node.kind()) {
            return Some(node);
        }
        current = node.parent();
    }
    None
}

/// `is_innermost` operational rule (ir-v1.md §3.3): the nearest enclosing loop
/// exists and nests no further loop in its own body. Nested function/class
/// bodies are skipped — their loops execute on a different schedule.
fn is_innermost_event(event: Node<'_>) -> bool {
    let Some(loop_node) = nearest_enclosing_loop(event) else {
        return false;
    };
    let Some(body) = loop_node.child_by_field_name("body") else {
        return false;
    };
    !contains_loop(body)
}

/// Whether `node` lies in a loop HEADER (init/condition/increment/iterable)
/// rather than its body. Header events are evaluated at the enclosing depth and
/// are never innermost body events. Function/class boundaries stop the climb:
/// a nested function body never inherits an enclosing loop's header context.
fn in_loop_header(node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if infer::is_function_kind(parent.kind()) || infer::is_class_kind(parent.kind()) {
            return false;
        }
        if LOOP_KINDS.contains(&parent.kind()) {
            return parent
                .child_by_field_name("body")
                .is_none_or(|body| body.id() != current.id());
        }
        current = parent;
    }
    false
}

fn contains_loop(node: Node<'_>) -> bool {
    if LOOP_KINDS.contains(&node.kind()) {
        return true;
    }
    if infer::is_function_kind(node.kind()) || infer::is_class_kind(node.kind()) {
        return false;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if contains_loop(cursor.node()) {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}
