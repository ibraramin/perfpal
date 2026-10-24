//! JavaScript backend inference (plan `js-ts`, wave B2).
//!
//! Deliberately syntactic-tier helpers: function-kind classification, name
//! derivation, parameter extraction, shape descriptors, identifier collection,
//! local-scope name collection, and the module-entry idiom predicates. Nothing
//! here claims semantics — every consumer stays capped at `syntactic` tier.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::text;
use crate::ir::{Shape, ShapeKind};

/// Every node kind the backend treats as a function record AND as a scope
/// boundary inside another function's body scan. Locked to the pinned
/// `tree-sitter-javascript` grammar (see `tests/javascript_grammar_link.rs`).
pub(super) const FUNCTION_KINDS: [&str; 6] = [
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "generator_function_declaration",
    "generator_function",
];

/// Whether `kind` names a function record/scope boundary.
pub(super) fn is_function_kind(kind: &str) -> bool {
    FUNCTION_KINDS.contains(&kind)
}

/// Class containers are scope boundaries for event ownership: methods and
/// arrow-function fields are their own records, never part of the enclosing
/// module/function. Accepted limitation: a non-arrow field initializer
/// expression and a `class_static_block` body are not event-scanned at all
/// (they belong to no record).
pub(super) fn is_class_kind(kind: &str) -> bool {
    matches!(kind, "class_declaration" | "class")
}

/// Parameter extraction from a `formal_parameters` node.
#[derive(Default)]
pub(super) struct ParamInfo {
    /// Raw annotation text per parameter. JavaScript has no annotations, so
    /// every entry is `"unknown"` — mirroring the Python backend's unannotated
    /// convention (tree_sitter_python/infer.rs `collect_params`). A later TS
    /// backend supplies real types.
    pub types: Vec<String>,
    /// Parameter names in declaration order.
    pub names: Vec<String>,
}

/// Walk a `formal_parameters` node collecting one `(name, "unknown")` entry per
/// NAMED parameter, including identifier-less destructuring patterns. Comments
/// are named extras in the tree but are not parameters and are excluded. A
/// destructuring pattern counts as ONE parameter (its leftmost identifier
/// becomes the name); when no identifier is derivable (`{}`, `[]`) the name is
/// empty but the parameter is still counted, matching the Python backend's
/// splat handling.
pub(super) fn collect_params(params: Node<'_>, source: &[u8]) -> ParamInfo {
    let mut info = ParamInfo::default();
    let mut cursor = params.walk();
    if cursor.goto_first_child() {
        loop {
            let param = cursor.node();
            if param.is_named() && !param.is_extra() {
                info.types.push("unknown".to_string());
                info.names
                    .push(pattern_name(param, source).unwrap_or_default());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    info
}

/// Name bound by one parameter/declarator pattern node.
pub(super) fn pattern_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => non_empty(text(node, source)),
        "assignment_pattern" => node
            .child_by_field_name("left")
            .and_then(|left| pattern_name(left, source)),
        "rest_pattern" => {
            let mut last = None;
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    if cursor.node().is_named() {
                        last = Some(cursor.node());
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            last.and_then(|inner| pattern_name(inner, source))
        }
        "array_pattern" | "object_pattern" => leftmost_identifier(node, source),
        _ => None,
    }
}

/// `async` functions carry an anonymous `async` token as a direct child in the
/// pinned grammar (same detector as `tools/js_canary.rs::has_async_token`).
pub(super) fn is_async_fn(node: Node<'_>) -> bool {
    (0..node.child_count() as u32).any(|i| node.child(i).is_some_and(|c| c.kind() == "async"))
}

/// Best-effort function name.
///
/// Precedence: the node's own `name` field (declarations, method definitions,
/// named function expressions), then the binding context — `variable_declarator`
/// name, `pair` key, class `field_definition`/`public_field_definition`
/// property, or `assignment_expression` left identifier. Anything else is
/// `<anonymous>`.
pub(super) fn derive_name(node: Node<'_>, source: &[u8]) -> String {
    if let Some(name) = node.child_by_field_name("name")
        && let Some(value) = node_text_name(name, source)
    {
        return value;
    }
    let Some(parent) = node.parent() else {
        return "<anonymous>".to_string();
    };
    let derived = match parent.kind() {
        "variable_declarator" => parent
            .child_by_field_name("name")
            .and_then(|n| pattern_name(n, source)),
        "pair" => parent
            .child_by_field_name("key")
            .and_then(|k| node_text_name(k, source)),
        "field_definition" | "public_field_definition" => parent
            .child_by_field_name("property")
            .and_then(|p| node_text_name(p, source)),
        "assignment_expression" => parent
            .child_by_field_name("left")
            .and_then(|l| pattern_name(l, source)),
        _ => None,
    };
    derived.unwrap_or_else(|| "<anonymous>".to_string())
}

/// Name text of an identifier-ish node (`identifier`, `property_identifier`,
/// `private_property_identifier`, string/number keys). String keys are
/// unquoted.
fn node_text_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "string" => unquote(text(node, source)),
        "computed_property_name" => None,
        _ => non_empty(text(node, source)),
    }
}

