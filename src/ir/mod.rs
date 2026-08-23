//! `ir` — the seam (ir-v1.md): parser-agnostic record/event/shape types.
//!
//! File-level records (FileRecord, ParseHealth, IncludeEdge, GlobalVar,
//! StructDef, MacroEvent), FunctionRecord, and typed events (Call, Assignment,
//! BinaryOp, Comparison, AccessChain, LoopNode) sharing a Shape descriptor.
//!
//! Conventions (ir-v1.md §2): spans are 1-based lines and byte-columns with
//! INCLUSIVE ends; engine-native conversion (+1/−1) happens only at the
//! scanner boundary. Identity keys are `(path, name, span)` — resolvers never
//! change them.
//!
//! Confidence downgrade hooks (ir-v1.md §5): macro_origin, cross-file edges,
//! error-region overlap.
//!
//! P0 status: scaffold only. Types land in P1 (vertical slice).
