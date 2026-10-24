# Active Context

## Current Task
**Atlas v2 steps 9–12 are implemented (2026-10-23/24); step 12 (progressive charts) is uncommitted
pending founder commit approval; the step-13 final gate is PENDING.**
- **Step 9:** signed founder decision (Option A: port G; `report.html#map` capture; detail rail step 11; F
  candidate step 12) under `docs/evidence/atlas-visual/v2/`.
- **Step 10:** first-page fit (canvas bottom 862 ≤ 900), G warm paper/charcoal/burnt-orange + sans
  chrome/mono data, 58px masthead, 53px key-metrics strip outside the canvas, one footer, mid-ellipsis,
  path-only edge labels, opaque test/bench fills, explicit unreached totals. Founder-approved 2026-10-24.
- **Step 11 (committed `189df91`):** G-style right rail (`#atlas-rail`, 530px) beside the 868px canvas —
  12 cards (entry first, production-first), role, M-04 cost anchor, top-3 functions, dominant finding;
  exact function/finding links; eight-state rail; canvas bottom 869 ≤ 900; 1280×800 stacks.
- **Step 12:** four designed below-graph charts (weight, blast, file detail, coverage/links) replace the
  seven-block text wall; Q2/weight drills moved to `atlas-q2.html` / `atlas-weight.html`; `#map`
  **1,848px** on hyperfine and cJSON (from 8,895/14,598); canvas bottom 869; 1280×800 no horizontal
  scroll; A8 re-baselined to the strip as the first answer surface. F treemap candidate prototyped
  under `docs/evidence/atlas-visual/v2/step12/` — **F remains undecided**; Explorer treemap untouched.
  Evidence: `docs/evidence/atlas-visual/v2/{STEP12.md,step12.json,gate-step12.txt,VISION-STEP12.md}`.
- **Panel:** R1/R2/Q1/Q2/R4 **APPROVED**; zero unresolved blocking findings; recorded in the handoff.
- **Committed 2026-10-24 with founder approval; no push; no schema/JS/deps; holdouts vaulted; `map-build` NOT done; A1b open;
  A2/ship (steps 14–15) blocked by R-pf-109.**

## Recent Decisions
- **D-pf-251 (live, 2026-10-24):** F treemap decision — **keep W1; F stays undecided**; no F code
  shipped; prototype retained under `docs/evidence/atlas-visual/v2/step12/`; Explorer treemap and F
  data untouched.
- **D-pf-250 (live, 2026-10-24):** Atlas v2 step 12 progressive charts implemented; seven-block text
  wall retired; weight/blast/file/coverage blocks; drills standalone; `#map` 1,848; A8 re-baselined;
  F candidate prototyped but **undecided**; uncommitted pending founder approval; step 13 pending.
- **D-pf-249 (live, 2026-10-24):** Atlas v2 step 11 detail rail implemented (committed `189df91`).
- **D-pf-248 (live):** steps 9–10 implemented; Option A signed; step-10 founder check approved.
- **D-pf-247 (live):** plan reordered — steps 9–13 = v2 visual rework; A2/ship 14–15 blocked.
- **D-pf-238/239/240** moved to the cold ledger (historical Atlas model/contract decisions;
  preserved in ADR-0009/0010 and the ledger).
- **R-pf-109 (open, high):** Atlas visual regression; A2/ship blocked until the step-13 gate.

## In Progress
- Step 12 code/tests/evidence are green; final vision read PASS (step-13 proxy) and the parallel
  panel are the remaining gates.
- **Committed 2026-10-24** with founder approval; no push.
- Next: step 13 founder visual gate + G-baseline diff + V8 A1/strip rerun.

## Next
- Founder decision recorded: **keep W1; F stays undecided**; commit approved 2026-10-24.
- `/new` → **Session A3: step 13** (founder visual gate + G-baseline diff + V8 A1/strip rerun).
- Do **not** start A2/ship until step 13 passes; holdouts vaulted; `map-build` not done; A1b open.

## Blockers
- **Atlas A2 (R-pf-108):** founder timed test on application + library classes; blocked until step 13.
- **Atlas visual gate:** step-10 first page APPROVED 2026-10-24; step-13 final composition remains.
- **Map legibility:** R-pf-104/R-pf-105/R-pf-106; ranking recall A1b open.
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prereg + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Map research:** R1–R4 → D-pf-215.
- **I1–I4:** entries C/Rust/Python + cost-v2 (`e0df47a`); entry map + bands (`d78bb85`);
  annotations + findings stem (`f56d072`).
- **I5:** proof + demo + **A5 FAIL** (`dbb9bbe`→`6f31a4f`), successor plan created.
- **V0–V2:** prereg → diagnosis → recall → review fixes → methodology + spec lock.
- **V3–V6a:** 18 prototypes + 27 blind reads → V4 key views + Explorer → V5 validation + no-loop
  falsifier → V6a KV2 production-first amendment (6/6 reads PASS); V6 timed test scrapped.
- **V7:** F/G prototypes + 12/12 blind reads (`docs/evidence/map-vision/v7/`); founder decision
  D-pf-238 (G core, F supporting); Atlas named.
- **Atlas planning:** needs research → metrics contract → data-gap audit → layout + inclusion rule
  → ADR-0009/0010 + V8 amendments → `atlas-build` plan. Docs only.
- **Atlas ratification (2026-10-23):** ADR-0009/0010 **Accepted**; V8 ratified; D-pf-239/240
  confirmed; D-pf-241/242/243 recorded; library A2 holdout pre-registered (`memchr` crate payload).
- **Atlas build sessions 1–3 (2026-10-23):** steps 2–8 green (view model/metrics/canvas, brief/
  drills/#map wiring, test/golden lock, V8 16/16 A1 + 16/16 strip); post-V8 visual audit found the
  composition regression (`R-pf-109`); v2 rework planned (steps 9–13), A2/ship moved to 14–15.
- **Atlas v2 Session 4 (2026-10-23/24):** steps 9–10; signed Option A; first-page fit 862 ≤ 900;
  warm/sans/mono system; key-metrics strip; chrome hygiene; step-10 founder check approved 2026-10-24.
- **Atlas v2 Session A1 (2026-10-24):** step 11 detail rail (868+530 stage, 12-card rail, roles,
  exact function/finding links, eight-state lock, collision-aware canvas labels); vision PASS;
  panel R1/Q1/R2/Q2/R3 approved; committed `189df91`; step 13 pending.
- **Atlas v2 Session A2 (2026-10-24):** step 12 progressive charts; seven-block wall retired;
  weight/blast/file/coverage blocks; `atlas-weight.html`/`atlas-q2.html` drills; `#map` 1,848;
  F treemap candidate prototyped (F undecided); step 13 remains the founder gate.
