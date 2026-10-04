//! tree-sitter-python scanner backend (ADR-0007; plan `python-backend`, W2).
//!
//! Binds the official org-published grammar crate (`tree-sitter-python`, MIT,
//! exact-pinned per ADR-0001 G5) to the parser-agnostic IR seam. It mirrors the
//! C/Rust backends' interface ([`Extraction`]), ParseHealth contract, span
//! conversion and structure, and adds the Python-specific extraction plan from
//! `docs/architecture/python-backend.md`:
//!
//! - imports → [`IncludeEdge`] (raw module text, `IncludeForm::Quoted`,
//!   `resolved = false`) matching `scanner::resolve_python_edges`;
//! - a synthetic `"<module>"` [`FunctionRecord`] (`is_module_scope = true`)
//!   anchoring top-level statements — required or script-style AI files yield
//!   zero loop/alloc findings;
//! - classes → [`StructDef`] (annotated body assignments are best-effort
//!   members), decorators → [`MacroEvent`] `Use`, module assignments →
//!   [`GlobalVar`] (`StorageClass::None`);
//! - `async def` → `is_async`, comprehensions → real [`LoopNode`]s, and the
//!   `for`-iterable once-evaluated fix (see [`scan`]).
//!
//! Contract (modules.md §4.2): engine-native spans convert to IR spans HERE and
//! never leak node kinds past this boundary. Parse honesty: ERROR/missing nodes
//! become [`ParseHealth`] partial data; a failed parse yields
//! [`ParseStatus::Failed`]. `extract` is total and never panics on malformed
//! input. Every Python event is `syntactic`-tier with `resolver: None` (§8).

mod infer;
mod scan;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use tree_sitter::{Node, Parser};

use super::Extraction;
use crate::ir::{
    FileRecord, FunctionRecord, GlobalVar, IncludeCounts, IncludeEdge, IncludeForm, LocCounts,
    MacroEvent, MacroEventKind, NativePoint, ParseHealth, ParseStatus, ResolverTier, Span,
    StorageClass, StructDef, StructKind, StructMember,
};
use crate::scanner::FunctionUnit;
use infer::AliasMap;

/// Extract one file's IR records/events from parsed Python source.
///
/// Never panics on malformed input: syntax errors surface as
/// `ParseHealth{partial}` with error regions/count.
#[must_use]
pub fn extract(path: &str, source: &[u8]) -> Extraction {
    let mut parser = Parser::new();
    // `set_language` only fails on ABI mismatch, which the pinned-pair link
    // test guards; falling back to a `failed` health keeps this total.
    let parse_result = if parser.set_language(&language()).is_ok() {
        parser.parse(source, None)
    } else {
        None
    };

    let Some(tree) = parse_result else {
        return failed_extraction(path);
    };

    let root = tree.root_node();
    let (error_regions, error_node_count) = collect_errors(root, source);
    let status = if error_node_count == 0 {
        ParseStatus::Ok
    } else {
        ParseStatus::Partial
    };

    // Pass 1: file-level records + the per-file import-alias map. Globals are
    // collected before functions so target scope classification can consult
    // them regardless of declaration order.
    let mut includes = Vec::new();
    let mut global_vars = Vec::new();
    let mut structs = Vec::new();
    let mut macros = Vec::new();
    let mut alias: AliasMap = BTreeMap::new();
    walk_file_level(root, &mut |node| match node.kind() {
        "import_statement" => {
            push_import_edges(node, path, source, &mut includes, &mut alias);
        }
        "import_from_statement" => {
            push_from_edges(node, path, source, &mut includes, &mut alias);
        }
        "class_definition" => {
            if let Some(def) = class_def(node, path, source) {
                structs.push(def);
            }
        }
        "decorator" => push_decorator(node, path, source, &mut macros),
        "assignment" if !inside_class_body(node) => {
            push_globals(node, path, source, &mut global_vars);
        }
        _ => {}
    });
    let global_names: Vec<String> = global_vars.iter().map(|g| g.name.clone()).collect();

    // Pass 2: functions. The synthetic module-scope record is emitted FIRST so
    // top-level script statements (very common in AI Python) are never invisible.
    let mut functions = Vec::new();
    functions.push(module_unit(root, path, source, &alias, &global_names));
    walk_file_level(root, &mut |node| {
        if node.kind() == "function_definition" {
            functions.push(function_unit(node, path, source, &alias, &global_names));
        }
    });

    // Comment lines feed the LOC stat; Python lexes one `comment` kind.
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    let mut comment_cursor = root.walk();
    walk(&mut comment_cursor, &mut |node| {
        if node.kind() == "comment" {
            let range = node.byte_range();
            comment_ranges.push((range.start, range.end));
        }
    });
    comment_ranges.sort_unstable();

    let loc = loc_counts(source, &comment_ranges);
    let include_counts = IncludeCounts {
        quoted: includes.len() as u32,
        angle: 0,
    };
    let file = FileRecord {
        path: path.to_string(),
        language: "python".to_string(),
        loc,
        function_count: functions.len() as u32,
        global_var_count: global_vars.len() as u32,
        include_counts,
    };

    Extraction {
        file,
        parse_health: ParseHealth {
            status,
            error_regions,
            error_node_count,
            resolver: None,
        },
        includes,
        global_vars,
        structs,
        macros,
        functions,
    }
}

