//! tree-sitter-rust scanner backend (ADR-0001; plan `rust-backend`, phase R1).
//!
//! Binds the official org-published grammar crate (`tree-sitter-rust`, MIT,
//! exact-pinned in Cargo.toml per ADR-0001 guardrail G5) to the
//! parser-agnostic IR seam. It mirrors the shipped C backend's interface
//! ([`Extraction`]), ParseHealth contract, span conversion and structure, and
//! adds nothing C-specific (no declarator unwinding, no `preproc_*` handling).
//!
//! Contract (modules.md §4.2): given `(path, source bytes)`, emit that file's
//! IR records/events. Engine-native spans convert to IR spans HERE and never
//! leak node kinds past this boundary (ir-v1.md §2). Parse honesty: ERROR /
//! missing nodes become [`ParseHealth`] partial data; a failed parse yields
//! [`ParseStatus::Failed`]. `extract` is total and never panics on malformed
//! input.
//!
//! # Founder-approved R1 decisions (D1–D8)
//!
//! - **D1 include form.** [`IncludeEdge::form`] is `Quoted` for internal module
//!   paths (`crate::`/`self::`/`super::`, and bare `mod foo;`) and `Angle` for
//!   everything else (`std`/`core`/`alloc`/any crate name/`extern crate`).
//!   `to_path` is the full path text with `::` for `use`, or the module/crate
//!   name for `mod`/`extern crate`. `use` leaf paths never match a file, so the
//!   scanner's Rust resolution pass leaves them unresolved by design (R3 owns
//!   module-graph semantics; wildcard leaves drop the trailing `::*`).
//! - **D2 callee kind.** `identifier`/`scoped_identifier`/`field_expression`/
//!   `generic_function` as the `function` field ⇒ [`CalleeKind::Direct`];
//!   anything else ⇒ [`CalleeKind::Indirect`]; missing `function` ⇒
//!   [`CalleeKind::Unknown`]. `callee_name` is the final segment: `identifier`
//!   text, `scoped_identifier` `name`, `field_expression` `field`, and
//!   `generic_function` recurses into its `function`. Documented FP class: a
//!   closure-field invocation (`self.callback()`) classifies `Direct`.
//! - **D3 `LoopKind`.** `loop_expression` maps to the additive
//!   [`LoopKind::Loop`] variant; no other IR change.
//! - **D4 structs.** `struct_item` ⇒ [`StructDef`] (named members from
//!   `field_declaration_list`); tuple structs (`ordered_field_declaration_list`)
//!   ⇒ `StructDef` with synthetic positional member names `"0"`, `"1"`, ….
//!   R3 extends this: `union_item` ⇒ `StructDef { kind: Union }` (members from
//!   its `field_declaration_list`), `enum_item` ⇒ `StructDef { kind: Enum }`
//!   with NO members (variants are not layout fields; a size/padding heuristic
//!   must inspect `kind == Struct`), and `StructMember.is_pointer` /
//!   `array_dim_literal` are derived from the member `type` (`&T`/`*const T`,
//!   `[u8; 16]`).
//! - **D5 signatures.** `function_signature_item` (body-less trait method) is
//!   SKIPPED, mirroring C where prototypes are not functions.
//! - **D6 bindings.** `let_declaration` is NOT an assignment event;
//!   `assignment_expression` and `compound_assignment_expr` ARE. `scope_class`
//!   is `Global` iff the target root identifier is a collected `static`/`const`
//!   name, `Param` iff it is a parameter name (including `self`), else `Local`;
//!   a non-identifier target is `Unknown`.
//! - **D7 macros.** `macro_definition` ⇒ `MacroEvent { kind: Definition }`;
//!   every `macro_invocation` ⇒ `MacroEvent { kind: Use, name: <macro field> }`
//!   with NO expansion.
//! - **D8 stats (extended by R3).** `alloc_site_count` now counts Rust
//!   allocation-family call sites ([`RUST_ALLOC_FAMILY`]: qualified
//!   constructors, method-only names, and `vec!`/`format!` macro invocations);
//!   `macro_origin` is wired for every event type + `LoopNode` (span intersects
//!   a recognized wrapper, or `callee_name` equals a local `macro_rules!`
//!   definition name). Each `macro_invocation` additionally yields a
//!   function-attached `CallEvent { is_macro: true }` (the macro-as-call
//!   surface) alongside its D7 `MacroEvent { kind: Use }`.
//!
//! # Documented blind spot (`token_tree`)
//!
//! `macro_rules!` bodies and macro invocation arguments parse as opaque
//! `token_tree` nodes — raw tokens the grammar does not structure into
//! `function_item`, `call_expression`, loops, etc. Macro-generated items are
//! therefore invisible this milestone (no IR record, no heuristic, no
//! macro-origin span), exactly as `preproc_arg` is opaque in the C backend.
//! `macro_definition`/`macro_invocation` nodes themselves are structured and
//! are recorded; their contents are not expanded or re-parsed.
//!
//! # Attribute honesty
//!
//! `#[...]`/`#![...]` are ordinary sibling `attribute_item`/`inner_attribute_item`
//! nodes, not wrappers, so an annotated item is still extracted normally
//! (locked by the attribute-sibling tests). tree-sitter is a parser, not a
//! preprocessor: BOTH arms of a `#[cfg(...)]` pair present in the source are
//! extracted, since the predicate sits in opaque attribute text.

use tree_sitter::{Node, Parser};

use super::Extraction;
use crate::ir::{
    ALLOC_WRAPPER_PATTERN_ID, AccessChainEvent, AssignmentEvent, BinaryOpEvent, BindingKind,
    CallEvent, CalleeKind, ComparisonEvent, EventMeta, FileRecord, FunctionRecord, GlobalVar,
    IncludeCounts, IncludeEdge, IncludeForm, LocCounts, LoopKind, LoopNode, MacroEvent,
    MacroEventKind, NativePoint, ParseHealth, ParseStatus, RUST_ALLOC_FAMILY, ResolverTier,
    ScopeClass, Shape, ShapeKind, Span, StatementRole, StorageClass, StructDef, StructKind,
    StructMember,
};
use crate::scanner::FunctionUnit;

