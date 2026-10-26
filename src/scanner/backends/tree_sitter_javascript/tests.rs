//! JavaScript backend unit tests (plan `js-ts`, wave B2). They lock the pinned
//! grammar's node kinds, the synthetic module record, entry-declaration
//! detection, include-edge encoding, nested-function ownership, loop/alloc/
//! unary/await/try events, and determinism over literal source strings.

use super::*;
use crate::ir::{
    AllocKind, CalleeKind, IncludeForm, LoopKind, ParseStatus, ShapeKind, StatementRole,
};
use crate::scanner::FunctionUnit;

fn extract_js(source: &str) -> Extraction {
    extract("test.js", source.as_bytes())
}

fn function<'a>(ex: &'a Extraction, name: &str) -> &'a FunctionUnit {
    ex.functions
        .iter()
        .find(|f| f.record.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

/// A clean module parses `Ok`, emits the synthetic `<module>` record first, and
/// reports the JS language token.
#[test]
fn clean_parse_yields_ok_and_module_record_first() {
    let ex = extract_js("const x = 1;\n");
    assert_eq!(ex.parse_health.status, ParseStatus::Ok);
    assert_eq!(ex.parse_health.error_node_count, 0);
    assert_eq!(ex.file.language, "js");
    assert_eq!(ex.functions[0].record.name, "<module>");
    assert!(ex.functions[0].record.is_module_scope);
    assert_eq!(ex.file.function_count, 1);
}

/// Malformed source → `Partial` with outermost error regions; never panics.
#[test]
fn malformed_source_is_partial_with_regions() {
    let ex = extract_js("function broken( {\n  return 1;\n");
    assert_eq!(ex.parse_health.status, ParseStatus::Partial);
    assert!(ex.parse_health.error_node_count >= 1);
    assert!(!ex.parse_health.error_regions.is_empty());
}

/// Relative specifiers are `Quoted`; bare/URL/`node:` specifiers are `Angle`.
/// Static `require("…")` calls count as edges too, and dynamic requires do not.
#[test]
fn import_export_and_require_edge_encoding() {
    let src = "import a from \"./a.js\";\n\
               import b from \"pkg\";\n\
               export { c } from \"../c.js\";\n\
               export * from \"node:fs\";\n\
               const d = require(\"./d\");\n\
               const e = require(\"legacy-module\");\n\
               const f = require(name);\n";
    let ex = extract_js(src);
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|e| e.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(
        raws,
        [
            "./a.js",
            "pkg",
            "../c.js",
            "node:fs",
            "./d",
            "legacy-module"
        ],
        "{:?}",
        ex.includes
    );
    let forms: Vec<IncludeForm> = ex.includes.iter().map(|e| e.form).collect();
    assert_eq!(
        forms,
        [
            IncludeForm::Quoted,
            IncludeForm::Angle,
            IncludeForm::Quoted,
            IncludeForm::Angle,
            IncludeForm::Quoted,
            IncludeForm::Angle,
        ]
    );
    assert!(
        ex.includes
            .iter()
            .all(|e| !e.resolved && !e.is_mod && e.from_path == "test.js")
    );
    assert_eq!(ex.file.include_counts.quoted, 3);
    assert_eq!(ex.file.include_counts.angle, 3);
}

/// Classes become `StructDef`s with field names and empty type text; top-level
/// declarations become globals, declarations inside functions do not.
#[test]
fn classes_and_top_level_globals() {
    let src = "const LIMIT = 10;\n\
               class Report {\n  name = \"\";\n  count = 0;\n  render() { return this.name; }\n}\n\
               const anon = class { x = 1; };\n\
               function f() { const inner = 1; return inner; }\n";
    let ex = extract_js(src);
    let report = ex.structs.iter().find(|s| s.name == "Report").unwrap();
    let member_names: Vec<&str> = report.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(member_names, ["name", "count"], "{:?}", report.members);
    assert!(report.members.iter().all(|m| m.type_text.is_empty()));
    assert!(ex.structs.iter().any(|s| s.name.is_empty()));

    let globals: Vec<&str> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(globals, ["LIMIT", "anon"], "{:?}", ex.global_vars);
}

/// Nested functions are their own records and own their events; the enclosing
/// record sees only its own loop.
#[test]
fn nested_function_ownership_and_boundaries() {
    let src = "function outer() {\n\
                 for (let i = 0; i < 2; i++) {}\n\
                 function inner() {\n\
                   for (let j = 0; j < 3; j++) {}\n\
                 }\n\
               }\n";
    let ex = extract_js(src);
    let outer = function(&ex, "outer");
    let inner = function(&ex, "inner");
    assert_eq!(outer.loops.len(), 1, "{:?}", outer.loops);
    assert_eq!(outer.record.max_loop_depth, 1);
    assert_eq!(inner.loops.len(), 1, "{:?}", inner.loops);
    assert_eq!(inner.record.max_loop_depth, 1);
}

/// All four loop kinds map to `LoopNode`; canonical `for` bounds infer a trip
/// count (including `<=` +1) and clamp to `MAX_TRIP`; non-canonical shapes stay
/// `None`.
#[test]
fn loop_kinds_and_trip_counts() {
    let src = "function f(items) {\n\
                 for (let i = 0; i < 10; i++) {}\n\
                 for (i = 0; i <= 2; i++) {}\n\
                 for (let k = 0; k < 10000000; k++) {}\n\
                 for (let u = 0; u < items.length; u++) {}\n\
                 for (const item of items) {}\n\
                 for (const key in items) {}\n\
                 while (items.length) {}\n\
                 do {} while (items.length);\n\
               }\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    let loops = &f.loops;
    assert_eq!(loops.len(), 8, "{:?}", loops);
    assert_eq!(loops[0].kind, LoopKind::For);
    assert_eq!(loops[0].trip_count, Some(10));
    assert_eq!(loops[1].trip_count, Some(3), "<= bound adds one");
    assert_eq!(loops[2].trip_count, Some(crate::ir::MAX_TRIP));
    assert_eq!(loops[3].trip_count, None, "non-literal bound");
    assert_eq!(loops[4].kind, LoopKind::For);
    assert_eq!(loops[4].trip_count, None, "for...of is not a literal range");
    assert_eq!(loops[5].trip_count, None);
    assert_eq!(loops[6].kind, LoopKind::While);
    assert_eq!(loops[7].kind, LoopKind::Do);
    assert_eq!(f.record.max_loop_depth, 1);
}

/// Loop headers are walked at the enclosing depth (not per body iteration).
#[test]
fn loop_header_events_are_outside_body_depth() {
    let src =
        "function f(items) {\n  for (let i = 0; i < count(items); i++) {\n    use(i);\n  }\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    let count = f.calls.iter().find(|c| c.callee_name == "count").unwrap();
    assert_eq!(count.meta.in_loop_depth, 0);
    assert!(!count.meta.is_innermost);
    let use_call = f.calls.iter().find(|c| c.callee_name == "use").unwrap();
    assert_eq!(use_call.meta.in_loop_depth, 1);
    assert!(use_call.meta.is_innermost);
}

/// `await` and `try` events count and carry loop context; `async` is flagged.
#[test]
fn await_try_and_async_flags() {
    let src = "async function f(xs) {\n\
                 for (const x of xs) {\n\
                   try { await consume(x); } catch (err) { fallback(); }\n\
                 }\n\
                 await finish();\n\
               }\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    assert!(f.record.is_async);
    assert_eq!(f.awaits.len(), 2, "{:?}", f.awaits);
    assert!(f.awaits.iter().any(|a| a.meta.in_loop_depth >= 1));
    assert!(f.awaits.iter().any(|a| a.meta.in_loop_depth == 0));
    assert_eq!(f.try_blocks.len(), 1);
    assert_eq!(f.try_blocks[0].meta.in_loop_depth, 1);
}

/// Only `delete` emits a `UnaryEvent`; other unary operators do not.
#[test]
fn delete_unary_event_only() {
    let src = "function f(obj) {\n  delete obj.prop;\n  !obj.flag;\n  obj.count++;\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    assert_eq!(f.unaries.len(), 1, "{:?}", f.unaries);
    assert_eq!(f.unaries[0].op, "delete");
    assert_eq!(f.unaries[0].operand.kind, ShapeKind::MemberAccess);
    assert_eq!(
        f.unaries[0].operand.name.as_deref(),
        Some("prop"),
        "member-expression shapes must carry the property name (TYPE-001 evidence)"
    );
    assert_eq!(
        f.unaries[0].operand.depth,
        Some(1),
        "member-chain depth is preserved alongside the property name"
    );
}

/// TYPE-001 fresh-object gate (B5): `operand_root_fresh` is true only when the
/// receiver root was bound in the same function to a fresh object/array/`new`
/// value before the `delete`.
#[test]
fn delete_operand_root_fresh_flag_tracks_fresh_bindings() {
    // Declarator-bound object literal at module scope → fresh.
    let ex = extract_js("const merged = { a: 1 };\ndelete merged.a;\n");
    let module = function(&ex, "<module>");
    assert_eq!(module.unaries.len(), 1, "{:?}", module.unaries);
    assert!(
        module.unaries[0].operand_root_fresh,
        "a declarator-bound object literal is fresh"
    );

    // A parameter is caller-owned shared state → not fresh.
    let ex = extract_js("function f(record) {\n  delete record.prop;\n}\n");
    let f = function(&ex, "f");
    assert_eq!(f.unaries.len(), 1, "{:?}", f.unaries);
    assert!(
        !f.unaries[0].operand_root_fresh,
        "a parameter receiver is not fresh"
    );

    // A copy built from a spread object literal is itself fresh.
    let ex =
        extract_js("function g(shared) {\n  const copy = { ...shared };\n  delete copy.x;\n}\n");
    let g = function(&ex, "g");
    assert_eq!(g.unaries.len(), 1, "{:?}", g.unaries);
    assert!(
        g.unaries[0].operand_root_fresh,
        "a spread object literal binding is fresh"
    );

    // Rebinding to a non-fresh value clears the flag (source order).
    let ex = extract_js("function h() {\n  let o = {};\n  o = getGlobal();\n  delete o.x;\n}\n");
    let h = function(&ex, "h");
    assert_eq!(h.unaries.len(), 1, "{:?}", h.unaries);
    assert!(
        !h.unaries[0].operand_root_fresh,
        "a later non-fresh assignment must clear the binding"
    );

    // A delete BEFORE the fresh assignment is not fresh (source order).
    let ex = extract_js("function i() {\n  delete o.x;\n  o = {};\n}\n");
    let i = function(&ex, "i");
    assert_eq!(i.unaries.len(), 1, "{:?}", i.unaries);
    assert!(
        !i.unaries[0].operand_root_fresh,
        "a fresh assignment after the delete must not back-date the flag"
    );
}

/// B5 (`R-tb-017`): only a LEXICAL BINDING can be the target of a bare
/// self-call. Declarations and named function expressions bind their name;
/// `const`/assignment-bound-anonymous functions inherit the declarator binding.
/// Class/object methods and property/field arrows are property keys, not
/// bindings, so a bare same-name call inside them is delegation.
#[test]
fn name_is_lexical_binding_distinguishes_declarations_from_method_properties() {
    let src = "function declared() {}\n\
               const arrow = () => {};\n\
               const named = function inner() {};\n\
               const anonymous = function () {};\n\
               class C { method() {} }\n\
               const obj = { property: () => {}, method() {}, fn: function () {} };\n\
               class F { field = () => {}; }\n";
    let ex = extract_js(src);
    for name in ["declared", "arrow", "inner", "anonymous"] {
        assert!(
            function(&ex, name).record.name_is_lexical_binding,
            "{name} must be a lexical binding"
        );
    }
    for name in ["method", "property", "fn", "field"] {
        assert!(
            !function(&ex, name).record.name_is_lexical_binding,
            "{name} is a property name, not a binding"
        );
    }
}

/// `new`, object and array literals each emit an `AllocEvent` and count toward
/// `alloc_site_count`; `referenced_names` is sorted/deduped and excludes keys;
/// only array literals with a spread child set `has_spread`.
#[test]
fn allocation_events_and_referenced_names() {
    let src = "function f(a, b) {\n  const o = { a: 1, b: [x, y, x] };\n  const n = new Widget(b, a);\n  const s = [...a, b];\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    assert_eq!(f.record.alloc_site_count, 4, "{:?}", f.allocations);
    let kinds: Vec<AllocKind> = f.allocations.iter().map(|a| a.kind).collect();
    assert_eq!(
        kinds,
        [
            AllocKind::Object,
            AllocKind::Array,
            AllocKind::New,
            AllocKind::Array
        ]
    );
    assert_eq!(f.allocations[0].referenced_names, ["x", "y"]);
    assert_eq!(f.allocations[1].referenced_names, ["x", "y"]);
    assert_eq!(f.allocations[2].referenced_names, ["a", "b"]);
    // Object/new never carry a spread fact; the spread array does.
    assert!(!f.allocations[0].has_spread);
    assert!(!f.allocations[1].has_spread, "plain array literal");
    assert!(!f.allocations[2].has_spread);
    assert!(
        f.allocations[3].has_spread,
        "`[...a, b]` has a spread child"
    );
    assert_eq!(f.allocations[3].referenced_names, ["a", "b"]);
    let widget = f
        .calls
        .iter()
        .find(|c| c.callee_name == "Widget")
        .expect("constructor call event");
    assert_eq!(widget.statement_role, StatementRole::Nested);
}

/// Member/subscript calls classify and carry the chain receiver root and full
/// callee path; a lone call statement is `Primary`.
#[test]
fn call_classification_and_statement_role() {
    let src = "function f(handlers, conn) {\n  g();\n  conn.execute(q);\n  handlers[0]();\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    let g = f.calls.iter().find(|c| c.callee_name == "g").unwrap();
    assert_eq!(g.callee_kind, CalleeKind::Direct);
    assert_eq!(g.statement_role, StatementRole::Primary);
    let execute = f.calls.iter().find(|c| c.callee_name == "execute").unwrap();
    assert_eq!(execute.callee_kind, CalleeKind::Direct);
    assert_eq!(execute.receiver_root.as_deref(), Some("conn"));
    assert_eq!(execute.callee_path.as_deref(), Some("conn.execute"));
    let subscript = f
        .calls
        .iter()
        .find(|c| c.callee_name == "handlers")
        .unwrap();
    assert_eq!(subscript.callee_kind, CalleeKind::Indirect);
}

/// Assignments classify Param/Local/Unknown; `+=` emits a `BinaryOpEvent`.
#[test]
fn assignment_scope_class_and_augmented_binop() {
    let src = "function f(p) {\n  let local = 0;\n  local += 1;\n  p = local;\n  other = 2;\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    let param_target = f
        .assignments
        .iter()
        .find(|a| a.target.name.as_deref() == Some("p"))
        .unwrap();
    assert_eq!(param_target.scope_class, crate::ir::ScopeClass::Param);
    let other = f
        .assignments
        .iter()
        .find(|a| a.target.name.as_deref() == Some("other"))
        .unwrap();
    assert_eq!(other.scope_class, crate::ir::ScopeClass::Unknown);
    assert!(
        f.binary_ops.iter().any(|b| b.op == "+="),
        "{:?}",
        f.binary_ops
    );
}

/// Comparisons and plain binary operators split into the right event streams;
/// member chains emit exactly one `AccessChainEvent`.
#[test]
fn comparison_binary_and_access_chain_events() {
    let src = "function f(a, b, s) {\n  if (a < b && s.a[0].b) { return a + b; }\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    assert!(
        f.comparisons.iter().any(|c| c.operator == "<"),
        "{:?}",
        f.comparisons
    );
    assert!(
        f.binary_ops.iter().any(|b| b.op == "+"),
        "{:?}",
        f.binary_ops
    );
    assert!(
        f.binary_ops.iter().any(|b| b.op == "&&"),
        "{:?}",
        f.binary_ops
    );
    assert_eq!(f.access_chains.len(), 1, "{:?}", f.access_chains);
    assert_eq!(f.access_chains[0].root, "s");
    assert_eq!(f.access_chains[0].elements.len(), 3);
}