/// The pinned Python grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

fn failed_extraction(path: &str) -> Extraction {
    Extraction {
        file: FileRecord {
            path: path.to_string(),
            language: "python".to_string(),
            ..FileRecord::default()
        },
        parse_health: ParseHealth {
            status: ParseStatus::Failed,
            error_regions: Vec::new(),
            error_node_count: 0,
            resolver: None,
        },
        includes: Vec::new(),
        global_vars: Vec::new(),
        structs: Vec::new(),
        macros: Vec::new(),
        functions: Vec::new(),
    }
}

/// Engine-native node extent → IR span. THE conversion point (ir-v1.md §2).
pub(super) fn ir_span(node: Node<'_>, source: &[u8]) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span::from_native(
        source,
        NativePoint {
            row: start.row,
            column: start.column,
        },
        NativePoint {
            row: end.row,
            column: end.column,
        },
    )
}

pub(super) fn text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

/// Preorder walk over `node`'s entire subtree.
pub(super) fn walk(cursor: &mut tree_sitter::TreeCursor, visit: &mut impl FnMut(Node<'_>)) {
    loop {
        visit(cursor.node());
        if cursor.goto_first_child() {
            walk(cursor, visit);
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Recursive file-level walk: visits every node but does NOT descend into
/// `function_definition` bodies (their events belong to that function's unit),
/// while DESCENDING into `class_definition` bodies so methods and class-level
/// records are reachable. `decorated_definition` is transparent: its wrapped
/// definition is visited normally. Nested functions are NOT separate records —
/// an accepted limitation of this milestone.
pub(super) fn walk_file_level(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if node.kind() == "function_definition" {
        return;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_file_level(cursor.node(), visit);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Walk `node`'s subtree stopping at scope boundaries (`function_definition`,
/// `class_definition`, `decorated_definition`), used by the binding lattice so a
/// nested scope's bindings never leak into the enclosing one.
pub(super) fn walk_scope(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if matches!(
        node.kind(),
        "function_definition" | "class_definition" | "decorated_definition"
    ) {
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

/// Count ALL error/missing nodes; record outermost extents as regions.
fn collect_errors(root: Node<'_>, source: &[u8]) -> (Vec<Span>, u32) {
    let mut count = 0u32;
    let mut cursor = root.walk();
    walk(&mut cursor, &mut |node| {
        if node.is_error() || node.is_missing() {
            count += 1;
        }
    });
    let mut regions = Vec::new();
    if count > 0 {
        collect_outermost_errors(root, source, &mut regions);
    }
    (regions, count)
}

fn collect_outermost_errors(node: Node<'_>, source: &[u8], regions: &mut Vec<Span>) {
    if node.is_error() || node.is_missing() {
        regions.push(ir_span(node, source));
        return; // outermost extent only; nested errors are counted, not re-spanned
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_outermost_errors(cursor.node(), source, regions);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// LOC counting (modules.md §3.2): blank = whitespace-only; comment = overlaps
/// any comment node range; rest = code. The whitespace rule wins on a blank
/// line inside a comment block.
fn loc_counts(source: &[u8], comment_ranges: &[(usize, usize)]) -> LocCounts {
    let mut loc = LocCounts::default();
    let mut offset = 0usize;
    for line in split_lines(source) {
        let len = line.len();
        if line.iter().all(|b| b.is_ascii_whitespace()) {
            loc.blank += 1;
        } else if comment_ranges
            .iter()
            .any(|&(cs, ce)| cs < offset + len && offset < ce)
        {
            loc.comment += 1;
        } else {
            loc.code += 1;
        }
        // `split_lines` yields lines INCLUDING their terminating newline, so the
        // next line's byte offset advances by exactly this slice's length.
        offset += len;
    }
    loc
}

fn split_lines(source: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut iter = source.split_inclusive(|b| *b == b'\n').peekable();
    std::iter::from_fn(move || match iter.next() {
        Some(b"") => None,
        other => other,
    })
}

/// `import a.b.c` / `import a.b as x` / `import a, b` → one edge per imported
/// name (raw dotted module path); aliased imports also populate the alias map.
fn push_import_edges(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    out: &mut Vec<IncludeEdge>,
    alias: &mut AliasMap,
) {
    let span = ir_span(node, source);
    let mut cursor = node.walk();
    let names: Vec<Node<'_>> = node.children_by_field_name("name", &mut cursor).collect();
    for name_node in names {
        match name_node.kind() {
            "dotted_name" => emit_include(out, path, text(name_node, source), span),
            "aliased_import" => {
                let raw = name_node
                    .child_by_field_name("name")
                    .map(|n| text(n, source))
                    .unwrap_or("");
                if let Some(a) = name_node.child_by_field_name("alias") {
                    alias.insert(text(a, source).to_string(), raw.to_string());
                }
                if !raw.is_empty() {
                    emit_include(out, path, raw, span);
                }
            }
            _ => {}
        }
    }
}

/// `from ... import ...` → edges encoding exactly what
/// `resolve_python_edges` expects (see its doc): ONE module edge for an
/// absolute or dotted relative module, ONE edge per name for a bare relative
/// (`from . import x`), and ONE module edge for a wildcard. Aliased from-names
/// populate the alias map (`DF → pandas.DataFrame`).
fn push_from_edges(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    out: &mut Vec<IncludeEdge>,
    alias: &mut AliasMap,
) {
    let span = ir_span(node, source);
    let module_node = node.child_by_field_name("module_name");
    let raw_module = module_node.map(|m| text(m, source)).unwrap_or_default();

    if has_wildcard_child(node) {
        if !raw_module.is_empty() {
            emit_include(out, path, raw_module, span);
        }
        return;
    }

    let mut names_cursor = node.walk();
    let names: Vec<Node<'_>> = node
        .children_by_field_name("name", &mut names_cursor)
        .collect();
    if names.is_empty() {
        return;
    }

    // A "bare relative" module (`.`, `..`, …) with no trailing package imports
    // one name per edge (`from . import x, y` → `.x`, `.y`). The grammar wraps
    // the leading dots in a named `import_prefix` child and the package in a
    // sibling `dotted_name`; only the latter's presence means a real module.
    let bare_dots = module_node.and_then(|m| {
        if m.kind() != "relative_import" {
            return None;
        }
        let has_dotted = m.children(&mut m.walk()).any(|c| c.kind() == "dotted_name");
        if has_dotted {
            None
        } else {
            let dots = raw_module.chars().take_while(|&c| c == '.').count();
            Some(raw_module[..dots].to_string())
        }
    });

    if let Some(dots) = bare_dots {
        for name_node in names {
            let (name, alias_name) = name_and_alias(name_node, source);
            if name.is_empty() {
                continue;
            }
            let raw = format!("{dots}{name}");
            emit_include(out, path, &raw, span);
            match alias_name {
                // Explicit `as a` wins.
                Some(alias_name) => {
                    alias.insert(alias_name, raw);
                }
                // `from . import mod_b` binds the plain name `mod_b` to the
                // same relative dotted target the `as` branch uses.
                None => {
                    alias.entry(name).or_insert(raw);
                }
            }
        }
    } else {
        emit_include(out, path, raw_module, span);
        for name_node in names {
            let (name, alias_name) = name_and_alias(name_node, source);
            if name.is_empty() {
                continue;
            }
            let resolved = if raw_module.is_empty() {
                name.clone()
            } else {
                format!("{raw_module}.{name}")
            };
            match alias_name {
                // Explicit `as a` wins.
                Some(alias_name) => {
                    alias.insert(alias_name, resolved);
                }
                // `from M import n` binds `n` to `M.n` (first-seen wins).
                None => {
                    alias.entry(name).or_insert(resolved);
                }
            }
        }
    }
}

/// Whether an `import_from_statement` has a wildcard (`*`) child. The grammar
/// exposes it as the named `wildcard_import` node (whose only token is `*`).
fn has_wildcard_child(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if matches!(cursor.node().kind(), "wildcard_import" | "*") {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}

/// Imported-name text + optional alias (`dotted_name` / `aliased_import`).
fn name_and_alias(node: Node<'_>, source: &[u8]) -> (String, Option<String>) {
    match node.kind() {
        "aliased_import" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| text(n, source).to_string())
                .unwrap_or_default();
            let alias = node
                .child_by_field_name("alias")
                .map(|a| text(a, source).to_string());
            (name, alias)
        }
        _ => (text(node, source).to_string(), None),
    }
}

fn emit_include(out: &mut Vec<IncludeEdge>, path: &str, raw: &str, span: Span) {
    if raw.is_empty() {
        return;
    }
    out.push(IncludeEdge {
        from_path: path.to_string(),
        to_path: Some(raw.to_string()),
        form: IncludeForm::Quoted,
        resolved: false,
        is_mod: false,
        span,
    });
}

/// A `decorator` (`@pkg.cache(...)`) → [`MacroEvent`] `Use`.
fn push_decorator(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<MacroEvent>) {
    let name = node
        .named_child(0)
        .map(|e| text(e, source).trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            text(node, source)
                .trim_start_matches('@')
                .trim()
                .to_string()
        });
    out.push(MacroEvent {
        kind: MacroEventKind::Use,
        name,
        path: path.to_string(),
        span: ir_span(node, source),
        pattern_id: None,
    });
}

/// `class_definition` → [`StructDef`] with best-effort members: annotated body
/// assignments become [`StructMember`]s; methods are NOT members.
fn class_def(node: Node<'_>, path: &str, source: &[u8]) -> Option<StructDef> {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let mut members = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        collect_class_members(body, source, &mut members);
    }
    Some(StructDef {
        name,
        path: path.to_string(),
        span: ir_span(node, source),
        members,
        kind: StructKind::Struct,
    })
}

fn collect_class_members(body: Node<'_>, source: &[u8], out: &mut Vec<StructMember>) {
    let mut cursor = body.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let stmt = cursor.node();
        if stmt.kind() == "expression_statement"
            && let Some(inner) = stmt.named_child(0)
            && inner.kind() == "assignment"
            && let Some(type_node) = inner.child_by_field_name("type")
            && let Some(left) = inner.child_by_field_name("left")
            && let Some(name) = infer::leftmost_identifier(left, source)
        {
            out.push(StructMember {
                name,
                type_text: text(type_node, source).to_string(),
                array_dim_literal: None,
                is_pointer: false,
            });
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Module-level `assignment` target names → [`GlobalVar`] (`StorageClass::None`).
fn push_globals(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<GlobalVar>) {
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    let span = ir_span(node, source);
    for name in target_identifiers(left, source) {
        out.push(GlobalVar {
            name,
            path: path.to_string(),
            span,
            storage: StorageClass::None,
        });
    }
}

/// Simple identifiers a target pattern binds; `attribute`/`subscript` targets
/// bind no file-level name and are skipped.
fn target_identifiers(node: Node<'_>, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "identifier" => vec![text(node, source).to_string()],
        "pattern_list" | "tuple_pattern" | "list_pattern" | "parenthesized_expression" => {
            let mut names = Vec::new();
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    names.extend(target_identifiers(cursor.node(), source));
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

/// A class-body assignment must NOT be a file-level global even though
/// `walk_file_level` descends into class bodies to reach methods.
fn inside_class_body(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "class_definition" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// The synthetic module-scope function: all top-level statements extracted as
/// if in one body, `is_module_scope = true`, name `"<module>"`.
fn module_unit(
    root: Node<'_>,
    path: &str,
    source: &[u8],
    alias: &AliasMap,
    global_names: &[String],
) -> FunctionUnit {
    let span = ir_span(root, source);
    let binding = infer::build_lattice(root, &BTreeMap::new(), alias, source);
    let result = scan::scan(root, source, alias, &[], global_names, &binding);
    let record = FunctionRecord {
        id: format!("{path}#<module>#{}:{}", span.start_line, span.start_col),
        name: "<module>".to_string(),
        path: path.to_string(),
        span,
        param_count: 0,
        param_types: Vec::new(),
        cyclomatic_complexity: result.cyclomatic,
        cognitive_complexity: result.cognitive,
        max_loop_depth: result.max_loop_depth,
        alloc_site_count: result.alloc_site_count,
        length_lines: span.end_line - span.start_line + 1,
        resolver_tier: ResolverTier::default(),
        is_async: false,
        is_module_scope: true,
    };
    FunctionUnit {
        record,
        calls: result.calls,
        assignments: result.assignments,
        binary_ops: result.binary_ops,
        comparisons: result.comparisons,
        access_chains: result.access_chains,
        loops: result.loops,
        awaits: result.awaits,
        try_blocks: result.try_blocks,
    }
}

/// One real `function_definition` → [`FunctionUnit`] (module-level or method).
fn function_unit(
    fdef: Node<'_>,
    path: &str,
    source: &[u8],
    alias: &AliasMap,
    global_names: &[String],
) -> FunctionUnit {
    let name = fdef
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    let params = fdef
        .child_by_field_name("parameters")
        .map(|p| infer::collect_params(p, source))
        .unwrap_or_default();
    let span = ir_span(fdef, source);

    let param_kinds = param_kinds(&params);
    let body = fdef.child_by_field_name("body");
    let binding = body.map_or_else(
        || param_kinds.clone(),
        |b| infer::build_lattice(b, &param_kinds, alias, source),
    );
    let start = body.unwrap_or(fdef);
    let result = scan::scan(start, source, alias, &params.names, global_names, &binding);

    let record = FunctionRecord {
        id: format!("{path}#{name}#{}:{}", span.start_line, span.start_col),
        name,
        path: path.to_string(),
        span,
        param_count: params.types.len() as u32,
        param_types: params.types,
        cyclomatic_complexity: result.cyclomatic,
        cognitive_complexity: result.cognitive,
        max_loop_depth: result.max_loop_depth,
        alloc_site_count: result.alloc_site_count,
        length_lines: span.end_line - span.start_line + 1,
        resolver_tier: ResolverTier::default(),
        is_async: infer::is_async_fn(fdef),
        is_module_scope: false,
    };
    FunctionUnit {
        record,
        calls: result.calls,
        assignments: result.assignments,
        binary_ops: result.binary_ops,
        comparisons: result.comparisons,
        access_chains: result.access_chains,
        loops: result.loops,
        awaits: result.awaits,
        try_blocks: result.try_blocks,
    }
}

/// Seed the binding lattice from annotated parameters.
fn param_kinds(params: &infer::ParamInfo) -> BTreeMap<String, crate::ir::BindingKind> {
    let mut map = BTreeMap::new();
    for (name, annotation) in params.names.iter().zip(params.types.iter()) {
        let kind = infer::kind_of_annotation(annotation);
        if kind != crate::ir::BindingKind::Unknown {
            map.insert(name.clone(), kind);
        }
    }
    map
}
