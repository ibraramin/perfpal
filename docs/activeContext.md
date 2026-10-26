# Active Context

## Current Task
**Atlas v2 steps 1–15 are DONE; `atlas-build` is marked done 2026-10-26 (D-pf-258).** Step 15
docs/EOL + DoD 1–13 + the parallel panel are complete; the change set is committed `57d91a2`; no push.
- **Step 9–10:** signed Option A; G warm/sans/mono system; first-page fit (862 ≤ 900); key-metrics
  strip; chrome hygiene; founder-approved 2026-10-24.
- **Step 11 (`189df91`):** G-style right rail (530px, 12 cards, roles, exact function/finding links,
  eight-state rail); canvas 869 ≤ 900.
- **Step 12 (`9a94905`):** four designed below-graph charts; drills moved to `atlas-weight.html` /
  `atlas-q2.html`; seven-block text wall retired; `#map` 1,848px; F treemap prototyped but **F remains
  undecided** (D-pf-251).
- **Step 13 (D-pf-253, APPROVED 2026-10-24; delegated evidence-based):** real-page DOM gate all pass
  (canvas 869 ≤ 900; `#map` 1,848; no h-scroll; one definition each; A9/A12 green); V8 A1 16/16 +
  strip 16/16; G diff SAME SYSTEM. Evidence `docs/evidence/atlas-visual/v2/...`.
- **A2 / step 14 (D-pf-255, delegated PASS 2026-10-24/26):** postgres `856a659` (2,566 files, 102,933
  findings, 181 s) + memchr crate payload `bd6068c3` (45 `.rs`, 530 findings, 1 s; `benchmarks/` +
  `fuzz/` excluded). Four fresh-context blind lanes (2/class) named 3 actions in **87–120 s** and
  answered understandable **YES**; pause freezes (68/68, 45/45), reduced motion and print show the
  final frame; geometry postgres **871.2 ≤ 900**, memchr **1,282.2 > 900**. Evidence
  `docs/evidence/map-vision/v8/a2.md` + `a2/`.
- **Step 15 (D-pf-256/257/258, DONE 2026-10-26):** docs pass (output contract/README/specs/V7
  archive/CHANGELOG); **memchr fold accepted as a documented beta residual** (D-pf-256, fix trigger);
  **delegated A2 closes the beta gate** (D-pf-257; founder V6 gold standard open, fresh holdout
  needed); DoD 1–13 PASS (`docs/evidence/atlas-visual/v2/gate-step15.txt`); parallel panel approved
  after fixes; plan done; **committed `57d91a2`; no push**. No schema/JS/deps/golden
  change; no visual tuning.

## Recent Decisions
- **D-pf-258 (live, 2026-10-26):** step 15 docs/EOL done; DoD 1–13 PASS; panel approved; `atlas-build`
  done; commit `57d91a2`; no push.
- **D-pf-257 (live, 2026-10-26):** the delegated A2 PASS closes the A2 gate for this beta; the
  founder's own timed eye-test remains the V6 gold standard and needs a fresh holdout.
- **D-pf-256 (live, 2026-10-26):** memchr library fold (1,282 px > 900) accepted as a documented beta
  residual, not a layout-contract pass; fix trigger in `accepted-limitations.md` §D.
- **D-pf-255 (live):** A2 delegated PASS (postgres + memchr; 4/4 blind lanes).
- **D-pf-253 (live):** step 13 composition approved (delegated, evidence-based).
- **D-pf-251 (live):** F treemap remains **undecided**; keep W1; no F code shipped.
- **D-pf-250/252/253/254 (archived):** full text in `state/archive/ledger.jsonl`; pointer D-pf-249.
- **R-pf-109 (closed 2026-10-26):** visual regression resolved by the v2 rework + A2; kept as a
  record. **R-pf-104 (closed):** A2 pass; ranking recall remains R-pf-105/106/A1b.
- **D-pf-249 (archived pointer):** archived decisions are in `state/archive/ledger.jsonl`.

## In Progress
- The step-15 change set is committed (`57d91a2`); **no push**. No code, schema, JS, dependency or
  golden change.

## Next
- Publish gate B-pf-001 (contact) / B-pf-002 (minisign secrets) — founder-owned.
- Post-beta: F decision (D-pf-251), memchr fold fix trigger (D-pf-256), `map-build`/A1b ranking
  recall (R-pf-105/106/108).

## Blockers
- **Publish gate:** B-pf-001 (contact) + B-pf-002 (minisign secrets) — founder-owned.
- **Founder V6 gold standard:** the founder's own timed A2 run was not applied; a fresh holdout is
  needed for an untainted re-run. Recorded, not waived.
- **Map/ranking:** R-pf-105/106/108 open; A1b open; `map-build` not done.
- **F undecided:** treemap candidate deferred; no F code ships.

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
  D-pf-238 (G core, F supporting); Atlas named. **V7 is now archived evidence (2026-10-26).**
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
  F treemap candidate prototyped (F undecided); committed `9a94905`.
- **Atlas v2 Session A3 (2026-10-24):** step 13 gate evidence — real-page DOM gate all pass at
  1440×900/1280×800; V8 A1 16/16 + strip 16/16 on the reworked page; G-baseline diff (SAME SYSTEM);
  founder composition sign-off APPROVED 2026-10-24 (delegated); evidence committed `86c0537`.
- **Atlas v2 Session A4 (2026-10-24/26):** step 14 A2 holdout (postgres + memchr payload); 4/4 blind
  lanes, 3 actions 87–120 s, understandable YES; pause/reduced/print PASS; memchr fold residual
  (1,282 > 900) carried to step 15; evidence committed `9631137`.
- **Atlas v2 Session A5 (2026-10-26):** step 15 docs/EOL; D-pf-256 (memchr fold accepted), D-pf-257
  (delegated A2 closes beta gate), D-pf-258 (plan done); DoD 1–13 PASS; parallel panel R1/R2/Q1/Q2 +
  R3/R4 approved after fixes; state/handoff/activeContext/track/brief updated; **committed
  `57d91a2`; no push**.
