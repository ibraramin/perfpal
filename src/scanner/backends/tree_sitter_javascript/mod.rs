//! tree-sitter-javascript scanner backend (plan `js-ts`, wave B2).
//!
//! Binds the official org-published grammar crate (`tree-sitter-javascript`,
//! MIT, exact-pinned per ADR-0001 G5 / ADR-0011) to the parser-agnostic IR
//! seam. It mirrors the C/Rust/Python backends' interface ([`Extraction`]),
//! ParseHealth contract and span conversion, and adds the JavaScript-specific
//! extraction plan:
//!
//! - ESM `import`/`export … from` sources plus static `require("…")` calls →
//!   [`IncludeEdge`] (`Quoted` for `./`/`../` relative specifiers, `Angle` for
//!   bare/absolute/URL/`node:` specifiers; raw text, `resolved = false`);
//! - a synthetic `"<module>"` [`FunctionRecord`] (`is_module_scope = true`)
//!   anchoring top-level statements, carrying the JS entry-declaration fact
//!   (`entry_declared`) from the `require.main === module` idiom or an
//!   `import.meta.url` guard;
//! - `class_declaration` / `class` → [`StructDef`], top-level declarations →
//!   [`GlobalVar`] (`StorageClass::None`);
//! - nested functions are their own records (unlike the Python backend's
//!   flattened limitation), and every event belongs to the INNERMOST function.
//!
//! Contract (modules.md §4.2): engine-native spans convert to IR spans HERE
//! and never leak node kinds past this boundary. Parse honesty: ERROR/missing
//! nodes become [`ParseHealth`] partial data; a failed parse yields
//! [`ParseStatus::Failed`]. `extract` is total and never panics on malformed
//! input. Every JS event is `syntactic`-tier with `resolver: None` (§8).
//!
//! Wiring is part of the B2-W1 change: `LanguageKind::Js` selects `.js`/`.jsx`/
//! `.mjs`/`.cjs`, `extract_unit` dispatches here, and `SUPPORTED_LANGUAGES`
//! carries the `js` token. The TS/TSX dispatch is B3.

mod infer;
mod scan;
#[cfg(test)]
mod tests;

use tree_sitter::{Node, Parser};

use super::Extraction;
use crate::ir::{
    FileRecord, FunctionRecord, GlobalVar, IncludeCounts, IncludeEdge, IncludeForm, LocCounts,
    NativePoint, ParseHealth, ParseStatus, ResolverTier, Span, StorageClass, StructDef, StructKind,
    StructMember,
};
use crate::scanner::FunctionUnit;
use infer::ParamInfo;
use scan::ScanInputs;

/// Extract one file's IR records/events from parsed JavaScript source.
///
/// Never panics on malformed or JSX-bearing input: syntax errors surface as
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

    // Pass 1: file-level records. Imports/requires and classes are collected in
    // one deterministic preorder walk each (source order, no set iteration).
    let mut includes = Vec::new();
    preorder(root, &mut |node| match node.kind() {
        "import_statement" | "export_statement" => {
            push_source_edge(node, path, source, &mut includes);
        }
        "call_expression" => push_require_edge(node, path, source, &mut includes),
        _ => {}
    });

    let mut structs = Vec::new();
    let mut global_vars = Vec::new();
    preorder(root, &mut |node| {
        if infer::is_class_kind(node.kind())
            && let Some(def) = class_def(node, path, source)
        {
            structs.push(def);
        }
        if matches!(node.kind(), "variable_declaration" | "lexical_declaration")
            && is_top_level_declaration(node)
        {
            push_globals(node, path, source, &mut global_vars);
        }
    });

    // Pass 2: functions. The synthetic module-scope record is emitted FIRST so
    // top-level statements are never invisible; real functions (including
    // nested ones and class-field arrows) follow in source order.
    let mut functions = Vec::new();
    functions.push(module_unit(root, path, source));
    preorder(root, &mut |node| {
        if infer::is_function_kind(node.kind()) {
            functions.push(function_unit(node, path, source));
        }
    });

    // Comment lines feed the LOC stat; the JS grammar lexes one `comment` kind
    // for both `//` and `/* ... */`, so string contents can never match.
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    preorder(root, &mut |node| {
        if node.kind() == "comment" {
            let range = node.byte_range();
            comment_ranges.push((range.start, range.end));
        }
    });
    comment_ranges.sort_unstable();

    let loc = loc_counts(source, &comment_ranges);
    let quoted = includes
        .iter()
        .filter(|edge| edge.form == IncludeForm::Quoted)
        .count();
    let angle = includes
        .iter()
        .filter(|edge| edge.form == IncludeForm::Angle)
        .count();
    let include_counts = IncludeCounts {
        quoted: u32::try_from(quoted).unwrap_or(u32::MAX),
        angle: u32::try_from(angle).unwrap_or(u32::MAX),
    };
    let file = FileRecord {
        path: path.to_string(),
        language: "js".to_string(),
        loc,
        function_count: u32::try_from(functions.len()).unwrap_or(u32::MAX),
        global_var_count: u32::try_from(global_vars.len()).unwrap_or(u32::MAX),
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
        macros: Vec::new(),
        functions,
    }
}

