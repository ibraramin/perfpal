# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P0: Cargo crate scaffold per modules.md §7 with CI (fmt/clippy/test/license-audit), parser-free fixture-IR harness, exact version pinning (tree-sitter/tree-sitter-c), serde_json `preserve_order` OFF guard. **DONE — scaffold complete, dual-review passed, all fixes applied, validation fully green.**

## Phase
executing mvp-implementation — P0 COMPLETE; next session starts **P1 vertical slice**

## Key Decisions
- **P0 scaffold shipped:** nine contract-stubbed modules (`cli`, `scanner`+`backends/tree_sitter_c`, `ir`, `analyzer`, `aggregator`, `schema`, `reporter`, `charts`, `writer`) each citing its §3.x contract; §7's hyphenated `tree-sitter-c` rendered as Rust module `tree_sitter_c` (mapping noted in backends/mod.rs).
- **Exact pins (Cargo.toml `=` + committed Cargo.lock):** tree-sitter =0.26.13, tree-sitter-c =0.24.2 (link test proves ABI pair parses a minimal C TU with zero ERROR nodes), serde =1.0.229, serde_json =1.0.151 (default features only), clap =4.6.1 (4.6.6 unavailable via local index at add time — locked 4.6.1 is fine; revisit opportunistically), petgraph =0.8.3, anyhow =1.0.104, thiserror =2.0.20.
- **Byte-determinism defense-in-depth:** exact serde_json pin + CI grep on Cargo.lock for `preserve_order` + behavioral tests (sorted-key serialization regardless of insertion order).
- **CI:** fmt --check; clippy `-D warnings` **--locked**; test **--locked**; cargo-deny license audit (deny.toml allowlist bans GPL-family per §6 criterion 4; `LicenseRef-Proprietary` declared for perfpal itself so the audit stays honest). Toolchain pinned via rust-toolchain.toml = 1.92.0 (+ `rust-version = "1.92.0"` in Cargo.toml) — satisfies §6 "stable Rust pinned in CI" literally.
- **Fixture-IR harness:** tests/fixtures/ir/ corpus (README: `<RULE-ID>__<scenario>.json` naming, ParseHealth fields exercisable parser-free) + tests/common/mod.rs loader/determinism helpers + smoke tests; fixtures/metrics/ and golden/report/ staged as placeholders.

## Reviews
- @requirements-reviewer: **CHANGES_REQUESTED → resolved.** All 7 P0 requirements met; its two open items were the EOL ritual itself (commit + state files — pending founder approval by design); one real nit fixed (stale intra-doc link in backends/mod.rs).
- @reviewer: **SHIP-WITH-FIXES, zero MAJOR.** Applied: `--locked` on CI clippy/test (lockfile = pin enforcement), toolchain pin file, `rust-version`, doc-mapping note. **Rejected with rationale:** gitignore state/ (§7 says never *shipped*, not never committed; AGENTS.md mandates committing state files — every prior session did). Deferred to P6: Actions SHA-pinning NIT.

## Risks
- clap locked at 4.6.1 because the local crates.io index lagged clap_derive 4.6.4; harmless now, but if a future dep demands clap ≥4.6.6 the index/cache must be refreshed first.
- tree-sitter-c C99 grammar vs C11/C17 ERROR-rate risk unchanged; link test now guards the baseline.
- Hand-rolled DAG layout quality unproven (unchanged, P4 concern).

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P1 vertical slice**: ir types (IR v1 records/events/spans/ParseHealth), scanner backend (lexicographic discovery, dot-dir skip, no symlink follow, span ±1 conversion, quoted-include resolution, MacroEvent recognizer, ParseHealth capture), analyzer registry + ALLOC-001 end-to-end, schema serializer + embedded JSON Schema byte-copy, aggregator minimal assembly, writer atomic swap + safety guard, cli flags/exit codes 0/1/2/3, span round-trip unit tests → first end-to-end metrics.json from a sample C repo. Resume with: plan mvp-implementation, commit \<P0 hash\>.

## Uncommitted Files (pending founder-approved commit)
- `Cargo.toml`, `Cargo.lock`, `deny.toml`, `rust-toolchain.toml`, `.gitignore`
- `.github/workflows/ci.yml`
- `src/lib.rs`, `src/main.rs`, `src/{cli,scanner,ir,analyzer,aggregator,schema,reporter,charts,writer}/mod.rs`, `src/scanner/backends/mod.rs`, `src/scanner/backends/tree_sitter_c/mod.rs`
- `tests/common/mod.rs`, `tests/determinism_guard.rs`, `tests/harness_smoke.rs`, `tests/fixtures/ir/{README.md,sample-empty.json}`, `tests/fixtures/metrics/.gitkeep`, `tests/golden/report/.gitkeep`
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`
