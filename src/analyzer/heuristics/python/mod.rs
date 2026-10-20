//! Python (`PERF-PY-*`) heuristic families (plan `python-backend` W3).
//!
//! Family layout mirrors `heuristics/` and `heuristics/rust/`: [`alloc`]
//! (PY-ALLOC-001/003/004), [`arch`] (PY-ARCH-001/002/003), [`call`]
//! (PY-CALL-001/002), [`comprehension`] (PY-COMPREHENSION-001), [`executor`]
//! (PY-EXECUTOR-001), `r#loop` (PY-LOOP-001/002/003/007/009), [`mem`]
//! (PY-MEM-002), [`n1`] (PY-N1-001), [`pandas`] (PY-PANDAS-001/002),
//! [`async_rules`] (PY-ASYNC-001/002) and [`str`] (PY-STR-001).
//!
//! These functions are only EVER invoked with `language == "python"` units
//! (registry language gating, `analyzer::analyze`); they are pure IR views
//! with no I/O, clock, global state, or sibling-order dependence
//! (modules.md §4.1). Every finding inherits `resolver_tier` from its source
//! IR event — the Python backend is syntactic-only, so no rule may ever claim
//! [`crate::ir::ResolverTier::Semantic`].
//!
//! Confidence ceiling (docs/architecture/python-backend.md §4): every rule
//! gated on inference (`binding_kind`, `receiver_root`, alias map) is capped
//! at [`crate::analyzer::Confidence::Medium`]; purely syntactic rules may be
//! `High`. Messages cite the CPython mechanism that causes the cost and keep
//! the project's "candidate optimization, not a guaranteed bottleneck"
//! framing. Traversals are sorted for byte-determinism; no set iteration
//! reaches output.

mod alloc;
mod arch;
mod async_rules;
mod call;
mod comprehension;
mod executor;
mod r#loop;
mod mem;
mod n1;
mod pandas;
mod r#str;

pub(crate) use alloc::{python_alloc_001, python_alloc_003, python_alloc_004};
pub(crate) use arch::{python_arch_001, python_arch_002, python_arch_003};
pub(crate) use async_rules::{python_async_001, python_async_002};
pub(crate) use call::{python_call_001, python_call_002};
pub(crate) use comprehension::python_comprehension_001;
pub(crate) use executor::python_executor_001;
pub(crate) use r#loop::{
    python_loop_001, python_loop_002, python_loop_003, python_loop_007, python_loop_009,
};
pub(crate) use mem::python_mem_002;
pub(crate) use n1::python_n1_001;
pub(crate) use pandas::{python_pandas_001, python_pandas_002};
pub(crate) use r#str::python_str_001;

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::Finding;
use crate::ir::{BindingKind, LoopNode, Shape, ShapeKind, Span};
use crate::scanner::FunctionUnit;

/// Stable per-rule finding order: `(start_line, start_col, path)`. Applied at
/// the end of every Python rule so direct unit-test calls are deterministic
/// (the analyzer re-sorts globally afterwards).
pub(super) fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|a, b| {
        (a.span.start_line, a.span.start_col, a.path.as_str()).cmp(&(
            b.span.start_line,
            b.span.start_col,
            b.path.as_str(),
        ))
    });
}

/// Per-function single-assignment binding lattice, keyed by the target
/// identifier. `Unknown` entries are omitted (they carry no signal). The
/// backend already degrades conflicting reassignments to `Unknown`, so the
/// first non-`Unknown` kind observed for a name is authoritative.
pub(super) fn binding_kinds(func: &FunctionUnit) -> BTreeMap<String, BindingKind> {
    let mut map: BTreeMap<String, BindingKind> = BTreeMap::new();
    for assignment in &func.assignments {
        if assignment.target.kind != ShapeKind::Identifier {
            continue;
        }
        let Some(name) = assignment.target.name.as_deref() else {
            continue;
        };
        if assignment.binding_kind == BindingKind::Unknown {
            continue;
        }
        map.entry(name.to_string())
            .or_insert(assignment.binding_kind);
    }
    map
}

/// Names classified with the given [`BindingKind`] (deterministic order).
pub(super) fn names_with_kind(func: &FunctionUnit, kind: BindingKind) -> BTreeSet<String> {
    binding_kinds(func)
        .into_iter()
        .filter(|(_, k)| *k == kind)
        .map(|(name, _)| name)
        .collect()
}

/// Innermost loop (deepest span-containing, earliest start on ties) enclosing
/// `span`; the Python twin of the Rust family's helper.
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