/// Class-field arrow functions are function records named after the field.
#[test]
fn class_field_arrow_function_is_a_record() {
    let src = "class C {\n  handler = (x) => x + 1;\n  async run() { return await go(); }\n}\n";
    let ex = extract_js(src);
    let handler = function(&ex, "handler");
    assert!(!handler.record.is_async);
    assert_eq!(handler.record.param_count, 1);
    let run = function(&ex, "run");
    assert!(run.record.is_async);
}

/// A function nested inside a loop is a SCOPE BOUNDARY: its body does not
/// inherit the enclosing loop's depth, and its events are never "innermost"
/// of a loop that surrounds the function declaration.
#[test]
fn nested_arrow_in_a_loop_does_not_inherit_loop_context() {
    let src = "function f() {\n\
                 for (let i = 0; i < 3; i++) {\n\
                   const g = () => use();\n\
                 }\n\
               }\n";
    let ex = extract_js(src);
    let g = function(&ex, "g");
    let use_call = g
        .calls
        .iter()
        .find(|c| c.callee_name == "use")
        .expect("use() inside the arrow");
    assert_eq!(use_call.meta.in_loop_depth, 0, "{:?}", use_call.meta);
    assert!(
        !use_call.meta.is_innermost,
        "a loop outside the arrow cannot make its body innermost"
    );
}

