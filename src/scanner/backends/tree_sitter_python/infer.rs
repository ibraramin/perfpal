//! Python backend inference (plan `python-backend` W2).
//!
//! The deliberate syntactic-tier ceiling: a per-file import-alias map, a
//! per-function **single-assignment** [`BindingKind`] lattice (no dataflow), the
//! call-receiver root, `async` detection, and parameter/annotation extraction.
//! Nothing here claims semantics — every consumer stays capped at MEDIUM
//! confidence with `resolver_tier = syntactic` (ADR-0007 §5).

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::{text, walk_scope};
use crate::ir::BindingKind;

/// Per-file import-alias map: locally bound name → imported dotted path.
///
/// `import pandas as pd` ⇒ `pd → pandas`; `from pandas import DataFrame as DF`
/// ⇒ `DF → pandas.DataFrame`; `from pandas import DataFrame` ⇒
/// `DataFrame → pandas.DataFrame` (unaliased from-names bind to `M.n` too).
pub(super) type AliasMap = BTreeMap<String, String>;

/// Apply the alias map to the **first dotted segment** of a callee path.
///
/// `pd.concat` with `pd → pandas` ⇒ `pandas.concat`; `DF` with
/// `DF → pandas.DataFrame` ⇒ `pandas.DataFrame`; deeper segments are untouched.
pub(super) fn apply_alias_first_segment(path: &str, alias: &AliasMap) -> String {
    if let Some((first, rest)) = path.split_once('.') {
        if let Some(mapped) = alias.get(first) {
            return format!("{mapped}.{rest}");
        }
    } else if let Some(mapped) = alias.get(path) {
        return mapped.clone();
    }
    path.to_string()
}

/// Parameter type/name extraction from a `parameters` node.
#[derive(Default)]
pub(super) struct ParamInfo {
    /// Raw annotation text, or `"unknown"` for an unannotated parameter.
    pub types: Vec<String>,
    /// Parameter names in declaration order (scope-classification + lattice
    /// seeding). `self` is present as a name for methods.
    pub names: Vec<String>,
}

