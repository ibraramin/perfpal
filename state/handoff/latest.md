# Handoff — Latest

## Objective
Session 27: plan `rust-backend` **R3** — the Rust language tier analyzed end-to-end: full 23-rule `PERF-RS-*` heuristic catalog with a **language-tagged registry**, Rust `use`-graph resolution (+ Cargo.toml-aware crate-root detection), the Rust allocation family (+ `macro_rules!` wrapper recognizer + `macro_origin`), and the enum/union/tuple-struct IR decisions. **Complete and COMMITTED as `b775bbc`**; dual review APPROVED (final round 3/3).

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · **`rust-backend` R0 ✅ / R1 ✅ / R2 ✅ / R3 ✅ (R4 next)** · `beta-release` (not started).

## Version snapshot
- **Released: `v0.1.0`** (tag, target `f368133`). `Cargo.toml` = `0.1.0` (R5 owns the 0.2.0 bump).
- **`metrics.json` schema = 1.2 — UNCHANGED by R3** (no schema bump: every R3 IR addition is IR-only, never serialized into the document).
- `main` unreleased: semantic resolver + `examples/` C corpus + guarded-scan fix + roadmap re-plan + R0 + R1 + R2 + **R3 (`b775bbc`)**.

## Key Decisions
- **R3 approval gate (founder, question tool — all "recommended"):** (1) execute **all** of R3 in staged waves; (2) mirror **all 23** families with **honest reframing**; (3) **language-tagged registry** (`PERF-C-*` C-only / `PERF-RS-*` Rust-only); (4) add a lightweight **macro-as-call surface**; (5) additive `StructDef.kind` + union/enum + pointer/array-dim derivation **and** resolve `use` edges + **Cargo.toml-aware crate-root detection**.
- **WS-A (IR + backend):** additive `StructKind{struct,union,enum}` + `StructDef.kind`; `CallEvent.callee_path: Option<String>` + `CallEvent.is_macro: bool`; shared `RUST_ALLOC_FAMILY` (19 names, `matches_site`). Rust backend now computes `alloc_site_count`, recognizes alloc-wrapper `macro_rules!` bodies (`RecognizedPattern`), wires `macro_origin` (span-intersects-recognized OR callee-name-equals-local-macro-def), emits union/enum `StructDef`s, derives `is_pointer`/`array_dim_literal`, and emits a function-attached macro-as-call `CallEvent`.
- **WS-B (module graph):** `use` edges resolve to files via a module-path→file map (longest **FILE-backed** prefix; `crate`/`self`/`leading super`); unresolved internal `use` → `to_path=null, resolved=false` (supersedes R1 raw-`::` retention); **Cargo.toml-aware** crate-root detection with the R1 suffix heuristic as no-manifest fallback; `r#` raw-identifier prefixes stripped.
- **WS-C (analyzer):** `RuleLanguage`-tagged registry; `analyze` groups units once by language and passes each rule only its slice; `cross_file_call_edges` keyed by `(language, name)`; **23 `PERF-RS-*` rules** in `src/analyzer/heuristics/rust/`.
- **WS-D (reporter):** catalog extended to **46 ids**; `RegistryEntry.rule_id` + `registered_rule_ids()` drive a registry-derived completeness lock; language-neutral architecture prose + include-graph caption.
- **N1 fix:** additive IR-only `IncludeEdge.is_mod` — `PERF-RS-ARCH-001` runs over the **`use` import graph only** (`mod` declarations excluded; they still drive resolution and stay in `include_graph` data).
- **Dual review:** `@reviewer` 1 CHANGES_REQUESTED (C1 raw-ident module resolution + M1 ARCH-001 unresolved FP / M2 reporter lock binding / M3 single-file target) → fix → 2 CHANGES_REQUESTED (**N1** parent/child false cycles, pre-existing) → fix → **3 APPROVE**. `@requirements-reviewer` **APPROVED** (8/8 clauses, scope clean). Orchestrator-direct: `is_mod` added to `ir-v1.md`.