/// A call inside an `if` inside a loop is conditionally guarded, at depth 1,
/// and innermost (no inner loop).
#[test]
fn call_inside_if_inside_loop_is_guarded_and_innermost() {
    let src = "function f() {\n\
                 for (let i = 0; i < 3; i++) {\n\
                   if (cond()) { use(); }\n\
                 }\n\
               }\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    let use_call = f
        .calls
        .iter()
        .find(|c| c.callee_name == "use")
        .expect("use()");
    assert!(use_call.conditional_guarded, "{:?}", use_call);
    assert_eq!(use_call.meta.in_loop_depth, 1);
    assert!(use_call.meta.is_innermost);
}

/// An arrow declared in a `for` HEADER is a separate function record; its body
/// does not inherit the loop depth of the header it is written in.
#[test]
fn arrow_in_a_for_header_does_not_inherit_loop_depth() {
    let src = "function f() {\n\
                 for (let tick = () => use(); tick(); ) {}\n\
               }\n";
    let ex = extract_js(src);
    let tick = function(&ex, "tick");
    let use_call = tick
        .calls
        .iter()
        .find(|c| c.callee_name == "use")
        .expect("use() inside the header arrow");
    assert_eq!(use_call.meta.in_loop_depth, 0, "{:?}", use_call.meta);
    assert!(!use_call.meta.is_innermost);
}