/// Extract one file's IR records/events from parsed Rust source.
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

    // Pass 1: file-level records. `walk_file_level` stops at `function_item`
    // bodies but DOES descend into `impl_item`/`trait_item`/`mod_item` bodies
    // and `declaration_list`s, so methods and nested modules are reachable and
    // function-local items stay function-local. Globals are collected before
    // functions so assignment scope classification can consult them regardless
    // of declaration order.
    let mut includes = Vec::new();
    let mut global_vars = Vec::new();
    let mut structs = Vec::new();
    walk_file_level(root, &mut |node| match node.kind() {
        "use_declaration" => push_use_edges(node, path, source, &mut includes),
        "mod_item" => {
            // `mod foo { … }` is an inline submodule (no edge; its body is
            // walked); `mod foo;` references `foo.rs` / `foo/mod.rs`.
            if node.child_by_field_name("body").is_none()
                && let Some(name) = node.child_by_field_name("name")
            {
                includes.push(IncludeEdge {
                    from_path: path.to_string(),
                    to_path: Some(mod_edge_raw(node, name, source)),
                    form: IncludeForm::Quoted,
                    resolved: false,
                    // Rust `mod`-declaration edge: still emitted and still
                    // drives `RustModuleMap`/`use` resolution, but
                    // `PERF-RS-ARCH-001` excludes it (import graph only).
                    is_mod: true,
                    span: ir_span(node, source),
                });
            }
        }
        "extern_crate_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                includes.push(IncludeEdge {
                    from_path: path.to_string(),
                    to_path: Some(text(name, source).to_string()),
                    form: IncludeForm::Angle,
                    resolved: false,
                    is_mod: false,
                    span: ir_span(node, source),
                });
            }
        }
        "static_item" if !inside_type_body(node) => {
            push_global(node, path, source, StorageClass::Static, &mut global_vars)
        }
        "const_item" if !inside_type_body(node) => {
            push_global(node, path, source, StorageClass::None, &mut global_vars)
        }
        "struct_item" => {
            if let Some(def) = type_def(node, StructKind::Struct, path, source) {
                structs.push(def);
            }
        }
        "union_item" => {
            if let Some(def) = type_def(node, StructKind::Union, path, source) {
                structs.push(def);
            }
        }
        "enum_item" => {
            // Enum variants are NOT layout members: the record is emitted with
            // `kind = Enum` and NO members, so a size/padding heuristic can
            // exclude it by inspecting `kind` (documented on `StructKind`).
            if let Some(def) = type_def(node, StructKind::Enum, path, source) {
                structs.push(def);
            }
        }
        _ => {}
    });

    // Macro events (D7 + R3 recognizer): definitions, recognized alloc
    // wrappers, and EVERY invocation (including inside function bodies) are
    // gathered over the whole tree in source order. This runs BEFORE the
    // function pass so macro-origin classification can consult the gathered
    // definition names + recognized-pattern spans.
    let mut macros = Vec::new();
    let mut macro_cursor = root.walk();
    walk(&mut macro_cursor, &mut |node| match node.kind() {
        "macro_definition" | "macro_invocation" => {
            push_macro_event(node, path, source, &mut macros)
        }
        _ => {}
    });
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

    // Pass 2: functions. Every `function_item` reachable at file level or
    // inside an `impl_item`/`trait_item`/`mod_item` body is a unit; body-less
    // `function_signature_item` (D5) is skipped.
    let mut functions = Vec::new();
    let global_names: Vec<String> = global_vars.iter().map(|g| g.name.clone()).collect();
    walk_file_level(root, &mut |node| {
        if node.kind() == "function_item" {
            functions.push(function_unit(
                node,
                path,
                source,
                &global_names,
                &macro_names,
                &pattern_spans,
            ));
        }
    });

    // Comment lines anywhere in the file (including inside function bodies)
    // feed the LOC stat. Rust has no `comment` supertype: both `line_comment`
    // and `block_comment` are `extra: true` and must be matched.
    let mut comment_ranges: Vec<(usize, usize)> = Vec::new();
    let mut comment_cursor = root.walk();
    walk(&mut comment_cursor, &mut |node| {
        if matches!(node.kind(), "line_comment" | "block_comment") {
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
        language: "rust".to_string(),
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

/// The pinned Rust grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_rust::LANGUAGE.into()
}

fn empty_file_record(path: &str) -> FileRecord {
    FileRecord {
        path: path.to_string(),
        language: "rust".to_string(),
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
/// descend into `function_item` subtrees (their bodies are function-local
/// scope, handled by the function pass). It DOES descend into
/// `impl_item`/`trait_item`/`mod_item` bodies, so methods and nested-module
/// items are reachable. Rust's `#[...]` attributes are ordinary siblings, so
/// no guarded-content recovery is needed (see the module doc).
fn walk_file_level(node: Node<'_>, visit: &mut impl FnMut(Node<'_>)) {
    visit(node);
    if node.kind() == "function_item" {
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

/// M1: an associated `const`/`static` lives inside an `impl`/`trait` body and
/// is NOT a file-level global, even though `walk_file_level` descends into
/// those bodies to reach methods. Module/file-level `static`/`const` have no
/// such ancestor and are still collected.
fn inside_type_body(node: Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(parent.kind(), "impl_item" | "trait_item") {
            return true;
        }
        current = parent.parent();
    }
    false
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
/// code. A blank line inside a block comment counts as blank (whitespace rule
/// wins); the rule is deterministic either way.
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

/// D1: one `IncludeEdge` per leaf path of a `use_declaration` (fan-out through
/// `use_list` / `scoped_use_list` / `use_as_clause` / `use_wildcard`).
fn push_use_edges(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<IncludeEdge>) {
    let Some(argument) = node.child_by_field_name("argument") else {
        return;
    };
    let span = ir_span(node, source);
    let mut leaves = Vec::new();
    flatten_use(argument, "", source, &mut leaves);
    for raw in leaves {
        if raw.is_empty() {
            continue;
        }
        out.push(IncludeEdge {
            from_path: path.to_string(),
            form: use_edge_form(&raw),
            to_path: Some(raw),
            resolved: false,
            is_mod: false,
            span,
        });
    }
}

/// Flatten a `use` path expression into full `::`-joined leaf paths.
fn flatten_use(node: Node<'_>, prefix: &str, source: &[u8], out: &mut Vec<String>) {
    match node.kind() {
        "use_list" => {
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    if cursor.node().is_named() {
                        flatten_use(cursor.node(), prefix, source, out);
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        "scoped_use_list" => {
            let path_text = node
                .child_by_field_name("path")
                .map(|p| text(p, source))
                .unwrap_or("");
            let joined = join_path(prefix, path_text);
            if let Some(list) = node.child_by_field_name("list") {
                flatten_use(list, &joined, source, out);
            }
        }
        "use_as_clause" => {
            // The alias does not change the imported module path.
            if let Some(p) = node.child_by_field_name("path") {
                flatten_use(p, prefix, source, out);
            }
        }
        "use_wildcard" => {
            // `use a::*` → leaf module `a` (trailing `::*`/`*` dropped).
            let mut leaf = text(node, source).trim().to_string();
            if let Some(stripped) = leaf.strip_suffix("::*") {
                leaf = stripped.to_string();
            } else if let Some(stripped) = leaf.strip_suffix('*') {
                leaf = stripped.trim_end_matches(':').to_string();
            }
            out.push(join_path(prefix, &leaf));
        }
        "self" => {
            // `use path::{self, x}`: the `self` leaf denotes `path` itself,
            // not `path::self`. A bare `use self;` (empty prefix) is a no-op
            // self-import and is accepted as such — nothing to record.
            if !prefix.is_empty() {
                out.push(prefix.to_string());
            }
        }
        _ => {
            // Leaf path kinds: identifier / crate / super / scoped_identifier.
            out.push(join_path(prefix, text(node, source).trim()));
        }
    }
}

fn join_path(prefix: &str, segment: &str) -> String {
    // C1: normalize raw identifiers in every `::`-separated segment so a
    // `use crate::r#loop::X` edge carries `crate::loop::X` (module resolution
    // operates on bare names; `r#loop` was a guaranteed resolution miss).
    let prefix = strip_raw_idents(prefix);
    let segment = strip_raw_idents(segment);
    if prefix.is_empty() {
        segment
    } else if segment.is_empty() {
        prefix
    } else {
        format!("{prefix}::{segment}")
    }
}

/// C1: strip a leading `r#` from each `::`-separated segment of a path, so
/// raw identifiers (`r#type`/`r#loop`/`r#str`) normalize to their bare names
/// for module resolution. Idempotent and harmless on paths with no raw
/// identifiers (`r#` can only appear at a segment start).
fn strip_raw_idents(path: &str) -> String {
    path.split("::")
        .map(|segment| segment.strip_prefix("r#").unwrap_or(segment))
        .collect::<Vec<_>>()
        .join("::")
}

/// D1 form classification: internal module paths are `Quoted`, everything else
/// (`std`/`core`/`alloc`/any crate name) is `Angle`.
fn use_edge_form(raw: &str) -> IncludeForm {
    let internal = matches!(raw, "crate" | "self" | "super")
        || raw.starts_with("crate::")
        || raw.starts_with("self::")
        || raw.starts_with("super::");
    if internal {
        IncludeForm::Quoted
    } else {
        IncludeForm::Angle
    }
}

/// Minor-1: edge raw for a `mod foo;` declaration — the `/`-joined chain of
/// enclosing INLINE modules plus `foo` (`mod outer { mod inner; }` →
/// `outer/inner`). `/` (never `::`) keeps it distinct from `use` paths, which
/// the resolver classifies by the presence of `::`.
fn mod_edge_raw(node: Node<'_>, name: Node<'_>, source: &[u8]) -> String {
    let mut segments = Vec::new();
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "mod_item"
            && let Some(outer_name) = parent.child_by_field_name("name")
        {
            // C1: a raw-identifier module (`mod r#loop;`) must yield its bare
            // name so `<module_dir>/loop.rs` is probed, not `r#loop.rs`.
            segments.push(strip_raw_idents(text(outer_name, source)));
        }
        current = parent.parent();
    }
    segments.reverse();
    segments.push(strip_raw_idents(text(name, source)));
    segments.join("/")
}

/// `static_item` / `const_item` → [`GlobalVar`] with the mapped storage class.
fn push_global(
    node: Node<'_>,
    path: &str,
    source: &[u8],
    storage: StorageClass,
    out: &mut Vec<GlobalVar>,
) {
    let Some(name) = node.child_by_field_name("name") else {
        return;
    };
    out.push(GlobalVar {
        name: text(name, source).to_string(),
        path: path.to_string(),
        span: ir_span(node, source),
        storage,
    });
}

/// D7 + R3 recognizer: macro definition / invocation → [`MacroEvent`] (never
/// expanded). A `macro_definition` always yields a `Definition` event; when
/// the raw source of any `macro_rule.right` token tree calls an alloc-family
/// constructor/method, a SECOND event `RecognizedPattern` with
/// [`ALLOC_WRAPPER_PATTERN_ID`] is emitted (mirrors the C backend's
/// `#define`-body recognizer). Every `macro_invocation` yields a `Use` event.
fn push_macro_event(node: Node<'_>, path: &str, source: &[u8], out: &mut Vec<MacroEvent>) {
    match node.kind() {
        "macro_definition" => {
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
            if macro_body_calls_alloc_family(node, source) {
                out.push(MacroEvent {
                    kind: MacroEventKind::RecognizedPattern,
                    name,
                    path: path.to_string(),
                    span,
                    pattern_id: Some(ALLOC_WRAPPER_PATTERN_ID.to_string()),
                });
            }
        }
        "macro_invocation" => {
            let Some(name_node) = node.child_by_field_name("macro") else {
                return;
            };
            out.push(MacroEvent {
                kind: MacroEventKind::Use,
                name: text(name_node, source).to_string(),
                path: path.to_string(),
                span: ir_span(node, source),
                pattern_id: None,
            });
        }
        _ => {}
    }
}

/// Does any `macro_rule.right` token tree of `def` call an alloc-family
/// constructor/method (name then optional whitespace then `(`)? This mirrors
/// the C backend's `calls_alloc_family` matcher over `#define` bodies; the
/// token tree is opaque text, so a raw-text scan is the only option.
fn macro_body_calls_alloc_family(def: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = def.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.node().kind() == "macro_rule"
                && let Some(right) = cursor.node().child_by_field_name("right")
                && calls_alloc_family(text(right, source))
            {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}

/// Minimal curated matcher: does `body` contain `<alloc-name>(`?
fn calls_alloc_family(body: &str) -> bool {
    RUST_ALLOC_FAMILY.all().any(|fam| {
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

/// R3 D4: a `struct_item` / `union_item` / `enum_item` → [`StructDef`] with
/// the matching [`StructKind`]. Named members come from `field_declaration_list`
/// (or a union's required one); tuple structs (`ordered_field_declaration_list`)
/// get synthetic positional names `"0"`, `"1"`, …. Enums carry NO members
/// (variants are not layout fields).
fn type_def(node: Node<'_>, kind: StructKind, path: &str, source: &[u8]) -> Option<StructDef> {
    let name = node
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let mut members = Vec::new();
    if kind != StructKind::Enum
        && let Some(body) = node.child_by_field_name("body")
    {
        collect_members(body, source, &mut members);
    }
    Some(StructDef {
        name,
        path: path.to_string(),
        span: ir_span(node, source),
        members,
        kind,
    })
}

/// Collect named members from a `field_declaration_list` or positional
/// members from an `ordered_field_declaration_list` (tuple structs).
fn collect_members(body: Node<'_>, source: &[u8], members: &mut Vec<StructMember>) {
    match body.kind() {
        "field_declaration_list" => {
            let mut cursor = body.walk();
            if cursor.goto_first_child() {
                loop {
                    let field = cursor.node();
                    if field.kind() == "field_declaration" {
                        let member_name = field
                            .child_by_field_name("name")
                            .map(|n| text(n, source).to_string())
                            .unwrap_or_default();
                        let type_node = field.child_by_field_name("type");
                        let type_text = type_node
                            .map(|t| text(t, source).to_string())
                            .unwrap_or_default();
                        let (is_pointer, array_dim_literal) = member_flags(type_node, source);
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
        }
        "ordered_field_declaration_list" => {
            let mut position = 0u32;
            let mut cursor = body.walk();
            if cursor.goto_first_child() {
                loop {
                    if cursor.field_name() == Some("type") {
                        let type_node = cursor.node();
                        let (is_pointer, array_dim_literal) = member_flags(Some(type_node), source);
                        members.push(StructMember {
                            name: position.to_string(),
                            type_text: text(type_node, source).to_string(),
                            array_dim_literal,
                            is_pointer,
                        });
                        position += 1;
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

/// D4 member derivations: `is_pointer` is true for a `reference_type` (`&T`)
/// or `pointer_type` (`*const T`); `array_dim_literal` is the parsed length
/// of an `array_type` whose length is an `integer_literal` (`[u8; 16]` ⇒
/// `Some(16)`), `None` otherwise (e.g. a const-generic length).
fn member_flags(type_node: Option<Node<'_>>, source: &[u8]) -> (bool, Option<u64>) {
    let Some(t) = type_node else {
        return (false, None);
    };
    let is_pointer = matches!(t.kind(), "reference_type" | "pointer_type");
    (is_pointer, array_dim_literal(t, source))
}

/// Best-effort integer array dimension from an `array_type` (`[u8; 16]`).
fn array_dim_literal(node: Node<'_>, source: &[u8]) -> Option<u64> {
    if node.kind() != "array_type" {
        return None;
    }
    let length = node
        .child_by_field_name("length")
        .or_else(|| node.named_child(1))?;
    if length.kind() != "integer_literal" {
        return None;
    }
    text(length, source)
        .trim()
        .replace('_', "")
        .parse::<u64>()
        .ok()
}

/// Count of parameters in a `parameter`/`self_parameter` list, plus the
/// parameter type texts and names (D6 scope classification inputs).
struct ParamInfo {
    types: Vec<String>,
    names: Vec<String>,
}

fn collect_params(params: Node<'_>, source: &[u8]) -> ParamInfo {
    let mut info = ParamInfo {
        types: Vec::new(),
        names: Vec::new(),
    };
    let mut cursor = params.walk();
    if cursor.goto_first_child() {
        loop {
            let param = cursor.node();
            match param.kind() {
                "parameter" => {
                    // Param type text comes from the `type` field directly
                    // (Rust's declarator carries the pattern separately).
                    if let Some(t) = param.child_by_field_name("type") {
                        info.types.push(text(t, source).trim().to_string());
                    }
                    if let Some(pattern) = param.child_by_field_name("pattern")
                        && let Some(name) = leftmost_identifier(pattern, source)
                    {
                        info.names.push(name);
                    }
                }
                "self_parameter" => {
                    // No identifier; the receiver is name "self", raw text type.
                    info.types.push(text(param, source).trim().to_string());
                    info.names.push("self".to_string());
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

/// First descendant of `kind` (parity with the C backend helper set; Rust's
/// flat fields make it unnecessary in the main pass, so it exists only for the
/// unit tests).
#[cfg(test)]
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

/// Leftmost identifier (or `self`) inside a pattern / expression subtree.
fn leftmost_identifier(node: Node<'_>, source: &[u8]) -> Option<String> {
    if matches!(node.kind(), "identifier" | "field_identifier" | "self") {
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

/// Extract one function's record + attached event streams with a single body
/// walk (metrics and events share traversal state: loop depth, nesting).
fn function_unit(
    fdef: Node<'_>,
    path: &str,
    source: &[u8],
    global_names: &[String],
    macro_names: &[String],
    pattern_spans: &[Span],
) -> FunctionUnit {
    let name = fdef
        .child_by_field_name("name")
        .map(|n| text(n, source).to_string())
        .unwrap_or_default();
    let params = fdef
        .child_by_field_name("parameters")
        .map(|p| collect_params(p, source))
        .unwrap_or(ParamInfo {
            types: Vec::new(),
            names: Vec::new(),
        });

    let span = ir_span(fdef, source);
    let length_lines = span.end_line - span.start_line + 1;

    let mut scan = FuncScan {
        source,
        macro_names,
        pattern_spans,
        param_names: &params.names,
        global_names,
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
    let record = FunctionRecord {
        id,
        name,
        path: path.to_string(),
        span,
        param_count: params.types.len() as u32,
        param_types: params.types,
        cyclomatic_complexity: scan.cyclomatic,
        cognitive_complexity: scan.cognitive,
        max_loop_depth: scan.max_loop_depth,
        // R3: Rust allocation-family call sites (qualified constructors,
        // method-only members, and `vec!`/`format!` macro invocations).
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
        // v1.5 I1a (plan `map-build`): C-only this burst — linkage, entry
        // and reachability stay `None` for Rust (I1b owns them).
        is_static: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
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
    }
}

const LOOP_KINDS: [&str; 3] = ["for_expression", "while_expression", "loop_expression"];

fn loop_kind(kind: &str) -> Option<LoopKind> {
    match kind {
        "for_expression" => Some(LoopKind::For),
        "while_expression" => Some(LoopKind::While),
        "loop_expression" => Some(LoopKind::Loop),
        _ => None,
    }
}

/// Single-pass body walker producing events + branch/nesting metrics.
///
/// Metric conventions (catalog §2 approximations, documented here since the
/// catalog fixes thresholds, not formulas):
/// - cyclomatic = 1 + #\{if_expression, match_arm, for, while, loop, &&, ||, try_expression\}
/// - cognitive (SonarSource-style approximation): if_expression / loops /
///   try_expression cost `1 + current nesting` and increase nesting for their
///   bodies; match_arm and short-circuit operators cost a flat 1.
struct FuncScan<'a> {
    source: &'a [u8],
    /// `macro_rules!` definition names in the file (R3 macro-origin input).
    macro_names: &'a [String],
    /// Spans of recognized alloc-wrapper macro definitions (R3 macro-origin
    /// input).
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
    assignments: Vec<AssignmentEvent>,
    binary_ops: Vec<BinaryOpEvent>,
    comparisons: Vec<ComparisonEvent>,
    access_chains: Vec<AccessChainEvent>,
    loops: Vec<LoopNode>,
}

impl FuncScan<'_> {
    fn visit(&mut self, node: Node<'_>) {
        match node.kind() {
            "if_expression" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "try_expression" => {
                self.cyclomatic += 1;
                self.cognitive += 1 + self.nesting;
                self.nesting += 1;
                self.visit_children(node);
                self.nesting -= 1;
            }
            "match_arm" => {
                self.cyclomatic += 1;
                self.cognitive += 1;
                self.visit_children(node);
            }
            k @ ("for_expression" | "while_expression" | "loop_expression") => {
                let kind = loop_kind(k).expect("matched loop kind");
                self.enter_loop(node, kind);
            }
            "binary_expression" => self.visit_binary(node),
            "call_expression" => {
                self.emit_call(node);
                self.visit_children(node);
            }
            "macro_invocation" => {
                // R3 macro-as-call surface: the invocation is emitted as BOTH
                // a file-level `MacroEvent { kind: Use }` (D7, gathered over
                // the whole tree) and a function-attached `CallEvent` with
                // `is_macro = true` here. The duplication is deliberate:
                // call-family heuristics consume CallEvents, macro-honesty
                // consumes MacroEvents. The arguments live in an opaque
                // `token_tree`, so `args` stays empty.
                self.emit_macro_call(node);
                self.visit_children(node);
            }
            "assignment_expression" | "compound_assignment_expr" => {
                self.emit_assignment(node);
                self.visit_children(node);
            }
            "index_expression" | "field_expression" => {
                self.maybe_emit_access_chain(node);
                self.visit_children(node);
            }
            "function_item" => {
                // A nested item `fn` is its own unit; its body's events and
                // metrics must not be attributed to the enclosing function
                // (mirrors the file-level stop rule). Note the walk starts at
                // the enclosing function's `body` block, so THIS function is
                // never matched here.
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

    /// [`visit_children`](Self::visit_children) but skipping the direct child
    /// whose `Node::id` matches `skip` (used by [`enter_loop`](Self::enter_loop)
    /// to avoid re-visiting the `for` iterable, which it already visited at the
    /// outer depth).
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
        let macro_origin = self.macro_origin_of(&span, "");
        let induction_vars = if kind == LoopKind::For {
            node.child_by_field_name("pattern")
                .map(|p| pattern_identifiers(p, self.source))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        self.loops.push(LoopNode {
            kind,
            depth,
            induction_vars,
            span,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin,
            // No do-while shape in Rust: the `(0)`-wrapper gate never fires.
            condition_is_zero: false,
        });

        // Rust `for` differs from C's per-iteration increment: the iterable
        // expression (`value` field, e.g. `text.chars()` in `for c in
        // text.chars()`) is evaluated ONCE, before the loop body — it is NOT
        // part of the per-iteration body. Visit it at the OUTER depth and skip
        // it in the body walk; `in_for_iterable` applies the same exclusion to
        // `is_innermost` and `statement_role` (via `meta_for`/`call_position`),
        // so every event inside a `for` header is consistently out-of-loop.
        // `while`/`loop` conditions and bodies keep the inner depth.
        // See `docs/architecture/rust-backend.md` §3.8 and rust-catalog.md §3
        // LOOP-003.
        let iterable = if kind == LoopKind::For {
            node.child_by_field_name("value")
        } else {
            None
        };
        if let Some(it) = iterable {
            self.visit(it);
        }

        self.loop_depth = depth;
        self.nesting += 1;
        self.visit_children_except(node, iterable.map(|i| i.id()));
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
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                let meta =
                    self.meta_for(node, self.macro_origin_of(&ir_span(node, self.source), ""));
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
            "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" | "<<" | ">>" => {
                let meta =
                    self.meta_for(node, self.macro_origin_of(&ir_span(node, self.source), ""));
                let left = node.child_by_field_name("left").map(|n| self.shape_of(n));
                let right = node.child_by_field_name("right").map(|n| self.shape_of(n));
                if let (Some(left), Some(right)) = (left, right) {
                    self.binary_ops.push(BinaryOpEvent {
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
        let (callee_kind, callee_name, callee_path) = match callee {
            None => (CalleeKind::Unknown, String::new(), None),
            Some(c) => match c.kind() {
                "identifier" | "scoped_identifier" | "field_expression" | "generic_function" => (
                    CalleeKind::Direct,
                    final_callee_name(c, self.source),
                    Some(callee_path_of(c, self.source)),
                ),
                _ => (
                    CalleeKind::Indirect,
                    text(c, self.source).to_string(),
                    Some(text(c, self.source).to_string()),
                ),
            },
        };
        if callee_kind == CalleeKind::Direct
            && RUST_ALLOC_FAMILY.matches_site(callee_path.as_deref(), &callee_name, false)
        {
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
                        if child.is_named() && child.kind() != "attribute_item" {
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
            callee_path,
            is_macro: false,
            args,
            statement_role,
            conditional_guarded,
            semantic: None,
            receiver_root: None,
        });
    }

    /// R3 macro-as-call: emit a `CallEvent` for a `macro_invocation` with
    /// `is_macro = true`, `callee_kind = Direct`, `callee_name` = final
    /// segment of the `macro` path, `callee_path` = full macro path text, and
    /// empty `args` (the token tree is opaque). See the call site in
    /// [`FuncScan::visit`] for the deliberate MacroEvent+CallEvent duplication.
    fn emit_macro_call(&mut self, node: Node<'_>) {
        let Some(macro_node) = node.child_by_field_name("macro") else {
            return;
        };
        let callee_name = final_callee_name(macro_node, self.source);
        let callee_path = Some(text(macro_node, self.source).to_string());
        if RUST_ALLOC_FAMILY.matches_site(callee_path.as_deref(), &callee_name, true) {
            self.alloc_site_count += 1;
        }
        let (statement_role, conditional_guarded) = call_position(node);
        let macro_origin = self.macro_origin_of(&ir_span(node, self.source), &callee_name);
        let meta = self.meta_for(node, macro_origin);
        self.calls.push(CallEvent {
            meta,
            callee_name,
            callee_kind: CalleeKind::Direct,
            callee_path,
            is_macro: true,
            args: Vec::new(),
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
        let meta = self.meta_for(node, self.macro_origin_of(&ir_span(node, self.source), ""));
        self.assignments.push(AssignmentEvent {
            meta,
            target,
            scope_class,
            binding_kind: BindingKind::Unknown,
        });
    }

    /// Emit an [`AccessChainEvent`] only at the TOP of a member/subscript
    /// chain (parent is not itself a chain link) and only when there are ≥ 2
    /// elements (deeper chains are what LOOP-008 needs).
    fn maybe_emit_access_chain(&mut self, node: Node<'_>) {
        if node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "index_expression" | "field_expression"))
        {
            return;
        }
        let Some((root, elements)) = self.access_chain(node) else {
            return;
        };
        if elements.len() < 2 {
            return;
        }
        let meta = self.meta_for(node, self.macro_origin_of(&ir_span(node, self.source), ""));
        self.access_chains.push(AccessChainEvent {
            meta,
            root,
            elements,
        });
    }

    /// Decompose `a.b[i].c` into (root identifier, element shapes).
    fn access_chain(&self, node: Node<'_>) -> Option<(String, Vec<Shape>)> {
        match node.kind() {
            "index_expression" => {
                // No fields: base and index are positional named children.
                let base = node.named_child(0)?;
                let (root, mut elements) = self.access_chain(base)?;
                let index_ident = node
                    .named_child(1)
                    .and_then(|i| leftmost_identifier(i, self.source));
                elements.push(Shape {
                    kind: ShapeKind::Subscript,
                    index_ident,
                    ..Shape::default()
                });
                Some((root, elements))
            }
            "field_expression" => {
                // Base field is `value` (C's field_expression base is `argument`).
                let base = node.child_by_field_name("value")?;
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

    /// D6 scope classification: `Param` (incl. `self`) beats `Global`
    /// (static/const) as in the C backend; a non-identifier target is
    /// `Unknown`.
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

    /// R3 macro-origin detection (mirrors the C backend): true iff the event
    /// span intersects a recognized alloc-wrapper span, OR the event's
    /// `callee_name` equals a `macro_rules!` definition name in the file
    /// (wrapper USE site). Non-call events pass `""` and match on span only.
    fn macro_origin_of(&self, span: &Span, callee_name: &str) -> bool {
        self.pattern_spans.iter().any(|ps| ps.intersects(span))
            || (!callee_name.is_empty() && self.macro_names.iter().any(|m| m == callee_name))
    }

    fn meta_for(&self, node: Node<'_>, macro_origin: bool) -> EventMeta {
        EventMeta {
            span: ir_span(node, self.source),
            in_loop_depth: self.loop_depth,
            // A `for`-iterable event is out-of-loop by construction
            // (`in_loop_depth == 0`): it must not also claim `is_innermost`,
            // or `is_innermost`-keyed rules false-fire (RS-LOOP-004).
            is_innermost: !in_for_iterable(node) && is_innermost_event(node),
            resolver_tier: ResolverTier::Syntactic,
            macro_origin,
        }
    }

    fn shape_of(&self, node: Node<'_>) -> Shape {
        match node.kind() {
            "integer_literal" | "float_literal" | "string_literal" | "raw_string_literal"
            | "boolean_literal" | "char_literal" => Shape::literal(text(node, self.source)),
            "identifier" | "field_identifier" | "type_identifier" | "self" => {
                Shape::identifier(text(node, self.source))
            }
            "call_expression" => Shape {
                kind: ShapeKind::Call,
                name: node
                    .child_by_field_name("function")
                    .map(|c| text(c, self.source).to_string()),
                ..Shape::default()
            },
            "index_expression" => Shape {
                kind: ShapeKind::Subscript,
                index_ident: node
                    .named_child(1)
                    .and_then(|i| leftmost_identifier(i, self.source)),
                ..Shape::default()
            },
            "field_expression" => Shape {
                kind: ShapeKind::MemberAccess,
                depth: Some(chain_depth(node)),
                ..Shape::default()
            },
            "unary_expression" | "reference_expression" => Shape {
                kind: ShapeKind::Unary,
                ..Shape::default()
            },
            "binary_expression" => Shape {
                kind: ShapeKind::Binary,
                ..Shape::default()
            },
            "parenthesized_expression" => node
                .named_child(0)
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
        "index_expression" => node.named_child(0).map_or(1, |base| 1 + chain_depth(base)),
        "field_expression" => node
            .child_by_field_name("value")
            .map_or(1, |base| 1 + chain_depth(base)),
        _ => 0,
    }
}

/// D2/R3 full callee path text: `identifier` text, the `::`-joined
/// `scoped_identifier` text, the `<base>.<field>` `field_expression` text,
/// and for `generic_function` the underlying function's path WITHOUT the
/// turbofish (`foo::<T>()` ⇒ `"foo"`).
fn callee_path_of(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "generic_function" => node
            .child_by_field_name("function")
            .map(|f| callee_path_of(f, source))
            .unwrap_or_else(|| text(node, source).to_string()),
        _ => text(node, source).to_string(),
    }
}

/// D2 final callee segment.
fn final_callee_name(node: Node<'_>, source: &[u8]) -> String {
    match node.kind() {
        "identifier" => text(node, source).to_string(),
        "scoped_identifier" => node
            .child_by_field_name("name")
            .map(|n| text(n, source).to_string())
            .unwrap_or_else(|| text(node, source).to_string()),
        "field_expression" => node
            .child_by_field_name("field")
            .map(|f| text(f, source).to_string())
            .unwrap_or_default(),
        "generic_function" => node
            .child_by_field_name("function")
            .map(|f| final_callee_name(f, source))
            .unwrap_or_default(),
        _ => text(node, source).to_string(),
    }
}

/// Best-effort loop induction identifiers: identifiers named in a `for`
/// `pattern` (`for (i, x) in …` ⇒ `["i", "x"]`).
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

/// `is_innermost` operational rule (ir-v1.md §3.3 "inside the deepest
/// enclosing loop"): climb the ancestor chain to the NEAREST enclosing loop;
/// the event is innermost iff that loop exists AND nests no further loop
/// anywhere in its own body.
fn is_innermost_event(event: Node<'_>) -> bool {
    let Some(loop_node) = nearest_enclosing_loop(event) else {
        return false;
    };
    // A `for`'s iterable is evaluated once, outside the body, so a loop nested
    // inside it does NOT make the `for` non-innermost for body events (mirrors
    // `in_for_iterable`). Without this skip, `for x in v.iter().filter(|_| { while
    // … }) { … x / d … }` wrongly demoted the body's division to non-innermost.
    let iterable = if loop_node.kind() == "for_expression" {
        loop_node.child_by_field_name("value")
    } else {
        None
    };
    let mut cursor = loop_node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if Some(child.id()) != iterable.map(|i| i.id()) && contains_loop(child) {
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

/// Whether `node` lies inside the iterable expression (the `value` field) of its
/// **nearest** enclosing loop, when that loop is a `for_expression`. Rust's `for`
/// iterable is evaluated once, before the body, so such nodes are neither
/// in-loop nor body statements (`in_loop_depth == 0`, `is_innermost == false`,
/// `statement_role == Unknown`).
///
/// The walk stops at the **nearest loop of any kind**, so an event inside a
/// `while`/`loop` that itself sits in a `for` iterable is correctly in-loop
/// (that nearest loop is not a `for`) rather than wrongly excluded.
///
/// Accepted limitation (syntactic tier): a closure body inside the iterable
/// (e.g. `v.iter().map(|y| y / d)`) is treated as once-evaluated even though an
/// iterator adaptor may invoke it per element — the tier cannot know a closure's
/// invocation count. Documented as an accepted false-negative class in
/// `docs/heuristics/rust-catalog.md` §5.
fn in_for_iterable(node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if LOOP_KINDS.contains(&parent.kind()) {
            return parent.kind() == "for_expression"
                && parent
                    .child_by_field_name("value")
                    .is_some_and(|value| value.id() == current.id());
        }
        current = parent;
    }
    false
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

/// Position of a call relative to its enclosing loop (ir-v1.md §3.3): climb
/// ancestors innermost-out; bare `expression_statement` links are transparent,
/// as are `let_condition`/`let_chain` (`while let`/`if let` scrutinee
/// scaffolding) and a `block` that IS the enclosing loop's direct body child
/// (the common braced form must not bury the call);
/// `if_expression`/`match_expression` mark `conditional_guarded`; anything
/// else buries the call (`nested`). No enclosing loop ⇒ `unknown`.
fn call_position(node: Node<'_>) -> (StatementRole, bool) {
    // A call inside a `for` iterable (`value` field) is evaluated once, outside
    // the loop body: it is neither the loop's primary statement nor in-loop.
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
            // `while let` / `if let` wrap the condition scrutinee in a
            // `let_condition` (chained predicates in a `let_chain`); both are
            // transparent scaffolding, not a conditional branch.
            "let_condition" | "let_chain" => {}
            "block"
                if parent
                    .parent()
                    .is_some_and(|g| LOOP_KINDS.contains(&g.kind())) => {}
            "if_expression" | "match_expression" => {
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

#[cfg(test)]
mod tests {
    //! Backend extraction tests lock the Rust grammar assumptions above AND
    //! the span/ParseHealth contract (ir-v1.md §7) on real parsed nodes.

    use super::*;
    use crate::ir::{CalleeKind, IncludeForm, LoopKind, ParseStatus, ShapeKind, StatementRole};

    const SAMPLE: &str = r#"
use std::collections::HashMap;
use crate::util::{helper, Config as Cfg};
use super::module;

extern crate serde;

mod inline_mod {
    pub fn nested() -> i32 { 1 }
}

mod external_mod;

const LIMIT: usize = 16;
static mut COUNTER: i64 = 0;

struct Point {
    x: i64,
    y: i64,
}

struct Pair(u32, i64);

macro_rules! twice {
    ($x:expr) => { $x * 2 };
}

impl Point {
    fn sum(&self) -> i64 {
        self.x + self.y
    }
}

trait Shape {
    fn area(&self) -> i64;
    fn label(&self) -> &'static str { "shape" }
}

fn process(p: &Point, n: i32, items: &[i32]) -> i32 {
    let mut total = 0; // let_declaration is NOT an assignment event
    for value in items {
        total += value;
    }
    while total < n {
        total = total + 1;
    }
    loop {
        total += 1;
        if total > 100 { break; }
    }
    let doubled = twice!(total);
    let s = p.sum();
    let sz = std::mem::size_of::<i64>();
    log_value(doubled, s, sz);
    total
}
"#;

    fn sample_extraction() -> Extraction {
        extract("src/sample.rs", SAMPLE.as_bytes())
    }

    fn function<'a>(ex: &'a Extraction, name: &str) -> &'a FunctionUnit {
        ex.functions
            .iter()
            .find(|f| f.record.name == name)
            .unwrap_or_else(|| panic!("missing function {name}"))
    }

    #[test]
    fn extracts_file_level_records() {
        let ex = sample_extraction();
        assert_eq!(ex.parse_health.status, ParseStatus::Ok);
        assert_eq!(ex.file.language, "rust");
        // Functions: nested (inline_mod), sum (impl), label (trait default),
        // process. `area` is a body-less signature item (D5) and is skipped.
        let names: Vec<&str> = ex
            .functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect();
        assert_eq!(names, ["nested", "sum", "label", "process"]);
        assert_eq!(ex.file.function_count, 4);
        assert!(
            !ex.functions.iter().any(|f| f.record.name == "area"),
            "function_signature_item must be skipped (D5)"
        );
        // Globals: const LIMIT and static mut COUNTER, storage-mapped.
        let globals: Vec<(&str, StorageClass)> = ex
            .global_vars
            .iter()
            .map(|g| (g.name.as_str(), g.storage))
            .collect();
        assert_eq!(
            globals,
            [
                ("LIMIT", StorageClass::None),
                ("COUNTER", StorageClass::Static)
            ]
        );
        assert_eq!(ex.file.global_var_count, 2);
    }

    #[test]
    fn use_edges_fan_out_with_form_and_raw_paths() {
        let ex = sample_extraction();
        let raw_forms: Vec<(&str, IncludeForm)> = ex
            .includes
            .iter()
            .map(|e| (e.to_path.as_deref().unwrap_or(""), e.form))
            .collect();
        // HashMap (angle), helper + Config (crate:: ⇒ quoted), super::module
        // (quoted), external_mod (quoted mod), serde (angle extern crate).
        assert_eq!(
            raw_forms,
            [
                ("std::collections::HashMap", IncludeForm::Angle),
                ("crate::util::helper", IncludeForm::Quoted),
                ("crate::util::Config", IncludeForm::Quoted),
                ("super::module", IncludeForm::Quoted),
                ("serde", IncludeForm::Angle),
                ("external_mod", IncludeForm::Quoted),
            ]
        );
        assert_eq!(ex.file.include_counts.quoted, 4);
        assert_eq!(ex.file.include_counts.angle, 2);
        // Inline `mod inline_mod { … }` produces no edge (body walked).
        assert!(
            !ex.includes
                .iter()
                .any(|e| e.to_path.as_deref() == Some("inline_mod"))
        );
    }

    #[test]
    fn struct_defs_named_and_tuple() {
        let ex = sample_extraction();
        let point = ex.structs.iter().find(|s| s.name == "Point").unwrap();
        assert_eq!(point.members.len(), 2);
        assert_eq!(point.members[0].name, "x");
        assert_eq!(point.members[0].type_text, "i64");
        let pair = ex.structs.iter().find(|s| s.name == "Pair").unwrap();
        let positional: Vec<(&str, &str)> = pair
            .members
            .iter()
            .map(|m| (m.name.as_str(), m.type_text.as_str()))
            .collect();
        assert_eq!(positional, [("0", "u32"), ("1", "i64")]);
    }

    #[test]
    fn macro_definition_and_invocation_emitted() {
        let ex = sample_extraction();
        let twice_def = ex
            .macros
            .iter()
            .find(|m| m.kind == MacroEventKind::Definition && m.name == "twice")
            .expect("macro_definition");
        assert_eq!(twice_def.pattern_id, None);
        let twice_use = ex
            .macros
            .iter()
            .find(|m| m.kind == MacroEventKind::Use && m.name == "twice")
            .expect("macro_invocation use event");
        assert_eq!(twice_use.pattern_id, None);
    }

    #[test]
    fn function_record_fields_and_complexity() {
        let ex = sample_extraction();
        let f = function(&ex, "process");
        // p, n, items
        assert_eq!(f.record.param_count, 3);
        assert_eq!(
            f.record.param_types,
            vec![
                "&Point".to_string(),
                "i32".to_string(),
                "&[i32]".to_string()
            ]
        );
        // 1 + for + while + loop + if = 5
        assert_eq!(f.record.cyclomatic_complexity, 5);
        // for 1, while 1, loop 1, if (1 + nesting 1) = 2 ⇒ 5
        assert_eq!(f.record.cognitive_complexity, 5);
        assert_eq!(f.record.max_loop_depth, 1);
        assert_eq!(f.record.alloc_site_count, 0);
        assert_eq!(
            f.record.length_lines,
            f.record.span.end_line - f.record.span.start_line + 1
        );
        assert_eq!(
            f.record.id,
            format!(
                "src/sample.rs#process#{}:{}",
                f.record.span.start_line, f.record.span.start_col
            )
        );
    }

    #[test]
    fn method_receiver_is_a_parameter_and_self_target_is_param_scoped() {
        let ex = sample_extraction();
        let sum = function(&ex, "sum");
        assert_eq!(sum.record.param_count, 1);
        assert_eq!(sum.record.param_types, vec!["&self".to_string()]);
        // `self.x + self.y` has two field_expression shapes; no assignment.
        assert!(sum.assignments.is_empty());
    }

    #[test]
    fn calls_classify_callee_kind_and_final_name() {
        let ex = sample_extraction();
        let f = function(&ex, "process");
        let call = |name: &str| {
            f.calls
                .iter()
                .find(|c| c.callee_name == name)
                .unwrap_or_else(|| panic!("call {name}"))
        };
        // field_expression callee → Direct with field name (documented FP class).
        assert_eq!(call("sum").callee_kind, CalleeKind::Direct);
        // generic_function → recurse to the scoped name's final segment.
        assert_eq!(call("size_of").callee_kind, CalleeKind::Direct);
        assert_eq!(call("log_value").callee_kind, CalleeKind::Direct);
    }

    #[test]
    fn assignments_scope_and_let_exclusion() {
        let ex = sample_extraction();
        let f = function(&ex, "process");
        // assignment_expression (`total = total + 1`) + 2 compound assignments
        // (`total += value`, `total += 1`). `let mut total = 0` and
        // `let doubled = …` are NOT assignments (D6).
        assert_eq!(f.assignments.len(), 3, "{:?}", f.assignments);
        assert!(
            f.assignments
                .iter()
                .all(|a| a.scope_class == ScopeClass::Local)
        );
        // Global target classification against the collected statics/consts.
        let src = "static mut G: i64 = 0;\nfn f(mut g: i64) { G = 1; g = 2; }\n";
        let gx = extract("scope.rs", src.as_bytes());
        let scopes: Vec<ScopeClass> = gx.functions[0]
            .assignments
            .iter()
            .map(|a| a.scope_class)
            .collect();
        assert_eq!(scopes, [ScopeClass::Global, ScopeClass::Param]);
        // Non-identifier target → Unknown.
        let src2 = "struct S { a: i64 }\nfn f(mut s: S) { s.a = 1; }\n";
        let sx = extract("unknown.rs", src2.as_bytes());
        assert_eq!(
            sx.functions[0].assignments[0].scope_class,
            ScopeClass::Unknown
        );
    }

    #[test]
    fn loops_kinds_depth_and_induction_vars() {
        let ex = sample_extraction();
        let f = function(&ex, "process");
        let kinds: Vec<(LoopKind, u32)> = f.loops.iter().map(|l| (l.kind, l.depth)).collect();
        assert_eq!(
            kinds,
            [
                (LoopKind::For, 1),
                (LoopKind::While, 1),
                (LoopKind::Loop, 1)
            ]
        );
        assert_eq!(f.loops[0].induction_vars, vec!["value".to_string()]);
        assert!(f.loops[1].induction_vars.is_empty());
        assert_eq!(f.record.max_loop_depth, 1);
        // The `loop {}` is the innermost enclosing loop for its body events.
        let guarded = f
            .calls
            .iter()
            .find(|c| c.callee_name == "log_value")
            .unwrap();
        assert_eq!(guarded.meta.in_loop_depth, 0);
        assert!(!guarded.meta.is_innermost);
        assert!(
            !guarded.meta.macro_origin,
            "log_value is neither inside a recognized wrapper nor named by a local macro_definition"
        );
    }

    #[test]
    fn comparisons_and_binary_ops_emitted_with_rust_operators() {
        let ex = sample_extraction();
        let f = function(&ex, "process");
        // `total < n`, `total > 100`
        assert_eq!(f.comparisons.len(), 2);
        // `total + 1` and `total + value` pairs? `total += value` is compound
        // (assignment, not binary); `total = total + 1` yields one `+`.
        assert!(
            f.binary_ops.iter().any(|b| b.op == "+"),
            "additive binary op present"
        );
        assert!(f.binary_ops.iter().all(|b| b.op != "+="));
    }

    #[test]
    fn access_chain_from_field_and_index() {
        let src = "struct S { a: [i64; 4] }\nfn f(s: &S, i: usize) -> i64 { s.a[i] }\n";
        let ex = extract("chain.rs", src.as_bytes());
        let f = &ex.functions[0];
        assert_eq!(f.access_chains.len(), 1, "{:?}", f.access_chains);
        assert_eq!(f.access_chains[0].root, "s");
        assert_eq!(f.access_chains[0].elements.len(), 2);
        assert_eq!(f.access_chains[0].elements[0].kind, ShapeKind::MemberAccess);
        assert_eq!(f.access_chains[0].elements[1].kind, ShapeKind::Subscript);
    }

    /// M1: associated `const`/`static` inside `impl`/`trait` bodies are NOT
    /// file-level globals (they must not pollute `global_var_count` or the
    /// scope-classification name set).
    #[test]
    fn associated_const_static_are_not_file_level_globals() {
        let src = concat!(
            "struct S;\n",
            "impl S {\n",
            "    const MAX: i32 = 1;\n",
            "    static G: i32 = 2;\n",
            "    fn m(&self) {}\n",
            "}\n",
        );
        let ex = extract("assoc.rs", src.as_bytes());
        assert_eq!(ex.file.global_var_count, 0);
        assert!(ex.global_vars.is_empty(), "{:?}", ex.global_vars);

        // A local that shadows an associated const name stays Local, not Global.
        let src2 = concat!(
            "struct S;\n",
            "impl S { const MAX: i32 = 1; }\n",
            "fn f() { let mut MAX = 0; MAX = 1; }\n",
        );
        let ex2 = extract("shadow.rs", src2.as_bytes());
        let scopes: Vec<ScopeClass> = ex2.functions[0]
            .assignments
            .iter()
            .map(|a| a.scope_class)
            .collect();
        assert_eq!(scopes, [ScopeClass::Local]);
    }

    /// N4: associated consts inside an `impl`/`trait` nested in an inline
    /// module are still not file-level globals (`inside_type_body` holds
    /// regardless of module nesting).
    #[test]
    fn associated_const_in_nested_module_impl_is_not_global() {
        let src = concat!(
            "mod m {\n",
            "    struct T;\n",
            "    impl T { const C: u8 = 0; }\n",
            "}\n",
            "trait Tr { const K: u8; }\n",
        );
        let ex = extract("nested_impl.rs", src.as_bytes());
        assert_eq!(ex.file.global_var_count, 0);
        assert!(ex.global_vars.is_empty(), "{:?}", ex.global_vars);
    }

    /// M1 regression: genuine module/file-level `static`/`const` are still
    /// collected and still classify an assignment target as `Global`.
    #[test]
    fn module_level_static_const_still_collected_and_global() {
        let src = "const K: i32 = 1;\nstatic mut G: i64 = 0;\nfn f() { G = 1; }\n";
        let ex = extract("module_globals.rs", src.as_bytes());
        let names: Vec<&str> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, ["K", "G"]);
        assert_eq!(ex.file.global_var_count, 2);
        assert_eq!(
            ex.functions[0].assignments[0].scope_class,
            ScopeClass::Global
        );
    }

    /// m1: `use path::{self, x}` — the `self` leaf is `path` itself.
    #[test]
    fn grouped_use_self_leaf_resolves_to_prefix() {
        let ex = extract("selfuse.rs", b"use crate::util::{self, helper};\n");
        let raws: Vec<&str> = ex
            .includes
            .iter()
            .map(|e| e.to_path.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(raws, ["crate::util", "crate::util::helper"]);
        assert!(ex.includes.iter().all(|e| e.form == IncludeForm::Quoted));
    }

    /// m5 (D1): wildcard `use` leaves record the module path (trailing
    /// `::*`/`*` dropped), including inside a grouped list.
    #[test]
    fn wildcard_use_leaves_record_module_path() {
        let ex = extract(
            "wild.rs",
            b"use std::collections::*;\nuse crate::alpha::{beta, *};\n",
        );
        let pairs: Vec<(&str, IncludeForm)> = ex
            .includes
            .iter()
            .map(|e| (e.to_path.as_deref().unwrap_or(""), e.form))
            .collect();
        assert_eq!(
            pairs,
            [
                ("std::collections", IncludeForm::Angle),
                ("crate::alpha::beta", IncludeForm::Quoted),
                ("crate::alpha", IncludeForm::Quoted),
            ]
        );
    }

    /// Round-4 Minor 1: a nested `mod foo;` carries the inline-module `/`
    /// prefix in its edge raw (never `::`, so it stays a mod edge).
    #[test]
    fn inline_module_mod_edge_uses_slash_prefix() {
        let ex = extract("src/lib.rs", b"mod outer { mod inner { mod leaf; } }\n");
        let raws: Vec<&str> = ex
            .includes
            .iter()
            .map(|e| e.to_path.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(raws, ["outer/inner/leaf"]);
        assert!(ex.includes.iter().all(|e| e.form == IncludeForm::Quoted));
    }

    /// D4 (extended by R3): `struct_item`/`union_item`/`enum_item` all produce
    /// a `StructDef`, discriminated by `kind`.
    #[test]
    fn struct_union_enum_emit_struct_def_with_kind() {
        let ex = extract(
            "unions.rs",
            b"union U { a: i32, b: f32 }\nenum E { A, B(i32) }\nstruct S { x: i32 }\n",
        );
        let kinds: Vec<(&str, StructKind)> = ex
            .structs
            .iter()
            .map(|s| (s.name.as_str(), s.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("U", StructKind::Union),
                ("E", StructKind::Enum),
                ("S", StructKind::Struct)
            ]
        );
        let union = ex.structs.iter().find(|s| s.name == "U").unwrap();
        assert_eq!(union.members.len(), 2, "union members from field list");
        let enumeration = ex.structs.iter().find(|s| s.name == "E").unwrap();
        assert!(
            enumeration.members.is_empty(),
            "enum variants are not layout members"
        );
    }

    /// D4 (R3): member pointer/array derivation for named + tuple members.
    #[test]
    fn member_flags_derive_pointer_and_array_dim() {
        let src = concat!(
            "struct S { a: &str, b: *const u8, c: [u8; 16], d: [u8; N] }\n",
            "struct P(&'static str, *const T, [i64; 4]);\n",
        );
        let ex = extract("flags.rs", src.as_bytes());
        let s = ex.structs.iter().find(|s| s.name == "S").unwrap();
        assert!(s.members[0].is_pointer, "&str is a reference");
        assert!(s.members[1].is_pointer, "*const u8 is a pointer");
        assert_eq!(s.members[2].array_dim_literal, Some(16));
        assert_eq!(s.members[3].array_dim_literal, None, "non-literal length");
        let p = ex.structs.iter().find(|s| s.name == "P").unwrap();
        assert!(p.members[0].is_pointer);
        assert!(p.members[1].is_pointer);
        assert_eq!(p.members[2].array_dim_literal, Some(4));
    }

    /// m3: a nested item `fn` body is not attributed to the enclosing fn.
    #[test]
    fn nested_function_body_excluded_from_outer() {
        let src = concat!(
            "fn outer() {\n",
            "    helper();\n",
            "    fn inner() { inner_helper(); }\n",
            "}\n",
        );
        let ex = extract("nested_fn.rs", src.as_bytes());
        let names: Vec<&str> = ex
            .functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect();
        assert_eq!(names, ["outer"], "nested fn is not a separate unit");
        let outer = &ex.functions[0];
        assert!(outer.calls.iter().any(|c| c.callee_name == "helper"));
        assert!(
            !outer.calls.iter().any(|c| c.callee_name == "inner_helper"),
            "inner fn events must not leak into outer: {:?}",
            outer.calls
        );
    }

    /// m4: a call in a `while let` condition is the loop's primary statement,
    /// not buried (the `let_condition` link is transparent).
    #[test]
    fn call_in_while_let_condition_is_primary() {
        let src = concat!(
            "fn f(n: i32) {\n",
            "    while let Some(x) = fetch(n) {\n",
            "        consume(x);\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("whilelet.rs", src.as_bytes());
        let f = &ex.functions[0];
        let fetch = f.calls.iter().find(|c| c.callee_name == "fetch").unwrap();
        assert_eq!(fetch.statement_role, StatementRole::Primary);
        assert_eq!(fetch.meta.in_loop_depth, 1);
        assert!(!fetch.conditional_guarded);
        let consume = f.calls.iter().find(|c| c.callee_name == "consume").unwrap();
        assert_eq!(consume.statement_role, StatementRole::Primary);
    }

    /// R4 dogfood fix: the `for` iterable is evaluated ONCE, before the body,
    /// so `for c in text.chars()` is NOT char-wise work inside the loop
    /// (RS-LOOP-003). The iterable is visited at the outer depth; the body
    /// keeps the inner depth.
    #[test]
    fn for_iterable_is_outside_loop_body_depth() {
        let src = concat!(
            "fn count(text: &str) -> usize {\n",
            "    let mut total = 0;\n",
            "    for _c in text.chars() {\n",
            "        total += 1;\n",
            "    }\n",
            "    total\n",
            "}\n",
        );
        let ex = extract("forheader.rs", src.as_bytes());
        let f = &ex.functions[0];
        let chars = f.calls.iter().find(|c| c.callee_name == "chars").unwrap();
        assert_eq!(
            chars.meta.in_loop_depth, 0,
            "a `for` iterable is outside the loop body"
        );
        assert!(
            !chars.meta.is_innermost,
            "a `for` iterable event must not claim `is_innermost`"
        );
        assert_eq!(chars.statement_role, StatementRole::Unknown);
        assert_eq!(f.loops.len(), 1);
        assert_eq!(f.loops[0].depth, 1);
        assert_eq!(f.record.max_loop_depth, 1);
    }

    /// Companion positive: the same `chars()` call inside the loop BODY keeps
    /// depth 1 and `Primary`, so RS-LOOP-003 still fires on the intended shape.
    #[test]
    fn charwise_call_in_loop_body_stays_in_loop() {
        let src = concat!(
            "fn scan(text: &str, n: usize) {\n",
            "    for _i in 0..n {\n",
            "        text.chars();\n",
            "    }\n",
            "}\n",
        );
        let ex = extract("forbody.rs", src.as_bytes());
        let f = &ex.functions[0];
        let chars = f.calls.iter().find(|c| c.callee_name == "chars").unwrap();
        assert_eq!(chars.meta.in_loop_depth, 1);
        assert_eq!(chars.statement_role, StatementRole::Primary);
    }

    /// A `for` header whose iterable itself contains a call still runs that
    /// call at depth 0 while the body's work stays in-loop (mixed check).
    #[test]
    fn for_iterable_call_does_not_shadow_body_depth() {
        let src = concat!(
            "fn run(rows: &[u8], n: usize) {\n",
            "    for r in rows.iter() {\n",
            "        consume(*r);\n",
            "    }\n",
            "    let _ = n;\n",
            "}\n",
        );
        let ex = extract("formixed.rs", src.as_bytes());
        let f = &ex.functions[0];
        let it = f.calls.iter().find(|c| c.callee_name == "iter").unwrap();
        assert_eq!(it.meta.in_loop_depth, 0);
        assert!(!it.meta.is_innermost);
        let consume = f.calls.iter().find(|c| c.callee_name == "consume").unwrap();
        assert_eq!(consume.meta.in_loop_depth, 1);
    }

    /// Reviewer H1 regression: arithmetic in the `for` iterable is evaluated
    /// once, outside the body, so it must NOT be `is_innermost` — RS-LOOP-004 is
    /// keyed on `is_innermost` alone and false-fired on `for i in 0..(n / d)`.
    /// The identical operation inside the body stays innermost + in-loop.
    #[test]
    fn for_iterable_arithmetic_is_not_innermost() {
        let iterable = concat!(
            "fn f(n: u32, d: u32) -> u32 {\n",
            "    let mut t = 0;\n",
            "    for i in 0..(n / d) {\n",
            "        t += i;\n",
            "    }\n",
            "    t\n",
            "}\n",
        );
        let ex = extract("foriter.rs", iterable.as_bytes());
        let f = &ex.functions[0];
        let div = f.binary_ops.iter().find(|b| b.op == "/").unwrap();
        assert_eq!(div.meta.in_loop_depth, 0);
        assert!(
            !div.meta.is_innermost,
            "iterable arithmetic is not innermost"
        );

        let body = concat!(
            "fn g(n: u32, d: u32) -> u32 {\n",
            "    let mut t = 0;\n",
            "    for i in 0..n {\n",
            "        t += i / d;\n",
            "    }\n",
            "    t\n",
            "}\n",
        );
        let ex = extract("forbodyarith.rs", body.as_bytes());
        let f = &ex.functions[0];
        let div = f.binary_ops.iter().find(|b| b.op == "/").unwrap();
        assert_eq!(div.meta.in_loop_depth, 1);
        assert!(div.meta.is_innermost, "body arithmetic stays innermost");
    }

    /// Reviewer NEW-1 regression: a `while`/`loop` nested inside a `for`
    /// iterable is a REAL loop, so its body stays in-loop + innermost — the
    /// exclusion must stop at the nearest loop of ANY kind, not walk through it.
    #[test]
    fn while_inside_for_iterable_stays_in_loop() {
        let src = concat!(
            "fn f(v: &[u32], d: u32) -> u32 {\n",
            "    let mut t = 0;\n",
            "    for x in v.iter().filter(|_z| {\n",
            "        let mut i = 1u32;\n",
            "        while i < 4 { let _q = d / i; i += 1; }\n",
            "        true\n",
            "    }) { t += x; }\n",
            "    t\n",
            "}\n",
        );
        let ex = extract("forwhileiter.rs", src.as_bytes());
        let f = &ex.functions[0];
        let div = f.binary_ops.iter().find(|b| b.op == "/").unwrap();
        assert_eq!(
            div.meta.in_loop_depth, 1,
            "the `while` nested in the iterable is a real loop"
        );
        assert!(div.meta.is_innermost, "nearest loop is the nested `while`");
    }

    /// Aligned in R4 (was a pre-existing FN): a loop nested inside a `for`
    /// iterable must NOT demote the `for` body's events from `is_innermost`.
    #[test]
    fn loop_in_for_iterable_does_not_demote_body_innermost() {
        let src = concat!(
            "fn f(v: &[u32], d: u32) -> u32 {\n",
            "    let mut t = 0;\n",
            "    for x in v.iter().filter(|_z| { let mut i = 1u32; while i < 4 { i += 1; } true }) {\n",
            "        t += x / d;\n",
            "    }\n",
            "    t\n",
            "}\n",
        );
        let ex = extract("foriterbody.rs", src.as_bytes());
        let f = &ex.functions[0];
        let div = f.binary_ops.iter().find(|b| b.op == "/").unwrap();
        assert_eq!(div.meta.in_loop_depth, 1);
        assert!(
            div.meta.is_innermost,
            "a loop in the iterable must not demote the body"
        );
    }

    #[test]
    fn attribute_siblings_do_not_suppress_annotated_items() {
        // The guarded-content non-recurrence lock: `#[cfg]`/`#[inline]` are
        // ordinary siblings, so the annotated items extract normally.
        let src = concat!(
            "#[cfg(unix)]\n",
            "#[inline]\n",
            "fn guarded(n: i32) -> i32 {\n",
            "    n + 1\n",
            "}\n",
            "\n",
            "#[cfg(not(unix))]\n",
            "fn other() -> i32 { 0 }\n",
        );
        let ex = extract("attrs.rs", src.as_bytes());
        let names: Vec<&str> = ex
            .functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect();
        assert_eq!(names, ["guarded", "other"]);
        assert_eq!(ex.parse_health.status, ParseStatus::Ok);
    }

    #[test]
    fn malformed_source_yields_partial_health_with_regions() {
        let ex = extract("bad.rs", b"fn broken( {");
        assert_eq!(ex.parse_health.status, ParseStatus::Partial);
        assert!(ex.parse_health.error_node_count >= 1);
        assert!(!ex.parse_health.error_regions.is_empty());
        assert_eq!(ex.file.language, "rust");
    }

    #[test]
    fn span_conversion_is_exact_for_a_known_node() {
        let src = "fn main() {\n    helper();\n}\n";
        let ex = extract("span.rs", src.as_bytes());
        let call = &ex.functions[0].calls[0];
        assert_eq!(call.meta.span.start_line, 2);
        // `    helper();` → `helper` starts at 1-based byte column 5, the call
        // spans through `)` (inclusive) at column 12.
        assert_eq!(call.meta.span.start_col, 5);
        assert_eq!(call.meta.span.end_line, 2);
        assert_eq!(call.meta.span.end_col, 12);
    }

    #[test]
    fn parse_health_ok_and_loc_comment_kinds() {
        let src = "// line comment\n/* block\n   comment */\nfn main() {}\n";
        let ex = extract("loc.rs", src.as_bytes());
        assert_eq!(ex.file.loc.comment, 3);
        assert_eq!(ex.file.loc.code, 1);
    }

    /// `find_descendant` parity helper: still resolves a nested kind.
    #[test]
    fn find_descendant_resolves_nested_kind() {
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        let tree = parser.parse("fn f() { let x = 1; }", None).expect("tree");
        assert!(find_descendant(tree.root_node(), "let_declaration").is_some());
    }
}
