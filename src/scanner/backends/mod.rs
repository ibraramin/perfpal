//! Scanner backends — one interface per language (modules.md §4 plugin seam).
//!
//! Directory mapping: this module tree implements §7's `scanner/backends/tree-sitter-c`
//! (hyphens are invalid in Rust module paths, hence `tree_sitter_c`).
//!
//! v1 ships exactly one backend: C via tree-sitter ([`tree_sitter_c`]).
//! Adding a language = new backend module + MINOR-bump schema re-emission
//! (modules.md §5 extension points).

pub mod tree_sitter_c;
