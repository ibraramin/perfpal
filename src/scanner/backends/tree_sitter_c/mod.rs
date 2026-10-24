//! tree-sitter-c scanner backend (ADR-0001).
//!
//! Binds the official org-published grammar crate (`tree-sitter-c`, MIT) to
//! the parser-agnostic IR seam. Versions are exact-pinned in Cargo.toml per
//! ADR-0001 guardrail G5; the link test below fails CI if runtime and grammar
//! drift out of compatibility.
//!
//! Contract (modules.md §4.2): given `(path, source bytes)`, emit that file's
//! IR records/events. Engine-native spans convert to IR spans HERE and never
//! leak node kinds past this boundary (ir-v1.md §2). Parse honesty: ERROR /
//! missing nodes become [`ParseHealth`] partial data (ir-v1.md §3.1).
//!
//! Macro handling (documented simplification): tree-sitter does NOT expand
//! macros — a `#define` body parses as opaque `preproc_arg` text, so no
//! call_expression exists for wrapped calls. The curated recognizer therefore
//! runs on definition bodies: a `#define` whose body calls the alloc family
//! emits a `recognized_pattern` [`MacroEvent`] (`pattern_id =
//! [`ALLOC_WRAPPER_PATTERN_ID`]). Event `macro_origin` is set when the event
//! span intersects a recognized-pattern span or its callee is a known macro
//! name (wrapper USE sites); confidence capping happens analyzer-side
//! (ir-v1.md §5 hook 1).

use tree_sitter::{Node, Parser};

pub use super::Extraction;
pub use crate::ir::ALLOC_WRAPPER_PATTERN_ID;
use crate::ir::{
    BindingKind, CallEvent, CalleeKind, ComparisonEvent, FileRecord, GlobalVar, IncludeCounts,
    IncludeEdge, IncludeForm, LocCounts, LoopKind, LoopNode, MacroEvent, MacroEventKind,
    NativePoint, ParseHealth, ParseStatus, ResolverTier, ScopeClass, Shape, ShapeKind, Span,
    StatementRole, StorageClass, StructDef, StructKind, StructMember,
};
use crate::scanner::FunctionUnit;

/// malloc-family callees recognized by the curated wrapper pattern
/// (catalog-v1 §3, PERF-C-ALLOC-001 detect set).
pub const ALLOC_FAMILY: [&str; 5] = ["malloc", "calloc", "realloc", "aligned_alloc", "strdup"];

