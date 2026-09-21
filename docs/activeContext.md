# Active Context

## Current Task
Session 24: plan `rust-backend` **R0** — grammar pin + pinned-pair link test + backend design doc. **Complete**; commit pending founder approval.

## Recent Decisions
- Pinned `tree-sitter-rust = "=0.24.2"` (MIT, official tree-sitter org); ABI **15 ∈ runtime `tree-sitter 0.26.13` window [13,15]** (primary-source verified: `api.h` + the grammar's `parser.c`).
- `tree-sitter-language` stays **transitive** (single shared lock entry `0.1.7`) — no direct dep (founder-approved; supersedes the plan line's literal mention).
- `tests/rust_grammar_link.rs` = the standalone pinned-pair link test (ABI-window assert + zero-ERROR representative TU parse + malformed-source non-vacuity guard).
- `docs/architecture/rust-backend.md` = the R0 mapping/design doc; **no new ADR** (IR contract unchanged; `modules.md` §5.2 is the sanctioned extension point).
- Dual review passed: `@reviewer` SHIP-WITH-FIXES (0 Critical / 0 Major, all 5 nits applied), `@requirements-reviewer` APPROVED.

## In Progress
- Nothing — R0 boundary reached; commit pending.

## Next
- **Next session (`/new`) = plan `rust-backend` R1** (`src/scanner/backends/tree_sitter_rust/` + language dispatch). Read state files + `readPlan rust-backend` FIRST. Resolve the R0 open items (IncludeEdge.form, CalleeKind, LoopKind).

## Blockers
- None.