/// The pinned JavaScript grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_javascript::LANGUAGE.into()
}

fn failed_extraction(path: &str) -> Extraction {
    Extraction {
        file: FileRecord {
            path: path.to_string(),
            language: "js".to_string(),
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

/// Preorder walk over `node`'s entire subtree (Python backend parity).
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

/// Recursive preorder walk over `node`'s subtree.
fn preorder(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            preorder(cursor.node(), visit);
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

/// `import_statement` / `export_statement` `source` string → [`IncludeEdge`].
/// Only a literal `source` field yields an edge (dynamic/computed specifiers
/// are deliberately skipped).
fn push_source_edge(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<IncludeEdge>) {
    let Some(specifier) = node
        .child_by_field_name("source")
        .and_then(|s| infer::unquote(text(s, source)))
    else {
        return;
    };
    push_edge(out, path, &specifier, ir_span(node, source));
}

/// Static `require("…")` call → [`IncludeEdge`]. The callee must be exactly the
/// identifier `require` with one string-literal argument.
fn push_require_edge(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<IncludeEdge>) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if function.kind() != "identifier" || text(function, source) != "require" {
        return;
    }
    let Some(arguments) = node.child_by_field_name("arguments") else {
        return;
    };
    if arguments.kind() != "arguments" || arguments.named_child_count() != 1 {
        return;
    }
    let Some(argument) = arguments.named_child(0) else {
        return;
    };
    if argument.kind() != "string" {
        return;
    }
    let Some(specifier) = infer::unquote(text(argument, source)) else {
        return;
    };
    push_edge(out, path, &specifier, ir_span(node, source));
}

/// One raw include edge: `Quoted` for relative (`./`, `../`), `Angle`
/// otherwise; always unresolved (a later wiring wave owns JS resolution).
fn push_edge(out: &mut Vec<IncludeEdge>, path: &str, specifier: &str, span: Span) {
    if specifier.is_empty() {
        return;
    }
    let form = if specifier.starts_with("./") || specifier.starts_with("../") {
        IncludeForm::Quoted
    } else {
        IncludeForm::Angle
    };
    out.push(IncludeEdge {
        from_path: path.to_string(),
        to_path: Some(specifier.to_string()),
        form,
        resolved: false,
        is_mod: false,
        span,
    });
}

/// `class_declaration` / `class` → [`StructDef`]. Anonymous classes carry the
/// empty name. Members are `field_definition`/`public_field_definition`
/// properties only (methods are functions, not layout members).
fn class_def(node: Node<'_>, path: &str, source: &[u8]) -> Option<StructDef> {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    let mut members = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        if cursor.goto_first_child() {
            loop {
                let member = cursor.node();
                if matches!(
                    member.kind(),
                    "field_definition" | "public_field_definition"
                ) && let Some(property) = member.child_by_field_name("property")
                    && let Some(member_name) = member_name(property, source)
                {
                    members.push(StructMember {
                        name: member_name,
                        type_text: String::new(),
                        array_dim_literal: None,
                        is_pointer: false,
                    });
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    Some(StructDef {
        name,
        path: path.to_string(),
        span: ir_span(node, source),
        members,
        kind: StructKind::Struct,
    })
}

/// Class-field property name (`computed` keys have no static name).
fn member_name(property: Node<'_>, source: &[u8]) -> Option<String> {
    match property.kind() {
        "computed_property_name" => None,
        "string" => infer::unquote(text(property, source)),
        _ => {
            let name = text(property, source).trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        }
    }
}

/// A declaration is top-level iff its parent is the `program` or an
/// `export_statement` directly under it. This excludes declarations nested in
/// function bodies, class bodies, static blocks, and loop headers.
fn is_top_level_declaration(node: Node<'_>) -> bool {
    match node.parent() {
        Some(parent) if parent.kind() == "program" => true,
        Some(parent) if parent.kind() == "export_statement" => parent
            .parent()
            .is_some_and(|grandparent| grandparent.kind() == "program"),
        _ => false,
    }
}

/// Top-level declarators → [`GlobalVar`] (`StorageClass::None`).
fn push_globals(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<GlobalVar>) {
    let span = ir_span(node, source);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "variable_declarator"
                && let Some(name) = child.child_by_field_name("name")
            {
                for bound in infer::pattern_names(name, source) {
                    out.push(GlobalVar {
                        name: bound,
                        path: path.to_string(),
                        span,
                        storage: StorageClass::None,
                    });
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// The synthetic module-scope function: all top-level statements extracted as
/// if in one body, `is_module_scope = true`, name `"<module>"`.
fn module_unit(root: Node<'_>, path: &str, source: &[u8]) -> FunctionUnit {
    let span = ir_span(root, source);
    let no_names: Vec<String> = Vec::new();
    let result = scan::scan(
        root,
        ScanInputs {
            source,
            param_names: &no_names,
            local_names: &no_names,
        },
    );
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
        // v1.4/v1.5 whole-program fields are aggregator-owned: backends leave
        // them `None`.
        call_fanout: None,
        call_fanin: None,
        call_depth: None,
        is_recursive: None,
        idiom_hash: None,
        error_path: None,
        alloc_family: None,
        cfg_gated: None,
        is_static: None,
        // JS has no `__main__`-style guard; the JS-specific fact is
        // `entry_declared` below.
        has_main_guard: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
        // Plan `js-ts`: `Some(true)` for a proven module-entry declaration
        // idiom, `Some(false)` for a guard/declaration-less JS module.
        entry_declared: Some(infer::module_entry_declared(root, source)),
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
        allocations: result.allocations,
        unaries: result.unaries,
    }
}

/// One real function node (declaration, expression, arrow, method, generator,
/// or class-field arrow) → [`FunctionUnit`].
fn function_unit(node: Node<'_>, path: &str, source: &[u8]) -> FunctionUnit {
    let name = infer::derive_name(node, source);
    let params = function_params(node, source);
    let span = ir_span(node, source);

    let body = node.child_by_field_name("body");
    let mut local_names = params.names.clone();
    if let Some(body) = body {
        for local in infer::collect_local_names(body, source) {
            if !local_names.iter().any(|existing| existing == &local) {
                local_names.push(local);
            }
        }
    }

    let scan_start = body.unwrap_or(node);
    let result = scan::scan(
        scan_start,
        ScanInputs {
            source,
            param_names: &params.names,
            local_names: &local_names,
        },
    );

    let record = FunctionRecord {
        id: format!("{path}#{name}#{}:{}", span.start_line, span.start_col),
        name,
        path: path.to_string(),
        span,
        param_count: u32::try_from(params.types.len()).unwrap_or(u32::MAX),
        param_types: params.types,
        cyclomatic_complexity: result.cyclomatic,
        cognitive_complexity: result.cognitive,
        max_loop_depth: result.max_loop_depth,
        alloc_site_count: result.alloc_site_count,
        length_lines: span.end_line - span.start_line + 1,
        resolver_tier: ResolverTier::default(),
        is_async: infer::is_async_fn(node),
        is_module_scope: false,
        call_fanout: None,
        call_fanin: None,
        call_depth: None,
        is_recursive: None,
        idiom_hash: None,
        error_path: None,
        alloc_family: None,
        cfg_gated: None,
        is_static: None,
        has_main_guard: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
        // Entry-declaration facts live on the synthetic `<module>` record only.
        entry_declared: None,
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
        allocations: result.allocations,
        unaries: result.unaries,
    }
}

/// Parameters of any JS function shape: `formal_parameters` (declarations,
/// methods, parenthesized arrows) or the single-identifier `parameter` field of
/// an unparenthesized arrow.
fn function_params(node: Node<'_>, source: &[u8]) -> ParamInfo {
    if let Some(params) = node.child_by_field_name("parameters") {
        return infer::collect_params(params, source);
    }
    if let Some(param) = node.child_by_field_name("parameter")
        && let Some(name) = infer::pattern_name(param, source)
    {
        return ParamInfo {
            types: vec!["unknown".to_string()],
            names: vec![name],
        };
    }
    ParamInfo::default()
}
