# Track A — Atlas beta (main)

**Status: Atlas v2 is DONE (2026-10-26).** `atlas-build` is marked done (D-pf-258); DoD 1–13 and the
parallel panel are green; the step-15 docs/state change set is **committed `57d91a2`**;
**no push**. Next: the founder-owned publish gate B-pf-001/002.

**Start this tab:** open `/home/shafin/Opencode/Perfpal` (branch `main`), read
`state/handoff/latest.md` first. There is no further Atlas build session; see the handoff for the
post-beta follow-ups.

**Track handoff:** `state/handoff/latest.md` · **Plan:** `docs/plans/atlas-build.md` (done) · **Contract:** `docs/evidence/atlas-visual/v2/DECISION.md` · **Brief:** `docs/atlas/AGENT-BRIEF.md` · **Audit:** `docs/evidence/atlas-audit/AUDIT.md`.
**Base:** `821b984`. **Beta:** this is the release train; B/C/D are non-blocking.

## Read first

1. `state/handoff/latest.md` — step-15 done, D-pf-256/257/258, the next actions, the known residuals.
2. `docs/plans/atlas-build.md` steps 11–15 and the DoD.
3. `docs/evidence/atlas-visual/v2/DECISION.md` — signed Option A; rail step 11; F/charts step 12; founder gate step 13.
4. `docs/atlas/AGENT-BRIEF.md` + `docs/evidence/atlas-audit/AUDIT.md`.
5. `docs/specs/atlas-layout.md` and the current report at `http://127.0.0.1:8767/report.html#map`.

## Session A1 — step 11: detail rail

**Goal:** restore the G-style per-file detail layer without turning the canvas into a dashboard.

**Build:** a right rail / overlay / designed below-chart carrying, per top file: role (production/test/bench/entry/unreached), cost + `× median function` + `top N%`, top functions, dominant finding ID, and a reachable finding/function link. The canvas stays graph-only (A9); the rail is detail, not a metric panel. 1440×900: canvas bottom **≤900**; 1280×800: no horizontal scroll.

**Do not:** start step 12 (simplified page/charts) or ship; no schema/JS/deps; no B/C/D files; no merge
of other branches. *(A2/step 14 is no longer blocked: step 13 was APPROVED 2026-10-24.)*

**Definition of done:** the rail renders in all eight states; functions and finding IDs are reachable from the first page; A9/A12/A4/A10 green; `cargo fmt`, `clippy`, `test`, `deny`, and the semantic gates exit 0; evidence + parallel review panel.

**EOL:** update `state/handoff/latest.md`, `docs/activeContext.md`, `state/session-state.json`; commit only with founder approval.

## Sessions A2–A5 (signposts)

- **A2 — step 12: simplified human page + progressive charts.** First page = graph + key metrics + top-3 actions + **one** caveat; below = weight/blast/file charts; retire the seven-block text wall; `#map` ≤~5k; link to `report.md`/`metrics.json` (and C’s `agent-report.md` when it lands). No schema/JS. This is the two-layer contract.
- **A3 — step 13: founder visual gate + G-baseline diff + V8 A1/strip rerun.** **APPROVED 2026-10-24**
  (D-pf-253, delegated evidence-based); A2 (step 14) **DONE** (delegated), step 15 schedulable, ship
  blocked on the memchr fold residual.
- **A4 — step 14: A2 holdout.** memchr + django/postgres; animation on; 3 actions in <10 minutes; record the verdict. **DONE 2026-10-24/26 (delegated):** postgres + memchr payload, 4/4 blind lanes PASS, pause/reduced-motion/print PASS; `docs/evidence/map-vision/v8/a2.md`; memchr fold residual carried to step 15.
- **A5 — step 15: ship/docs/EOL. DONE 2026-10-26 (D-pf-258).** README/output contract/specs/V7
  archive/CHANGELOG; full DoD 1–13 PASS; parallel panel approved; `atlas-build` marked done; memchr
  fold accepted (D-pf-256); delegated A2 closes the beta gate (D-pf-257); **committed `57d91a2`; no push**. Publish gate B-pf-001/002 is founder-owned.

## Guardrails / ownership

- A owns `main` state, `atlas_*.rs`/`atlas_brief.rs`/`atlas.rs`, Atlas tests/goldens, `docs/specs/atlas-*`, `docs/plans/atlas-build.md`, `docs/evidence/atlas-visual/`.
- Do not edit B/C/D files or their branches; they read the Atlas surfaces.
- `docs/tracks/` is the parallel index; `.playwright-mcp/` is scratch — do not commit either unless the founder asks.
- The live server `:8767` serves `out-atlas-v2-hyperfine`; regenerate there and refresh.
