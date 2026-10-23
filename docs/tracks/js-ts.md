# Track B — JS/TS backend

**Start this tab:** open `/home/shafin/Opencode/Perfpal-js-ts` (`feat/js-ts`), first message:

> `Read /home/shafin/Opencode/Perfpal-js-ts/docs/tracks/js-ts.md and execute Session B1.`

**Track handoff:** `state/tracks/js-ts/handoff.md` · **Spec:** `docs/specs/js-ts.md` · **Plan:** `docs/plans/js-ts.md` · **ADR:** `docs/adr/ADR-0011-js-ts-backend.md` · **Inventory:** `docs/architecture/js-ts.md` · **Base:** `821b984`.
**Scope:** add JS, then TS/TSX, as first-class backends in the existing Rust core — like C/Rust/Python. **Not a rewrite.** A later release; not beta-blocking.

## Read first

1. `AGENTS.md` — exact pins, no copyleft, determinism, exit-code/schema invariants.
2. `docs/specs/perfpal-core-spec.md`, `docs/architecture/modules.md`, `docs/schema/`, `docs/adr/`.
3. `src/scanner/` (including `src/scanner/backends/`), `src/analyzer/` (rules + heuristics), `src/schema/`, `src/aggregator/`, `Cargo.toml`, `deny.toml`, `rust-toolchain.toml`.
4. The existing C/Rust/Python fixtures and goldens for the pattern.

## Session B1 — Phase 0 (planning only) — **DONE** (2026-10-24)

**Goal:** a spec + plan + ADR; **no production code**.

**Close-out:** delivered `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, `docs/adr/ADR-0011-js-ts-backend.md`,
`docs/architecture/js-ts.md` (Phase 0 inventory), and `state/tracks/js-ts/{session-state.json,handoff.md}`.
Decided: tree-sitter-javascript `=0.25.0` (ABI 15, MIT) + tree-sitter-typescript `=0.23.2` (ABI 14, MIT,
two parsers); tokens `js`/`ts`/`tsx` on grammar boundaries; one MINOR schema bump v1.7 -> v1.8 in B2 declaring
all three tokens; 12-rule JS candidate catalog to be frozen by a B2-W0 canary; global directory skips +
JS/TS-only file-name/size guard. Next: **B2** (JS backend).
**Panel (2026-10-24):** Q1 APPROVED; R1/R2/Q2 returned CHANGES_REQUESTED — all blocking findings fixed
(schema version-pin sweep, `GOLDEN_ARTIFACTS` filename, global-vs-file discovery split, reporter-catalog
ownership blocker, plan DoD/exit codes, named determinism suites). Final: R1, R2, Q2 and a fresh R3 lane
all **APPROVED**, zero unresolved blocking findings.

**Decide and record:** parser (tree-sitter-javascript / tree-sitter-typescript vs oxc/swc); exact pins + license/no-copyleft; Rust crates; schema language values (`js`, `ts`, `tsx`?) + version/ADR; language detection; rule scope (which heuristics/metrics); cost model; output contract; tests/goldens/fixtures; CLI behavior. **JS first, TS/TSX second.**

**Deliver:** `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, the ADR, a Phase 0 inventory, and `state/tracks/js-ts/handoff.md`. No code.

**Do not:** touch `atlas_*.rs`/`atlas_brief.rs`/Atlas specs/goldens; no main state; no C/D files; no schema change without the ADR; no new dependency before the license check; no `views.rs`/`mod.rs` language wiring until A step 12 (rebase).

**EOL:** update the track handoff; commit only with founder approval; no merge to `main`.

## Sessions B2–B4 (signposts)

- **B2 — JS backend.** Parser + language detection + rules + schema values + fixtures/goldens; full gate; panel.
- **B3 — TS/TSX backend.** Same for TS/TSX; reuse the JS path where possible; schema/ADR update only if needed.
- **B4 — tests/goldens/docs.** New JS/TS golden trees; report language labels; output contract; semver; review panel.
- Merge after A step 12; B is a later release.

## Guardrails / ownership

- B owns `src/scanner` (incl. `backends/`), `src/analyzer` (rules + heuristics), `src/schema` language values, grammars, JS/TS fixtures, `Cargo.toml`/`Cargo.lock`, `deny.toml`, JS/TS spec/plan/ADR.
- A/C/D read; no edits. `views.rs`/`mod.rs` language wiring after A step 12.
- Rebase on `main` at session start; use a separate `CARGO_TARGET_DIR`; exact pins; no copyleft.