/// Destructured parameters count as one parameter each and yield a usable
/// name: a shorthand property and a rest element must not collapse the count.
#[test]
fn destructured_parameter_names_are_reported() {
    let source = "function f({a, b: c}, ...rest) { return a + c + rest.length; }\n";
    let ex = extract_js(source);
    let f = function(&ex, "f");
    assert_eq!(f.record.param_count, 2, "{:?}", f.record.param_types);
    assert_eq!(f.record.param_types.len(), f.record.param_count as usize);

    // `collect_params` gives the bound names; both must be non-empty (the
    // record itself carries no name list, only the count/types).
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language())
        .expect("pinned grammar links");
    let tree = parser.parse(source, None).expect("parse");
    let params = find_kind(tree.root_node(), "formal_parameters").expect("parameters");
    let info = infer::collect_params(params, source.as_bytes(), Flavor::JavaScript);
    assert_eq!(info.names, ["a", "rest"], "{:?}", info.names);
    assert!(info.names.iter().all(|name| !name.is_empty()));
    assert_eq!(info.types.len(), 2);
}

fn find_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if let Some(found) = find_kind(cursor.node(), kind) {
                return Some(found);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

/// Identifier-less destructuring params (`{}`, `[]`) are still parameters: one
/// entry each, with an empty name, so `param_count` never undercounts.
#[test]
fn identifier_less_destructured_params_still_count() {
    let source = "function f({}, []) {}\n";
    let ex = extract_js(source);
    let f = function(&ex, "f");
    assert_eq!(f.record.param_count, 2, "{:?}", f.record.param_types);

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language())
        .expect("pinned grammar links");
    let tree = parser.parse(source, None).expect("parse");
    let params = find_kind(tree.root_node(), "formal_parameters").expect("parameters");
    let info = infer::collect_params(params, source.as_bytes(), Flavor::JavaScript);
    assert_eq!(info.types.len(), 2);
    assert_eq!(info.names, ["", ""], "{:?}", info.names);
}

