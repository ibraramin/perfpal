# Active Context

## Current Task
**Atlas steps 2–4 GREEN (2026-10-23).** The ratified build landed: `src/reporter/atlas.rs` (pure
view model), `src/reporter/atlas_metrics.rs` (M-01…M-18 display contract), and
`src/reporter/atlas_render.rs` (deterministic static SVG + animated HTML canvas), each with unit
tests. Commits `fe58299` (view model), `f76a63b` (metrics), `3fa7c07` (canvas), `5e6b45e`/`6c0f0f3`
(review fixes). **No schema change, no `#map` wiring, no new JS/dependency, no holdout reads.**
- **Review panel:** R1 correctness, R2 contract, Q1 scope and Q2 evidence all **APPROVED** on
  `6c0f0f3`; zero unresolved blocking findings.
- **Residuals (recorded):** dynamic canvas height can exceed the 1440×900 canonical first
  viewport/print page-1/A1 frame for very deep paths (A2/vision judge); pausing during the initial
  1.2s reveal freezes the current frame; step-7 integration tests and step-8 vision gate remain.
- **Structural edges (D-pf-241):** ADR-0009 option A — LOW name-based + MEDIUM semantic, labelled
  structural, `N MEDIUM / M LOW` mix printed, never `resolved`/`reachable_from_entry`.
- **A8 amendment (D-pf-242):** first block = canvas carrying **entry + HOTTEST + path**; the one
  `path:line` action is in the first scroll (answer line + KV1).
- **A12 (D-pf-243):** exactly one fixed existing `REPORT_SCRIPT` (unchanged); Atlas canvas/SVG/
  animation add zero scripts, zero handlers, zero network; CSS keyframes/SMIL only.
- **A2 library holdout pre-registered (A-pf-116):** published `memchr` crate payload @
  `bd6068c30e9074a90c285e47912fa0b047d07597`; root crate has no bin/build.rs/workspace; dev
  `benchmarks/` + `fuzz/` excluded; vaulted until A2 (`docs/evidence/map-vision/v8/PREREGISTRATION.md`).

## Recent Decisions
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
- **Nothing active in the hot tree.** Steps 2–4 are green; the plan is still active.
- **`atlas-build` steps 5–10 remain queued:** brief/drills, `#map` wiring, integration tests/goldens,
  V8 vision gate, founder A2 (application + library), ship/EOL.

## Next
- **`/new` → atlas-build session 2: steps 5–6 only.**
  - Step 5: `src/reporter/atlas_brief.rs` — the fixed seven below-graph blocks + Q2 reverse drill,
    complete action list, profile queue, weight detail, per-component pages; use `AtlasView`/
    `atlas_metrics`; M-08 top P% must use `blast.dependent_top_pct`.
  - Step 6: wire `report.html#map` (canvas first, brief below, answer line in the first scroll; keep
    Explorer and the single fixed `REPORT_SCRIPT`); no new JS; no schema.
- **Read first:** `docs/plans/atlas-build.md`, `atlas-metrics.md`, `atlas-layout.md`, ADR-0009/0010,
  and the three committed modules.
- **Keep holdouts vaulted:** django/postgres + the `memchr` crate payload until A2; no scan/render/
  agent reads (dev `benchmarks/`/`fuzz/` excluded).
- **Agent report track** remains queued (A-pf-111).
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