/// Walk a `parameters` node collecting one `(name, type)` entry per parameter.
pub(super) fn collect_params(params: Node<'_>, source: &[u8]) -> ParamInfo {
    let mut info = ParamInfo::default();
    let mut cursor = params.walk();
    if cursor.goto_first_child() {
        loop {
            let param = cursor.node();
            match param.kind() {
                "identifier" => {
                    info.types.push("unknown".to_string());
                    info.names.push(text(param, source).to_string());
                }
                "typed_parameter" => {
                    let name = param
                        .named_child(0)
                        .and_then(|n| leftmost_identifier(n, source));
                    let ty = param
                        .child_by_field_name("type")
                        .map(|t| text(t, source).trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    if let Some(name) = name {
                        info.types.push(ty);
                        info.names.push(name);
                    }
                }
                "default_parameter" => {
                    if let Some(name) = param
                        .child_by_field_name("name")
                        .and_then(|n| leftmost_identifier(n, source))
                    {
                        info.types.push("unknown".to_string());
                        info.names.push(name);
                    }
                }
                "typed_default_parameter" => {
                    let name = param
                        .child_by_field_name("name")
                        .and_then(|n| leftmost_identifier(n, source));
                    let ty = param
                        .child_by_field_name("type")
                        .map(|t| text(t, source).trim().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    if let Some(name) = name {
                        info.types.push(ty);
                        info.names.push(name);
                    }
                }
                "list_splat_pattern" | "dictionary_splat_pattern" => {
                    if let Some(name) = leftmost_identifier(param, source) {
                        info.types.push("unknown".to_string());
                        info.names.push(name);
                    }
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    info
}

/// `async def` produces a `function_definition` whose first token is the
/// anonymous `async` keyword (ADR-0007 §6).
pub(super) fn is_async_fn(node: Node<'_>) -> bool {
    node.child(0).is_some_and(|c| c.kind() == "async")
}

/// Leftmost identifier on an expression subtree (the receiver/root of an
/// access chain, the bound name of a pattern).
pub(super) fn leftmost_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "identifier" {
        return Some(text(node, source).to_string());
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

/// Receiver root of an `attribute`/`subscript` callee chain: `self` for
/// `self.results.append`, `df` for `df.groupby`. The caller only invokes this
/// for a chain callee (a bare identifier has no chain and yields `None`).
pub(super) fn receiver_root_of(callee: Node<'_>, source: &[u8]) -> Option<String> {
    match callee.kind() {
        "attribute" => callee
            .child_by_field_name("object")
            .and_then(|o| receiver_root_of(o, source)),
        "subscript" => callee
            .child_by_field_name("value")
            .and_then(|v| receiver_root_of(v, source)),
        "identifier" => Some(text(callee, source).to_string()),
        _ => None,
    }
}

/// Annotation text → [`BindingKind`], best-effort (primary static evidence).
/// `unknown` for anything not recognised.
pub(super) fn kind_of_annotation(annotation: &str) -> BindingKind {
    let t = annotation.to_ascii_lowercase();
    if t == "unknown" || t.is_empty() {
        BindingKind::Unknown
    } else if t.contains("dataframe") {
        BindingKind::Dataframe
    } else if t.contains("series") {
        BindingKind::Series
    } else if t.contains("ndarray") || t.contains("array") {
        BindingKind::Ndarray
    } else if t.contains("textio") || t.contains("binaryio") || t.contains("file") {
        BindingKind::Filehandle
    } else if t.contains("list") {
        BindingKind::List
    } else if t.contains("dict") || t.contains("mapping") {
        BindingKind::Dict
    } else if t.contains("set") {
        BindingKind::Set
    } else if t.contains("bytes") {
        BindingKind::Bytes
    } else if t.contains("str") {
        BindingKind::Str
    } else {
        BindingKind::Unknown
    }
}

/// Kind of an initializer expression (the RHS of an assignment), best-effort.
pub(super) fn kind_of_expr(node: Node<'_>, alias: &AliasMap, source: &[u8]) -> BindingKind {
    match node.kind() {
        "list" => BindingKind::List,
        "dictionary" => BindingKind::Dict,
        "set" => BindingKind::Set,
        "list_comprehension" => BindingKind::List,
        "set_comprehension" => BindingKind::Set,
        "dictionary_comprehension" => BindingKind::Dict,
        "string" => {
            if text(node, source)
                .trim_start()
                .chars()
                .next()
                .is_some_and(|c| c == 'b' || c == 'B')
            {
                BindingKind::Bytes
            } else {
                BindingKind::Str
            }
        }
        "concatenated_string" => BindingKind::Str,
        "parenthesized_expression" => node.named_child(0).map_or(BindingKind::Unknown, |inner| {
            kind_of_expr(inner, alias, source)
        }),
        "call" => kind_of_call(node, alias, source),
        _ => BindingKind::Unknown,
    }
}

/// Kind implied by a constructor call, applying the alias map to the callee.
fn kind_of_call(node: Node<'_>, alias: &AliasMap, source: &[u8]) -> BindingKind {
    let Some(callee) = node.child_by_field_name("function") else {
        return BindingKind::Unknown;
    };
    let resolved = apply_alias_first_segment(text(callee, source), alias);
    let final_segment = resolved.rsplit('.').next().unwrap_or(resolved.as_str());
    match final_segment {
        "list" => BindingKind::List,
        "dict" => BindingKind::Dict,
        "set" => BindingKind::Set,
        "str" => BindingKind::Str,
        "bytes" => BindingKind::Bytes,
        "open" => BindingKind::Filehandle,
        "DataFrame" => BindingKind::Dataframe,
        "Series" => BindingKind::Series,
        "array" | "ndarray" => BindingKind::Ndarray,
        _ => BindingKind::Unknown,
    }
}

/// Build the per-function single-assignment `BindingKind` lattice.
///
/// Seeds from annotated parameters, then folds every assignment /
/// `with open(...) as f` / walrus in source order. The lattice rule (ADR-0007
/// §5): first binding wins; a conflicting *known* reassignment degrades to
/// [`BindingKind::Unknown`]; an unresolved (`Unknown`) RHS never overwrites a
/// resolved kind, and a resolved RHS upgrades an `Unknown`. No dataflow —
/// this is the deliberate ceiling.
pub(super) fn build_lattice(
    body: Node<'_>,
    param_kinds: &BTreeMap<String, BindingKind>,
    alias: &AliasMap,
    source: &[u8],
) -> BTreeMap<String, BindingKind> {
    let mut map = param_kinds.clone();
    walk_scope(body, &mut |node| match node.kind() {
        "assignment" => {
            if let (Some(left), Some(right)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("right"),
            ) {
                let kind = kind_of_expr(right, alias, source);
                for name in target_names(left, source) {
                    update(&mut map, &name, kind);
                }
            }
        }
        "augmented_assignment" => {
            if let Some(left) = node.child_by_field_name("left") {
                for name in target_names(left, source) {
                    map.entry(name).or_insert(BindingKind::Unknown);
                }
            }
        }
        "named_expression" => {
            if let (Some(name_node), Some(value)) = (
                node.child_by_field_name("name"),
                node.child_by_field_name("value"),
            ) && let Some(name) = leftmost_identifier(name_node, source)
            {
                let kind = kind_of_expr(value, alias, source);
                update(&mut map, &name, kind);
            }
        }
        "with_statement" => collect_with_filehandles(node, alias, source, &mut map),
        _ => {}
    });
    map
}

/// Fold `with open(...) as f:` bindings to [`BindingKind::Filehandle`].
fn collect_with_filehandles(
    with_node: Node<'_>,
    alias: &AliasMap,
    source: &[u8],
    map: &mut BTreeMap<String, BindingKind>,
) {
    let mut cursor = with_node.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        if cursor.node().kind() == "with_clause" {
            let mut items = cursor.node().walk();
            if items.goto_first_child() {
                loop {
                    if items.node().kind() == "with_item"
                        && let Some(value) = items.node().child_by_field_name("value")
                    {
                        collect_one_with_item(value, alias, source, map);
                    }
                    if !items.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn collect_one_with_item(
    value: Node<'_>,
    alias: &AliasMap,
    source: &[u8],
    map: &mut BTreeMap<String, BindingKind>,
) {
    if value.kind() != "as_pattern" {
        return;
    }
    let Some(expr) = value.named_child(0) else {
        return;
    };
    let Some(alias_node) = value.child_by_field_name("alias") else {
        return;
    };
    let resolved = expr
        .child_by_field_name("function")
        .map(|callee| apply_alias_first_segment(text(callee, source), alias));
    if resolved.as_deref() == Some("open") || resolved.as_deref() == Some("io.open") {
        let name = text(alias_node, source).trim().to_string();
        if !name.is_empty() {
            update(map, &name, BindingKind::Filehandle);
        }
    }
}

/// Collect the simple names a pattern binds (skips `attribute`/`subscript`
/// targets, which the single-assignment lattice deliberately does not track).
fn target_names(node: Node<'_>, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "identifier" => vec![text(node, source).to_string()],
        "pattern_list" | "tuple_pattern" | "list_pattern" | "parenthesized_expression" => {
            let mut names = Vec::new();
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    names.extend(target_names(cursor.node(), source));
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            names
        }
        _ => Vec::new(),
    }
}

fn update(map: &mut BTreeMap<String, BindingKind>, name: &str, new: BindingKind) {
    match map.get(name).copied() {
        None => {
            map.insert(name.to_string(), new);
        }
        Some(old) if old == new => {}
        Some(BindingKind::Unknown) => {
            map.insert(name.to_string(), new);
        }
        Some(_) if new == BindingKind::Unknown => {}
        Some(_) => {
            map.insert(name.to_string(), BindingKind::Unknown);
        }
    }
}
