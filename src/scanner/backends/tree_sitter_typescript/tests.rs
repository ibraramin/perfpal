//! TypeScript/TSX backend unit tests (plan `js-ts`, wave B3). They lock the
//! pinned grammars' node kinds, TS parameter annotations, type-land
//! suppression, type-only import erasure, expression-wrapper transparency,
//! abstract-class/field layout, enum/JSX behavior and parse health.

use tree_sitter::{Node, Parser, Tree};

use super::*;
use crate::scanner::FunctionUnit;
use crate::scanner::backends::Extraction;
use crate::scanner::backends::tree_sitter_javascript::{Flavor, infer};

fn extract_ts(source: &str) -> Extraction {
    extract("test.ts", source.as_bytes())
}

fn function<'a>(ex: &'a Extraction, name: &str) -> &'a FunctionUnit {
    ex.functions
        .iter()
        .find(|f| f.record.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

fn parse_ts(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .expect("pinned TypeScript grammar links");
    parser.parse(source, None).expect("parse")
}

fn find_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
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

fn count_kind(node: Node<'_>, kind: &str) -> usize {
    let mut count = usize::from(node.kind() == kind);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            count += count_kind(cursor.node(), kind);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    count
}

/// Grammar node-kind locks: the constructs this backend maps exist with the
/// expected names, and the TS ternary is `ternary_expression` (the JS grammar's
/// `conditional_expression` does not exist here).
#[test]
fn grammar_node_kinds_are_locked() {
    let src = "interface Shape { area(): number; }\n\
               type Pair<T> = [T, T];\n\
               abstract class Base { name: string; abstract run(): void; }\n\
               class Square extends Base { area(): number { return 1; } }\n\
               function f(a: string, b?: number) {\n\
                 for (let i = 0; i < 2; i++) {}\n\
                 for (const x of [a, b]) {}\n\
                 return a ? 1 : 2;\n\
               }\n";
    let tree = parse_ts(src);
    for kind in [
        "interface_declaration",
        "type_alias_declaration",
        "abstract_class_declaration",
        "class_declaration",
        "method_definition",
        "public_field_definition",
        "required_parameter",
        "optional_parameter",
        "for_statement",
        "for_in_statement",
        "ternary_expression",
    ] {
        assert!(
            count_kind(tree.root_node(), kind) >= 1,
            "representative source must contain `{kind}` (node-types drift?)"
        );
    }
    assert_eq!(
        count_kind(tree.root_node(), "conditional_expression"),
        0,
        "the TS grammar names ternaries `ternary_expression`"
    );
}

/// TS parameters: annotations become `param_types` without their `:`, `this`
/// is erased, optional/rest/destructured shapes keep one entry each, and the
/// names come from the `pattern` (fallback `name`) field.
#[test]
fn typescript_parameters_carry_names_and_annotations() {
    let src = "function f(this: Foo, a: string, b?: number, { c, d }: Opts, ...rest: any[]) { return a; }\n";
    let ex = extract_ts(src);
    let f = function(&ex, "f");
    assert_eq!(f.record.param_count, 4, "{:?}", f.record.param_types);
    assert_eq!(
        f.record.param_types,
        ["string", "number", "Opts", "any[]"],
        "annotation text excludes the leading colon"
    );

    let tree = parse_ts(src);
    let params = find_kind(tree.root_node(), "formal_parameters").expect("formal_parameters");
    let info = infer::collect_params(params, src.as_bytes(), Flavor::TypeScript);
    assert_eq!(info.names, ["a", "b", "c", "rest"], "{:?}", info.names);
    assert_eq!(info.types, ["string", "number", "Opts", "any[]"]);
    assert_eq!(info.types.len(), info.names.len());
}

/// Unannotated TS parameters report `unknown`, never an invented type.
#[test]
fn unannotated_typescript_parameter_is_unknown() {
    let ex = extract_ts("function f(a, b = 1) { return a; }\n");
    let f = function(&ex, "f");
    assert_eq!(f.record.param_types, ["unknown", "unknown"]);
}

/// `for (let i: number = 0; i < 10; i++)` still exposes its literal trip count.
#[test]
fn typed_canonical_for_has_a_literal_trip_count() {
    let ex = extract_ts("function f() {\n  for (let i: number = 0; i < 10; i++) {}\n}\n");
    let f = function(&ex, "f");
    assert_eq!(f.loops.len(), 1);
    assert_eq!(f.loops[0].trip_count, Some(10));
}

/// Type land produces no functions, calls, allocations or loops: interfaces,
/// type aliases, `declare` signatures and overload signatures are erased.
#[test]
fn type_only_source_emits_no_runtime_facts() {
    let src = "interface Shape { area(): number; name: string; }\n\
               type Pair<T> = [T, T];\n\
               declare function ambient(x: number): string;\n\
               function over(a: string): string;\n\
               function over(a: number): number;\n\
               function over(a: any): any { return a; }\n";
    let ex = extract_ts(src);

    assert_eq!(
        ex.functions
            .iter()
            .filter(|f| f.record.name == "over")
            .count(),
        1,
        "only the implementation produces a record"
    );
    assert!(
        !ex.functions.iter().any(|f| {
            f.record.name == "ambient"
                || f.record.name == "area"
                || f.record.name == "Shape"
                || f.record.name == "Pair"
        }),
        "no type-land declarations become function records"
    );
    assert_eq!(
        ex.file.function_count, 2,
        "synthetic module + implementation"
    );

    let module = &ex.functions[0];
    assert!(module.record.is_module_scope);
    assert!(module.calls.is_empty(), "{:?}", module.calls);
    assert!(module.allocations.is_empty(), "{:?}", module.allocations);
    assert!(module.loops.is_empty(), "{:?}", module.loops);
}

/// `import type` / `import typeof` / `export type` are erased; value imports and
/// both require forms stay edges.
#[test]
fn type_only_imports_are_erased_but_value_imports_remain() {
    let src = "import type { A } from \"./a\";\n\
               import typeof B from \"./b\";\n\
               export type { C } from \"./c\";\n\
               import { D } from \"./d\";\n\
               import E = require(\"./e\");\n\
               const F = require(\"./f\");\n";
    let ex = extract_ts(src);
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|edge| edge.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(
        raws,
        ["./d", "./e", "./f"],
        "type-only statements add no IncludeEdge: {:?}",
        ex.includes
    );
    assert!(
        ex.includes
            .iter()
            .all(|edge| edge.form == crate::ir::IncludeForm::Quoted)
    );
}

/// `as` / `satisfies` / `!` wrappers keep the inner runtime events.
#[test]
fn expression_wrappers_preserve_inner_events() {
    let src = "function f(x: any, arr: any[], obj: any) {\n\
                 const a = make() as Foo;\n\
                 const b = { x: 1 } as Foo;\n\
                 const c = arr!;\n\
                 const d = obj satisfies Shape;\n\
                 consume(a as any);\n\
               }\n";
    let ex = extract_ts(src);
    let f = function(&ex, "f");
    let calls: Vec<&str> = f.calls.iter().map(|c| c.callee_name.as_str()).collect();
    assert_eq!(calls, ["make", "consume"], "{:?}", f.calls);
    assert_eq!(f.allocations.len(), 1, "{:?}", f.allocations);
    assert_eq!(
        f.allocations[0].kind,
        crate::ir::AllocKind::Object,
        "the wrapped object literal is still an allocation"
    );
}

/// Abstract classes become `StructDef`s; members come from
/// `public_field_definition.name`; abstract method signatures produce neither
/// records nor events.
#[test]
fn abstract_class_layout_and_members() {
    let src = "abstract class Base {\n  name: string;\n  count = 0;\n  abstract run(): void;\n}\n";
    let ex = extract_ts(src);
    let base = ex
        .structs
        .iter()
        .find(|s| s.name == "Base")
        .expect("abstract class StructDef");
    let members: Vec<&str> = base.members.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(members, ["name", "count"], "{:?}", base.members);
    assert!(
        !ex.functions.iter().any(|f| f.record.name == "run"),
        "abstract method signatures are not runtime records"
    );

    // `declare class` is ambient: no StructDef even though it has a body.
    let ambient = extract_ts("declare class Ambient { name: string; }\n");
    assert!(
        !ambient.structs.iter().any(|s| s.name == "Ambient"),
        "ambient declarations have no runtime layout"
    );
}

/// Enums contribute no rule inputs (their initializer literals emit nothing).
#[test]
fn enum_initializers_emit_no_findings_inputs() {
    let ex = extract_ts("enum Mode { Fast = \"fast\", Slow = \"slow\" }\n");
    assert_eq!(ex.functions.len(), 1, "synthetic module only");
    let module = &ex.functions[0];
    assert!(module.calls.is_empty());
    assert!(module.allocations.is_empty());
    assert!(module.loops.is_empty());
}

/// JSX is structural: only embedded `{ … }` expressions emit events.
#[test]
fn tsx_jsx_emits_events_only_for_embedded_expressions() {
    let src = "const el = <div title={compute()}><span>{render(x)}</span></div>;\n";
    let ex = extract_tsx("test.tsx", src.as_bytes());
    let module = &ex.functions[0];
    let calls: Vec<&str> = module
        .calls
        .iter()
        .map(|c| c.callee_name.as_str())
        .collect();
    assert_eq!(calls, ["compute", "render"], "{:?}", module.calls);
    assert_eq!(ex.file.language, "tsx");
}

/// The TS ternary is a branch point (cyclomatic +1, flat cognitive +1).
#[test]
fn ternary_expression_is_a_branch_point() {
    let ex = extract_ts("function f(a) {\n  return a ? 1 : 2;\n}\n");
    let f = function(&ex, "f");
    assert_eq!(f.record.cyclomatic_complexity, 2);
    assert_eq!(f.record.cognitive_complexity, 1);
}

/// A clean TS file parses `Ok`; TSX reports the `tsx` token and parses `Ok`.
#[test]
fn clean_typescript_and_tsx_parse_ok_with_own_tokens() {
    let ts = extract_ts("const x: number = 1;\n");
    assert_eq!(ts.parse_health.status, crate::ir::ParseStatus::Ok);
    assert_eq!(ts.parse_health.error_node_count, 0);
    assert_eq!(ts.file.language, "ts");

    let tsx = extract_tsx("test.tsx", "const el = <div>{x}</div>;\n".as_bytes());
    assert_eq!(tsx.parse_health.status, crate::ir::ParseStatus::Ok);
    assert_eq!(tsx.file.language, "tsx");
}

/// A `typeof x` inside a type position is not a runtime reference: it must not
/// appear in an allocation's `referenced_names` (the loop-invariance gates).
#[test]
fn type_queries_are_not_referenced_names() {
    let ex = extract_ts(
        "function f(x: number, loopVar: number) {\n  return { value: x as typeof loopVar };\n}\n",
    );
    let f = function(&ex, "f");
    assert_eq!(f.allocations.len(), 1, "{:?}", f.allocations);
    assert_eq!(f.allocations[0].referenced_names, ["x"]);
}

/// TS expression wrappers are transparent for callee/receiver decomposition:
/// `(fn as any)()` and `<any>fn()` are direct calls of `fn`, and
/// `arr!.includes(x)` keeps its receiver root (LOOP-007/CALL-001/ALLOC-003
/// false-negative guard).
#[test]
fn wrapped_callees_and_receivers_are_decomposed() {
    let ex = extract_ts(
        "function descend(n: ChainNode): ChainNode {\n\
           const a = (descend as any)(n.next);\n\
           const b = <any>descend(n.next);\n\
           const c = n.next!;\n\
           return c ?? descend(n.next);\n\
         }\n\
         function membership(ids: string[], seen: string[]): boolean {\n\
           for (const id of ids) {\n\
             if (seen!.includes(id)) { return true; }\n\
           }\n\
           return false;\n\
         }\n",
    );
    let descend = function(&ex, "descend");
    let calls: Vec<(&str, bool)> = descend
        .calls
        .iter()
        .map(|c| (c.callee_name.as_str(), c.receiver_root.is_some()))
        .collect();
    assert!(
        calls.iter().all(|(name, _)| *name == "descend"),
        "wrapped callees must resolve to `descend`: {calls:?}"
    );
    assert!(
        descend
            .calls
            .iter()
            .all(|c| c.callee_kind == crate::ir::CalleeKind::Direct),
        "wrapped callees stay Direct: {:?}",
        descend
            .calls
            .iter()
            .map(|c| c.callee_kind)
            .collect::<Vec<_>>()
    );

    let membership = function(&ex, "membership");
    let includes = membership
        .calls
        .iter()
        .find(|c| c.callee_name == "includes")
        .expect("includes call");
    assert_eq!(
        includes.receiver_root.as_deref(),
        Some("seen"),
        "non-null receiver keeps its root"
    );
}

/// A type position inside an `as`/`satisfies` wrapper must never emit events:
/// `x as import("./m").T` is not a runtime dynamic import.
#[test]
fn wrapped_type_positions_emit_no_runtime_events() {
    let ex = extract_ts(
        "function cast(x: unknown) {\n\
           const a = x as import(\"./m\").T;\n\
           const b = x satisfies import(\"./m\").T;\n\
           return a ?? b;\n\
         }\n",
    );
    let f = function(&ex, "cast");
    assert!(
        f.calls.iter().all(|c| c.callee_name != "import"),
        "a type-position import must not become a CallEvent: {:?}",
        f.calls.iter().map(|c| &c.callee_name).collect::<Vec<_>>()
    );
    assert!(
        f.calls.is_empty(),
        "no runtime calls exist in the source: {:?}",
        f.calls
    );
}

/// Comments between `import`/`export` and the `type` marker are extra nodes;
/// erasure must not depend on a fixed child index.
#[test]
fn commented_type_import_marker_is_still_erased() {
    let ex = extract_ts(
        "import /* c */ type { A } from \"./a\";\n\
         export /* c */ type { B } from \"./b\";\n\
         import { C } from \"./c\";\n",
    );
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|edge| edge.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(raws, ["./c"], "commented type-only statements stay erased");
}

/// Comments are named extras. A comment between a wrapper's delimiter and its
/// value must not be mistaken for the value (`( /*c*/ make() ) as T`).
#[test]
fn comments_inside_wrappers_do_not_hide_the_value() {
    let ex = extract_ts(
        "function f(obj: any) {\n\
           const a = ( /* c */ make() ) as any;\n\
           const b = <any> /* c */ make();\n\
           const c = ( /* c */ obj.method )();\n\
           return a ?? b ?? c;\n\
         }\n",
    );
    let f = function(&ex, "f");
    let calls: Vec<(&str, Option<&str>)> = f
        .calls
        .iter()
        .map(|c| (c.callee_name.as_str(), c.receiver_root.as_deref()))
        .collect();
    assert_eq!(
        calls,
        [("make", None), ("make", None), ("method", Some("obj"))],
        "comment extras must not replace the wrapped value"
    );
}
