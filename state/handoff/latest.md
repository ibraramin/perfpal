# Handoff — Latest

## Objective
Session 37 **COMPLETE** — plan `evidence-beta` **M2 (Benchmark suite — build + RUN)** landed. A deterministic ground-truth mutation catalog drives a **scale ladder** that measures per-rule precision/recall with the REAL pipeline, plus a **strict-provenance byte-determinism proof**; the suite is RUN and recorded as evidence. **Both reviewers APPROVED** (round 2). **Commit pending founder approval.**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (B0–B6; publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ → M3–M6+)**.

## M2 outcome (measured, RUN)
| Rung | Files | LOC | Findings | Scoped rules | Result |
|------|------:|----:|---------:|-------------:|--------|
| `small` | 69 | 1025 | 50 | all **45 active** + retired control | every rule **1.00/1.00** |
| `medium` | 132 | 1656 | 86 | 9 gate rules | every rule **1.00/1.00** |
| `large` | 418 | 5244 | 268 | 9 gate rules | every rule **1.00/1.00** |
| `real-code` (unlabeled) | 60 | — | 418 | (counts only) | determinism PASS |

- Ground truth = deterministic in-code mutation catalog `tests/benchmark/corpus.rs`; harness `tests/benchmark/mod.rs`; entry `tests/benchmark.rs`. **No generated corpus committed** (materialized into tempdirs at RUN).
- `small` plants a TP **+ silent control for all 45 active rules** (23 C + 22 Rust) + the retired `PERF-RS-STR-001` silent control; `medium`/`large` replicate the **9 existential gate rules** (`ALLOC-001`, `ARCH-003`, `MEM-002`, `LOOP-008`, `STR-001` × C+Rust) with per-replica unique names.
- Ratchet: `tests/fixtures/benchmark/baseline.json` (46 rules, intended **1.0/1.0**, never lowered) + no-vacuous-cell guard + 45-id coverage lock + stale-baseline check.
- Determinism: every rung (and the mixed real-code rung) run **twice**; byte-identical under a **provenance-only** mask (metrics.json: only `scan.started_at`/`duration_ms`/`root_path`; report.md: provenance + scan root only, **§4 block NOT stripped**; charts + bundled schema byte-exact).
- All corpora parse clean (**0 partial / 0 failed**). Harness always-on **~4.3 s** (default; `semantic-clang` ~12 s from libclang init).

## Verification
- `cargo test --locked --test benchmark -- --nocapture` **PASS** (~4.3 s; all rules 1.00/1.00; determinism PASS).
- `cargo test --locked` **399 / 0** · `--features semantic-clang` **404 / 0** · `cargo fmt --check` clean · `cargo clippy --all-targets --locked -- -D warnings` clean.
- `src/` **unchanged**; baseline **not lowered**. Dual review: round 1 (@requirements-reviewer APPROVED; @reviewer CHANGES_REQUESTED 4 MAJOR/6 MINOR/2 NIT) → fix wave → **round 2 APPROVED**. Two non-blocking rev-2 items also applied (panic-path diff dir persists; README extension instructions).

## Risks / Open (append-only)
- **M2 ladder corpora are synthetic/generated** (exact ground truth); real-repo credibility is **M4**. The real-code rung is internal committed code (unlabeled).
- **IR-only semantic sub-cases** (proven-`fclose` ALLOC-002, resolved-format STR-003, resolved-edge ARCH-003; ARCH-001 deep-chain/hub) are covered by `tests/fixtures/ir/` + `semantic-clang`, **not** the plain-source benchmark (cycle TP planted instead).
- **Wall-clock is informational, not gated** (P/R + parse health + determinism are the gates).
- NIT (rev-2): `replace_json_value` first-occurrence substitution; `active_rule_ids()` hand-maintained 45-id duplicate (reporter lock keeps sync).
- **Carried (M1):** semantic-tier USR ambiguity for duplicate external C definitions (hardening candidate).
- **M5 publish gates unchanged:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD`.

## Next Actions
1. **Founder approves the M2 commit** → commit the working tree (list below; **EXCLUDE `.opencode/**`**). Suggested message:
   `feat(evidence-beta): M2 benchmark suite (build+run) — deterministic mutation catalog + scale ladder (small 45-rule / medium+large 9-gate) + strict-provenance byte-determinism proof + evidence record; all rules 1.00/1.00; dual review APPROVED`
2. **NEXT SESSION (`/new`) = plan `evidence-beta` M3 — Python backend.** Read `state/session-state.json` + this file + `docs/activeContext.md` FIRST, then `readPlan evidence-beta`.

## Change set (this session, uncommitted; EXCLUDE `.opencode/**`)
- **tests/**: `benchmark.rs` (new entry), `benchmark/mod.rs` (new harness), `benchmark/corpus.rs` (new catalog), `common/mod.rs` (`markdown_escaped` made `pub` — no behavior change).
- **tests/fixtures/benchmark/**: `baseline.json` (new, 46 rules), `README.md` (new contract).
- **docs/**: `evidence/benchmark-suite.md` (new RUN record), `roadmap.md` (M2 status block).
- **Root**: `CHANGELOG.md` (M2 Added bullet).
- **state/**: `session-state.json`, `handoff/latest.md`, `activeContext.md`.