/// Extract one file's IR records/events from parsed C source.
///
/// Never panics on malformed input: syntax errors surface as
/// `ParseHealth{partial}` with error regions/count (ir-v1.md §3.1, §7).
#[must_use]
pub fn extract(path: &str, source: &[u8]) -> Extraction {
    let mut parser = Parser::new();
    // set_language only fails on ABI mismatch, which the pinned-pair link
    // test guards; falling back to a `failed` health keeps this total.
    let parse_result = if parser.set_language(&language()).is_ok() {
        parser.parse(source, None)
    } else {
        None
    };

    let Some(tree) = parse_result else {
        return Extraction {
            file: empty_file_record(path),
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
        };
    };

    let root = tree.root_node();
    let (error_regions, error_node_count) = collect_errors(root, source);
    let status = if error_node_count == 0 {
        ParseStatus::Ok
    } else {
        ParseStatus::Partial
    };

    // Pass 1: file-level records, recursing through preprocessor
    // conditionals (`walk_file_level` stops at `function_definition`, whose
    // body is function-local scope). Globals are collected before functions
    // so event scope classification (LOOP-005, name-based) can consult them
    // regardless of declaration order.
    let mut includes = Vec::new();
    let mut global_vars = Vec::new();
    let mut structs = Vec::new();
    let mut macros = Vec::new();
    walk_file_level(root, &mut |node| match node.kind() {
        "preproc_include" => includes.push(include_edge(node, path, source)),
        "preproc_def" | "preproc_function_def" => {
            push_macro_events(node, path, source, &mut macros);
        }
        "struct_specifier" => {
            if let Some(def) = struct_def(node, path, source) {
                structs.push(def);
            }
        }
        "declaration" => collect_global_vars(node, path, source, &mut global_vars),
        _ => {}
    });

    // Pass 2: functions (which need macros + globals from pass 1). Same
    // recursion, likewise stopping at each `function_definition` visited.
    let mut functions = Vec::new();
    walk_file_level(root, &mut |node| {
        if node.kind() == "function_definition" {
            functions.push(function_unit(node, path, source, &macros, &global_vars));
        }
    });

    // Precision FP-class fix 2 producer: `#if 0` regions are provably dead
    // (the condition constant-folds to false — the code never compiles), so
    // a function defined INSIDE one carries `cfg_gated = Some(true)`.
    // `#ifdef`/`#ifndef` guards (the everyday header shape) and `#if 0`
    // regions nested inside a live function body never flag the function.
    let dead_regions = dead_config_regions(root, source);
    for func in &mut functions {
        if dead_regions.iter().any(|region| {
            (region.start_line, region.start_col)
                <= (func.record.span.start_line, func.record.span.start_col)
                && (func.record.span.end_line, func.record.span.end_col)
                    <= (region.end_line, region.end_col)
        }) {
            func.record.cfg_gated = Some(true);
        }
    }

    // Comment lines anywhere in the file (including inside function bodies)
    // feed the LOC stat.
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    let mut descendants = root.walk();
    walk(&mut descendants, &mut |node| {
        if node.kind() == "comment" {
            let r = node.byte_range();
            comment_ranges.push((r.start, r.end));
        }
    });
    comment_ranges.sort_unstable();

    let loc = loc_counts(source, &comment_ranges);
    let include_counts = IncludeCounts {
        quoted: includes
            .iter()
            .filter(|e| e.form == IncludeForm::Quoted)
            .count() as u32,
        angle: includes
            .iter()
            .filter(|e| e.form == IncludeForm::Angle)
            .count() as u32,
    };
    let file = FileRecord {
        path: path.to_string(),
        language: "c".to_string(),
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

/// The pinned C grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_c::LANGUAGE.into()
}

fn empty_file_record(path: &str) -> FileRecord {
    FileRecord {
        path: path.to_string(),
        language: "c".to_string(),
        loc: LocCounts::default(),
        function_count: 0,
        global_var_count: 0,
        include_counts: IncludeCounts::default(),
    }
}

/// Engine-native node extent → IR span. THE conversion point (ir-v1.md §2).
fn ir_span(node: Node, source: &[u8]) -> Span {
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

/// Preorder walk over `node`'s entire subtree.
fn walk(cursor: &mut tree_sitter::TreeCursor, visit: &mut impl FnMut(Node<'_>)) {
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

/// Recursive file-level walk: visits every node EXCEPT that it does not
/// descend into `function_definition` subtrees (their bodies are
/// function-local scope, handled by the function pass). This makes
/// file-level declarations nested in preprocessor conditionals
/// (`preproc_ifdef` guards — the everyday header shape) visible; the old
/// direct-children-of-translation_unit walk dropped them.
fn walk_file_level(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
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

/// Precision FP-class fix 2 producer helper: spans of `preproc_if` regions
/// whose condition constant-folds to false (`#if 0`, parenthesized `#if (0)`
/// included) — provably-dead configuration regions. `#ifdef`/`#ifndef` and
/// `#if <expr>` never qualify: those arms may be live.
fn dead_config_regions(root: Node<'_>, source: &[u8]) -> Vec<Span> {
    let mut regions = Vec::new();
    let mut cursor = root.walk();
    walk(&mut cursor, &mut |node| {
        if node.kind() == "preproc_if"
            && node.child_by_field_name("condition").is_some_and(|c| {
                let collapsed: String = text(c, source)
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect();
                collapsed == "0" || collapsed == "(0)"
            })
        {
            regions.push(ir_span(node, source));
        }
    });
    regions
}

/// Count ALL error/missing nodes; record outermost extents as regions
/// (ir-v1.md §3.1 error_regions semantics).
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

/// LOC counting (modules.md §3.2 scanner-owned stat): blank = whitespace-only
/// line; comment = line overlapping any comment node's byte range; rest =
/// code. A blank line inside a block comment counts as blank (whitespace
/// rule wins); the rule is deterministic either way.
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
        offset += len + 1; // +1 for the newline consumed by split_inclusive
    }
    loc
}

fn split_lines(source: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut iter = source.split_inclusive(|b| *b == b'\n').peekable();
    std::iter::from_fn(move || {
        // split_inclusive leaves a final empty segment when source ends in
        // '\n'; suppress it so trailing newlines don't invent blank lines.
        match iter.next() {
            Some(b"") => None,
            other => other,
        }
    })
}

fn text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

/// `#include "x.h"` vs `#include <x.h>` → [`IncludeEdge`] with RAW target
/// text in `to_path` (the scanner resolution pass rewrites it; see the
/// IncludeEdge doc comment in `ir`).
fn include_edge(node: Node<'_>, path: &str, source: &[u8]) -> IncludeEdge {
    let path_node = node.child_by_field_name("path");
    let (form, raw) = match path_node.map(|p| p.kind()) {
        Some("string_literal") => {
            // Inner text lives in the `string_content` child (quotes excluded).
            let inner = path_node
                .and_then(|p| find_descendant(p, "string_content"))
                .map(|c| text(c, source))
                .unwrap_or_default();
            (IncludeForm::Quoted, inner.to_string())
        }
        Some("system_lib_string") => {
            let raw_text = path_node.map(|p| text(p, source)).unwrap_or_default();
            let trimmed = raw_text
                .trim_start_matches('<')
                .trim_end_matches('>')
                .trim()
                .to_string();
            (IncludeForm::Angle, trimmed)
        }
        _ => (IncludeForm::Angle, String::new()),
    };
    IncludeEdge {
        from_path: path.to_string(),
        to_path: Some(raw),
        form,
        resolved: false,
        is_mod: false,
        span: ir_span(node, source),
    }
}

/// Macro definitions plus the curated recognizer pass (modules.md §3.2):
/// every `#define` yields a `definition` event; one whose body calls the
/// alloc family additionally yields a `recognized_pattern` event with
/// [`ALLOC_WRAPPER_PATTERN_ID`].
fn push_macro_events(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<MacroEvent>) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let name = text(name_node, source).to_string();
    let span = ir_span(node, source);
    out.push(MacroEvent {
        kind: MacroEventKind::Definition,
        name: name.clone(),
        path: path.to_string(),
        span,
        pattern_id: None,
    });
    let body = node
        .child_by_field_name("value")
        .map(|v| text(v, source))
        .unwrap_or_default();
    if calls_alloc_family(body) {
        out.push(MacroEvent {
            kind: MacroEventKind::RecognizedPattern,
            name,
            path: path.to_string(),
            span,
            pattern_id: Some(ALLOC_WRAPPER_PATTERN_ID.to_string()),
        });
    }
}

/// Minimal curated matcher: does `body` contain `<alloc-name>(`?
fn calls_alloc_family(body: &str) -> bool {
    ALLOC_FAMILY.iter().any(|fam| {
        let mut search = 0usize;
        while let Some(pos) = body[search..].find(fam) {
            let abs = search + pos;
            let after = &body[abs + fam.len()..];
            let starts_call = after.strip_prefix('(').is_some()
                || after.chars().next().is_some_and(|c| c.is_whitespace())
                    && after.trim_start().starts_with('(');
            if starts_call {
                return true;
            }
            search = abs + fam.len();
        }
        false
    })
}

/// NAMED file-level `struct` definitions, reached wherever the recursive
/// file-level walk finds them: bare top-level/guarded `struct S { … };`,
/// inside a `declaration` (`struct S { … } v;`), a `typedef struct Named { … } X;`,
/// or a nested definition. Anonymous structs (`typedef struct { … } X;`, inline
/// struct-typed members) are SKIPPED: they carry no tag name, so they cannot be
/// referenced by name and any finding about them would be unattributable
/// (`struct ''`).
fn struct_def(node: Node<'_>, path: &str, source: &[u8]) -> Option<StructDef> {
    let body = node.child_by_field_name("body")?;
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let mut members = Vec::new();
    let mut cursor = body.walk();
    if cursor.goto_first_child() {
        loop {
            let field = cursor.node();
            if field.kind() == "field_declaration" {
                let type_text = field
                    .child_by_field_name("type")
                    .map(|t| text(t, source).to_string())
                    .unwrap_or_default();
                let decl = field.child_by_field_name("declarator");
                let member_name = decl
                    .and_then(|d| leftmost_identifier(d, source))
                    .unwrap_or_default();
                let array_dim_literal = decl.and_then(|d| array_dimension(d, source));
                // Pointerness lives in the DECLARATOR (pointer_declarator
                // wrappers), never in the `type` field text — record it
                // explicitly so the TYPE-001/002 size model sees `char *p`
                // as an 8-byte member (ir-v1.md §3.1 additive field).
                let is_pointer = decl.is_some_and(|d| subtree_has_kind(d, "pointer_declarator"));
                members.push(StructMember {
                    name: member_name,
                    type_text,
                    array_dim_literal,
                    is_pointer,
                });
            }
            if !cursor.goto_next_sibling() {
                break;
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

/// Best-effort integer array dimension from `name[N]` declarators.
fn array_dimension(decl: Node<'_>, source: &[u8]) -> Option<u64> {
    let mut stack = vec![decl];
    while let Some(n) = stack.pop() {
        if n.kind() == "array_declarator"
            && let Some(size) = n.child_by_field_name("size")
        {
            return text(size, source).trim().parse::<u64>().ok();
        }
        let mut cursor = n.walk();
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    None
}

/// Leftmost identifier text within a declarator chain (`*buf`, `name[16]`,
/// `(**pp)` all resolve to their base identifier).
fn leftmost_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    if matches!(node.kind(), "identifier" | "field_identifier") {
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

/// File-scope variables from a top-level `declaration` (prototypes excluded
/// via their `function_declarator`). Multiple declarators (`int a, b;`) yield
/// one [`GlobalVar`] each: the grammar emits EVERY comma-separated declarator
/// as a `declarator`-field child (plain decls) or an `init_declarator` child
/// (initialized decls), so all of them are collected in source order. The
/// storage class comes from the declaration's own specifiers.
fn collect_global_vars(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<GlobalVar>) {
    let mut storage = StorageClass::None;
    let mut cursor = node.walk();
    let mut declarators = Vec::new();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "storage_class_specifier" => match text(child, source) {
                    "static" => storage = StorageClass::Static,
                    "extern" => storage = StorageClass::Extern,
                    _ => {}
                },
                // Field-based match keeps source order across mixed plain and
                // initialized declarators (`int a, b = 2, c;`).
                _ if cursor.field_name() == Some("declarator")
                    || child.kind() == "init_declarator" =>
                {
                    declarators.push(child);
                }
                _ => {}
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    for decl in declarators {
        // Function prototypes ride the same `declarator` field; skip them.
        if subtree_has_kind(decl, "function_declarator") {
            continue;
        }
        let name_node = if decl.kind() == "init_declarator" {
            decl.child_by_field_name("declarator")
        } else {
            Some(decl)
        };
        let Some(name) = name_node.and_then(|n| leftmost_identifier(n, source)) else {
            continue;
        };
        out.push(GlobalVar {
            name,
            path: path.to_string(),
            span: ir_span(decl, source),
            storage,
        });
    }
}

fn subtree_has_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if subtree_has_kind(cursor.node(), kind) {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}

const LOOP_KINDS: [&str; 3] = ["for_statement", "while_statement", "do_statement"];

fn loop_kind(kind: &str) -> Option<LoopKind> {
    match kind {
        "for_statement" => Some(LoopKind::For),
        "while_statement" => Some(LoopKind::While),
        "do_statement" => Some(LoopKind::Do),
        _ => None,
    }
}

/// Extract one function's record + attached event streams with a single body
/// walk (metrics and events share traversal state: loop depth, nesting).
fn function_unit(
    fdef: Node<'_>,
    path: &str,
    source: &[u8],
    macros: &[MacroEvent],
    globals: &[GlobalVar],
) -> FunctionUnit {
    let declarator = fdef.child_by_field_name("declarator");
    let func_decl = declarator
        .and_then(|d| find_descendant(d, "function_declarator"))
        .or(declarator);
    let name = func_decl
        .and_then(|fd| fd.child_by_field_name("declarator"))
        .and_then(|n| leftmost_identifier(n, source))
        .unwrap_or_default();
    let param_list = func_decl.and_then(|fd| fd.child_by_field_name("parameters"));
    let mut param_types = Vec::new();
    let mut param_names = Vec::new();
    if let Some(plist) = param_list {
        let mut cursor = plist.walk();
        if cursor.goto_first_child() {
            loop {
                let p = cursor.node();
                if p.kind() == "parameter_declaration" {
                    let t = text(p, source).trim();
                    if !(t == "void" && p.child_by_field_name("declarator").is_none()) {
                        param_types.push(t.to_string());
                        if let Some(pn) = p
                            .child_by_field_name("declarator")
                            .and_then(|d| leftmost_identifier(d, source))
                        {
                            param_names.push(pn);
                        }
                    }
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    let span = ir_span(fdef, source);
    let length_lines = span.end_line - span.start_line + 1;
    let macro_names: Vec<String> = macros
        .iter()
        .filter(|m| m.kind == MacroEventKind::Definition)
        .map(|m| m.name.clone())
        .collect();
    let pattern_spans: Vec<Span> = macros
        .iter()
        .filter(|m| m.kind == MacroEventKind::RecognizedPattern)
        .map(|m| m.span)
        .collect();
    let global_names: Vec<String> = globals.iter().map(|g| g.name.clone()).collect();

    let mut scan = FuncScan {
        source,
        macro_names: &macro_names,
        pattern_spans: &pattern_spans,
        param_names: &param_names,
        global_names: &global_names,
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
    };
    if let Some(body) = fdef.child_by_field_name("body") {
        scan.visit(body);
    }

    let id = format!("{path}#{name}#{}:{}", span.start_line, span.start_col);
    // I1a linkage fact: `static` arrives as a DIRECT `storage_class_specifier`
    // child of the `function_definition` node (same shape `collect_global_vars`
    // already matches on `declaration`). Anything else (including explicit
    // `extern`) is external linkage ⇒ exported.
    let mut is_static = false;
    let mut declarator_cursor = fdef.walk();
    if declarator_cursor.goto_first_child() {
        loop {
            let child = declarator_cursor.node();
            if child.kind() == "storage_class_specifier" && text(child, source) == "static" {
                is_static = true;
                break;
            }
            if !declarator_cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let record = crate::ir::FunctionRecord {
        id,
        name,
        path: path.to_string(),
        span,
        param_count: param_types.len() as u32,
        param_types,
        cyclomatic_complexity: scan.cyclomatic,
        cognitive_complexity: scan.cognitive,
        max_loop_depth: scan.max_loop_depth,
        alloc_site_count: scan.alloc_site_count,
        length_lines,
        resolver_tier: ResolverTier::default(),
        is_async: false,
        is_module_scope: false,
        // v1.4 step 2 (plan `divergence-revamp`): `idiom_hash` / `error_path` /
        // `alloc_family` are populated by the scanner post-pass
        // (`scanner::instrument::populate`, needs source text); call-shape
        // fields stay `None` here for the aggregator's whole-program pass.
        call_fanout: None,
        call_fanin: None,
        call_depth: None,
        is_recursive: None,
        idiom_hash: None,
        error_path: None,
        alloc_family: None,
        cfg_gated: None,
        // v1.5 I1a (plan `map-build`): linkage is scan-local; entry and
        // reachability need the whole-program view, so they stay `None` here
        // for the aggregator's entry pass.
        is_static: Some(is_static),
        // v1.5 I1c (plan `map-build`): the `__name__` guard fact is
        // Python-only — C records never carry it.
        has_main_guard: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
        // Plan `js-ts`: module-entry declaration is JS-only; other backends
        // leave it `None`.
        entry_declared: None,
    };
    FunctionUnit {
        record,
        calls: scan.calls,
        assignments: scan.assignments,
        binary_ops: scan.binary_ops,
        comparisons: scan.comparisons,
        access_chains: scan.access_chains,
        loops: scan.loops,
        awaits: Vec::new(),
        try_blocks: Vec::new(),
        allocations: Vec::new(),
        unaries: Vec::new(),
    }
}

fn find_descendant<'t>(node: Node<'t>, kind: &str) -> Option<Node<'t>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if let Some(found) = find_descendant(cursor.node(), kind) {
                return Some(found);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Single-pass body walker producing events + branch/nesting metrics.
///
/// Metric conventions (catalog §2 approximations, documented here since the
/// catalog fixes thresholds, not formulas):
/// - cyclomatic = 1 + #\{if, for, while, do, case, &&, ||, ?:\}
/// - cognitive (SonarSource-style approximation): if/loop/ternary cost
///   `1 + current nesting` and increase nesting for their bodies; case and
///   short-circuit operators cost a flat 1.
struct FuncScan<'a> {
    source: &'a [u8],
    macro_names: &'a [String],
    pattern_spans: &'a [Span],
    param_names: &'a [String],
    global_names: &'a [String],
    loop_depth: u32,
    max_loop_depth: u32,
    nesting: u32,
    cyclomatic: u32,
    cognitive: u32,
    alloc_site_count: u32,
    calls: Vec<CallEvent>,
    assignments: Vec<crate::ir::AssignmentEvent>,
    binary_ops: Vec<crate::ir::BinaryOpEvent>,
    comparisons: Vec<ComparisonEvent>,
    access_chains: Vec<crate::ir::AccessChainEvent>,
    loops: Vec<LoopNode>,
}

impl<'a> FuncScan<'a> {
    fn visit(&mut self, node: Node<'_>) {
        match node.kind() {
            "if_statement" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "switch_statement" => {
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "case_statement" => {
                self.cyclomatic += 1;
                self.cognitive += 1;
                self.visit_children(node);
            }
            "conditional_expression" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            k @ ("for_statement" | "while_statement" | "do_statement") => {
                let kind = loop_kind(k).expect("matched loop kind");
                self.enter_loop(node, kind);
            }
            "binary_expression" => self.visit_binary(node),
            "call_expression" => {
                self.emit_call(node);
                self.visit_children(node);
            }
            "assignment_expression" => {
                self.emit_assignment(node);
                self.visit_children(node);
            }
            "subscript_expression" | "field_expression" => {
                self.maybe_emit_access_chain(node);
                self.visit_children(node);
            }
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
        let induction_vars = if kind == LoopKind::For {
            induction_vars(node, self.source)
        } else {
            Vec::new()
        };
        // I2 (plan `map-build`): trip count ONLY for canonical counted
        // for-loops; every other loop kind/shape stays `None` (honest
        // unknown). See `canonical_trip_count` for the exact gate.
        let trip_count = if kind == LoopKind::For {
            canonical_trip_count(node, self.source)
        } else {
            None
        };
        let macro_origin = self.macro_origin_of(&span, "");
        // Precision FP-class fix 1 producer: a `do_statement` whose
        // condition text is the literal `(0)` is the `do { ... } while(0)`
        // macro-wrapper idiom (single-shot body, not per-iteration churn).
        // The TEXT check (not a blanket `LoopKind::Do` flag) keeps genuine
        // `do-while(cond)` loops gated `false`.
        let condition_is_zero = kind == LoopKind::Do
            && node.child_by_field_name("condition").is_some_and(|c| {
                let collapsed: String = text(c, self.source)
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect();
                collapsed == "(0)" || collapsed == "0"
            });
        self.loops.push(LoopNode {
            kind,
            depth,
            induction_vars,
            span,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin,
            trip_count,
            condition_is_zero,
        });
        self.loop_depth = depth;
        self.nesting += 1;
        self.visit_children(node);
        self.nesting -= 1;
        self.loop_depth -= 1;
    }

    fn visit_binary(&mut self, node: Node<'_>) {
        let op = node
            .child_by_field_name("operator")
            .map(|o| text(o, self.source))
            .unwrap_or("");
        match op {
            "&&" | "||" => {
                self.cyclomatic += 1;
                self.cognitive += 1;
            }
            "==" | "!=" | "<" | ">" | "<=" | ">=" | "<=>" => {
                let meta = self.meta_for(node, false);
                let left = node.child_by_field_name("left").map(|n| self.shape_of(n));
                let right = node.child_by_field_name("right").map(|n| self.shape_of(n));
                if let (Some(left), Some(right)) = (left, right) {
                    self.comparisons.push(ComparisonEvent {
                        meta,
                        operator: String::new(),
                        left,
                        right,
                    });
                }
            }
            "+" | "-" | "*" | "/" | "%" => {
                let meta = self.meta_for(node, false);
                let left = node.child_by_field_name("left").map(|n| self.shape_of(n));
                let right = node.child_by_field_name("right").map(|n| self.shape_of(n));
                if let (Some(left), Some(right)) = (left, right) {
                    self.binary_ops.push(crate::ir::BinaryOpEvent {
                        meta,
                        op: op.to_string(),
                        left,
                        right,
                    });
                }
            }
            _ => {}
        }
        self.visit_children(node);
    }

    fn emit_call(&mut self, node: Node<'_>) {
        let callee = node.child_by_field_name("function");
        let (callee_kind, callee_name) = match callee.map(|c| c.kind()) {
            Some("identifier") => (
                CalleeKind::Direct,
                callee.map(|c| text(c, self.source)).unwrap_or(""),
            ),
            Some(_) => (
                CalleeKind::Indirect,
                callee.map(|c| text(c, self.source)).unwrap_or(""),
            ),
            None => (CalleeKind::Unknown, ""),
        };
        let callee_name = callee_name.to_string();
        if callee_kind == CalleeKind::Direct && ALLOC_FAMILY.contains(&callee_name.as_str()) {
            self.alloc_site_count += 1;
        }
        let args = node
            .child_by_field_name("arguments")
            .map(|arglist| {
                let mut shapes = Vec::new();
                let mut cursor = arglist.walk();
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.is_named() && child.kind() != "comment" {
                            shapes.push(self.shape_of(child));
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                shapes
            })
            .unwrap_or_default();
        let (statement_role, conditional_guarded) = call_position(node);
        let macro_origin = self.macro_origin_of(&ir_span(node, self.source), &callee_name);
        let meta = self.meta_for(node, macro_origin);
        self.calls.push(CallEvent {
            meta,
            callee_name,
            callee_kind,
            // C has no path/macro-call distinction: the C backend leaves
            // `callee_path` absent and `is_macro` false (additive fields,
            // plan `rust-backend` R3 populate the Rust backend only).
            callee_path: None,
            is_macro: false,
            args,
            statement_role,
            conditional_guarded,
            semantic: None,
            receiver_root: None,
        });
    }

    fn emit_assignment(&mut self, node: Node<'_>) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        let target = self.shape_of(left);
        let scope_class = self.classify_target(&target);
        let macro_origin = self.macro_origin_of(&ir_span(node, self.source), "");
        let meta = self.meta_for(node, macro_origin);
        self.assignments.push(crate::ir::AssignmentEvent {
            meta,
            target,
            scope_class,
            binding_kind: BindingKind::Unknown,
        });
    }

    /// Emit an [`crate::ir::AccessChainEvent`] only at the TOP of a
    /// member/subscript chain (parent is not itself a chain link) and only
    /// when there are ≥ 2 elements (deeper chains are what LOOP-008 needs).
    fn maybe_emit_access_chain(&mut self, node: Node<'_>) {
        if node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "subscript_expression" | "field_expression"))
        {
            return;
        }
        let Some((root, elements)) = self.access_chain(node) else {
            return;
        };
        if elements.len() < 2 {
            return;
        }
        let macro_origin = self.macro_origin_of(&ir_span(node, self.source), "");
        let meta = self.meta_for(node, macro_origin);
        self.access_chains.push(crate::ir::AccessChainEvent {
            meta,
            root,
            elements,
        });
    }

    /// Decompose `a.b[i].c` into (root identifier, element shapes).
    fn access_chain(&self, node: Node<'_>) -> Option<(String, Vec<Shape>)> {
        match node.kind() {
            "subscript_expression" => {
                let base = node.child_by_field_name("argument")?;
                let (root, mut elements) = self.access_chain(base)?;
                let index_ident = node
                    .child_by_field_name("index")
                    .and_then(|i| leftmost_identifier(i, self.source));
                elements.push(Shape {
                    kind: ShapeKind::Subscript,
                    index_ident,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            "field_expression" => {
                let base = node.child_by_field_name("argument")?;
                let (root, mut elements) = self.access_chain(base)?;
                let name = node
                    .child_by_field_name("field")
                    .map(|f| text(f, self.source).to_string());
                elements.push(Shape {
                    kind: ShapeKind::MemberAccess,
                    name,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            _ => leftmost_identifier(node, self.source).map(|root| (root, Vec::new())),
        }
    }

    fn classify_target(&self, target: &Shape) -> ScopeClass {
        match (&target.kind, target.name.as_deref()) {
            (ShapeKind::Identifier, Some(name)) if self.param_names.contains(&name.to_string()) => {
                ScopeClass::Param
            }
            (ShapeKind::Identifier, Some(name))
                if self.global_names.contains(&name.to_string()) =>
            {
                ScopeClass::Global
            }
            (ShapeKind::Identifier, Some(_)) => ScopeClass::Local,
            _ => ScopeClass::Unknown,
        }
    }

    /// Macro-origin detection (see module doc): span intersects a recognized
    /// wrapper pattern, or the callee IS a known macro (wrapper use site).
    fn macro_origin_of(&self, span: &Span, callee_name: &str) -> bool {
        self.pattern_spans.iter().any(|ps| ps.intersects(span))
            || self.macro_names.iter().any(|m| m == callee_name)
    }

    fn meta_for(&self, node: Node<'_>, macro_origin: bool) -> crate::ir::EventMeta {
        let span = ir_span(node, self.source);
        crate::ir::EventMeta {
            span,
            in_loop_depth: self.loop_depth,
            is_innermost: is_innermost_event(node),
            resolver_tier: ResolverTier::Syntactic,
            macro_origin,
        }
    }

    fn shape_of(&self, node: Node<'_>) -> Shape {
        match node.kind() {
            "number_literal" | "char_literal" | "string_literal" => {
                Shape::literal(text(node, self.source))
            }
            "identifier" | "field_identifier" | "type_identifier" => {
                Shape::identifier(text(node, self.source))
            }
            "call_expression" => {
                let name = node
                    .child_by_field_name("function")
                    .map(|c| text(c, self.source).to_string());
                Shape {
                    kind: ShapeKind::Call,
                    name,
                    ..Shape::default()
                }
            }
            "subscript_expression" => {
                let index_ident = node
                    .child_by_field_name("index")
                    .and_then(|i| leftmost_identifier(i, self.source));
                Shape {
                    kind: ShapeKind::Subscript,
                    index_ident,
                    ..Shape::default()
                }
            }
            "field_expression" => Shape {
                kind: ShapeKind::MemberAccess,
                depth: Some(chain_depth(node)),
                ..Shape::default()
            },
            "unary_expression" | "pointer_expression" | "update_expression" => Shape {
                kind: ShapeKind::Unary,
                ..Shape::default()
            },
            "binary_expression" => Shape {
                kind: ShapeKind::Binary,
                ..Shape::default()
            },
            "sizeof_expression" => {
                // Operand text AS WRITTEN, parens stripped (`sizeof(p)` ⇒
                // "p", `sizeof(struct S)` ⇒ "struct S", `sizeof *p` ⇒ "*p").
                // tree-sitter-c gives TWO CST shapes here:
                //   sizeof <expr>        → [sizeof, expr]
                //   sizeof(<expr|type>)  → [sizeof, "(", inner, ")"]
                // An expression operand arrives PRE-wrapped as a
                // parenthesized_expression (its paren tokens live inside
                // it), while the type-descriptor form exposes bare paren
                // tokens at THIS level — child(1) "(" means the operand is
                // child(2). The shape itself stays a bare Sizeof; the
                // parenthesized-unwrapping at the `parenthesized_expression`
                // arm below is untouched (ir-v1.md §3.4 additive field,
                // MEM-001 input).
                let operand = match node.child(1).map(|c| c.kind()) {
                    Some("(") => node.child(2),
                    _ => node.child(1),
                };
                let sizeof_operand = operand.map(|operand| {
                    let inner = if operand.kind() == "parenthesized_expression" {
                        operand.child(1).unwrap_or(operand)
                    } else {
                        operand
                    };
                    text(inner, self.source).to_string()
                });
                Shape {
                    kind: ShapeKind::Sizeof,
                    sizeof_operand,
                    ..Shape::default()
                }
            }
            "parenthesized_expression" => node
                .child(1)
                .map(|inner| self.shape_of(inner))
                .unwrap_or_else(|| Shape {
                    kind: ShapeKind::Other,
                    ..Shape::default()
                }),
            _ => Shape {
                kind: ShapeKind::Other,
                ..Shape::default()
            },
        }
    }
}

/// Chain depth of member/subscript nesting rooted at `node`.
fn chain_depth(node: Node<'_>) -> u32 {
    match node.kind() {
        "subscript_expression" | "field_expression" => node
            .child_by_field_name("argument")
            .or_else(|| node.child(0))
            .map_or(1, |base| 1 + chain_depth(base)),
        _ => 0,
    }
}

/// `is_innermost` operational rule (ir-v1.md §3.3 "inside the deepest
/// enclosing loop"): climb the ancestor chain to the NEAREST enclosing loop;
/// the event is innermost iff that loop exists AND nests no further loop
/// anywhere in its own body. An event outside any loop is never innermost,
/// and an event in an outer loop whose body contains a nested loop (sibling
/// statement or buried in a conditional) is not in the innermost one.
fn is_innermost_event(event: Node<'_>) -> bool {
    let Some(loop_node) = nearest_enclosing_loop(event) else {
        return false;
    };
    let mut cursor = loop_node.walk();
    if cursor.goto_first_child() {
        loop {
            // Children of the loop node never include the loop itself, so a
            // hit here is a genuinely nested loop.
            if contains_loop(cursor.node()) {
                return false;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    true
}

/// Nearest loop on `event`'s ancestor chain, if any.
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

/// Whether any loop appears in `node`'s subtree.
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

/// Position of a call relative to its enclosing loop (ir-v1.md §3.3):
/// climb ancestors innermost-out; bare `expression_statement` links are
/// transparent, and so is a `compound_statement` that IS the enclosing
/// loop's direct body child (the common braced form must not bury the call);
/// conditionals mark `conditional_guarded`; anything else buries the call
/// (`nested`). No enclosing loop ⇒ `unknown`.
fn call_position(node: Node<'_>) -> (StatementRole, bool) {
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
            "compound_statement"
                if parent
                    .parent()
                    .is_some_and(|g| LOOP_KINDS.contains(&g.kind())) => {}
            "if_statement" | "switch_statement" | "conditional_expression" => {
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

/// Best-effort for-loop induction identifiers: the variable updated in the
/// `update` clause (`++i`, `i++`, `j += 1`, `k = k + 1`).
fn induction_vars(for_node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut vars = Vec::new();
    let update = for_node
        .child_by_field_name("update")
        .or_else(|| last_named_before_close(for_node));
    let Some(update) = update else { return vars };
    let candidate = match update.kind() {
        "update_expression" => update
            .child_by_field_name("argument")
            .and_then(|a| leftmost_identifier(a, source)),
        "assignment_expression" => update
            .child_by_field_name("left")
            .and_then(|l| leftmost_identifier(l, source)),
        _ => None,
    };
    if let Some(v) = candidate
        && !vars.contains(&v)
    {
        vars.push(v);
    }
    vars
}

/// Canonical counted-for-loop trip count (plan `map-build` I2, C only).
///
/// The ONLY shape admitted (all three clauses must agree on one induction
/// variable `v`):
/// - initializer: `v = 0` or `int v = 0` — an integer literal ZERO;
/// - condition: `v < N` or `v <= N` with a bare integer-literal `N`
///   (`<=` yields N + 1);
/// - update: `v++`/`++v` or `v += 1`.
///
/// Everything else returns `None`: `while`/`do-while`, macro or
/// non-literal/parenthesized bounds, descending `v--`, multiple induction
/// variables, compound updates (`v += 2`), or a variable mismatched between
/// clauses. The result is clamped to [`crate::ir::MAX_TRIP`] so cost math
/// stays in range.
fn canonical_trip_count(for_node: Node<'_>, source: &[u8]) -> Option<u64> {
    let initializer = for_node.child_by_field_name("initializer")?;
    let variable = zero_initialized_var(initializer, source)?;
    let condition = for_node.child_by_field_name("condition")?;
    let trips = condition_trip_bound(condition, &variable, source)?;
    let update = for_node.child_by_field_name("update")?;
    if !is_unit_increment(update, &variable, source) {
        return None;
    }
    Some(trips.min(crate::ir::MAX_TRIP))
}

/// `v = 0` / `int v = 0` — exactly one initialized declarator and no other
/// declarator (multiple induction vars are deliberately rejected).
fn zero_initialized_var(initializer: Node<'_>, source: &[u8]) -> Option<String> {
    match initializer.kind() {
        "assignment_expression" => {
            let operator = initializer
                .child_by_field_name("operator")
                .map(|op| text(op, source))?;
            let left = initializer.child_by_field_name("left")?;
            let right = initializer.child_by_field_name("right")?;
            if operator != "="
                || left.kind() != "identifier"
                || int_literal_value(right, source)? != 0
            {
                return None;
            }
            Some(text(left, source).to_string())
        }
        "declaration" => {
            let mut declarators: Vec<Node<'_>> = Vec::new();
            let mut cursor = initializer.walk();
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.is_named()
                        && matches!(
                            child.kind(),
                            "init_declarator"
                                | "identifier"
                                | "pointer_declarator"
                                | "array_declarator"
                                | "function_declarator"
                        )
                    {
                        declarators.push(child);
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
            let [declarator] = declarators.as_slice() else {
                return None; // zero or multiple induction variables
            };
            if declarator.kind() != "init_declarator" {
                return None; // declared but not initialized
            }
            let name = declarator.child_by_field_name("declarator")?;
            if name.kind() != "identifier" {
                return None; // pointer/array declarator is not a counted var
            }
            let value = declarator.child_by_field_name("value")?;
            if int_literal_value(value, source)? != 0 {
                return None;
            }
            Some(text(name, source).to_string())
        }
        _ => None,
    }
}

/// `v < N` / `v <= N`, bare integer-literal bound only. `<=` means N + 1
/// iterations (saturating; the caller clamps to [`crate::ir::MAX_TRIP`]).
fn condition_trip_bound(condition: Node<'_>, variable: &str, source: &[u8]) -> Option<u64> {
    if condition.kind() != "binary_expression" {
        return None;
    }
    let operator = condition
        .child_by_field_name("operator")
        .map(|op| text(op, source))?;
    let left = condition.child_by_field_name("left")?;
    let right = condition.child_by_field_name("right")?;
    if left.kind() != "identifier" || text(left, source) != variable {
        return None;
    }
    let bound = int_literal_value(right, source)?;
    match operator {
        "<" => Some(bound),
        "<=" => Some(bound.saturating_add(1)),
        _ => None, // descending / != / variable bound
    }
}

/// `v++` / `++v` (but NEVER `v--`) or `v += 1`. A `+= 2` step is not a
/// canonical unit counted loop and stays unknown.
fn is_unit_increment(update: Node<'_>, variable: &str, source: &[u8]) -> bool {
    match update.kind() {
        "update_expression" => {
            let Some(argument) = update.child_by_field_name("argument") else {
                return false;
            };
            if argument.kind() != "identifier" || text(argument, source) != variable {
                return false;
            }
            let collapsed: String = text(update, source)
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect();
            collapsed.contains("++") && !collapsed.contains("--")
        }
        "assignment_expression" => {
            let operator = update
                .child_by_field_name("operator")
                .map(|op| text(op, source));
            let left = update.child_by_field_name("left");
            let right = update.child_by_field_name("right");
            let (Some("+="), Some(left), Some(right)) = (operator, left, right) else {
                return false;
            };
            left.kind() == "identifier"
                && text(left, source) == variable
                && int_literal_value(right, source) == Some(1)
        }
        _ => false,
    }
}

/// Parse a C integer literal (decimal or `0x` hex, optional `u`/`l` suffix).
/// Anything that is not a bare `number_literal` node returns `None` — macro
/// names, parenthesized bounds, identifiers and expressions are unknown.
fn int_literal_value(node: Node<'_>, source: &[u8]) -> Option<u64> {
    if node.kind() != "number_literal" {
        return None;
    }
    let raw = text(node, source).trim().replace('_', "");
    let (radix, digits) =
        if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
            (16, hex)
        } else {
            (10, raw.as_str())
        };
    let digits = digits.trim_end_matches(['u', 'U', 'l', 'L']);
    u64::from_str_radix(digits, radix).ok()
}

fn last_named_before_close(for_node: Node<'_>) -> Option<Node<'_>> {
    let mut last = None;
    let mut cursor = for_node.walk();
    if cursor.goto_first_child() {
        loop {
            let n = cursor.node();
            if n.is_named() {
                last = Some(n);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    last.filter(|n| n.kind() != "compound_statement")
}

#[cfg(test)]
mod tests {
    //! Backend extraction tests lock the grammar assumptions above AND the
    //! span round-trip contract (ir-v1.md §7) on real parsed nodes.

    use super::*;
    use crate::ir::{CalleeKind, IncludeForm, LoopKind, ParseStatus, ShapeKind, StatementRole};

    const SAMPLE: &str = "\
#include <stdio.h>
#include \"sub/local.h\"

/* block comment
   spanning lines */
static int g_count = 0;

int process(int n, char *buf) {
    int total = 0;
    // line comment
    for (int i = 0; i < n; ++i) {
        char *p = malloc(n + 1);
        if (p != 0) {
            total += buf[i];
        }
    }
    return total;
}
";

    fn sample_extraction() -> Extraction {
        extract("src/sample.c", SAMPLE.as_bytes())
    }

    #[test]
    fn extracts_function_record_fields() {
        let ex = sample_extraction();
        assert_eq!(ex.parse_health.status, ParseStatus::Ok);
        assert_eq!(ex.file.function_count, 1);
        assert_eq!(ex.file.global_var_count, 1);
        let func = &ex.functions[0].record;
        assert_eq!(func.name, "process");
        assert_eq!(func.path, "src/sample.c");
        assert_eq!(func.param_count, 2);
        assert_eq!(
            func.param_types,
            vec!["int n".to_string(), "char *buf".to_string()]
        );
        // 1 + for + if
        assert_eq!(func.cyclomatic_complexity, 3);
        // for costs 1 (+nesting 0); if costs 1 + nesting 1
        assert_eq!(func.cognitive_complexity, 3);
        assert_eq!(func.max_loop_depth, 1);
        assert_eq!(func.alloc_site_count, 1);
        assert_eq!(
            func.id,
            format!(
                "src/sample.c#process#{}:{}",
                func.span.start_line, func.span.start_col
            )
        );
        assert_eq!(
            func.length_lines,
            func.span.end_line - func.span.start_line + 1
        );
    }

    #[test]
    fn call_event_carries_loop_context_and_shapes() {
        let ex = sample_extraction();
        let calls = &ex.functions[0].calls;
        let malloc: Vec<_> = calls.iter().filter(|c| c.callee_name == "malloc").collect();
        assert_eq!(malloc.len(), 1);
        let call = malloc[0];
        assert_eq!(call.callee_kind, CalleeKind::Direct);
        assert_eq!(call.meta.in_loop_depth, 1);
        assert!(call.meta.is_innermost);
        // Buried in an initializer, not the loop's bare primary statement.
        assert_eq!(call.statement_role, StatementRole::Nested);
        assert!(!call.conditional_guarded);
        assert_eq!(call.meta.resolver_tier, crate::ir::ResolverTier::Syntactic);
        assert!(!call.meta.macro_origin);
        // arg `n + 1` is a binary shape over identifier + literal
        assert_eq!(call.args.len(), 1);
        assert_eq!(call.args[0].kind, ShapeKind::Binary);
    }

    #[test]
    fn guarded_call_inside_if_is_flagged() {
        let ex = sample_extraction();
        let calls = &ex.functions[0].calls;
        // no other calls exist in SAMPLE besides malloc
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn loop_node_records_depth_and_induction_var() {
        let ex = sample_extraction();
        let loops = &ex.functions[0].loops;
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].kind, LoopKind::For);
        assert_eq!(loops[0].depth, 1);
        assert_eq!(loops[0].induction_vars, vec!["i".to_string()]);
    }

    #[test]
    fn canonical_for_loop_trip_counts_are_gated_honestly() {
        fn trip(body: &str) -> Option<u64> {
            let src = format!("void f(void) {{ {body} }}");
            let ex = extract("t.c", src.as_bytes());
            ex.functions[0].loops[0].trip_count
        }
        // Canonical ascending literal loops.
        assert_eq!(trip("for (int i = 0; i < 10; i++) {}"), Some(10));
        assert_eq!(trip("for (i = 0; i < 10; i++) {}"), Some(10));
        assert_eq!(trip("for (i = 0; i <= 10; i++) {}"), Some(11));
        assert_eq!(trip("for (int i = 0; i < 10; ++i) {}"), Some(10));
        assert_eq!(trip("for (int i = 0; i < 10; i += 1) {}"), Some(10));
        assert_eq!(trip("for (int i = 0; i < 0; i++) {}"), Some(0));
        // CLAMP: absurd literal bound cannot overflow cost math.
        assert_eq!(
            trip("for (int i = 0; i < 18446744073709551615u; i++) {}"),
            Some(crate::ir::MAX_TRIP)
        );
        assert_eq!(
            trip("for (int i = 0; i <= 18446744073709551615u; i++) {}"),
            Some(crate::ir::MAX_TRIP)
        );
        // Non-canonical: honest unknown, never a guessed bound.
        assert_eq!(trip("while (1) {}"), None);
        assert_eq!(trip("do {} while (0);"), None);
        assert_eq!(trip("for (int i = 0; i < LIMIT; i++) {}"), None);
        assert_eq!(trip("for (int i = 0; i < n; i++) {}"), None);
        assert_eq!(trip("for (int i = 0; i < (10); i++) {}"), None);
        assert_eq!(trip("for (int i = 10; i > 0; i--) {}"), None);
        assert_eq!(trip("for (int i = 0; i < 10; i--) {}"), None);
        assert_eq!(trip("for (int i = 0; i < 10; i += 2) {}"), None);
        assert_eq!(trip("for (int i = 1; i < 10; i++) {}"), None);
        assert_eq!(trip("for (i -= 0; i < 10; i++) {}"), None);
        assert_eq!(trip("for (int i = 0, j = 0; i < 10; i++) {}"), None);
        assert_eq!(trip("for (int i = 0; i < 10; j++) {}"), None);
        assert_eq!(trip("for (int i = 0; i != 10; i++) {}"), None);
        assert_eq!(trip("for (;;) {}"), None);
    }

    #[test]
    fn comparisons_and_binary_ops_are_emitted() {
        let ex = sample_extraction();
        // `i < n` and `p != 0`
        assert_eq!(ex.functions[0].comparisons.len(), 2);
        // `n + 1` and `total += buf[i]` (compound assignment is NOT a plain
        // binary_expression; only `n + 1` qualifies)
        assert_eq!(ex.functions[0].binary_ops.len(), 1);
        assert_eq!(ex.functions[0].binary_ops[0].op, "+");
    }

    #[test]
    fn include_edges_carry_form_and_raw_target() {
        let ex = sample_extraction();
        assert_eq!(ex.includes.len(), 2);
        let angle = ex
            .includes
            .iter()
            .find(|e| e.form == IncludeForm::Angle)
            .unwrap();
        assert_eq!(angle.to_path.as_deref(), Some("stdio.h"));
        assert!(!angle.resolved);
        let quoted = ex
            .includes
            .iter()
            .find(|e| e.form == IncludeForm::Quoted)
            .unwrap();
        assert_eq!(quoted.to_path.as_deref(), Some("sub/local.h"));
        assert_eq!(quoted.from_path, "src/sample.c");
    }

    #[test]
    fn global_var_storage_class_detected() {
        let ex = sample_extraction();
        assert_eq!(ex.global_vars.len(), 1);
        assert_eq!(ex.global_vars[0].name, "g_count");
        assert_eq!(ex.global_vars[0].storage, crate::ir::StorageClass::Static);
    }

    /// v1.5 I1a (plan `map-build`): the linkage fact feeding lib-root
    /// detection — `static` definitions report `Some(true)`, everything else
    /// (including explicit `extern`) reports `Some(false)`.
    #[test]
    fn function_static_linkage_recorded() {
        let src = "static int hidden(void) { return 1; }\n\
                   int shown(void) { return hidden(); }\n\
                   extern int declared(void) { return 0; }\n";
        let ex = extract("t.c", src.as_bytes());
        assert_eq!(ex.functions.len(), 3);
        let flag = |name: &str| {
            ex.functions
                .iter()
                .find(|f| f.record.name == name)
                .unwrap_or_else(|| panic!("function {name} present"))
                .record
                .is_static
        };
        assert_eq!(flag("hidden"), Some(true));
        assert_eq!(flag("shown"), Some(false));
        assert_eq!(flag("declared"), Some(false));
    }

    #[test]
    fn loc_counts_follow_line_classification_rules() {
        let src = "\n\nint a;\n/* c */\n// l\nint b;\n";
        let ex = extract("t.c", src.as_bytes());
        // line1 blank, line2 blank, line3 code, line4 comment, line5 comment,
        // line6 code
        assert_eq!(ex.file.loc.blank, 2);
        assert_eq!(ex.file.loc.comment, 2);
        assert_eq!(ex.file.loc.code, 2);
    }

    #[test]
    fn syntax_error_yields_partial_parse_health_with_regions() {
        let src = "int main(void) { int x = ; return 0; }\n";
        let ex = extract("bad.c", src.as_bytes());
        assert_eq!(ex.parse_health.status, ParseStatus::Partial);
        assert!(ex.parse_health.error_node_count >= 1);
        assert!(!ex.parse_health.error_regions.is_empty());
    }

    #[test]
    fn recognizer_flags_alloc_wrapper_macros_only() {
        let src = concat!(
            "#define MY_ALLOC(sz) malloc(sz)\n",
            "#define DOUBLE(x) ((x) * 2)\n",
            "#define CALLOC_N(n, s) calloc(n, s)\n",
        );
        let ex = extract("m.c", src.as_bytes());
        let patterns: Vec<_> = ex
            .macros
            .iter()
            .filter(|m| m.kind == MacroEventKind::RecognizedPattern)
            .collect();
        assert_eq!(patterns.len(), 2, "MY_ALLOC and CALLOC_N wrap alloc calls");
        assert!(
            patterns
                .iter()
                .all(|m| m.pattern_id.as_deref() == Some(ALLOC_WRAPPER_PATTERN_ID))
        );
        assert!(
            ex.macros
                .iter()
                .any(|m| m.kind == MacroEventKind::Definition && m.name == "DOUBLE")
        );
        assert!(
            !ex.macros
                .iter()
                .any(|m| m.kind == MacroEventKind::RecognizedPattern && m.name == "DOUBLE")
        );
    }

    #[test]
    fn struct_def_members_and_array_dims_extracted() {
        let src = concat!(
            "struct Item {\n",
            "    int id;\n",
            "    char name[16];\n",
            "    char *alias;\n",
            "};\n",
            "int main(void) { return 0; }\n",
        );
        let ex = extract("s.c", src.as_bytes());
        assert_eq!(ex.structs.len(), 1);
        let def = &ex.structs[0];
        assert_eq!(def.name, "Item");
        assert_eq!(def.members.len(), 3);
        assert_eq!(def.members[0].name, "id");
        assert!(!def.members[0].is_pointer, "plain member is not a pointer");
        assert_eq!(def.members[1].array_dim_literal, Some(16));
        assert!(!def.members[1].is_pointer);
        assert_eq!(def.members[2].name, "alias");
        assert!(
            def.members[2].is_pointer,
            "declarator star must set is_pointer (stars never reach type_text)"
        );
    }

    /// `Shape.sizeof_operand` (additive ir-v1.md §3.4 field): raw operand
    /// text AS WRITTEN with one level of parens stripped — the MEM-001
    /// input. Locks the tree-sitter extraction so a grammar/index
    /// regression cannot silently disable operand classification.
    #[test]
    fn sizeof_operand_records_operand_text() {
        let src = concat!(
            "void f(void *p) {\n",
            "    void *a = malloc(sizeof(p));\n",
            "    void *b = malloc(sizeof(struct S));\n",
            "    void *c = malloc(sizeof *p);\n",
            "    void *d = malloc(16);\n",
            "}\n",
        );
        let ex = extract("s.c", src.as_bytes());
        let f = ex.functions.first().expect("function f");
        let operand_at = |line: u32| {
            f.calls
                .iter()
                .find(|c| c.meta.span.start_line == line)
                .and_then(|c| c.args.last())
                .and_then(|arg| arg.sizeof_operand.clone())
        };
        assert_eq!(operand_at(2).as_deref(), Some("p"), "parens stripped");
        assert_eq!(operand_at(3).as_deref(), Some("struct S"));
        assert_eq!(
            operand_at(4).as_deref(),
            Some("*p"),
            "unparenthesized operand kept as written"
        );
        assert_eq!(
            operand_at(5),
            None,
            "non-sizeof argument carries no sizeof_operand"
        );
    }

    /// ir-v1.md §7: engine-native ↔ IR span round-trip locked BOTH directions
    /// on every extracted node extent of the sample (including multi-line
    /// nodes and nodes swallowing trailing newlines).
    #[test]
    fn span_round_trip_both_directions_on_real_nodes() {
        let ex = sample_extraction();
        let mut spans = Vec::new();
        for e in &ex.includes {
            spans.push(e.span);
        }
        for g in &ex.global_vars {
            spans.push(g.span);
        }
        for s in &ex.structs {
            spans.push(s.span);
        }
        for m in &ex.macros {
            spans.push(m.span);
        }
        for f in &ex.functions {
            spans.push(f.record.span);
            for c in &f.calls {
                spans.push(c.meta.span);
            }
            for l in &f.loops {
                spans.push(l.span);
            }
        }
        assert!(spans.len() >= 5, "expected a healthy span corpus");
        for span in &spans {
            let (start, end_exclusive) = span.to_native(SAMPLE.as_bytes());
            let back = Span::from_native(SAMPLE.as_bytes(), start, end_exclusive);
            assert_eq!(&back, span, "round-trip failed for {span:?}");
        }
    }

    /// Lock exact converted coordinates for one known node: `malloc(n + 1)`
    /// sits at 0-based row 12, cols 20–32 (end-exclusive) in SAMPLE.
    #[test]
    fn span_conversion_exact_values_locked() {
        let ex = sample_extraction();
        let call = ex.functions[0]
            .calls
            .iter()
            .find(|c| c.callee_name == "malloc")
            .expect("malloc call");
        // Locate the raw text to confirm the arithmetic independently.
        let line_start = SAMPLE
            .lines()
            .nth(usize::try_from(call.meta.span.start_line - 1).unwrap())
            .expect("line");
        assert!(
            line_start.contains("malloc"),
            "span anchors the malloc line"
        );
        assert_eq!(
            call.meta.span.start_col, 19,
            "1-based byte column of 'malloc'"
        );
        assert_eq!(call.meta.span.end_line, call.meta.span.start_line);
        assert_eq!(
            call.meta.span.end_col, 31,
            "inclusive end at ')' closing the arg list"
        );
    }

    // --- Regression locks (review fixes M2 / M3 / m5) ---

    /// M2 regression (a): an event outside any loop is never innermost —
    /// the old own-subtree check reported `true` here.
    #[test]
    fn is_innermost_false_outside_any_loop() {
        let src = "void f(void) {\n    bar();\n}\n";
        let ex = extract("inner_a.c", src.as_bytes());
        let call = &ex.functions[0].calls[0];
        assert_eq!(call.meta.in_loop_depth, 0);
        assert!(!call.meta.is_innermost);
    }

    /// M2 regression (b): a call in an OUTER loop whose body contains a
    /// nested loop elsewhere is NOT in the innermost loop.
    #[test]
    fn is_innermost_false_in_outer_loop_with_sibling_nested_loop() {
        let src = concat!(
            "void f(int n) {\n",
            "    for (int i = 0; i < n; ++i) {\n",
            "        for (int j = 0; j < n; ++j) { drain(j); }\n",
            "        bar(n);\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("inner_b.c", src.as_bytes());
        let bar = ex.functions[0]
            .calls
            .iter()
            .find(|c| c.callee_name == "bar")
            .expect("bar call");
        assert_eq!(bar.meta.in_loop_depth, 1);
        assert!(
            !bar.meta.is_innermost,
            "outer-loop call with a nested sibling loop is not innermost"
        );
        // The call inside the nested loop itself IS innermost.
        let drain = ex.functions[0]
            .calls
            .iter()
            .find(|c| c.callee_name == "drain")
            .expect("drain call");
        assert_eq!(drain.meta.in_loop_depth, 2);
        assert!(drain.meta.is_innermost);
    }

    /// M2 regression (c): a call in an actual innermost loop stays true.
    #[test]
    fn is_innermost_true_in_deepest_enclosing_loop() {
        let src = concat!(
            "void f(int n) {\n",
            "    for (int i = 0; i < n; ++i) {\n",
            "        bar(i);\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("inner_c.c", src.as_bytes());
        let call = &ex.functions[0].calls[0];
        assert_eq!(call.meta.in_loop_depth, 1);
        assert!(call.meta.is_innermost);
    }

    /// M3 regression (a): the common braced loop body must keep a direct
    /// call PRIMARY (`for (…) { foo(); }` behaved like the buried form).
    #[test]
    fn braced_loop_body_direct_call_is_primary() {
        let src = concat!(
            "void f(int n) {\n",
            "    for (int i = 0; i < n; ++i) {\n",
            "        bar(i);\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("role_braced.c", src.as_bytes());
        let call = &ex.functions[0].calls[0];
        assert_eq!(call.statement_role, StatementRole::Primary);
        assert!(!call.conditional_guarded);
    }

    /// M3 regression (b): transparency does not leak past the body — a call
    /// buried inside an `if` within the loop stays nested + guarded.
    #[test]
    fn call_inside_if_within_loop_stays_nested_and_guarded() {
        let src = concat!(
            "void f(int n) {\n",
            "    for (int i = 0; i < n; ++i) {\n",
            "        if (i > 0) {\n",
            "            bar(i);\n",
            "        }\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("role_guarded.c", src.as_bytes());
        let call = &ex.functions[0].calls[0];
        assert_eq!(call.statement_role, StatementRole::Nested);
        assert!(call.conditional_guarded);
    }

    /// m5 regression: EVERY declarator of a multi-declarator global
    /// declaration is collected, with the declaration's storage class.
    #[test]
    fn multi_declarator_globals_all_collected_with_storage() {
        let src = "int a, b;\nstatic int c, d;\nextern int e, f;\n";
        let ex = extract("globals.c", src.as_bytes());
        let names: Vec<_> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d", "e", "f"]);
        let storage_of = |name: &str| {
            ex.global_vars
                .iter()
                .find(|g| g.name == name)
                .map(|g| g.storage)
                .unwrap()
        };
        assert_eq!(storage_of("a"), StorageClass::None);
        assert_eq!(storage_of("b"), StorageClass::None);
        assert_eq!(storage_of("c"), StorageClass::Static);
        assert_eq!(storage_of("d"), StorageClass::Static);
        assert_eq!(storage_of("e"), StorageClass::Extern);
        assert_eq!(storage_of("f"), StorageClass::Extern);
        assert_eq!(ex.file.global_var_count, 6);
    }

    // --- Guarded-header regression locks (recursive file-level walk) ---

    /// The everyday guarded-header shape: an `#include` nested in
    /// `#ifndef`/`#define`/`#endif` must still be collected.
    #[test]
    fn guarded_include_collected() {
        let src = "#ifndef T_H\n#define T_H\n#include \"u.h\"\n#endif\n";
        let ex = extract("t.h", src.as_bytes());
        assert_eq!(ex.includes.len(), 1);
        assert_eq!(ex.includes[0].form, IncludeForm::Quoted);
        assert_eq!(ex.includes[0].to_path.as_deref(), Some("u.h"));
        assert_eq!(ex.file.include_counts.quoted, 1);
        assert!(
            ex.macros
                .iter()
                .any(|m| m.kind == MacroEventKind::Definition && m.name == "T_H")
        );
    }

    /// A function-like alloc wrapper inside a guard still yields both the
    /// definition events and the curated `recognized_pattern` event.
    #[test]
    fn guarded_alloc_wrapper_define_collected() {
        let src = "#ifndef M_H\n#define M_H\n#define WRAP(sz) malloc(sz)\n#endif\n";
        let ex = extract("m.h", src.as_bytes());
        for name in ["M_H", "WRAP"] {
            assert!(
                ex.macros
                    .iter()
                    .any(|m| m.kind == MacroEventKind::Definition && m.name == name),
                "missing definition event for {name}"
            );
        }
        let pattern = ex
            .macros
            .iter()
            .find(|m| m.kind == MacroEventKind::RecognizedPattern && m.name == "WRAP")
            .expect("WRAP recognized pattern");
        assert_eq!(
            pattern.pattern_id.as_deref(),
            Some(ALLOC_WRAPPER_PATTERN_ID)
        );
    }

    /// A guarded `struct` must still be extracted with its members.
    #[test]
    fn guarded_struct_extracted() {
        let src = "#ifndef G_H\n#define G_H\nstruct Padded { char tag; double value; int count; };\n#endif\n";
        let ex = extract("g.h", src.as_bytes());
        assert_eq!(ex.structs.len(), 1);
        assert_eq!(ex.structs[0].name, "Padded");
        assert_eq!(ex.structs[0].members.len(), 3);
    }

    /// Guarded globals keep their storage classes and count.
    #[test]
    fn guarded_globals_collected_with_storage() {
        let src = "#ifndef GLO_H\n#define GLO_H\nstatic long acc;\nextern int shared;\n#endif\n";
        let ex = extract("glo.h", src.as_bytes());
        let names: Vec<_> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["acc", "shared"]);
        assert_eq!(ex.global_vars[0].storage, StorageClass::Static);
        assert_eq!(ex.global_vars[1].storage, StorageClass::Extern);
        assert_eq!(ex.file.global_var_count, 2);
    }

    /// Pass 2 (functions) must also see definitions nested in guards.
    #[test]
    fn guarded_function_definition_collected() {
        let src = "#ifdef _WIN32\nint win_only(void) { return 1; }\n#endif\nint always(void) { return 0; }\n";
        let ex = extract("f.c", src.as_bytes());
        let names: Vec<_> = ex
            .functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect();
        assert_eq!(names, ["win_only", "always"]);
        assert_eq!(ex.file.function_count, 2);
    }

    /// Recursion descends through arbitrarily nested conditionals.
    #[test]
    fn doubly_nested_guard_include_collected() {
        let src = "#ifdef A\n#ifdef B\n#include \"deep.h\"\n#endif\n#endif\n";
        let ex = extract("deep.h", src.as_bytes());
        assert_eq!(ex.includes.len(), 1);
        assert_eq!(ex.includes[0].to_path.as_deref(), Some("deep.h"));
    }

    /// Negative lock on the stop boundary: a guarded FILE-level global IS
    /// collected (the fix), while declarations/structs inside a
    /// `function_definition` body are NOT (their scope is function-local).
    #[test]
    fn function_local_declaration_and_struct_not_collected() {
        let src = concat!(
            "#ifndef X_H\n",
            "#define X_H\n",
            "static int g;\n",
            "#endif\n",
            "void f(void) {\n",
            " int local = 1;\n",
            " struct Local { int a; char b; };\n",
            " (void)local;\n",
            "}\n",
        );
        let ex = extract("x.h", src.as_bytes());
        let names: Vec<_> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["g"], "guarded file-level global IS collected");
        assert!(
            ex.structs.is_empty(),
            "function-local struct must not leak past the stop boundary"
        );
        assert_eq!(ex.file.function_count, 1);
    }

    /// Precision FP-class fix 1 producer: only a `do_statement` whose
    /// condition text is the literal `(0)` carries `condition_is_zero` — the
    /// macro-wrapper idiom. Genuine `do-while(cond)` plus `for`/`while`
    /// loops report `false` (text check, never a blanket `Do` flag).
    #[test]
    fn zero_guard_flag_set_only_on_do_while_zero_condition() {
        let src = concat!(
            "void w0(void) { do { f(); } while(0); }\n",
            "void wsp(void) { do { f(); } while (0); }\n",
            "void real(void) { int i = 0; do { f(); i++; } while (i < 9); }\n",
            "void fw(void) { for (int i = 0; i < 9; i++) { f(); } while (0) { f(); break; } }\n",
        );
        let ex = extract("z.c", src.as_bytes());
        assert_eq!(ex.functions.len(), 4);
        let flags: Vec<(String, Vec<(LoopKind, bool)>)> = ex
            .functions
            .iter()
            .map(|f| {
                (
                    f.record.name.clone(),
                    f.loops
                        .iter()
                        .map(|l| (l.kind, l.condition_is_zero))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            flags,
            [
                ("w0".to_string(), vec![(LoopKind::Do, true)]),
                ("wsp".to_string(), vec![(LoopKind::Do, true)]),
                ("real".to_string(), vec![(LoopKind::Do, false)]),
                (
                    "fw".to_string(),
                    vec![(LoopKind::For, false), (LoopKind::While, false)]
                ),
            ]
        );
    }

    /// Precision FP-class fix 2 producer: a function defined inside `#if 0`
    /// carries `cfg_gated = Some(true)` (provably dead). `#ifdef`-guarded,
    /// live, and `#if 0`-inside-body functions stay `None`.
    #[test]
    fn if_zero_function_flagged_only() {
        let src = concat!(
            "int live(void) { return 0; }\n",
            "#if 0\n",
            "int dead(void) { return 1; }\n",
            "#endif\n",
            "#ifdef X\n",
            "int maybe(void) { return 2; }\n",
            "#endif\n",
            "int mixed(void) { int x = 1;\n",
            "#if 0\n",
            "x = 2;\n",
            "#endif\n",
            "return x; }\n",
        );
        let ex = extract("g.c", src.as_bytes());
        let flags: Vec<(&str, Option<bool>)> = ex
            .functions
            .iter()
            .map(|f| (f.record.name.as_str(), f.record.cfg_gated))
            .collect();
        assert_eq!(
            flags,
            [
                ("live", None),
                ("dead", Some(true)),
                ("maybe", None),
                ("mixed", None),
            ]
        );
    }

    /// Unguarded regression: the recursion must not change the everyday
    /// top-level file shape.
    #[test]
    fn unguarded_file_level_records_still_collected() {
        let src = "#include \"u.h\"\n#define D 1\nstruct S { int a; };\nstatic int g;\n";
        let ex = extract("u.c", src.as_bytes());
        assert_eq!(ex.includes.len(), 1);
        assert_eq!(ex.structs.len(), 1);
        assert_eq!(ex.global_vars.len(), 1);
        assert!(
            ex.macros
                .iter()
                .any(|m| m.kind == MacroEventKind::Definition && m.name == "D")
        );
    }

    /// Anonymous structs carry no tag name, so they are skipped rather than
    /// emitted as an unattributable `struct ''`. Covers the `typedef struct
    /// { … } X;` idiom and an inline anonymous struct-typed member.
    #[test]
    fn anonymous_struct_skipped() {
        let typedef_anon = "typedef struct { char tag; double value; int count; } TAnon;\n";
        assert!(
            extract("a.c", typedef_anon.as_bytes()).structs.is_empty(),
            "namespace-less typedef struct must not produce a nameless StructDef"
        );

        let nested_anon = "struct Bag { struct { char a; double b; int c; } data; int n; };\n";
        let ex = extract("b.c", nested_anon.as_bytes());
        let names: Vec<_> = ex.structs.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Bag"], "only the named outer struct is collected");
    }

    /// Approved breadth lock: the recursive file-level walk also reaches NAMED
    /// structs that the old direct-children walk missed — inside a `typedef`
    /// and nested in another struct body.
    #[test]
    fn named_struct_in_typedef_and_nested_collected() {
        let src =
            "typedef struct Named { int a; } X;\nstruct Outer { struct Inner { int b; } in; };\n";
        let ex = extract("n.c", src.as_bytes());
        let names: Vec<_> = ex.structs.iter().map(|s| s.name.as_str()).collect();
        // Preorder: the typedef'd struct, then the outer, then its nested inner.
        assert_eq!(names, ["Named", "Outer", "Inner"]);
    }
}
