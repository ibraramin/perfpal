//! tree-sitter-typescript scanner backend (plan `js-ts`, wave B3).
//!
//! TypeScript and TSX are the same *runtime* language as JavaScript (V8/Node),
//! so this backend does NOT duplicate the extraction visitor: it binds the
//! official `tree-sitter-typescript` grammars (MIT, exact-pinned per ADR-0001
//! G5 / ADR-0011) and delegates to [`tree_sitter_javascript::extract_with`]
//! with a TypeScript [`Flavor`].
//!
//! What lives where:
//! - this module: grammar selection (`LANGUAGE_TYPESCRIPT` for `.ts`/`.mts`/
//!   `.cts`, `LANGUAGE_TSX` for `.tsx`) and the `ts`/`tsx` tokens;
//! - `tree_sitter_javascript::infer`: TS parameter annotations, the type-land
//!   skip set, expression-wrapper unwrapping (`as`/`satisfies`/`!`/`<T>expr`),
//!   `abstract_class_declaration`, and `public_field_definition` naming;
//! - `tree_sitter_javascript::scan`: type-land event suppression and the TS
//!   `ternary_expression` complexity arm.
//!
//! Runtime-erased constructs are deliberately silent: `import type` /
//! `import typeof` / `export type` produce no [`IncludeEdge`], interfaces and
//! type aliases produce no records/events, and `declare …` ambient declarations
//! produce no [`StructDef`](crate::ir::StructDef). JSX is parsed structurally
//! and only embedded `{ … }` expressions emit events.
//!
//! Tier: `syntactic` throughout; every event carries `resolver: None` and no
//! semantic claim is made. `extract`/`extract_tsx` are total and never panic on
//! malformed input (ERROR/missing nodes become partial [`ParseHealth`]).

#[cfg(test)]
mod tests;

use super::Extraction;
use super::tree_sitter_javascript::{self, Dialect, Flavor};

/// The `.ts`/`.mts`/`.cts` grammar binding.
const TYPESCRIPT: Dialect = Dialect {
    language,
    token: "ts",
    flavor: Flavor::TypeScript,
};

/// The `.tsx` grammar binding.
const TSX: Dialect = Dialect {
    language: language_tsx,
    token: "tsx",
    flavor: Flavor::TypeScript,
};

/// Extract one TypeScript (`.ts`/`.mts`/`.cts`) file's IR.
#[must_use]
pub fn extract(path: &str, source: &[u8]) -> Extraction {
    tree_sitter_javascript::extract_with(&TYPESCRIPT, path, source)
}

/// Extract one TSX (`.tsx`) file's IR.
#[must_use]
pub fn extract_tsx(path: &str, source: &[u8]) -> Extraction {
    tree_sitter_javascript::extract_with(&TSX, path, source)
}

/// The pinned TypeScript grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

/// The pinned TSX grammar as a [`tree_sitter::Language`].
#[must_use]
pub fn language_tsx() -> tree_sitter::Language {
    tree_sitter_typescript::LANGUAGE_TSX.into()
}