/// Comments are extras, not parameters: a comment between parameters must not
/// inflate `param_count`.
#[test]
fn comments_between_params_are_not_counted() {
    let source = "function f(a /* c */, b) { return a + b; }\n";
    let ex = extract_js(source);
    let f = function(&ex, "f");
    assert_eq!(f.record.param_count, 2, "{:?}", f.record.param_types);
}

/// A legacy-octal bound (`010`) is not decimal 10: the trip count stays
/// honestly unknown instead of guessing a wrong value.
#[test]
fn legacy_octal_trip_bound_is_unknown() {
    let src = "function f() {\n  for (let i = 0; i < 010; i++) {}\n}\n";
    let ex = extract_js(src);
    let f = function(&ex, "f");
    assert_eq!(f.loops.len(), 1);
    assert_eq!(f.loops[0].trip_count, None, "legacy octal is ambiguous");
}

/// JSX trees are parsed and walked without panicking.
#[test]
fn jsx_tree_does_not_panic() {
    let src = "const el = <section title={name}>{value}</section>;\n";
    let ex = extract_js(src);
    assert_eq!(ex.file.language, "js");
    assert!(ex.functions.iter().any(|f| f.record.name == "<module>"));
}

/// Module entry declaration: the CommonJS `require.main === module` idiom and
/// the ESM `import.meta.url` guard both set `Some(true)`; a plain module is
/// `Some(false)`; real functions never carry the fact.
#[test]
fn module_entry_declaration_detection() {
    let cjs = extract_js("if (require.main === module) { run(); }\n");
    assert_eq!(cjs.functions[0].record.entry_declared, Some(true));

    let esm = extract_js(
        "import { fileURLToPath } from \"node:url\";\n\
         if (import.meta.url === `file://${process.argv[1]}`) { run(); }\n",
    );
    // The backtick template condition still compares `import.meta.url` on one
    // side of `===` — the documented guard predicate.
    assert_eq!(esm.functions[0].record.entry_declared, Some(true));

    let plain = extract_js("export function f() { return 1; }\n");
    assert_eq!(plain.functions[0].record.entry_declared, Some(false));
    assert_eq!(function(&plain, "f").record.entry_declared, None);

    // A nested function's `require.main` does not make the module an entry.
    let nested = extract_js("function f() { return require.main === module; }\n");
    assert_eq!(nested.functions[0].record.entry_declared, Some(false));
}

/// Comments count as comments (never strings); JSX/template contents do not.
#[test]
fn loc_comment_count_ignores_strings() {
    let ex = extract_js(
        "// line\nconst url = \"http://x\";\n/* block */\nconst t = `// not a comment`;\n",
    );
    assert_eq!(ex.file.loc.comment, 2, "{:?}", ex.file.loc);
    assert_eq!(ex.file.loc.code, 2, "{:?}", ex.file.loc);
}

/// Event order is source order and extraction is deterministic.
#[test]
fn event_order_is_source_order_and_deterministic() {
    let src = "function f() {\n  first();\n  a = 1;\n  second();\n}\n";
    let one = extract_js(src);
    let two = extract_js(src);
    assert_eq!(one, two);
    let f = function(&one, "f");
    let names: Vec<&str> = f.calls.iter().map(|c| c.callee_name.as_str()).collect();
    assert_eq!(names, ["first", "second"]);
}
