# Active Context

## Current Task
Session 37 **COMPLETE** — plan `evidence-beta` **M2 (Benchmark suite — build + RUN)** landed: a deterministic in-code mutation catalog + a **scale ladder** (small/medium/large) + a **strict-provenance byte-determinism proof**, RUN and recorded in `docs/evidence/benchmark-suite.md`. Both reviewers **APPROVED** (round 2). **Commit pending founder approval.**

## Recent Decisions
- **M2 scope (founder-approved):** deterministic in-code generator (corpora materialized at RUN; no committed corpus bloat; M1 corpus stays the hand-labeled anchor); **all 45 rules at `small`**, the **9 existential gate rules** at `medium`/`large`; all rungs **always-on** (fast).
- **Measured result:** every scoped rule **1.00/1.00** on every rung (default + `semantic-clang`); all corpora parse clean (0 partial/0 failed); determinism PASS; harness **~4.3 s**.
- **Fix wave (reviewer round 1 → 2):** added the no-vacuous-cell guard, a 45-id coverage lock + stale-baseline check, **benchmark-local strict provenance-only masks** (no JSON re-serialization, §4 report block not stripped), explicit Silent labels for all 45 active rules, and the m1–m6/n1–n2 fixes.
- **Disclosed deviation:** `registered_rule_ids()` is `pub(crate)`, so `active_rule_ids()` derives the 45 ids from public `finding::*` consts; the in-crate reporter lock still catches a new registry rule.

## In Progress
- Nothing — M2 session complete; commit pending founder approval.

## Next
- **NEXT SESSION (`/new`) = plan `evidence-beta` M3 — Python backend:** tree-sitter-python backend (MIT, exact pin), `PERF-PY-*` rules mirroring the full catalog, schema MINOR bump admitting `"python"`, an `examples-python/` corpus, `semantic-clang` stays C-only. Read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan evidence-beta`.
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; M2 non-blocking notes (`replace_json_value` uniqueness assert; expose `registered_rule_ids()`); re-run the M1/M2 KPIs over M3 corpora.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Commit:** pending founder approval (M2 change set — see `state/handoff/latest.md`).
