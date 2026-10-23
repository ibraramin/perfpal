# Active Context

## Current Task
**Atlas steps 7–8 are green, but the visual regression audit (2026-10-23) found the shipped page is
not A2-ready. A2/ship (now steps 14–15) are BLOCKED (R-pf-109); the visual rework is steps 9–13.**
- **Verdict:** the model/data are fine; the visual layer and the gate went wrong. G was a designed
  page; Atlas is an instrument panel. V8's 16/16 A1 + 16/16 strip reads passed because they measure
  comprehension, not composition.
- **Objective defects:** first screen clips the canvas (980px > 900 fold; answer line at 981); page
  22,942/34,831px (#map 8,982/14,685); brief 3,367/3,219; duplicate chrome (structural caveat 6×,
  median def 2×); leading-dot truncation; 49 edge labels with a path/back-edge hue collision; F
  treemap Explorer-only.
- **Visual drift:** G's warm cream/charcoal/burnt-orange + sans chrome + masthead + right-rail file
  cards became Atlas's white/slate all-monospace + six-line header + no rail.
- **Evidence:** `docs/evidence/atlas-audit/AUDIT.md`, `audit.json`,
  `atlas-{hyperfine,cJSON}-{final,full}.png`; comparative vision read (G vs Atlas) recorded in the
  audit.
- **New agents:** read **`docs/atlas/AGENT-BRIEF.md`** first — the issues, the founder's 2026-10-23
  model (first page = main graph + key metrics; more key charts below; restore G's visual language;
  F undecided), and the non-negotiable rules.
- **No code changes in this audit; holdouts vaulted; `map-build` NOT done; A1b open.**

## Recent Decisions
- **D-pf-247 (live, 2026-10-23):** plan reordered — steps 9–13 = v2 visual rework
  (G baseline/composition, first-page fit + G visual language, detail rail, progressive charts,
  founder visual gate + V8 rerun); A2/ship = 14–15, blocked. Easy-to-understand composition;
  main graph + small key-metrics strip; no 3,000px brief; F treemap candidate; no schema.
- **R-pf-109 (new, high):** Atlas visual regression; A2/ship (steps 14–15) blocked.
- **D-pf-246 (live):** Atlas steps 7–8 green (V8 16/16 + 16/16); renderer legibility fixes.
- **D-pf-245 (live):** Atlas steps 5–6 green (brief/drills + #map wiring).
- **D-pf-244 (live):** Atlas steps 2–4 green (view model/metrics/canvas).
- **D-pf-238 (live):** Perfpal Atlas; G core, F components supporting (not fallback); graph page
  100% graph; metrics below.

## In Progress
- **Blocked:** A2/ship. Do not run the founder timed test on the current page.
- **Next work:** visual rework against the G baseline + a founder visual gate; then re-run the V8
  gate; then A2.
- **Decision needed:** restore G's right rail/palette (the audit's recommendation) or keep the
  full-width canvas and fix the composition; F treemap on-page vs weight strip. Founder side-by-side.

## Next
- **`/new` → Atlas v2 visual rework Session 4: plan steps 9–10 only.**
  Read `docs/atlas/AGENT-BRIEF.md` → `docs/evidence/atlas-audit/AUDIT.md` → V7 G
  (`docs/evidence/map-vision/v7/g/g-hyperfine-final.png`, `g-hyperfine.html`) → `atlas-layout.md`.
  - **Step 9 (design first):** side-by-side G vs Atlas; the founder agrees the visual system (warm
    paper/charcoal/burnt-orange, sans chrome + mono data, masthead, detail rail) and the first-page
    composition (main graph + a **small** key-metrics strip; no full brief). Record the decision +
    G-baseline diff under `docs/evidence/atlas-visual/v2/`.
  - **Step 10:** implement first-page fit (`#atlas-canvas` bottom ≤ 900 incl. header/legend/footer),
    G palette/type/masthead, one definition each, the key-metrics strip, and the chrome fixes.
  - **Guiding rule:** easy to understand beats exhaustive metrics. Show the graph + a few related,
    plain-language numbers/charts; no metric wall, no 3,000px brief.
  - Do **not** start steps 11–12 (detail rail/charts), A2 (14) or ship (15).
- **Then Session 5:** steps 11–13 (detail rail, progressive charts, founder visual gate + V8 rerun).
- **Then Session 6:** steps 14–15 (A2 + ship).
- **Keep holdouts vaulted; no `map-build` done; A1b open.**

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
