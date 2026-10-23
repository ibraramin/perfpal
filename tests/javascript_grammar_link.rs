//! W0 pinned-pair link test for `tree-sitter-javascript` (plan `js-ts`).
//!
//! This test fails CI if the exact-pinned parser runtime (`tree-sitter`) and
//! the JavaScript grammar (`tree-sitter-javascript`) drift out of ABI
//! compatibility (ADR-0001 guardrail G5; ADR-0011). It is intentionally
//! independent of any backend module — the JavaScript scanner backend itself
//! lands in a later wave.

use tree_sitter::Parser;

/// Minimal but representative ES module exercising ESM imports/exports, a
/// CommonJS-style `require("x")` call, a function declaration, an `async`
/// function with `await`, an arrow function, a class with a constructor and a
/// method, C-style `for`, `for...of`, `for...in`, `while`, `do...while`,
/// `try`/`catch`/`finally`, `throw`, a `new` expression, an object literal, an
/// array literal, a template string, destructuring, and a JSX element (the JS
/// grammar embeds JSX rather than requiring a separate grammar).
const SOURCE: &str = r#"
import { readFile } from "node:fs/promises";
import defaultThing, { helper as aliased } from "./helper.js";

const cjs = require("legacy-module");

export function sum(values) {
  let acc = 0;
  for (let i = 0; i < values.length; i += 1) {
    acc += values[i];
  }
  return acc;
}

export async function load(url) {
  const data = await readFile(url, "utf8");
  return data;
}

const double = (x) => x * 2;

class Report {
  constructor(name) {
    this.name = name;
  }
  render() {
    return `<section>${this.name}</section>`;
  }
}

export function loops(items) {
  const out = [];
  for (const item of items) {
    out.push(item);
  }
  for (const key in items) {
    out.push(key);
  }
  let n = 0;
  while (n < 3) {
    n += 1;
  }
  do {
    n -= 1;
  } while (n > 0);
  try {
    if (n < 0) {
      throw new Error("negative");
    }
  } catch (err) {
    out.push(err.message);
  } finally {
    out.push("done");
  }
  return out;
}

const record = { a: 1, b: [2, 3] };
const { a, b } = record;
const [first, second] = b;
const el = <Report name={a}>{b}</Report>;

export default el;
export { double, aliased, cjs, defaultThing, first, second };
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

#[test]
fn pinned_javascript_runtime_and_grammar_link_and_parse_cleanly() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();

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

    assert!(
        parser.set_language(&language).is_ok(),
        "pinned tree-sitter runtime rejected the pinned javascript grammar (ABI drift)"
    );

    let tree = parser.parse(SOURCE, None).expect("parser yields a tree");
    assert_eq!(tree.root_node().kind(), "program");
    assert!(
        !tree.root_node().has_error(),
        "representative JavaScript source parsed with errors"
    );
    assert_clean_tree(tree.root_node());

    // Lock the constructs the js-ts design maps, so a grammar bump that
    // silently renames/removes one is caught here (not just as a doc drift).
    // NOTE: this grammar represents both `for...of` and `for...in` as
    // `for_in_statement` (distinguished by the `of`/`in` token).
    for kind in [
        "function_declaration",
        "arrow_function",
        "class_declaration",
        "method_definition",
        "for_statement",
        "for_in_statement",
        "while_statement",
        "do_statement",
        "try_statement",
        "catch_clause",
        "throw_statement",
        "await_expression",
        "new_expression",
        "object",
        "array",
        "template_string",
        "jsx_element",
        "import_statement",
        "export_statement",
        "lexical_declaration",
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
fn malformed_javascript_source_reports_error() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("pinned grammar must link");

    let tree = parser
        .parse("function broken( {\n  return 1;\n", None)
        .expect("parser yields a tree");
    assert!(
        tree.root_node().has_error(),
        "malformed source must report an error (guards the zero-ERROR assertion)"
    );
}
