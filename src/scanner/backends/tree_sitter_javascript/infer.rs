//! JavaScript backend inference (plan `js-ts`, wave B2).
//!
//! Deliberately syntactic-tier helpers: function-kind classification, name
//! derivation, parameter extraction, shape descriptors, identifier collection,
//! local-scope name collection, and the module-entry idiom predicates. Nothing
//! here claims semantics — every consumer stays capped at `syntactic` tier.

use std::collections::BTreeSet;

use tree_sitter::Node;

use super::{Flavor, text};
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
/// module/function. `abstract_class_declaration` is TypeScript-only and never
/// appears in the JS grammar; matching it here is harmless for JS.
/// Accepted limitation: a non-arrow field initializer expression and a
/// `class_static_block` body are not event-scanned at all (they belong to no
/// record).
pub(super) fn is_class_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration" | "class" | "abstract_class_declaration"
    )
}

/// Node kinds that live entirely in TypeScript's type land. They may not be
/// event-scanned: `type_query` contains `call_expression`, and
/// `interface_body`/`object_type` contain `method_signature` →
/// `formal_parameters` → `identifier`. The JS grammar never produces these
/// kinds, so this list is inert there.
pub(super) const TYPE_ONLY_KINDS: [&str; 42] = [
    "type_annotation",
    "type_predicate_annotation",
    "asserts_annotation",
    "type_arguments",
    "type_parameters",
    "implements_clause",
    "extends_type_clause",
    "interface_declaration",
    "interface_body",
    "type_alias_declaration",
    "ambient_declaration",
    "function_signature",
    "method_signature",
    "abstract_method_signature",
    "index_signature",
    "call_signature",
    "construct_signature",
    "property_signature",
    "object_type",
    "function_type",
    "constructor_type",
    "type_query",
    "conditional_type",
    "infer_type",
    "mapped_type_clause",
    "generic_type",
    "union_type",
    "intersection_type",
    "array_type",
    "tuple_type",
    "optional_type",
    "rest_type",
    "parenthesized_type",
    "literal_type",
    "type_identifier",
    "predefined_type",
    "this_type",
    "lookup_type",
    "index_type_query",
    "template_literal_type",
    "nested_type_identifier",
    "readonly_type",
];

/// Whether `kind` belongs to type land and must never be event-scanned.
pub(super) fn is_type_only_kind(kind: &str) -> bool {
    TYPE_ONLY_KINDS.contains(&kind)
}

/// Parameter extraction from a `formal_parameters` node.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ParamInfo {
    /// Raw annotation text per parameter. JavaScript has no annotations, so
    /// every entry is `"unknown"` — mirroring the Python backend's unannotated
    /// convention (tree_sitter_python/infer.rs `collect_params`). The TS
    /// dialect supplies the annotation text without its leading `:`.
    pub types: Vec<String>,
    /// Parameter names in declaration order.
    pub names: Vec<String>,
}

/// Walk a `formal_parameters` node collecting one entry per parameter, in
/// declaration order. The two dialects have different parameter shapes:
///
/// - JavaScript: direct named children (`identifier`, patterns); every type is
///   `"unknown"`.
/// - TypeScript: `required_parameter` / `optional_parameter` wrappers with
///   `pattern`, `name`, and `type` fields. A `this` parameter is skipped (it is
///   erased at runtime); the type annotation's text minus the leading `:` is
///   the reported type, or `"unknown"` when absent.
///
/// Comments are named extras in the tree but are not parameters and are
/// excluded. A destructuring pattern counts as ONE parameter (its leftmost
/// identifier becomes the name); when no identifier is derivable (`{}`, `[]`)
/// the name is empty but the parameter is still counted, matching the Python
/// backend's splat handling.
pub(crate) fn collect_params(params: Node<'_>, source: &[u8], flavor: Flavor) -> ParamInfo {
    match flavor {
        Flavor::JavaScript => collect_params_js(params, source),
        Flavor::TypeScript => collect_params_ts(params, source),
    }
}