/// Leftmost identifier on an expression subtree.
pub(super) fn leftmost_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        return non_empty(text(node, source));
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if let Some(found) = leftmost_identifier(cursor.node(), source) {
                return Some(found);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Receiver root of a `member_expression`/`subscript_expression` callee chain
/// (`self.results.append` ⇒ `self`; `df.groupby` ⇒ `df`).
pub(super) fn receiver_root_of(chain: Node<'_>, source: &[u8]) -> Option<String> {
    match chain.kind() {
        "member_expression" => chain
            .child_by_field_name("object")
            .and_then(|o| receiver_root_of(o, source)),
        "subscript_expression" => chain
            .child_by_field_name("object")
            .and_then(|o| receiver_root_of(o, source)),
        "identifier" => non_empty(text(chain, source)),
        _ => None,
    }
}

/// Final segment of a callee expression (`a.b.c` ⇒ `c`, `xs[i]` ⇒ `xs`).
pub(super) fn final_segment_of(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "member_expression" => node
            .child_by_field_name("property")
            .map(|p| text(p, source).to_string())
            .unwrap_or_default(),
        "subscript_expression" => node
            .child_by_field_name("object")
            .map(|o| final_segment_of(o, source))
            .unwrap_or_default(),
        "call_expression" => node
            .child_by_field_name("function")
            .map(|f| final_segment_of(f, source))
            .unwrap_or_default(),
        _ => text(node, source).trim().to_string(),
    }
}

/// Shape descriptor for an expression node (ir-v1.md §3.4 vocabulary).
pub(super) fn shape_of_node(node: Node<'_>, source: &[u8]) -> Shape {
    match node.kind() {
        "number" | "string" | "true" | "false" | "null" | "undefined" | "regex" => {
            Shape::literal(text(node, source))
        }
        "identifier"
        | "shorthand_property_identifier"
        | "shorthand_property_identifier_pattern" => Shape::identifier(text(node, source)),
        "call_expression" => Shape {
            kind: ShapeKind::Call,
            name: node
                .child_by_field_name("function")
                .map(|c| text(c, source).to_string()),
            ..Shape::default()
        },
        "new_expression" => Shape {
            kind: ShapeKind::Call,
            name: node
                .child_by_field_name("constructor")
                .map(|c| text(c, source).to_string()),
            ..Shape::default()
        },
        "subscript_expression" => Shape {
            kind: ShapeKind::Subscript,
            index_ident: node
                .child_by_field_name("index")
                .and_then(|i| leftmost_identifier(i, source)),
            ..Shape::default()
        },
        "member_expression" => Shape {
            kind: ShapeKind::MemberAccess,
            depth: Some(chain_depth(node)),
            ..Shape::default()
        },
        "unary_expression" => Shape {
            kind: ShapeKind::Unary,
            ..Shape::default()
        },
        "binary_expression" => Shape {
            kind: ShapeKind::Binary,
            ..Shape::default()
        },
        "parenthesized_expression" => node
            .named_child(0)
            .map(|inner| shape_of_node(inner, source))
            .unwrap_or_default(),
        _ => Shape::default(),
    }
}

/// Chain depth of member/subscript nesting rooted at `node`.
pub(super) fn chain_depth(node: Node<'_>) -> u32 {
    match node.kind() {
        "subscript_expression" => node
            .child_by_field_name("object")
            .map_or(1, |base| 1 + chain_depth(base)),
        "member_expression" => node
            .child_by_field_name("object")
            .map_or(1, |base| 1 + chain_depth(base)),
        _ => 0,
    }
}

/// Identifiers referenced inside a subtree, sorted and deduped.
///
/// Collects `identifier` plus the shorthand-property forms. Object-literal keys
/// are `property_identifier`/`string` nodes and are therefore NOT references,
/// which is why they never appear here.
pub(super) fn collect_identifier_names(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut names = BTreeSet::new();
    collect_identifiers_into(node, source, &mut names);
    names.into_iter().collect()
}

