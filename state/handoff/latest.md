# Handoff — Latest

## Objective
Session 24: plan `rust-backend` **R0** — pin the Rust grammar, prove the pinned pair links/parses, and write the backend design doc. No backend module, no `src/` change, no schema bump, no heuristics.

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · **`rust-backend` R0 ✅ (R1 next)** · `beta-release` (not started).

## Version snapshot
- **Released: `v0.1.0`** (tag, target `f368133`). `Cargo.toml` = `0.1.0`. `metrics.json` schema = **1.1**. Languages = **C only** (Rust lands R1–R3; schema 1.2 at R2).
- `main` unreleased: semantic resolver + `examples/` C corpus + guarded-scan fix (`5643737`) + roadmap re-plan (`ac97ecd`) + R0 (this session, uncommitted).

## Key Decisions
- R0 pinned `tree-sitter-rust = "=0.24.2"` (official tree-sitter-org grammar, MIT) per ADR-0001 guardrail G5. **ABI 15 ∈ runtime window [13,15]** — verified from primary sources (`api.h` + the grammar's `parser.c`).
- **`tree-sitter-language` kept transitive-only** (founder-approved; supersedes the plan line's literal "+ tree-sitter-language 0.1") — the lock resolves one shared `0.1.7` feeding the runtime and both grammars.
- The link test is a **standalone `tests/rust_grammar_link.rs`** (backend-independent), hardened with an explicit ABI-window assert.
- `docs/architecture/rust-backend.md` records the node-kind mapping (from the grammar's bundled `node-types.json`), the cfg/attribute sibling rule (the C guarded-content defect does **not** recur), the `token_tree` macro blind spot, and syntactic-tier-only scope. **No new ADR** — the IR contract is unchanged and modules.md §5.2 is the sanctioned extension point.
- Dual review passed: `@reviewer` SHIP-WITH-FIXES (0 Critical / 0 Major; all 5 doc/test nits applied), `@requirements-reviewer` APPROVED (all clauses present; scope boundary clean).

## Open items handed to R1 (rust-backend.md §7)
`IncludeEdge.form` mapping for use/mod; `CalleeKind` for method/scoped/turbofish callees; `LoopKind` for `loop_expression`; enum/union/tuple-struct IR targets; `function_signature_item`; `let_declaration` scope classification; macro-invocation events; any additive receiver/trait field (only if provably needed → R2 minor).

## Risks
- **Rust grammar-pin drift:** doc claims are read from `node-types.json` @ `tree-sitter-rust 0.24.2`; a grammar bump changes kinds/fields and **requires** a node-types diff review (ADR-0001 G5).
- `tree-sitter-language` is **not** exact-pinned (transitive; the committed lockfile is the guard — a deliberate `cargo update` could move it within `0.1.x`).
- Rust is **syntactic-tier only**; `cfg_if!`/`macro_rules!` bodies are a documented blind spot.
- Carried: beta tag visibility (attestations on private repos), ARCH-001 hub-fanout corpus sub-case still non-firing, Node-24 action pins, x64-darwin/msvc binary reproducibility, toolchain `1.92.0` mirrored in 5 places.

## Blockers
- None.

## Next Actions
1. Founder approves the R0 commit → commit the working tree (see Uncommitted Files; **EXCLUDE `.opencode/**`**).
2. **Next session (`/new`) = plan `rust-backend` R1** — create `src/scanner/backends/tree_sitter_rust/` + extension-based language dispatch. Read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST, then `readPlan rust-backend`.
3. Then R2 (schema 1.2) → R3 (`PERF-RS-*` heuristics) → R4 (dogfood + `examples-rust/`) → R5 (tests/gates/version `0.2.0`) → R6 (docs/close-out) → plan `beta-release`.

## Uncommitted Files
- **Added:** `tests/rust_grammar_link.rs`, `docs/architecture/rust-backend.md`
- **Modified:** `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `docs/activeContext.md`, `state/session-state.json`, `state/handoff/latest.md`
- **EXCLUDE:** `.opencode/**` (session-tooling noise)
- Working tree otherwise clean at `ac97ecd`.