fn collect_params_js(params: Node<'_>, source: &[u8]) -> ParamInfo {
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

fn collect_params_ts(params: Node<'_>, source: &[u8]) -> ParamInfo {
    let mut info = ParamInfo::default();
    let mut cursor = params.walk();
    if cursor.goto_first_child() {
        loop {
            let param = cursor.node();
            if !param.is_named() || param.is_extra() {
                // `,`/`(` are anonymous; comments are extras.
            } else if param.kind() == "required_parameter" || param.kind() == "optional_parameter" {
                let pattern = param.child_by_field_name("pattern");
                if pattern.is_some_and(|p| p.kind() == "this") {
                    // `this` is a type-only parameter; erased at runtime.
                } else {
                    let name = pattern
                        .or_else(|| param.child_by_field_name("name"))
                        .and_then(|p| pattern_name(p, source))
                        .unwrap_or_default();
                    let ty = param
                        .child_by_field_name("type")
                        .map(|annotation| annotation_type_text(annotation, source))
                        .filter(|ty| !ty.is_empty())
                        .unwrap_or_else(|| "unknown".to_string());
                    info.types.push(ty);
                    info.names.push(name);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    info
}

/// Type text of a `type_annotation` node without its leading `:`. The
/// annotation node's source text includes the colon (`: string`).
fn annotation_type_text(annotation: Node<'_>, source: &[u8]) -> String {
    let raw = text(annotation, source).trim();
    raw.strip_prefix(':').unwrap_or(raw).trim().to_string()
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
            .or_else(|| parent.child_by_field_name("name"))
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
/// (`self.results.append` ⇒ `self`; `df.groupby` ⇒ `df`). Transparent TS
/// wrappers are stripped at every level (`arr!.includes` ⇒ `arr`).
pub(super) fn receiver_root_of(chain: Node<'_>, source: &[u8]) -> Option<String> {
    let chain = strip_transparent_wrappers(chain);
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
    let node = strip_transparent_wrappers(node);
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
///
/// TypeScript expression wrappers (`as`, `satisfies`, `!`, `<T>expr`) are
/// transparent for shape purposes: the shape of the wrapped value is taken, so
/// `make() as Foo` is still a call and `{...} satisfies T` is still an object.
pub(super) fn shape_of_node(node: Node<'_>, source: &[u8]) -> Shape {
    match node.kind() {
        "as_expression" | "satisfies_expression" | "non_null_expression" | "type_assertion" => {
            // `type_assertion` puts `type_arguments` first; `type_arguments` is
            // type-only, so the first non-type named child is the expression.
            value_child(node)
                .map(|inner| shape_of_node(inner, source))
                .unwrap_or_default()
        }
        "number" | "string" | "template_string" | "true" | "false" | "null" | "undefined"
        | "regex" => Shape::literal(text(node, source)),
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
            name: node
                .child_by_field_name("property")
                .map(|p| text(p, source).to_string()),
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

/// First named child that is NOT type land (the value expression of a TS
/// expression wrapper).
pub(super) fn value_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named() && !child.is_extra() && !is_type_only_kind(child.kind()) {
                return Some(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Strip TypeScript expression wrappers that do not change the runtime value:
/// `as`, `satisfies`, non-null `!`, `<T>expr` (type assertion), and redundant
/// parentheses. Used by callee/receiver decomposition so `(fn as Foo)()` and
/// `arr!.includes(x)` keep the underlying identifier/member chain. The type
/// child is never entered (`value_child` skips type land).
pub(super) fn strip_transparent_wrappers(mut node: Node<'_>) -> Node<'_> {
    loop {
        match node.kind() {
            "as_expression"
            | "satisfies_expression"
            | "non_null_expression"
            | "type_assertion"
            | "parenthesized_expression" => match value_child(node) {
                Some(inner) => node = inner,
                None => return node,
            },
            _ => return node,
        }
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
    // TypeScript type land is not a runtime reference: `typeof someVar` in a
    // type position (`typeof` test / `type_query`) must not count as a
    // referenced name for the loop-invariance gates. The wrappers
    // (`as`/`satisfies`/`!`/<T>expr) are NOT type-only, so their value child
    // is still visited.
    if is_type_only_kind(node.kind()) {
        return;
    }
    // `as`/`satisfies`/`!`/<T>expr wrap a value and a type. Only the value is a
    // runtime reference; entering the type child would treat
    // `typeof someVar` / `import("x").T` as a referenced name.
    if matches!(
        node.kind(),
        "as_expression"
            | "satisfies_expression"
            | "non_null_expression"
            | "type_assertion"
            | "parenthesized_expression"
    ) {
        if let Some(value) = value_child(node) {
            collect_identifiers_into(value, source, out);
        }
        return;
    }
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
/// class bodies (scope boundaries) or into TypeScript type land.
fn walk_scope(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if is_function_kind(node.kind()) || is_class_kind(node.kind()) || is_type_only_kind(node.kind())
    {
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

/// Preorder walk of the module body, stopping at function/class boundaries and
/// at TypeScript type land.
fn walk_module_body(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if is_function_kind(node.kind()) || is_class_kind(node.kind()) || is_type_only_kind(node.kind())
    {
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