fn collect_identifiers_into(node: Node<'_>, source: &[u8], out: &mut BTreeSet<String>) {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier" | "shorthand_property_identifier_pattern"
    ) {
        let name = text(node, source);
        if !name.is_empty() {
            out.insert(name.to_string());
        }
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_identifiers_into(cursor.node(), source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Pattern names (declarators / loop targets / parameters), source order,
/// deduped. `{a: b}` yields `b` (the bound identifier), never the property key.
pub(super) fn pattern_names(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    collect_pattern_names_into(node, source, &mut names);
    names
}

fn collect_pattern_names_into(node: Node<'_>, source: &[u8], out: &mut Vec<String>) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = text(node, source);
            if !name.is_empty() && !out.iter().any(|existing| existing == name) {
                out.push(name.to_string());
            }
            return;
        }
        "assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_pattern_names_into(left, source, out);
            }
            return;
        }
        "rest_pattern" => {
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    if cursor.node().is_named() {
                        collect_pattern_names_into(cursor.node(), source, out);
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            return;
        }
        _ => {}
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_pattern_names_into(cursor.node(), source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Names declared in a function's own scope (excluding nested functions and
/// class bodies): `variable_declarator` names, `for...in`/`for...of` targets,
/// and `catch` parameters. Used only for `AssignmentEvent.scope_class`
/// (`Local` vs `Param` vs `Unknown`) — no dataflow.
pub(super) fn collect_local_names(body: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    walk_scope(body, &mut |node| match node.kind() {
        "variable_declarator" => {
            if let Some(name) = node.child_by_field_name("name") {
                names.extend(pattern_names(name, source));
            }
        }
        "for_in_statement" => {
            if let Some(left) = node.child_by_field_name("left") {
                names.extend(pattern_names(left, source));
            }
        }
        "catch_clause" => {
            if let Some(parameter) = node.child_by_field_name("parameter") {
                names.extend(pattern_names(parameter, source));
            }
        }
        _ => {}
    });
    let mut deduped = Vec::with_capacity(names.len());
    for name in names {
        if !deduped.iter().any(|existing| existing == &name) {
            deduped.push(name);
        }
    }
    deduped
}

/// Preorder walk of `node`'s subtree that does NOT descend into function or
/// class bodies (scope boundaries).
fn walk_scope(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if is_function_kind(node.kind()) || is_class_kind(node.kind()) {
        return;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_scope(cursor.node(), visit);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// True when the module body proves it is a JavaScript entry declaration.
///
/// EXACT predicates (v1, deliberately conservative):
/// 1. a `binary_expression` with operator `===` whose two operand texts are
///    `require.main` and `module` in either order (the CommonJS
///    `require.main === module` idiom), or
/// 2. an `if_statement` whose condition subtree contains a `binary_expression`
///    with operator `==`/`===` where either operand is exactly
///    `import.meta.url` (the ESM `if (import.meta.url === …)` guard).
///
/// Nested function and class bodies are excluded: entry facts are module-body
/// facts only.
pub(super) fn module_entry_declared(root: Node<'_>, source: &[u8]) -> bool {
    let mut found = false;
    walk_module_body(root, &mut |node| match node.kind() {
        "binary_expression" => {
            if text_of_field(node, "operator", source) == "===" {
                let left = node
                    .child_by_field_name("left")
                    .map(|n| text(n, source).trim())
                    .unwrap_or("");
                let right = node
                    .child_by_field_name("right")
                    .map(|n| text(n, source).trim())
                    .unwrap_or("");
                if (left == "require.main" && right == "module")
                    || (left == "module" && right == "require.main")
                {
                    found = true;
                }
            }
        }
        "if_statement" => {
            if let Some(condition) = node.child_by_field_name("condition")
                && condition_has_import_meta_url(condition, source)
            {
                found = true;
            }
        }
        _ => {}
    });
    found
}

/// Whether a condition subtree compares `import.meta.url` (`==`/`===`).
fn condition_has_import_meta_url(condition: Node<'_>, source: &[u8]) -> bool {
    if condition.kind() == "binary_expression" {
        let op = text_of_field(condition, "operator", source);
        if matches!(op.as_str(), "==" | "===") {
            let left = condition
                .child_by_field_name("left")
                .map(|n| text(n, source).trim())
                .unwrap_or("");
            let right = condition
                .child_by_field_name("right")
                .map(|n| text(n, source).trim())
                .unwrap_or("");
            if left == "import.meta.url" || right == "import.meta.url" {
                return true;
            }
        }
    }
    let mut cursor = condition.walk();
    if cursor.goto_first_child() {
        loop {
            if condition_has_import_meta_url(cursor.node(), source) {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}

/// Preorder walk of the module body, stopping at function/class boundaries.
fn walk_module_body(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if is_function_kind(node.kind()) || is_class_kind(node.kind()) {
        return;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_module_body(cursor.node(), visit);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Text of a field, trimmed (`""` when absent).
fn text_of_field(node: Node<'_>, field: &str, source: &[u8]) -> String {
    node.child_by_field_name(field)
        .map(|child| text(child, source).trim().to_string())
        .unwrap_or_default()
}

/// Strip a matching pair of string quotes; `None` when the text is not a
/// single-quoted/double-quoted literal.
pub(super) fn unquote(literal: &str) -> Option<String> {
    let bytes = literal.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return Some(literal[1..literal.len() - 1].to_string());
        }
    }
    None
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
