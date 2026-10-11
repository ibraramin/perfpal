# Active Context

## Current Task
Session 44 **python-backend W4** (task A-pf-072) **complete** — the Python tier's proof + docs package. `examples-python/` (a 20-file real package: `__init__.py` + 15 numbered family modules + a `pkg/` subpackage with the import-cycle pair + cross-module helper) fires **all 22 `PERF-PY-*` rules exactly** — **29 findings (0 hot / 12 warn / 17 info)** — locked by `tests/examples_python_coverage.rs` (22-id set-equality + 19 silent controls). 50 parser-free IR fixtures + `tests/python_negative_fixtures.rs`; the precision KPI gains 6 Python rules at 1.0/1.0; the benchmark `small` rung widens 45 → **67 rules**; a 4th golden tree `report-python`. **M3 (Python backend) is now COMPLETE.**

## Recent Decisions
- **D-pf-194 (live):** W4 DONE — corpus + IR fixtures + precision KPI + benchmark + golden + docs; full gate green (526 default / 531 semantic-clang). `@requirements-reviewer` APPROVED; `@reviewer` blocked by a provider content-filter outage (5 attempts) — disclosed self-review used instead.
- **D-pf-195 (live):** M3 COMPLETE — Python is the third first-class language (23 C + 22 Python + 22 Rust = 67 rules, schema 1.3).
- **`R-pf-093` scoped fixes:** removed the dead `key.0` in `python_arch_002`; `PERF-PY-ASYNC-001` treats only the builtin `open` as blocking; `PERF-PY-LOOP-007` gained a `.index()` shape. The **list-literal** half was intentionally skipped — the backend collapses literal containers to one shape.
- **Golden tree:** a separate `report-python` tree (mirrors `report-rust`), not an extension of `report-mixed`.

## In Progress
- Nothing — W4 complete and committed as `67fb682` (this EOL follow-up commit on top). Plan `python-backend` is complete.

## Next
- **NEXT SESSION (`/new`) = `evidence-beta` M4** (AI-code showcase): pinned Aider (Apache-2.0) + chunkhound (MIT), hand-verified true positives, a repro packet per repo + an accepted-limitations page. Read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan evidence-beta`.
- **Carry `R-pf-094` (low):** root-package relative imports (`from .sibling import …` with an empty package) don't resolve in `resolve_python_edges`; the corpus works around it by nesting the cycle in `pkg/`. Also the `R-pf-093` residual: `LOOP-007` index branch fires at `in_loop_depth ≥ 1`.
- **Re-run `@reviewer`** on `67fb682` in a fresh session to close the dual-review gap (tool outage likely transient).
- **Carry (from M1/M2/M4):** semantic-tier USR-ambiguity guard (`R-pf-077`); `PERF-RS-LOOP-008` deref-chain modelling (`R-pf-078`); `r#` raw-identifier receiver nit (`R-pf-080`); M2 synthetic-corpus credibility (`R-pf-082`).

## Blockers
- **M5 publish:** EULA/privacy contact address (`B-pf-001`) + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (`B-pf-002`) — unchanged.
- **Hygiene:** hot state **6247 B / 8192 B**; demoted 30 stale risks + settled `A-pf-072` to the cold ledger this session (502 → 533 records).

## History
- **L2 state split (2026-09-29):** hot/cold split applied — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in hot state, alignment-audit counter fields added.