/// Induction-variable names of every loop whose span contains `span`.
pub(super) fn enclosing_induction_vars(func: &FunctionUnit, span: &Span) -> BTreeSet<String> {
    func.loops
        .iter()
        .filter(|l| super::span_contains(&l.span, span))
        .flat_map(|l| l.induction_vars.iter().cloned())
        .collect()
}

/// Whether `text` contains any of `names` as a whole identifier token (so the
/// loop variable `i` does not match `index`).
pub(super) fn mentions_any(text: &str, names: &BTreeSet<String>) -> bool {
    if text.is_empty() || names.is_empty() {
        return false;
    }
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .any(|token| names.contains(token))
}

/// Best-effort raw text of a [`Shape`] for the loop-invariance check: name,
/// value, index or sizeof operand, whichever is populated.
pub(super) fn shape_text(shape: &Shape) -> String {
    shape
        .name
        .clone()
        .or_else(|| shape.value.clone())
        .or_else(|| shape.index_ident.clone())
        .or_else(|| shape.sizeof_operand.clone())
        .unwrap_or_default()
}

/// Final segment of a dotted callee path (`requests.get` → `get`).
pub(super) fn path_final_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

/// Whether an assignment targets `name` somewhere inside `span`.
pub(super) fn assigned_within(func: &FunctionUnit, name: &str, span: &Span) -> bool {
    func.assignments.iter().any(|a| {
        a.target.kind == ShapeKind::Identifier
            && a.target.name.as_deref() == Some(name)
            && super::span_contains(span, &a.meta.span)
    })
}

/// Parser-free IR fixtures shared by every Python family's inline tests, so
/// the rule tests do not depend on the tree-sitter grammar (ADR-0001
/// guardrail 2).
#[cfg(test)]
pub(crate) mod test_support {
    use crate::ir::{
        AccessChainEvent, AssignmentEvent, AwaitEvent, BinaryOpEvent, BindingKind, CallEvent,
        CalleeKind, ComparisonEvent, EventMeta, FileRecord, FunctionRecord, LocCounts, LoopKind,
        LoopNode, ResolverTier, ScopeClass, Shape, ShapeKind, Span, StatementRole, TryEvent,
    };
    use crate::scanner::{FileUnit, FunctionUnit};

    pub(crate) fn meta(span: Span, depth: u32) -> EventMeta {
        EventMeta {
            span,
            in_loop_depth: depth,
            is_innermost: depth >= 1,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        }
    }

