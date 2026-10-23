# Track B — JS/TS backend

**Start this tab:** open `/home/shafin/Opencode/Perfpal-js-ts` (`feat/js-ts`), first message:

> `Read /home/shafin/Opencode/Perfpal/docs/tracks/js-ts.md and execute Session B1.`

**Track handoff:** `state/tracks/js-ts/handoff.md` · **Spec:** `docs/specs/js-ts.md` · **Plan:** `docs/plans/js-ts.md` · **ADR:** `docs/adr/ADR-00xx-*.md` · **Base:** `821b984`.
**Scope:** add JS, then TS/TSX, as first-class backends in the existing Rust core — like C/Rust/Python. **Not a rewrite.** A later release; not beta-blocking.

## Read first

1. `AGENTS.md` — exact pins, no copyleft, determinism, exit-code/schema invariants.
2. `docs/specs/perfpal-core-spec.md`, `docs/architecture/modules.md`, `docs/schema/`, `docs/adr/`.
3. `src/scanner/`, `src/parser/`, `src/rules/`, `Cargo.toml`, `deny.toml`, `rust-toolchain.toml`.
4. The existing C/Rust/Python fixtures and goldens for the pattern.

## Session B1 — Phase 0 (planning only)

**Goal:** a spec + plan + ADR; **no production code**.

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

- B owns `src/scanner`, `src/parser`, `src/rules`, grammars, JS/TS fixtures, `src/schema` language values, `Cargo.toml`/`Cargo.lock`, `deny.toml`, JS/TS spec/plan/ADR.
- A/C/D read; no edits. `views.rs`/`mod.rs` language wiring after A step 12.
- Rebase on `main` at session start; use a separate `CARGO_TARGET_DIR`; exact pins; no copyleft.
