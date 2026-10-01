# Active Context

## Current Task
Session 39 **COMPLETE** — plan `python-backend` (evidence-beta **M3**) **W0 canary** executed. **Evidence-only, zero product code.** Pinned Aider + chunkhound were counted with a throwaway stdlib-`ast` counter, baselined against Ruff `PERF`, and ranked frequency × plausibility. Deliverable: `docs/evidence/python-canary.md` + committed `counter.py`/`canary_results.json`. **Commit pending founder approval.**

## Recent Decisions
- **W0 keeps:** `ALLOC-003` (flagship, 7.0/6.0 per kLOC prod), `LOOP-007` (flagship, must be loop-scoped + kind-gated), `N1-001` (native flagship), `ASYNC-001`/`ASYNC-002` (native; `async def` + loop gates proven necessary).
- **W0 reframes:** `ALLOC-001` — 98–99.6% of hits are f-strings → ship only as **loop-invariant construction** (Info-first) or not at all; `LOOP-002` — 72–83% `print`/`input` → drop them (or Info) and keep real I/O.
- **W0 adds:** `PERF-PY-LOOP-009` (try/except-in-loop; from Ruff `PERF203`) + `PERF-PY-PANDAS-001/002` (0–4 hits; Info + frequency caveat).
- **W0 defers:** `ARCH-001/002/003` — abundant structure (chunkhound depth 39 vs threshold 8) but the import-graph proxy is **not decision-grade**; thresholds measured in W2/W4.
- **`COMPREHENSION-001` cross-validated** vs Ruff `PERF401` (16 vs 13; 56 vs 49). ADR-0007 retire-10 / keep-~13 unchanged.
- **Determinism lesson:** set-iteration in an evidence metric caused run-to-run variation (`ARCH-001` 545↔488; `max_depth` 36↔39) — fixed by sorting all in-graph traversal; two runs now byte-identical.

## In Progress
- Nothing — W0 evidence complete; commit pending founder approval.

## Next
- **NEXT SESSION (`/new`) = plan `python-backend` W1:** read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan python-backend`. Deliver the grammar pin (`tree-sitter-python = "=0.25.0"`), `tests/python_grammar_link.rs`, and `docs/architecture/python-backend.md` (node-kind→IR map + the **durable disposition table**).
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; M2 non-blocking notes (`replace_json_value` uniqueness assert; expose `registered_rule_ids()`).
- **W1 design obligations:** comprehension-as-`LoopNode`; module-scope synthetic function record; `.ipynb` decision; `ARCH-003` Info/low on Python; `ALLOC-001` loop-invariant test; `LOOP-002` `print` family.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Commit:** pending founder approval (session-39 change set — see `state/handoff/latest.md`).

## L2 state split (2026-09-29)
Hot/cold split applied: hot `state/session-state.json` is now compact JSON (schema v2, ≤ 8 KB); full history
in cold `state/archive/ledger.jsonl`; AGENTS.md state rule updated. No product code touched; uncommitted.

## L2 reviewed (2026-09-29)
Hot ≤ 8 KB (7853 B); cold ledger `state/archive/ledger.jsonl` (481 records). Dual review: round-1
CHANGES_REQUESTED → round-2 reviewer APPROVED + requirements-reviewer APPROVED. Commit pending founder approval.

## L6 north-star pinned (2026-10-01)
Harness L6 (`harness-v2`): added founder-owned `docs/NORTH-STAR.md` (goal / non-goals / current stage / success
metrics; business KPIs marked `UNSET (founder to ratify)`), a `docs/NORTH-STAR.md` doc-pointer in `AGENTS.md`,
and `state/session-state.json` `north_star_ref` → `docs/NORTH-STAR.md` (+ optional `sessions_since_alignment_audit`
/ `last_alignment_audit` counter fields). Docs-only; no product code touched; uncommitted.
