//! R0 pinned-pair link test for `tree-sitter-rust` (plan `rust-backend`).
//!
//! This test fails CI if the exact-pinned parser runtime (`tree-sitter`) and
//! the Rust grammar (`tree-sitter-rust`) drift out of ABI compatibility
//! (ADR-0001 guardrail G5). It is intentionally independent of any backend
//! module — the Rust scanner backend itself lands in R1.

use tree_sitter::Parser;

/// Minimal but representative Rust translation unit exercising a free
/// function, a two-field struct, an impl block with a method, a `for` loop,
/// a `macro_rules!` definition, and an `#[inline]` attribute.
const SOURCE: &str = r#"
#[inline]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

struct Point {
    x: i64,
    y: i64,
}

impl Point {
    fn sum(&self) -> i64 {
        self.x + self.y
    }
}

fn total(values: &[i64]) -> i64 {
    let mut acc = 0;
    for v in values {
        acc += *v;
    }
    acc
}

macro_rules! double {
    ($n:expr) => {
        $n * 2
    };
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

#[test]
fn pinned_rust_runtime_and_grammar_link_and_parse_cleanly() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();

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
        "pinned tree-sitter runtime rejected the pinned rust grammar (ABI drift)"
    );

    let tree = parser.parse(SOURCE, None).expect("parser yields a tree");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(
        !tree.root_node().has_error(),
        "representative Rust source parsed with errors"
    );
    assert_clean_tree(tree.root_node());
}

#[test]
fn malformed_rust_source_reports_error() {
    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("pinned grammar must link");

    let tree = parser
        .parse("fn broken( {", None)
        .expect("parser yields a tree");
    assert!(
        tree.root_node().has_error(),
        "malformed source must report an error (guards the zero-ERROR assertion)"
    );
}
