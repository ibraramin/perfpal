# Active Context

## Current Task
Session 41 **python-backend W1** (task A-pf-069) **complete**: pinned `tree-sitter-python = "=0.25.0"` (MIT, ABI 15), added `tests/python_grammar_link.rs` (ABI window + zero-ERROR parse + 15-kind lock + malformed non-vacuity, 2/2 pass), and authored `docs/architecture/python-backend.md` (node-kind→IR map, comprehension/module-scope/async/notebook decisions, 4 additive IR fields, durable 10-retire/13-mirror/8-native/3-cut disposition table).

## Recent Decisions
- **D-pf-191 (live):** W1 outcome + design lock (see `state/session-state.json` and the W1 doc). Resolves R-pf-087/R-pf-088 (mirror-vs-retire) via the durable disposition table.
- **`.ipynb` excluded from M3** (documented accepted limitation; dispatcher returns `None` for `.ipynb`).
- **4 additive serde-default IR fields planned** (`FunctionRecord.is_async`, `FunctionRecord.is_module_scope`, `AssignmentEvent.binding_kind`, `CallEvent.receiver_root`) — IR-internal; no schema bump from inference.
- **Round-1 review corrections applied:** `await`/`lambda` are named nodes; `comparison_operator` has no `operands` field; no `positional_argument`; `not` is `not_operator`; added `PERF-PY-ALLOC-004` materialisation-churn disposition.

## In Progress
- Nothing — W1 complete and committed (work commit + expected EOL follow-up).

## Next
- **NEXT SESSION (`/new`) = plan `python-backend` W2:** read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan python-backend` + `docs/architecture/python-backend.md`. Deliver the Python backend module (`Extraction` contract + ParseHealth parity), minimal binding/alias inference + best-effort import resolution, and `LanguageKind::Python` / `language_of(.py/.pyi)` / `extract_unit` / `failed_unit` / `SUPPORTED_LANGUAGES` wiring.
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; M2 non-blocking notes (`replace_json_value` uniqueness assert; expose `registered_rule_ids()`).
- **W2 obligations:** comprehension-as-`LoopNode` + once-evaluated iterables; synthetic module-scope `FunctionRecord`; 4 additive IR fields; `__init__.py` one-level re-export rule; `ARCH-*` thresholds measure in W4.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Hygiene:** hot state at **8076 B / 8192 B** (~116 B headroom) — expect to demote settled decisions next session.

## History
- **L2 state split (2026-09-29):** hot/cold split applied — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md` (goal / non-goals / stage / metrics), `north_star_ref` in hot state, alignment-audit counter fields added.
