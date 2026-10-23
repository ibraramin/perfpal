# Active Context

## Current Task
**Atlas v2 steps 9–11 are implemented (2026-10-23/24); step 11 (detail rail) is uncommitted pending
founder commit approval; the step-13 final gate is PENDING. Step 12 (progressive charts) is next.**
- **Step 9:** side-by-side G vs Atlas at 1440×900 (hyperfine + cJSON), the G-baseline diff and the
  signed founder decision (Option A: port G; `report.html#map` capture; detail rail step 11; F
  candidate step 12) under `docs/evidence/atlas-visual/v2/`.
- **Step 10:** first-page fit (canvas bottom 862 ≤ 900), G warm paper/charcoal/burnt-orange + sans
  chrome/mono data, 58px masthead, 53px key-metrics strip outside the canvas, one footer, mid-ellipsis,
  path-only edge labels, opaque test/bench fills, explicit unreached totals. Founder-approved.
- **Step 11:** G-style right rail (`#atlas-rail`, 530px) beside the 868px canvas — 12 cards (entry
  first, production-first), role, M-04 cost anchor, top-3 functions, dominant finding; file →
  weight drill, function → exact `profile-queue.html#fn-…`, finding → exact
  `findings-detail.html#finding-…`; all links resolve; rail in all eight states; canvas bottom 869 ≤
  900; 1280×800 stacks, no horizontal scroll; `#map` +8/+7px (footer wrap; step-12 cut). Canvas node
  labels are the file name with a parent segment on collision; full path in `<title>`/rail.
  Evidence: `docs/evidence/atlas-visual/v2/{STEP11.md,step11.json,gate-step11.txt,VISION-STEP11.md}`.
- **Panel:** R1/Q1/R2/Q2/R3 **APPROVED**; final R4 confirmation on the polished tree.
- **Uncommitted; no push; no schema/JS/deps; holdouts vaulted; `map-build` NOT done; A1b open;
  A2/ship (steps 14–15) blocked by R-pf-109.**

## Recent Decisions
- **D-pf-249 (live, 2026-10-24):** Atlas v2 step 11 detail rail implemented; 868px canvas + 530px
  right rail; eight-state rail; exact function/finding links; canvas node labels = file name +
  collision parent; `#map` +8/+7 footer-wrap; panel R1/Q1/R2/Q2/R3 approved; uncommitted pending
  founder approval; step 13 pending; steps 12/14–15 not started.
- **D-pf-248 (live):** steps 9–10 implemented; Option A signed; step-10 founder check approved
  2026-10-24; step-13 final gate pending.
- **D-pf-247 (live):** plan reordered — steps 9–13 = v2 visual rework; A2/ship 14–15 blocked.
- **D-pf-238/239/240** moved to the cold ledger (historical Atlas model/contract decisions;
  preserved in ADR-0009/0010 and the ledger).
- **R-pf-109 (open, high):** Atlas visual regression; A2/ship blocked until the step-13 gate.

## In Progress
- Step 11 code/tests/evidence are green; vision PASS; panel green (R4 confirmation finalizing).
- **Uncommitted:** 15 modified/untracked source/test/evidence/doc paths; founder commit approval
  required by the track.
- Next: step 12 progressive charts/F candidate.

## Next
- Founder: approve the step-11 commit (then commit; exclude `.playwright-mcp/`; no push).
- `/new` → **Session A2: step 12** (progressive weight/blast/file charts + F treemap candidate;
  `#map` ≤~5k; retire the seven-block text wall; no schema/JS).
- Do **not** start step 13/A2/ship until step 12 lands; holdouts vaulted; `map-build` not done;
  A1b open.

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
  panel R1/Q1/R2/Q2/R3 approved; evidence `STEP11.md`/`step11.json`/`gate-step11.txt`; uncommitted
  pending founder approval; step 13 pending.
