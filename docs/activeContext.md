# Active Context

## Current Task
**Atlas v2 steps 9–10 are implemented (2026-10-23/24) and the step-10 first page is founder-approved
(2026-10-24); the step-13 final gate is PENDING. Step 11 (G-style detail rail) is next.**
- **Step 9:** side-by-side G vs Atlas at 1440×900 (hyperfine + cJSON), the G-baseline diff and the
  signed founder decision (Option A: port G; `report.html#map` capture; detail rail step 11; F
  candidate step 12) under `docs/evidence/atlas-visual/v2/`.
- **Step 10:** first-page fit (**canvas bottom 862 ≤ 900**), G warm paper/charcoal/burnt-orange +
  sans chrome/mono data, 58px masthead, **53px key-metrics strip outside the canvas** (entry,
  hottest, path, caps, top-3 KV1 actions, one legend/unit definition/caveat), one footer inside the
  canvas, mid-ellipsis + full-path titles, path-only edge labels, opaque test/bench fills, explicit
  unreached totals (21/11 hyperfine, 4/75 cJSON). Evidence: `step10.json`, `STEP10.md`, `gate.txt`,
  `atlas-v2-*-final.png`.
- **Panel:** R1/R2/Q1/Q2 + a fresh confirmation lane all **APPROVED**; zero blocking findings. The
  chip `+` wording, evidence transcript/vision record, README/state, fit disclosures and fail-closed
  reproducer were fixed.
- **Committed 2026-10-24 (`468d772` + this docs/state commit); no push; no schema/JS/deps; holdouts
  vaulted; `map-build` NOT done; A1b open; A2/ship (steps 14–15) blocked by R-pf-109.**

## Recent Decisions
- **D-pf-248 (live, 2026-10-23/24):** Atlas v2 steps 9–10 implemented; Option A signed; step 10
  first-page fit/G visual language/key-metrics strip/chrome hygiene; panel R1/R2/Q1/Q2 + fresh R3
  APPROVED; step-10 founder check approved 2026-10-24; step-13 final gate pending; steps 11–12 not started.
- **D-pf-247 (live):** plan reordered — steps 9–13 = v2 visual rework; A2/ship 14–15 blocked.
- **R-pf-109 (open, high):** Atlas visual regression; A2/ship blocked until the step-13 gate.
- **D-pf-246/245/244** moved to the cold ledger (historical session status); **A-pf-119** settled.

## In Progress
- Step 10 implementation + evidence are green; the panel is green; the founder approved the first page.
- **Committed 2026-10-24 (`468d772` + this docs/state commit); no push.**
- Next: step 11 detail rail.

## Next
- **Step 10 approved and committed;** `/new` → **Session 5: steps 11–13** (G-style
  detail rail; progressive weight/blast/file charts + F treemap candidate; founder visual gate +
  G-baseline diff + V8 A1/strip rerun).
- Do **not** start A2 (step 14) or ship (step 15); holdouts vaulted; `map-build` not done; A1b open.

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
  Docs/state only.
- **Atlas build session 1 (2026-10-23):** steps 2–4 implemented + unit tests (`fe58299`/`f76a63b`/
  `3fa7c07`, review fixes `5e6b45e`/`6c0f0f3`); full gate green; panel R1/R2/Q1/Q2 APPROVED. No
  schema/`#map` wiring/holdout reads.
- **Atlas build sessions 2–3 (2026-10-23):** steps 5–8 green (view model/metrics/canvas, brief/
  drills/#map wiring, test/golden lock, V8 16/16 A1 + 16/16 strip); post-V8 visual audit found the
  composition regression (`R-pf-109`); v2 rework planned (steps 9–13), A2/ship moved to 14–15.
- **Atlas v2 Session 4 (2026-10-23/24):** steps 9–10; signed Option A; G-baseline diff + 1440×900
  side-by-sides; first-page fit 862 ≤ 900; warm/sans/mono system; key-metrics strip; chrome hygiene;
  evidence under `docs/evidence/atlas-visual/v2/`. Step-10 founder check approved 2026-10-24; committed
  (`468d772` + docs/state); step-13 final gate pending.
