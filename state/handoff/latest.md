# Handoff — Latest

## Objective
Session 36 **COMPLETE** — plan `evidence-beta` **M1 (Precision Gate, C+Rust)** landed. The five dominant self-scan noise rules are disposed, a **measured** per-rule precision KPI harness is in place, goldens are rebaselined, and **both reviewers APPROVED** (round 2). Commit pending founder approval.

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (B0–B6; publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ → M2–M6+)**.

## M1 outcome (empirical, `perfpal src`, 33 files)
| Rule | Before | After | Disposition |
|------|-------:|------:|-------------|
| `PERF-RS-ALLOC-001` | 511 | 74 | aggregate one finding per innermost loop + literal-receiver suppression |
| `PERF-RS-ARCH-003` | 204 | 90 | name-uniqueness gate (shared producer → also C) |
| `PERF-RS-STR-001` | 145 | 0 | **retired for Rust** (C rule intact) |
| `PERF-RS-MEM-002` | 128 | 0 | drop `clone`/`to_vec` from the family |
| `PERF-RS-LOOP-008` | 96 | 4 | require a `Subscript` element |
| **5-rule total** | **1084** | **168** | |
| **All findings** | **1219** (658/214/347) | **304** (76/97/131) | |

Rust tier **23 → 22 rules** (45 total = 23 C + 22 Rust).

## Precision KPI harness
- Labeled ground-truth (mutation) corpus `tests/fixtures/precision/` (C+Rust) + `tests/precision_kpi.rs` runs the REAL full pipeline and ratchets per-rule precision/recall against `labels.json`; **all 9 C+Rust gate rules measure 1.0/1.0** (default + `semantic-clang`). Evidence: `docs/evidence/precision-baseline.md`.

## Verification
- `cargo test --locked` **398 / 0** · `--features semantic-clang` **403 / 0** · `cargo fmt --check` clean · `cargo clippy --all-targets --locked -- -D warnings` clean.
- Goldens: `report-rust` + `report-mixed` rebaselined; C `report` unchanged; byte-idempotent.
- Dual review: round 1 CHANGES_REQUESTED (both) → review-fix wave → **round 2 APPROVED** (@reviewer + @requirements-reviewer).

## Risks / Open (append-only)
- **Semantic-tier USR ambiguity (C-only; DEFERRED).** The resolved branch bypasses the ARCH-003 uniqueness gate on duplicate USRs (non-`static` same-name external defs share a location-independent USR). Blast radius = link-invalid C; default build + the Rust tier unaffected. `precision-baseline.md` §6; M2 hardening candidate.
- **`PERF-RS-LOOP-008` deref-chain silence** — pure deref chains stay silent (accepted FN, documented).
- **NIT (non-blocking):** `starts_with_string_literal` `r#` prefix also matches a raw-identifier receiver; tighten to `r#"`/`br#"` later (recorded, not fixed, to preserve the reviewed revision).
- **M5 publish gates unchanged:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD`.

## Next Actions
1. **Founder approves the commit(s).** ⚠️ The **session-35 formalization docs are STILL uncommitted** — commit them first, then the M1 change set (messages below).
2. **NEXT SESSION (`/new`) = plan `evidence-beta` M2 — Benchmark suite (build + RUN).** Read `state/session-state.json` + this file + `docs/activeContext.md` FIRST, then `readPlan evidence-beta`.

## Change set (this session, uncommitted; EXCLUDE `.opencode/**`)
- **src/**: `analyzer/heuristics/rust/{alloc,loop,mem,str,mod}.rs`, `analyzer/heuristics/{arch,mod}.rs`, `analyzer/{mod,finding}.rs`, `reporter/mod.rs`.
- **tests/**: `precision_kpi.rs` (new), `fixtures/precision/**` (new), `fixtures/ir/PERF-RS-ALLOC-001__{callee-cap,defensive-singleton,qualified-token,literal-receiver}.json`, `fixtures/ir/PERF-RS-ARCH-003__ambiguous.json`, deleted `PERF-RS-STR-001__*.json`, `rust_wave_c1.rs`, `rust_wave_c2.rs`, `rust_negative_fixtures.rs`, `reporter_rust.rs`, `examples_rust_coverage.rs`, `mixed_language_e2e.rs`, `arch_fixtures.rs`, `golden/report-rust/**`, `golden/report-mixed/**`.
- **examples-rust/**: `src/02_string_building.rs`, `src/08_globals_and_chains.rs`.
- **docs/**: `evidence/precision-baseline.md` (new), `evidence/dogfood-rust-backend.md`, `heuristics/rust-catalog.md`, `heuristics/catalog-v1.md`, `architecture/rust-backend.md`, `roadmap.md`.
- **Root**: `README.md`, `CHANGELOG.md`.
- **Session-35 docs (also still uncommitted)**: `docs/specs/evidence-beta-spec.md`, `docs/plans/evidence-beta.md`, `docs/adr/ADR-0003..0006`, `docs/architecture/modules.md`, plus state files.

## Commit messages (proposed)
1. *(session-35 formalization, separate first commit)*
   `docs(roadmap): formalize evidence-first AI-code beta — spec evidence-beta-spec + plan evidence-beta (M1-M6+) + ADR-0003..0006; roadmap re-based (parked v0.2.0-beta.1 = M5)`
2. *(M1)*
   `feat(evidence-beta): M1 precision gate (C+Rust) — ALLOC-001 aggregate/innermost + literal-receiver suppression, ARCH-003 name-uniqueness, MEM-002 drop clone/to_vec, LOOP-008 subscript, STR-001 retired; labeled precision KPI harness + goldens; self-scan 1219->304; Rust tier 22 rules`