    pub(crate) fn point(line: u32) -> Span {
        Span {
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 20,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn func(
        name: &str,
        span: Span,
        calls: Vec<CallEvent>,
        assignments: Vec<AssignmentEvent>,
        binary_ops: Vec<BinaryOpEvent>,
        comparisons: Vec<ComparisonEvent>,
        access_chains: Vec<AccessChainEvent>,
        loops: Vec<LoopNode>,
        awaits: Vec<AwaitEvent>,
        is_async: bool,
    ) -> FunctionUnit {
        FunctionUnit {
            record: FunctionRecord {
                id: format!("m.py#{name}#1:1"),
                name: name.to_string(),
                path: "m.py".to_string(),
                span,
                param_count: 0,
                param_types: Vec::new(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 0,
                max_loop_depth: loops.iter().map(|l| l.depth).max().unwrap_or(0),
                alloc_site_count: 0,
                length_lines: span.end_line,
                resolver_tier: ResolverTier::Syntactic,
                is_async,
                is_module_scope: false,
                call_fanout: None,
                call_fanin: None,
                call_depth: None,
                is_recursive: None,
                idiom_hash: None,
                error_path: None,
                alloc_family: None,
                cfg_gated: None,
                // v1.5 I1a: C-only — Python test fixtures stay unmarked;
                // v1.5 I1c keeps the `__name__` guard fact backend-owned.
                is_static: None,
                has_main_guard: None,
                entry_kind: None,
                entry_confidence: None,
                reachable_from_entry: None,
            },
            calls,
            assignments,
            binary_ops,
            comparisons,
            access_chains,
            loops,
            awaits,
            try_blocks: Vec::new(),
        }
    }

    pub(crate) fn unit(functions: Vec<FunctionUnit>) -> FileUnit {
        FileUnit {
            file: FileRecord {
                path: "m.py".to_string(),
                language: "python".to_string(),
                loc: LocCounts::default(),
                function_count: functions.len() as u32,
                global_var_count: 0,
                include_counts: Default::default(),
            },
            parse_health: Default::default(),
            includes: Vec::new(),
            global_vars: Vec::new(),
            structs: Vec::new(),
            macros: Vec::new(),
            functions,
        }
    }

    pub(crate) fn call(
        name: &str,
        path: Option<&str>,
        root: Option<&str>,
        span: Span,
        depth: u32,
    ) -> CallEvent {
        CallEvent {
            meta: meta(span, depth),
            callee_name: name.to_string(),
            callee_kind: CalleeKind::Direct,
            callee_path: path.map(str::to_string),
            is_macro: false,
            args: Vec::new(),
            statement_role: StatementRole::Primary,
            conditional_guarded: false,
            semantic: None,
            receiver_root: root.map(str::to_string),
        }
    }

    pub(crate) fn assign(name: &str, kind: BindingKind, span: Span, depth: u32) -> AssignmentEvent {
        AssignmentEvent {
            meta: meta(span, depth),
            target: Shape::identifier(name),
            scope_class: ScopeClass::Local,
            binding_kind: kind,
        }
    }

    pub(crate) fn binop(
        op: &str,
        left: Option<&str>,
        right: Option<&str>,
        span: Span,
        depth: u32,
    ) -> BinaryOpEvent {
        BinaryOpEvent {
            meta: meta(span, depth),
            op: op.to_string(),
            left: left.map_or_else(Shape::default, Shape::identifier),
            right: right.map_or_else(Shape::default, Shape::identifier),
        }
    }

    pub(crate) fn subscript_chain(
        root: &str,
        index: &str,
        span: Span,
        depth: u32,
    ) -> AccessChainEvent {
        AccessChainEvent {
            meta: meta(span, depth),
            root: root.to_string(),
            elements: vec![
                Shape {
                    kind: ShapeKind::Subscript,
                    index_ident: Some(index.to_string()),
                    ..Shape::default()
                },
                Shape {
                    kind: ShapeKind::MemberAccess,
                    ..Shape::default()
                },
            ],
        }
    }

    /// A lone `seq[i]` access chain (one subscript element) — the index-scan
    /// shape the backend now emits (plan `python-backend` W3).
    pub(crate) fn lone_subscript(
        root: &str,
        index: &str,
        span: Span,
        depth: u32,
    ) -> AccessChainEvent {
        AccessChainEvent {
            meta: meta(span, depth),
            root: root.to_string(),
            elements: vec![Shape {
                kind: ShapeKind::Subscript,
                index_ident: Some(index.to_string()),
                ..Shape::default()
            }],
        }
    }

    /// A `ComparisonEvent` with an explicit operator (feeds LOOP-007's
    /// membership branch).
    pub(crate) fn comparison(
        op: &str,
        left: Option<&str>,
        right: Option<&str>,
        span: Span,
        depth: u32,
    ) -> ComparisonEvent {
        ComparisonEvent {
            meta: meta(span, depth),
            operator: op.to_string(),
            left: left.map_or_else(Shape::default, Shape::identifier),
            right: right.map_or_else(Shape::default, Shape::identifier),
        }
    }

    /// A `TryEvent` with shared meta (feeds LOOP-009).
    pub(crate) fn try_event(span: Span, depth: u32) -> TryEvent {
        TryEvent {
            meta: meta(span, depth),
        }
    }

    pub(crate) fn loop_node(depth: u32, vars: &[&str], span: Span) -> LoopNode {
        LoopNode {
            kind: LoopKind::For,
            depth,
            induction_vars: vars.iter().map(|v| (*v).to_string()).collect(),
            span,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
            // Synthetic fixtures carry no iterable node, so the scanner's I2c
            // canonical `range(...)` literal gate cannot run here; the honest
            // value for a fabricated loop is unknown.
            trip_count: None,
            condition_is_zero: false,
        }
    }

    pub(crate) fn await_event(span: Span, depth: u32) -> AwaitEvent {
        AwaitEvent {
            meta: meta(span, depth),
        }
    }

    pub(crate) fn with_ident_arg(mut call: CallEvent, name: &str) -> CallEvent {
        call.args.push(Shape::identifier(name));
        call
    }

    pub(crate) fn with_literal_arg(mut call: CallEvent, value: &str) -> CallEvent {
        call.args.push(Shape::literal(value));
        call
    }

    pub(crate) fn with_call_arg(mut call: CallEvent, name: &str) -> CallEvent {
        call.args.push(Shape {
            kind: ShapeKind::Call,
            name: Some(name.to_string()),
            ..Shape::default()
        });
        call
    }

    pub(crate) fn with_keyword_arg(mut call: CallEvent, name: &str, value: &str) -> CallEvent {
        call.args.push(Shape {
            kind: ShapeKind::Other,
            name: Some(name.to_string()),
            value: Some(value.to_string()),
            ..Shape::default()
        });
        call
    }
}
