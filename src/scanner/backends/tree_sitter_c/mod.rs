//! tree-sitter-c scanner backend (ADR-0001).
//!
//! Binds the official org-published grammar crate (`tree-sitter-c`, MIT) to
//! the parser-agnostic IR seam. Versions are exact-pinned in Cargo.toml per
//! ADR-0001 guardrail G5; the link test below fails CI if runtime and grammar
//! drift out of compatibility.

/// The C grammar as a tree-sitter [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_c::LANGUAGE.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P0 link check: pinned runtime + pinned grammar must parse a minimal TU
    /// with zero ERROR nodes. Guards the version-pinning decision (ADR-0001 G5)
    /// and the C99-vs-C11/C17 ERROR-rate risk signal from day one.
    #[test]
    fn parses_minimal_translation_unit_without_error_nodes() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language())
            .expect("grammar/runtime ABI mismatch — check exact pins in Cargo.toml");
        let tree = parser
            .parse("int main(void) { return 0; }\n", None)
            .expect("parse produced no tree");
        assert!(
            !tree.root_node().has_error(),
            "ERROR node in minimal C TU — grammar/runtime pin problem"
        );
        assert_eq!(tree.root_node().kind(), "translation_unit");
    }
}
