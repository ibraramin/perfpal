//! Python backend event engine (plan `python-backend` W2).
//!
//! A single body walk produces events and complexity metrics with shared
//! traversal state (loop depth, nesting). It mirrors the Rust backend's
//! [`FuncScan`] strategy, adapted to Python node kinds:
//!
//! - `for_statement` / `while_statement` ⇒ [`LoopNode`];
//! - comprehensions ⇒ real [`LoopNode`]s (one `for_in_clause` = one nesting
//!   level) so loop-scoped rules fire inside them;
//! - a `for` iterable (`right` of `for_statement`/`for_in_clause`) is evaluated
//!   **once** at the clause's OUTER depth and skipped in the body walk
//!   (ADR-0007 §Consequences; mirrors Rust's `in_for_iterable`).

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::infer::{self, AliasMap};
use super::{ir_span, text};
use crate::ir::{
    AccessChainEvent, AssignmentEvent, AwaitEvent, BinaryOpEvent, BindingKind, CallEvent,
    CalleeKind, ComparisonEvent, EventMeta, LoopKind, LoopNode, ResolverTier, ScopeClass, Shape,
    ShapeKind, StatementRole, TryEvent,
};

/// Loop-bearing node kinds for ancestor climbing (`is_innermost` /
/// `contains_loop`). Comprehension containers count as loops here because the
/// `for_in_clause`s are siblings of the body, not ancestors.
const LOOP_KINDS: [&str; 7] = [
    "for_statement",
    "while_statement",
    "for_in_clause",
    "list_comprehension",
    "set_comprehension",
    "dictionary_comprehension",
    "generator_expression",
];

/// Curated Python object-construction family for `alloc_site_count` (best
/// effort, documented): container/type constructors plus the pandas/numpy
/// constructors the W3 `ALLOC`/`PANDAS` rules consume. Matched on the
/// alias-resolved final callee segment (so `pd.DataFrame` counts via
/// `DataFrame`). A generator expression / f-string is NOT a call and never
/// counts (ADR-0007 §9.2 ALLOC-001 honesty).
///
/// `pub(crate)` (not private) so the v1.4 instrument pass
/// ([`crate::scanner::instrument`]) classifies with the SAME curated set —
/// one source of truth, mirroring [`crate::ir::RUST_ALLOC_FAMILY`].
pub(crate) const PY_ALLOC_FAMILY: [&str; 13] = [
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
}

/// Walk `start` (a function/module body or the `module` root) emitting events
/// and metrics. Never panics: every node access is optional.
pub(super) fn scan(
    start: Node<'_>,
    source: &[u8],
    alias: &AliasMap,
    param_names: &[String],
    global_names: &[String],
    binding: &BTreeMap<String, BindingKind>,
) -> ScanResult {
    let mut state = FuncScan {
        source,
        alias,
        param_names,
        global_names,
        binding,
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
    }
}

struct FuncScan<'a> {
    source: &'a [u8],
    alias: &'a AliasMap,
    param_names: &'a [String],
    global_names: &'a [String],
    binding: &'a BTreeMap<String, BindingKind>,
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
}

