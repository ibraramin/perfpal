# Active Context

## Current Task
**Atlas steps 7–8 GREEN (2026-10-23).** Step 7's full test list is covered by 106 Atlas unit tests
+ 5 integration locks + the 8-state fixtures + the four regenerated `report.html` goldens;
`tests/atlas_vision.rs` renders the matrix on demand. Step 8's V8 gate rendered all eight states at
1440×900 and ran **16/16 blind A1 reads + 16/16 strip-specific reads** (all pass); evidence is in
`docs/evidence/map-vision/v8/`.
- The gate found and fixed real legibility/collision defects: 72rem→90rem body width (canonical
  1440 canvas), SVG text floors, four-line nodes/46px minimum, two-line column headers, compact
  `×N` edge pills with deterministic node avoidance, hull-label clearance, a first-screen key
  line, and tooltip-free capture. Only the four `report.html` goldens changed.
- Gate: fmt / clippy / `cargo test --locked` (55 suites) / deny / semantic clippy+test all exit 0;
  schema diff empty; A4/A9/A10/A12 pass.
- No schema/JS/dependency change; holdouts vaulted; treemap deferred. A2 is step 9.

## Recent Decisions
- **D-pf-246 (live, 2026-10-23):** Atlas steps 7–8 implemented and green; V8 A1 16/16 +
  strip 16/16; renderer legibility/collision fixes; steps 9–10 queued; no schema; holdouts vaulted.
- **D-pf-245 (live, 2026-10-23):** Atlas steps 5–6 implemented and green (brief/drills + `#map`
  wiring; A8/A9/A10/A12 integration tests; four report.html goldens; commits `672e4b6`/`a2a4c99`);
  review round 1 fixes applied; treemap deferred by session constraint; steps 7–10 queued; no
  schema; holdouts vaulted.
- **D-pf-244 (live, 2026-10-23):** Atlas steps 2–4 implemented and green (view model, M-01…M-18,
  canvas; 76 unit tests; panel APPROVED on `6c0f0f3`); steps 5–6 next; no schema; holdouts vaulted.
- **D-pf-241/242/243 (live, ratified 2026-10-23):** structural-edge option A; A8 canvas-first
  placement; A12 no-new-JS scope.
- **D-pf-239/240 (live, RATIFIED 2026-10-23):** Atlas model + contracts; ADRs Accepted; plan
  `atlas-build` active (steps 1–4 done).
- **D-pf-238 (live):** founder correction — G core, F components supporting (not fallback);
  full-page graph; metrics below.
- **D-pf-237/236 (cold):** entry-flow direction + prototypes; consolidation (demoted to ledger).

## In Progress
- **Nothing active in the hot tree.** Steps 7–8 are green; the plan is still active.
- **`atlas-build` steps 9–10 remain queued:** founder A2 (application + library, animation on) and
  ship/docs/EOL. The weight-treemap stays deferred; A1b stays open; `map-build` is NOT done.

## Next
- **`/new` → atlas-build session 4: steps 9–10 only** (founder A2 → ship/docs/EOL).
  - Run A2 on the application and library classes with animation on; record `a2.md`; a failure
    supersedes ADR-0009.
  - Then ship/docs/EOL: output contract, README, CHANGELOG, archive V7 prototypes, mark the plan
    done after the panel.
- **Keep holdouts vaulted** until A2; no scan/render/agent reads.
- **Do not mark `map-build` done.** A1b stays open. M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Atlas A2:** R-pf-108 — the decisive gate is the founder timed test on application **and**
  library classes (animation on); structural LOW/MEDIUM edges on the hero canvas remain the main
  honesty risk (option A reversal trigger recorded).
- **Map legibility:** R-pf-104/R-pf-105/R-pf-106 — views validated (A1), ranking recall (A1b)
  open; the brief carries KV1 with the complete-list guarantee; the rank-key/call-weighted
  question moves into the build.
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
