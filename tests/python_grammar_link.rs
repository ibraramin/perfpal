//! W1 pinned-pair link test for `tree-sitter-python` (plan `python-backend`).
//!
//! This test fails CI if the exact-pinned parser runtime (`tree-sitter`) and
//! the Python grammar (`tree-sitter-python`) drift out of ABI compatibility
//! (ADR-0001 guardrail G5; ADR-0007 §1). It is intentionally independent of
//! any backend module — the Python scanner backend itself lands in W2.

use tree_sitter::Parser;

/// Minimal but representative Python module exercising a module docstring,
/// aliased imports, a decorator, a typed function, a class with a base class
/// and a method, `for`/`while` loops, comprehensions, an f-string, an
/// `async def` with `await`, `try`/`except`/`finally`, a `with` statement, a
/// lambda, and membership (`in`).
const SOURCE: &str = r#"
"""Representative module for the pinned-pair link test."""

import pandas as pd
from collections import OrderedDict as OD
from os import path


@cached
def total(values: list[int]) -> int:
    acc = 0
    for v in values:
        acc += v
    return acc


class Report(OD):
    def run(self) -> str:
        return f"total={total([1, 2, 3])}"


async def fetch(url: str) -> bytes:
    data = await client.get(url)
    return data


def build(n: int) -> list[int]:
    squares = [i * i for i in range(n)]
    lookup = {i: i for i in range(n)}
    uniq = {s for s in squares if s % 2 == 0}
    lazy = (s + 1 for s in uniq)
    if not (0 < n < 5):
        return list(lazy)
    i = 0
    while i < n:
        i += 1
    try:
        with open(path.join("d", "f")) as fh:
            rows = list(map(lambda r: r.strip(), fh))
    except OSError:
        rows = []
    finally:
        pass
    return [s for s in squares if s in lookup]
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
fn pinned_python_runtime_and_grammar_link_and_parse_cleanly() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();

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
        "pinned tree-sitter runtime rejected the pinned python grammar (ABI drift)"
    );

    let tree = parser.parse(SOURCE, None).expect("parser yields a tree");
    assert_eq!(tree.root_node().kind(), "module");
    assert!(
        !tree.root_node().has_error(),
        "representative Python source parsed with errors"
    );
    assert_clean_tree(tree.root_node());

    // Lock the constructs the W1 design doc maps, so a grammar bump that
    // silently renames/removes one is caught here (not just as a doc drift).
    for kind in [
        "function_definition",
        "class_definition",
        "decorator",
        "list_comprehension",
        "set_comprehension",
        "dictionary_comprehension",
        "generator_expression",
        "for_statement",
        "while_statement",
        "try_statement",
        "with_statement",
        "lambda",
        "await",
        "not_operator",
        "comparison_operator",
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
fn malformed_python_source_reports_error() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("pinned grammar must link");

    let tree = parser
        .parse("def broken(:\n    pass\n", None)
        .expect("parser yields a tree");
    assert!(
        tree.root_node().has_error(),
        "malformed source must report an error (guards the zero-ERROR assertion)"
    );
}
