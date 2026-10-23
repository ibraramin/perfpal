//! W0 pinned-pair link test for `tree-sitter-typescript` (plan `js-ts`).
//!
//! This test fails CI if the exact-pinned parser runtime (`tree-sitter`) and
//! either grammar shipped by the `tree-sitter-typescript` crate
//! (`LANGUAGE_TYPESCRIPT` / `LANGUAGE_TSX`) drift out of ABI compatibility
//! (ADR-0001 guardrail G5; ADR-0011). It is intentionally independent of any
//! backend module — the TypeScript scanner backend itself lands in a later
//! wave.

use tree_sitter::Parser;

/// Minimal but representative TypeScript module exercising `import`/`export`,
/// an interface, a generic type alias, an enum, a class with `implements` and
/// a method, a generic function with type parameters, type annotations, an
/// `as` expression, a `satisfies` expression, a non-null `!` assertion,
/// `async`/`await`, C-style `for`, and `for...of`.
const TS_SOURCE: &str = r#"
import { EventEmitter } from "node:events";
import type { Readable } from "node:stream";

export interface Shape {
  readonly name: string;
  area(): number;
}

export type Pair<T> = [T, T];

export enum Mode {
  Fast = "fast",
  Slow = "slow",
}

export class Square implements Shape {
  readonly name = "square";
  constructor(private readonly side: number) {}
  area(): number {
    return this.side * this.side;
  }
}

export function identity<T>(value: T): T {
  return value;
}

export async function collect(source: Readable): Promise<string[]> {
  const rows: string[] = [];
  for (const chunk of source) {
    rows.push(String(chunk));
  }
  for (let i = 0; i < rows.length; i += 1) {
    rows[i] = rows[i].trim();
  }
  const text = (await Promise.resolve("x")) as string;
  const config = { mode: Mode.Fast } satisfies { mode: Mode };
  const maybe: string | null = text;
  const definite: string = maybe!;
  return [definite, ...rows];
}
"#;

/// Minimal but representative TSX module exercising a typed React-style
/// component returning JSX with a container element, a self-closing element,
/// attributes, and expression children.
const TSX_SOURCE: &str = r#"
import React from "react";

interface Props {
  label: string;
  count: number;
}

export function Badge({ label, count }: Props) {
  return (
    <div className="badge" data-count={count}>
      <span>{label}</span>
      <img src="icon.png" alt="" />
    </div>
  );
}
"#;

/// Recursively walks the whole tree asserting there are no `ERROR` nodes and
/// no missing nodes.
fn assert_clean_tree(node: tree_sitter::Node<'_>) {
    assert_ne!(
        node.kind(),
        "ERROR",
        "unexpected ERROR node spanning {:?}",
        node.byte_range()
    );
    assert!(
        !node.is_missing(),
        "unexpected missing node at {:?}",
        node.byte_range()
    );
    for i in 0..node.child_count() as u32 {
        assert_clean_tree(node.child(i).expect("child index < child_count"));
    }
}

/// Recursively counts nodes of `kind` (a `>= 1` assertion guards against the
/// representative source silently losing a mapped construct).
fn count_kind(node: tree_sitter::Node<'_>, kind: &str, acc: &mut usize) {
    if node.kind() == kind {
        *acc += 1;
    }
    for i in 0..node.child_count() as u32 {
        count_kind(node.child(i).expect("child index < child_count"), kind, acc);
    }
}

/// Asserts `language` sits inside the runtime's accepted ABI window, links to
/// the pinned runtime, and returns a parser ready to parse.
fn linked_parser(language: tree_sitter::Language, grammar: &str) -> Parser {
    // Make the ABI window explicit, not just implied by `set_language`: the
    // grammar's declared ABI must sit inside the runtime's accepted window
    // (ADR-0001 G5 — a grammar bump requires a node-types review, not an
    // automatic upgrade).
    let abi = language.abi_version();
    assert!(
        (tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION..=tree_sitter::LANGUAGE_VERSION)
            .contains(&abi),
        "grammar ABI {abi} is outside the runtime's accepted window \
         [{}, {}] (ABI drift; review the node-types diff per ADR-0001 G5)",
        tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION,
        tree_sitter::LANGUAGE_VERSION
    );

    let mut parser = Parser::new();
    assert!(
        parser.set_language(&language).is_ok(),
        "pinned tree-sitter runtime rejected the pinned {grammar} grammar (ABI drift)"
    );
    parser
}

#[test]
fn pinned_typescript_runtime_and_grammar_link_and_parse_cleanly() {
    let mut parser = linked_parser(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
    );

    let tree = parser.parse(TS_SOURCE, None).expect("parser yields a tree");
    assert_eq!(tree.root_node().kind(), "program");
    assert!(
        !tree.root_node().has_error(),
        "representative TypeScript source parsed with errors"
    );
    assert_clean_tree(tree.root_node());

    // Lock the constructs the js-ts design maps, so a grammar bump that
    // silently renames/removes one is caught here (not just as a doc drift).
    // NOTE: this grammar represents both `for...of` and `for...in` as
    // `for_in_statement` (distinguished by the `of`/`in` token).
    for kind in [
        "interface_declaration",
        "type_alias_declaration",
        "enum_declaration",
        "class_declaration",
        "method_definition",
        "type_annotation",
        "type_parameters",
        "as_expression",
        "satisfies_expression",
        "non_null_expression",
        "function_declaration",
        "for_statement",
        "for_in_statement",
        "await_expression",
    ] {
        let mut count = 0;
        count_kind(tree.root_node(), kind, &mut count);
        assert!(
            count >= 1,
            "representative source must contain at least one `{kind}` node \
             (node-types drift? review per ADR-0001 G5)"
        );
    }
}

#[test]
fn pinned_tsx_runtime_and_grammar_link_and_parse_cleanly() {
    let mut parser = linked_parser(tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx");

    let tree = parser
        .parse(TSX_SOURCE, None)
        .expect("parser yields a tree");
    assert_eq!(tree.root_node().kind(), "program");
    assert!(
        !tree.root_node().has_error(),
        "representative TSX source parsed with errors"
    );
    assert_clean_tree(tree.root_node());

    // Lock the JSX constructs so a grammar bump that silently renames/removes
    // one is caught here (not just as a doc drift).
    for kind in [
        "jsx_element",
        "jsx_self_closing_element",
        "jsx_attribute",
        "jsx_expression",
    ] {
        let mut count = 0;
        count_kind(tree.root_node(), kind, &mut count);
        assert!(
            count >= 1,
            "representative source must contain at least one `{kind}` node \
             (node-types drift? review per ADR-0001 G5)"
        );
    }
}

#[test]
fn tsx_generic_arrow_comma_disambiguation_parses_cleanly() {
    let mut parser = linked_parser(tree_sitter_typescript::LANGUAGE_TSX.into(), "tsx");

    // `<T,>` (trailing comma) is the canonical TSX disambiguation for a
    // generic arrow, since a bare `<T>` is read as a JSX element.
    let tree = parser
        .parse("<T,>(x: T) => x;", None)
        .expect("parser yields a tree");
    assert!(
        !tree.root_node().has_error(),
        "the generic-arrow `<T,>` disambiguation must parse without errors"
    );
    assert_clean_tree(tree.root_node());
}

#[test]
fn malformed_typescript_source_reports_error() {
    let mut parser = linked_parser(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "typescript",
    );

    let tree = parser
        .parse("interface Broken {\n  foo: ;\n}\n", None)
        .expect("parser yields a tree");
    assert!(
        tree.root_node().has_error(),
        "malformed source must report an error (guards the zero-ERROR assertion)"
    );
}
