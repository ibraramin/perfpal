//! Scanner backends — one interface per language (modules.md §4 plugin seam).
//!
//! Directory mapping: this module tree implements §7's `scanner/backends/tree-sitter-c`
//! (hyphens are invalid in Rust module paths, hence `tree_sitter_c`).
//!
//! v1 shipped exactly one backend (C via tree-sitter, [`tree_sitter_c`]);
//! plan `rust-backend` adds the Rust backend ([`tree_sitter_rust`]); plan
//! `python-backend` adds the Python backend ([`tree_sitter_python`]). Adding a
//! language = new backend module + MINOR-bump schema re-emission (modules.md §5
//! extension points).
//!
//! [`Extraction`] is the single source of truth for the per-file IR handoff
//! shape; every backend returns it and `scanner::extract_unit` folds it into a
//! [`crate::scanner::FileUnit`].

pub mod tree_sitter_c;
pub mod tree_sitter_python;
pub mod tree_sitter_rust;

use crate::ir::{FileRecord, GlobalVar, IncludeEdge, MacroEvent, ParseHealth, StructDef};
use crate::scanner::FunctionUnit;

/// Everything one source file yields across the IR boundary.
///
/// Hoisted here (plan `rust-backend` R1) so the C and Rust backends share one
/// definition — the C backend previously declared an identical private struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction {
    pub file: FileRecord,
    pub parse_health: ParseHealth,
    pub includes: Vec<IncludeEdge>,
    pub global_vars: Vec<GlobalVar>,
    pub structs: Vec<StructDef>,
    pub macros: Vec<MacroEvent>,
    pub functions: Vec<FunctionUnit>,
}
