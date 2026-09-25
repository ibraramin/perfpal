# Active Context

## Current Task
Session 36 **COMPLETE** — plan `evidence-beta` **M1 (Precision Gate, C+Rust)** landed: 5 dispositions + a measured precision KPI harness + goldens rebaselined; **both reviewers APPROVED** (round 2). Commit pending founder approval.

## Recent Decisions
- **M1 dispositions (founder-approved):** `PERF-RS-ALLOC-001` aggregate one finding per innermost loop + literal-receiver suppression; `PERF-RS-ARCH-003` name-uniqueness gate (shared `cross_file_call_edges` producer → also governs `PERF-C-ARCH-003`); `PERF-RS-MEM-002` drop `clone`/`to_vec`; `PERF-RS-LOOP-008` require a `Subscript` element; **`PERF-RS-STR-001` retired for Rust** (C rule intact) → Rust tier **22 rules**.
- **KPI harness depth = labeled ground-truth corpus + ratchet baseline** (M1 lightweight; M2 builds the full scale-ladder benchmark).
- **STR-001 retirement is a deliberate, founder-approved deviation** from the R3 "mirror all 23 C families" decision — recorded in `docs/heuristics/rust-catalog.md` + `docs/evidence/precision-baseline.md`.

## In Progress
- Nothing — M1 session complete; commit pending founder approval.

## Next
- **NEXT SESSION (`/new`) = plan `evidence-beta` M2 — Benchmark suite (build + RUN):** ground-truth mutation fixtures + scale ladder (small/medium/large) + determinism proof; **RUN** and record in `docs/evidence/`. Read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan evidence-beta`.
- **Carry (from M1 review):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; re-run the KPI over the M2 corpora.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Commit:** founder approval (session-35 formalization docs + the M1 change set).