## Risks
- **Rust is syntactic-tier only** — no semantic resolver (libclang is C-only); `cfg_if!`/`macro_rules!` `token_tree` bodies stay opaque (macro-generated items invisible).
- **`PERF-RS-ARCH-001` = `use`-import graph only** — module structure lives in `include_graph` data; deep-chain/hub reflect imports, not module nesting.
- **Rust `use` resolution is an approximation** — custom Cargo.toml `[lib] path`/`[[bin]] path` layouts out of scope; crate-root-item/inline-module imports stay unresolved; **no `unresolved_include` finding** for Rust (intentional FP-class removal).
- **Curated Rust name-lists** (syntactic): `clone` excluded from `RUST_ALLOC_FAMILY`; `+=` accumulation invisible; RS-MEM-002 has no explicit size arg; RS-TYPE-002 carries a `repr(Rust)` auto-reorder caveat; RS-LOOP-006 closure-field calls classify `Direct`.
- Carried: grammar-pin drift (node-types diff needed on any bump); `tree-sitter-language` transitive; x64-darwin/msvc binary reproducibility; toolchain `1.92.0` mirrored in 5 places.
- Pre-existing doc nits (R6): `stats-distributions-proposal.md` + dated `roadmap.md` snapshot name v1.1; `docs/website/scope-and-content.md` still says schema v1.1.

## Blockers
- None.

## Next Actions
1. R3 **committed** as `b775bbc` (64 files; `.opencode/**` excluded). Proceed to R4.
2. **Next session (`/new`) = plan `rust-backend` R4** — dogfood (run the binary over Perfpal's own `src/`, publish the report, triage findings vs accepted FP classes) + `examples-rust/` corpus (mirror `examples/` density + silent controls; verify every `PERF-RS-*` rule fires against real output). Read state files + `readPlan rust-backend` FIRST.
3. Then R5 (tests/gates + crate version `0.1.0 → 0.2.0`) → R6 (docs/close-out) → plan `beta-release` (`0.2.0-beta.1`).

## R3 Change Set (committed `b775bbc`)
- **Added (code):** `src/analyzer/heuristics/rust/` (`mod.rs`, `alloc.rs`, `str.rs`, `loop.rs`, `call.rs`, `arch.rs`, `mem.rs`, `type.rs`)
- **Added (docs):** `docs/heuristics/rust-catalog.md`
- **Added (tests):** `tests/rust_wave_c1.rs`, `tests/rust_wave_c2.rs`, `tests/rust_backend_r3.rs`, `tests/rust_module_graph.rs`, `tests/reporter_rust.rs`, `tests/mixed_language_e2e.rs`, and 25 `tests/fixtures/ir/PERF-RS-*.json`
- **Modified (code):** `src/ir/mod.rs`, `src/scanner/mod.rs`, `src/scanner/backends/tree_sitter_rust/mod.rs`, `src/scanner/backends/tree_sitter_c/mod.rs`, `src/analyzer/mod.rs`, `src/analyzer/finding.rs`, `src/analyzer/heuristics/{mod,arch,type}.rs`, `src/reporter/mod.rs`, `src/aggregator/mod.rs`, `src/resolver/mod.rs` (compiler-required literal sites only)
- **Modified (tests):** `tests/common/mod.rs`, `tests/analyzer_registry.rs`, `tests/arch_fixtures.rs`, `tests/reporter_render.rs`, `tests/rust_backend.rs`, `tests/schema_1_2.rs`, `tests/golden/report/report.md` (language-neutral prose)
- **Modified (docs/state):** `docs/architecture/rust-backend.md` (§3/§8 corrections + §9), `docs/architecture/ir-v1.md` (R3 additive note incl. `is_mod`), `CHANGELOG.md`, `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`
- **Gate status at commit:** `fmt --check` clean · `clippy -D warnings --all-targets` clean (default + `semantic-clang`) · **361 tests default / 366 semantic green, 0 fail** · golden report comparator green + regen idempotent · 3 schema copies `cmp`-identical · dogfood `src` → `PERF-RS-ARCH-001` = 5 hub-only (0 cycles)