impl FuncScan<'_> {
    fn visit(&mut self, node: Node<'_>) {
        match node.kind() {
            "if_statement" | "elif_clause" | "except_clause" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "boolean_operator" | "conditional_expression" => {
                self.cyclomatic += 1;
                self.cognitive += 1;
                self.visit_children(node);
            }
            "for_statement" => self.enter_loop(node, LoopKind::For),
            "while_statement" => self.enter_loop(node, LoopKind::While),
            "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => self.enter_comprehension(node),
            // Defensive: a `for_in_clause` is only reachable via a comprehension
            // container, which `enter_comprehension` owns; handle a stray one as
            // a single-level loop rather than descending blindly.
            "for_in_clause" => self.enter_clause_sequence(&[node], None),
            "call" => {
                self.emit_call(node);
                self.visit_children(node);
            }
            "assignment" | "augmented_assignment" | "named_expression" => {
                self.emit_assignment(node);
                // `augmented_assignment` ALSO emits a `BinaryOpEvent` carrying
                // the operator text (`"+="`) plus the target/value shapes, so
                // STR-001 can see the in-place string-accumulation form that the
                // pure `a = a + b` binary shape misses. Assignment emission is
                // unchanged (plan `python-backend` W3).
                if node.kind() == "augmented_assignment" {
                    self.emit_augmented_binop(node);
                }
                self.visit_children(node);
            }
            "binary_operator" => self.visit_binary(node),
            "comparison_operator" => self.emit_comparisons(node),
            "attribute" | "subscript" => {
                self.maybe_emit_access_chain(node);
                self.visit_children(node);
            }
            // One additive `AwaitEvent` per NAMED `await` node (the anonymous
            // `await` keyword token shares the kind but is not named), and STILL
            // descend so the awaited expression's own events are emitted
            // normally (plan `python-backend` W2; feeds ASYNC-002).
            "await" if node.is_named() => {
                self.awaits.push(AwaitEvent {
                    meta: self.meta_for(node),
                });
                self.visit_children(node);
            }
            // One additive `TryEvent` per `try_statement`, then descend so any
            // nested loops/tries (and their events) are captured normally. The
            // event's `meta.in_loop_depth` marks a `try` inside a loop body
            // (plan `python-backend` W3; feeds LOOP-009). `try_statement` itself
            // contributes no complexity here — its `except_clause` children
            // already do, in parity with the C/Rust branch counting.
            "try_statement" => {
                self.try_blocks.push(TryEvent {
                    meta: self.meta_for(node),
                });
                self.visit_children(node);
            }
            // Scope boundary: nested functions/classes (and decorators applied to
            // them) are separate records / file-level events — their bodies are
            // never attributed to the enclosing scope.
            "function_definition" | "class_definition" | "decorated_definition" => {}
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

    fn visit_children_except(&mut self, node: Node<'_>, skip: Option<usize>) {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if Some(child.id()) != skip {
                    self.visit(child);
                }
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
        let induction_vars = if kind == LoopKind::For {
            node.child_by_field_name("left")
                .map(|l| pattern_identifiers(l, self.source))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        // A Python `for` iterable is evaluated once, before the body — visit it
        // at the OUTER depth and skip it in the body walk (same fix as Rust R4).
        // A `while` condition keeps the inner depth (re-evaluated each pass).
        let iterable = if kind == LoopKind::For {
            node.child_by_field_name("right")
        } else {
            None
        };
        // I2c: canonical `range(...)`-over-integer-literals iterables are the
        // only statically known Python trip counts; everything else stays
        // honestly unknown (`None`).
        let trip_count = if kind == LoopKind::For {
            iterable.and_then(|it| canonical_range_trip_count(it, self.source))
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
            // No do-while shape in Python: the `(0)`-wrapper gate never fires.
            condition_is_zero: false,
        });
        if let Some(it) = iterable {
            self.visit(it);
        }

        self.loop_depth = depth;
        self.nesting += 1;
        self.visit_children_except(node, iterable.map(|i| i.id()));
        self.nesting -= 1;
        self.loop_depth -= 1;
    }

    /// Comprehensions are real loops: each `for_in_clause` adds one nesting
    /// level; each clause's iterable is evaluated at that clause's outer depth;
    /// `if_clause` filters add complexity but no loop depth.
    fn enter_comprehension(&mut self, node: Node<'_>) {
        let mut clauses = Vec::new();
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if matches!(child.kind(), "for_in_clause" | "if_clause") {
                    clauses.push(child);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        let body = node.child_by_field_name("body");
        self.enter_clause_sequence(&clauses, body);
    }

    fn enter_clause_sequence(&mut self, clauses: &[Node<'_>], body: Option<Node<'_>>) {
        let base_depth = self.loop_depth;
        let base_nesting = self.nesting;
        let mut depth = base_depth;
        let mut nesting = base_nesting;
        for clause in clauses {
            match clause.kind() {
                "for_in_clause" => {
                    depth += 1;
                    self.max_loop_depth = self.max_loop_depth.max(depth);
                    self.cyclomatic += 1;
                    self.cognitive += 1 + nesting;
                    let induction_vars = clause
                        .child_by_field_name("left")
                        .map(|l| pattern_identifiers(l, self.source))
                        .unwrap_or_default();
                    let iterable = clause.child_by_field_name("right");
                    // I2c: comprehension clauses use the same canonical
                    // `range(...)` literal gate as statement loops.
                    let trip_count =
                        iterable.and_then(|it| canonical_range_trip_count(it, self.source));
                    self.loops.push(LoopNode {
                        kind: LoopKind::For,
                        depth,
                        induction_vars,
                        span: ir_span(*clause, self.source),
                        resolver_tier: ResolverTier::Syntactic,
                        macro_origin: false,
                        trip_count,
                        // Comprehension clause: never a `(0)`-wrapper.
                        condition_is_zero: false,
                    });
                    self.loop_depth = depth - 1;
                    self.nesting = nesting;
                    if let Some(it) = iterable {
                        self.visit(it);
                    }
                    self.visit_children_except(*clause, iterable.map(|i| i.id()));
                    nesting += 1;
                }
                "if_clause" => {
                    self.cyclomatic += 1;
                    self.cognitive += 1 + nesting;
                    self.loop_depth = depth;
                    self.visit_children(*clause);
                }
                _ => {}
            }
        }
        self.nesting = nesting;
        self.loop_depth = depth;
        if let Some(b) = body {
            self.visit(b);
        }
        self.nesting = base_nesting;
        self.loop_depth = base_depth;
    }

    fn visit_binary(&mut self, node: Node<'_>) {
        let op = node
            .child_by_field_name("operator")
            .map(|o| text(o, self.source))
            .unwrap_or("");
        let left = node.child_by_field_name("left").map(|n| self.shape_of(n));
        let right = node.child_by_field_name("right").map(|n| self.shape_of(n));
        if let (Some(left), Some(right)) = (left, right) {
            let meta = self.meta_for(node);
            self.binary_ops.push(BinaryOpEvent {
                meta,
                op: op.to_string(),
                left,
                right,
            });
        }
        self.visit_children(node);
    }

    /// Python chained comparisons (`a < b < c`) yield ONE event per adjacent
    /// operand pair; operands are the named expression children and there is no
    /// `operands` field (W1 §3.9). The operator TEXT between each pair (one or
    /// two anonymous tokens — `not in` / `is not`) is captured and normalized by
    /// joining its tokens with a single space, so LOOP-007 can read `"in"` /
    /// `"not in"` from the IR (plan `python-backend` W3).
    fn emit_comparisons(&mut self, node: Node<'_>) {
        let mut operands = Vec::new();
        let mut operators: Vec<String> = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    if !operands.is_empty() {
                        operators.push(pending.join(" "));
                    }
                    pending.clear();
                    operands.push(child);
                } else {
                    let token = text(child, self.source);
                    if !token.trim().is_empty() {
                        pending.push(token.to_string());
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        for (pair_idx, pair) in operands.windows(2).enumerate() {
            let meta = self.meta_for(node);
            self.comparisons.push(ComparisonEvent {
                meta,
                operator: operators.get(pair_idx).cloned().unwrap_or_default(),
                left: self.shape_of(pair[0]),
                right: self.shape_of(pair[1]),
            });
        }
        self.visit_children(node);
    }

    /// `augmented_assignment` (`s += x`) → a [`BinaryOpEvent`] whose `op` is the
    /// operator token text and whose operands are the target/value shapes. This
    /// is the additive signal STR-001 needs for the in-place accumulation form;
    /// the ordinary `a = a + b` shape is still covered by `binary_operator`
    /// (plan `python-backend` W3).
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
            left: self.shape_of(left),
            right: self.shape_of(right),
        });
    }

    fn emit_call(&mut self, node: Node<'_>) {
        let callee = node.child_by_field_name("function");
        let (callee_kind, callee_name, callee_path) = match callee {
            None => (CalleeKind::Unknown, String::new(), None),
            Some(c) => {
                let resolved = infer::apply_alias_first_segment(text(c, self.source), self.alias);
                let final_segment = resolved.rsplit('.').next().unwrap_or(resolved.as_str());
                if PY_ALLOC_FAMILY.contains(&final_segment) {
                    self.alloc_site_count += 1;
                }
                let kind = match c.kind() {
                    // A statically named callee — plain name OR member/method
                    // path (`x.method()`, `pd.concat(...)`) — is `Direct`, in
                    // parity with the Rust backend's `field_expression` arm.
                    "identifier" | "attribute" => CalleeKind::Direct,
                    "call" | "subscript" => CalleeKind::Indirect,
                    _ => CalleeKind::Unknown,
                };
                (kind, final_segment_of(c, self.source), Some(resolved))
            }
        };
        let args = self.call_args(node);
        let (statement_role, conditional_guarded) = call_position(node);
        let receiver_root = callee.and_then(|c| match c.kind() {
            "attribute" | "subscript" => infer::receiver_root_of(c, self.source),
            _ => None,
        });
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

    fn call_args(&self, node: Node<'_>) -> Vec<Shape> {
        let Some(arguments) = node.child_by_field_name("arguments") else {
            return Vec::new();
        };
        if arguments.kind() == "generator_expression" {
            return vec![self.shape_of(arguments)];
        }
        let mut shapes = Vec::new();
        let mut cursor = arguments.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    if child.kind() == "keyword_argument" {
                        // Carry BOTH the keyword name and the value's raw source
                        // text so `apply(axis=1)` is distinguishable from
                        // `apply(1)` (plan `python-backend` W2; feeds
                        // `PERF-PY-PANDAS-001`).
                        shapes.push(Shape {
                            kind: ShapeKind::Other,
                            name: child
                                .child_by_field_name("name")
                                .map(|n| text(n, self.source).to_string()),
                            value: child
                                .child_by_field_name("value")
                                .map(|v| text(v, self.source).to_string()),
                            ..Shape::default()
                        });
                    } else {
                        shapes.push(self.shape_of(child));
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        shapes
    }

    fn emit_assignment(&mut self, node: Node<'_>) {
        let target_node = match node.kind() {
            "assignment" | "augmented_assignment" => node.child_by_field_name("left"),
            "named_expression" => node.child_by_field_name("name"),
            _ => None,
        };
        let Some(target_node) = target_node else {
            return;
        };
        let target = self.shape_of(target_node);
        let scope_class = self.classify_target(&target);
        let binding_kind = match (&target.kind, target.name.as_deref()) {
            (ShapeKind::Identifier, Some(name)) => self
                .binding
                .get(name)
                .copied()
                .unwrap_or(BindingKind::Unknown),
            _ => BindingKind::Unknown,
        };
        self.assignments.push(AssignmentEvent {
            meta: self.meta_for(node),
            target,
            scope_class,
            binding_kind,
        });
    }

    fn classify_target(&self, target: &Shape) -> ScopeClass {
        match (&target.kind, target.name.as_deref()) {
            (ShapeKind::Identifier, Some(name)) if self.param_names.iter().any(|p| p == name) => {
                ScopeClass::Param
            }
            (ShapeKind::Identifier, Some(name)) if self.global_names.iter().any(|g| g == name) => {
                ScopeClass::Global
            }
            (ShapeKind::Identifier, Some(_)) => ScopeClass::Local,
            _ => ScopeClass::Unknown,
        }
    }

    fn maybe_emit_access_chain(&mut self, node: Node<'_>) {
        if node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "attribute" | "subscript"))
        {
            return;
        }
        let Some((root, elements)) = self.access_chain(node) else {
            return;
        };
        // Emit for a LONE subscript/attribute chain too (`seq[i]`): LOOP-007
        // needs the root-bearing event to kind-gate the common index-scan shape.
        // An empty chain (a bare identifier) carries no chain and is skipped
        // (plan `python-backend` W3).
        if elements.is_empty() {
            return;
        }
        let meta = self.meta_for(node);
        self.access_chains.push(AccessChainEvent {
            meta,
            root,
            elements,
        });
    }

    /// Decompose `a.b[i].c` into (root identifier, element shapes).
    fn access_chain(&self, node: Node<'_>) -> Option<(String, Vec<Shape>)> {
        match node.kind() {
            "subscript" => {
                let base = node.child_by_field_name("value")?;
                let (root, mut elements) = self.access_chain(base)?;
                let index_ident = node
                    .child_by_field_name("subscript")
                    .and_then(|i| infer::leftmost_identifier(i, self.source));
                elements.push(Shape {
                    kind: ShapeKind::Subscript,
                    index_ident,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            "attribute" => {
                let base = node.child_by_field_name("object")?;
                let (root, mut elements) = self.access_chain(base)?;
                let name = node
                    .child_by_field_name("attribute")
                    .map(|f| text(f, self.source).to_string());
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
            is_innermost: !in_for_iterable(node) && is_innermost_event(node),
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        }
    }

    fn shape_of(&self, node: Node<'_>) -> Shape {
        match node.kind() {
            "integer" | "float" | "string" | "concatenated_string" | "true" | "false" | "none" => {
                Shape::literal(text(node, self.source))
            }
            "identifier" | "keyword_identifier" => Shape::identifier(text(node, self.source)),
            "call" => Shape {
                kind: ShapeKind::Call,
                name: node
                    .child_by_field_name("function")
                    .map(|c| text(c, self.source).to_string()),
                ..Shape::default()
            },
            "subscript" => Shape {
                kind: ShapeKind::Subscript,
                index_ident: node
                    .child_by_field_name("subscript")
                    .and_then(|i| infer::leftmost_identifier(i, self.source)),
                ..Shape::default()
            },
            "attribute" => Shape {
                kind: ShapeKind::MemberAccess,
                depth: Some(chain_depth(node)),
                ..Shape::default()
            },
            "unary_operator" | "not_operator" => Shape {
                kind: ShapeKind::Unary,
                ..Shape::default()
            },
            "binary_operator" => Shape {
                kind: ShapeKind::Binary,
                ..Shape::default()
            },
            "parenthesized_expression" => node
                .named_child(0)
                .map(|inner| self.shape_of(inner))
                .unwrap_or_default(),
            _ => Shape::default(),
        }
    }
}

/// Chain depth of member/subscript nesting rooted at `node`.
fn chain_depth(node: Node<'_>) -> u32 {
    match node.kind() {
        "subscript" => node
            .child_by_field_name("value")
            .map_or(1, |base| 1 + chain_depth(base)),
        "attribute" => node
            .child_by_field_name("object")
            .map_or(1, |base| 1 + chain_depth(base)),
        _ => 0,
    }
}

/// Final segment of a callee expression.
fn final_segment_of(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "keyword_identifier" => text(node, source).to_string(),
        "attribute" => node
            .child_by_field_name("attribute")
            .map(|a| text(a, source).to_string())
            .unwrap_or_default(),
        "subscript" => node
            .child_by_field_name("value")
            .map(|v| final_segment_of(v, source))
            .unwrap_or_default(),
        "call" => node
            .child_by_field_name("function")
            .map(|f| final_segment_of(f, source))
            .unwrap_or_default(),
        _ => text(node, source).to_string(),
    }
}

/// Best-effort induction identifiers of a `for` target pattern.
fn pattern_identifiers(pattern: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut vars = Vec::new();
    collect_identifiers(pattern, source, &mut vars);
    vars
}

fn collect_identifiers(node: Node<'_>, source: &[u8], out: &mut Vec<String>) {
    if node.kind() == "identifier" {
        let name = text(node, source).to_string();
        if !out.contains(&name) {
            out.push(name);
        }
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_identifiers(cursor.node(), source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Nearest loop-bearing ancestor of `event`, if any.
fn nearest_enclosing_loop(event: Node<'_>) -> Option<Node<'_>> {
    let mut current = event.parent();
    while let Some(node) = current {
        if LOOP_KINDS.contains(&node.kind()) {
            return Some(node);
        }
        current = node.parent();
    }
    None
}

/// `is_innermost` operational rule (ir-v1.md §3.3): the nearest enclosing loop
/// exists and nests no further loop in its own body. For a `for`, the iterable
/// is excluded (evaluated once). For a comprehension container, every
/// `for_in_clause` child is skipped: those clauses ARE the loop levels
/// enclosing the body, not loops nested below it.
fn is_innermost_event(event: Node<'_>) -> bool {
    let Some(loop_node) = nearest_enclosing_loop(event) else {
        return false;
    };
    let kind = loop_node.kind();
    let iterable = match kind {
        "for_statement" | "for_in_clause" => loop_node.child_by_field_name("right"),
        _ => None,
    };
    let is_comprehension = matches!(
        kind,
        "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression"
    );
    let mut cursor = loop_node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let skip = Some(child.id()) == iterable.map(|i| i.id())
                || (is_comprehension && child.kind() == "for_in_clause");
            if !skip && contains_loop(child) {
                return false;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    true
}

/// Whether `node` lies inside the iterable (`right` field) of its nearest
/// enclosing `for_statement`/`for_in_clause`. Such nodes are evaluated once,
/// before the body: neither in-loop nor body statements (`in_loop_depth == 0`,
/// `is_innermost == false`, `statement_role == Unknown`).
fn in_for_iterable(node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if LOOP_KINDS.contains(&parent.kind()) {
            return matches!(parent.kind(), "for_statement" | "for_in_clause")
                && parent
                    .child_by_field_name("right")
                    .is_some_and(|right| right.id() == current.id());
        }
        current = parent;
    }
    false
}

fn contains_loop(node: Node<'_>) -> bool {
    if LOOP_KINDS.contains(&node.kind()) {
        return true;
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

/// Position of a call relative to its enclosing loop (ir-v1.md §3.3). A call in
/// a `for` iterable is evaluated once and is `Unknown`/not-in-loop.
fn call_position(node: Node<'_>) -> (StatementRole, bool) {
    if in_for_iterable(node) {
        return (StatementRole::Unknown, false);
    }
    let mut role = StatementRole::Unknown;
    let mut guarded = false;
    let mut saw_loop = false;
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            k if LOOP_KINDS.contains(&k) => {
                saw_loop = true;
                if role == StatementRole::Unknown {
                    role = StatementRole::Primary;
                }
                break;
            }
            "expression_statement" => {}
            "block"
                if parent
                    .parent()
                    .is_some_and(|g| LOOP_KINDS.contains(&g.kind())) => {}
            "if_statement" | "elif_clause" | "if_clause" | "conditional_expression" => {
                guarded = true;
                role = StatementRole::Nested;
            }
            _ => role = StatementRole::Nested,
        }
        current = parent.parent();
    }
    if !saw_loop {
        return (StatementRole::Unknown, false);
    }
    (role, guarded)
}

/// Canonical Python `range(...)` trip count (plan `map-build` I2c; completes
/// I2 for C/Rust/Python).
///
/// The ONLY admitted shape is a plain `range(...)` call (bare `range`
/// identifier callee, not an attribute) whose positional arguments are
/// integer literals (decimal or `0x`/`0o`/`0b`, `_` separators, optional
/// unary minus):
/// - `range(N)` ⇒ `N` for `N > 0`;
/// - `range(A, B)` ⇒ `B - A` when `B > A` (`range(-5, 5)` ⇒ 10);
/// - `range(A, B, 1)` ⇒ `B - A`; any other literal step ⇒ `None`.
///
/// Everything else is honestly unknown: variable/computed bounds
/// (`range(n)`, `range(len(x))`), floats, non-`range` calls, keyword
/// arguments, zero or more than three arguments, empty/descending ranges,
/// parenthesized bounds, attribute callees (`builtins.range`), and `while`
/// loops (never routed here). Arbitrary-precision literals saturate, then
/// clamp to [`crate::ir::MAX_TRIP`].
fn canonical_range_trip_count(iterable: Node<'_>, source: &[u8]) -> Option<u64> {
    if iterable.kind() != "call" {
        return None;
    }
    let function = iterable.child_by_field_name("function")?;
    if function.kind() != "identifier" || text(function, source) != "range" {
        return None;
    }
    let arguments = iterable.child_by_field_name("arguments")?;
    if arguments.kind() != "argument_list" {
        return None;
    }
    let mut args: Vec<Node<'_>> = Vec::new();
    let mut cursor = arguments.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named() {
                args.push(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let trips = match args.as_slice() {
        [end] => {
            let end = python_int_literal_value(*end, source)?;
            if end <= 0 {
                return None; // range(0) / negative bound is an empty range
            }
            end
        }
        [start, end] => {
            let start = python_int_literal_value(*start, source)?;
            let end = python_int_literal_value(*end, source)?;
            if end <= start {
                return None; // empty / descending range
            }
            end.saturating_sub(start)
        }
        [start, end, step] => {
            if python_int_literal_value(*step, source)? != 1 {
                return None; // any step != 1 (negative included) is unknown
            }
            let start = python_int_literal_value(*start, source)?;
            let end = python_int_literal_value(*end, source)?;
            if end <= start {
                return None;
            }
            end.saturating_sub(start)
        }
        _ => return None, // 0 args, >3 args, keyword args, `*args`
    };
    let trips = u64::try_from(trips).unwrap_or(u64::MAX);
    Some(trips.min(crate::ir::MAX_TRIP))
}

/// Parse a Python `integer` node (optionally under a unary `-`) into `i128`.
///
/// Accepts decimal and `0x`/`0o`/`0b` radixes with `_` separators; only the
/// unary minus sign is admitted (a `+`/`~` prefix, float, or expression is
/// honestly unknown). Digits accumulate with saturating arithmetic, so an
/// arbitrary-precision literal saturates instead of overflowing; the caller
/// clamps to [`crate::ir::MAX_TRIP`].
fn python_int_literal_value(node: Node<'_>, source: &[u8]) -> Option<i128> {
    match node.kind() {
        "integer" => parse_python_int(text(node, source)),
        "unary_operator" => {
            let operator = node.child_by_field_name("operator")?;
            if text(operator, source) != "-" {
                return None;
            }
            let inner = node.child_by_field_name("argument")?;
            if inner.kind() != "integer" {
                return None;
            }
            parse_python_int(text(inner, source)).map(i128::saturating_neg)
        }
        _ => None,
    }
}

/// Parse Python integer-literal text with an optional radix prefix and `_`
/// separators. Returns `None` for empty/invalid digits.
fn parse_python_int(raw: &str) -> Option<i128> {
    let trimmed = raw.trim();
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
