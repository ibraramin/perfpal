# Map External Review — 2026-10-22

**Status:** input, not a decision. Not a spec, not an ADR, not an increment.
**Author:** external review session (Temp-Research/Code-Visualization), at founder request.
**Consumers:** `map-intelligibility` (V6 / V3 re-open), `maintainer-map`, ADR-0008.

## 0. How to use this file

Read this as evidence for the V6 gate, alongside `docs/evidence/map-vision/` and
`docs/evidence/map-idiom-ab.md`. It makes no code changes, no schema changes, and no visual
claims beyond the shipped screenshots. Everything below is either (a) a file:line observation
from this repo, or (b) a `@vision` read of the committed PNGs — both cited inline.

**Founder signal (unconfirmed as formal V6):** "very convoluted, not easy to understand at all."
If ratified, this is a V6 FAIL. Per `map-intelligibility.md:17` the stop criterion is *two distinct
V3 idioms failing → ADR-0008 amendment, not more polishing*. This review argues the current
problem is not a fifth idiom inside the three-slot design; it is that two designs ship at once.

## 1. Method and limits

- Code read: `src/reporter/views.rs` (KV1/KV2/KV3), `src/reporter/mod.rs` (HTML shell + nav),
  `src/charts/{mod,layers,component}.rs`, `src/graph.rs`, `src/aggregator/`, `tests/key_views.rs`.
- Screenshots read: `docs/evidence/map-vision/v5/{cJSON,hyperfine}-kv{1,2,3}.png` and
  `docs/evidence/map-vision/v6/kv2-amendment/{01,02,03}.png`.
- Not done: I did not run `perfpal` on a repo. Element counts below are from the committed
  renders, not a fresh scan. No accessibility or print test was run.

## 2. Findings

### F1 — The "map" is three dashboards, not one object

- KV1, KV2 and KV3 are HTML/CSS/Markdown (`src/reporter/views.rs:497-507`, `:529-637`,
  `:753-782`). Only the Explorer tier emits SVG. The three key views share a visual costume
  (header question + ranked rows/bars + footer stats) but visualise **three different entities**:
  findings, files-by-dependent-count, functions-by-cost.
- Consequence: there is no object to point at. A reader cannot move from "this file is #1" to
  "here is where it sits and what it connects to" without leaving the view.
- Vision read: dominant idiom is a repeated horizontal bar-chart dashboard; scores KV1 4/5,
  KV2 3/5, KV3 3/5.

### F2 — KV2 cannot answer its own question

- "What breaks if I touch it" is a **graph** property. KV2 renders `N dependents` as a bar
  (`views.rs:529-637`); the viewer learns *how many*, never *which*.
- The data needed is already present — `graphs.include_graph.edges` is consumed for the count
  and the same edges are drawn in `charts/component.rs` and `charts/mod.rs`.
- This is the strongest evidence that the key-view tier and the explorer tier are two products:
  the answer exists in one tier and is reduced to a scalar in the other.

### F3 — Colour is triple-booked, and the palette is doing work the layout should do

- 24 distinct hex values across generated SVGs. Red is max heat **and** `hot` severity **and**
  error borders; three near-identical blues cover info severity, node structure, bar fill and
  heading text.
- `docs/specs/maintainer-map.md:65` already requires one primary encoding per key view. Colour
  currently violates the spirit of that rule inside the charts.

### F4 — The Explorer tier is heavier than the product it supports

- cJSON render: **29 SVGs, 2,707 primitives, ~3,080 elements**; hyperfine: 20 SVGs, 2,013
  primitives (element counts from the committed `charts/` output).
- Largest renderer: `render_layer_entry` 461 lines (`src/charts/layers.rs:563-1023`);
  `render_layer_bands` 335 (`:1043-1377`). The edge loop is repeated at
  `charts/mod.rs:724-741`, `layers.rs:255-272,466-483,955-972,1291-1308,1524-1541`,
  `component.rs:285-302`; the node loop and caption emission are duplicated in the same pattern.
- Structural cause: ADR-0008 chose DSM (L0), Sugiyama DAG (L1), treemap (L2) — and those were
  built as **Explorer stems**. `map-intelligibility` V4 then added the three answer-first panels
  on top. Both tiers now ship, and they do not share an object model.

### F5 — KV1 is the part that works; keep it

- KV1 is text-first ranked actions with `path:line`, function, severity, confidence and a prose
  "why" (`views.rs:47-64`, `:396-414`). Vision: 4/5, clear start point, explicit ordinals.
- It won the V3 idiom A/B (`maintainer-map.md:96-98`). It should survive any redesign, as the
  brief or as the annotation strip over a spatial view.

## 3. Prior art — what to steal, what is already correctly rejected

Perfpal's own R1–R4 already rejected the wrong answers for the right reasons (3D code cities,
force layouts, Voronoi, history animation, flame graphs: ADR-0008:57-83). The gap is adoption,
not research. Only these affordances are still unbuilt:

| Source | Steal | Do not steal | Constraint that allows it |
|---|---|---|---|
| Structure101 / LSM | collapse cycles → level → drill cell-to-cell; the closest published answer to Q2 | package-only granularity | integer layout, static SVG — compatible |
| Lattix / DSM | matrix as blast radius; reordering exposes clusters and cycles | enterprise interaction model | already ADR-0008 L0; exists as `component-coupling.svg` |
| CodeCity (Wettel & Lanza) | one spatial object; area = size, one metric = colour, containment = districts | 3D, occlusion, viewpoint choice | 2D treemap satisfies it without the 3D cost |
| CodeCharta | metric selector; one legend, one meaning per colour | 3D city, heavy analysis pipeline | the encoding discipline is free |
| Sourcetrail | never draw the whole graph: focus + 1-hop neighbourhood + search | Qt desktop, Clang-weight indexer | becomes "selected file + dependents list", static |
| CodeCompass | clickable static SVG + info tree; web scale at C/C++/Java | build complexity, C-centric index | static SVG anchors already used in `report.html` |

**Correction to the obvious recommendation.** Vision recommends DSM for KV2. At file level a DSM
is quadratic in files; Perfpal caps nodes at 50 (`charts/mod.rs:71-76`) and components at 50
(`:80`), and the existing matrix is component-level (734×682 on cJSON). Keep DSM at **component
level** and drill to per-component file DSM. That is ADR-0008's L0→L2 split, not a new idiom.

## 4. Decision options

Each option is a different answer to "what is the one object on screen". All are static,
zero-JS, integer-layout, byte-deterministic.

### Option A — Treemap-first (one spatial object)

Repo as a 2D treemap: area = static cost, colour = one selected metric, hottest file outlined,
`<a href="#...">` links into the findings table. KV1 brief above; component DSM below for Q2;
Explorer demoted to on-demand anchors.

- For: finally shows "the whole codebase" as one object; area is pre-attentive; containment is
  honest; reuses the treemap already chosen for L2.
- Against: a treemap answers "how big / how hot", weakly "what connects"; needs a new renderer;
  retires or demotes 2 of 3 key views.
- Risk: violates `maintainer-map.md:55-57` ("one question, one primary encoding") if the treemap
  is asked to serve Q1/Q2/Q3 simultaneously. Requires a spec amendment or a strict
  one-metric-per-render rule.

### Option B — DSM-first (matrix as the hero)

Component DSM on the first screen with cluster ordering; KV1 rides as an annotation strip;
per-component file DSM on drill.

- For: best possible Q2; reuses `component-coupling.svg` and its tests; matches ADR-0008 L0;
  cheapest credible spatial upgrade.
- Against: a matrix does not answer "where do I start"; weak for Q3; needs row/column ordering
  work (cluster ordering) to be legible; not a "picture of the codebase" to a newcomer.

### Option C — Brief-first (no new idiom)

Keep KV1 as hero; make KV2/KV3 supporting evidence rather than peers; delete Explorer stems that
restate them; apply the P0 fixes in §5.

- For: cheapest; zero ADR/spec churn; consistent with the locked three-slot design; KV1 already
  scores 4/5.
- Against: does not deliver a spatial whole-codebase view — it optimises the list, not the map.
  If the founder's complaint is literally "I can't see the codebase", C does not answer it.

**Weakest assumption in this review:** that one spatial object is the right target. The locked
spec says three slots, one question each; A and B both strain that. **What would change my mind:**
if the V6 timed test passes on django/postgres with the current design, Option C + §5 defects is
correct and A/B are scope creep, not improvement.

## 5. P0 defects independent of the option chosen

1. **KV3 number formatting.** `2000008 static cost units` and `1000004× median` appear
   unformatted in v5 (vision). Format to `2.0M` / clamp the multiplier; no 7-digit integers.
2. **KV2 axis and clipping.** No scale anchor (`0 ──── N dependents`); v6/02 clips `2.5×` at the
   frame edge; 1-dependent bars are visually indistinguishable from zero.
3. **KV2 demotion is colour-only.** Add a text marker; `maintainer-map.md:84` already requires
   demotions to be printed, not implied.
4. **KV1 badge stack.** Four pills × five rows repeats the same information five times; collapse
   to one severity chip plus a single legend.
5. **Palette.** Red = severity only; heat = one sequential ramp; remove the three near-identical
   blues.

Every one of these is a `@vision`-gated change per `AGENTS.md:27-30` — re-read the affected PNG
before claiming done, and commit the transcript as evidence.

## 6. Roadmap consequence — fix before JS/TS

The aggregator carries three parallel per-language families:

- `detect_{c,rust,python}_entries` — `src/aggregator/mod.rs:628,822,995`
- `resolved_{c,rust,python}_edges` — `:701,909,1084`
- `{c,rust,python}_reachable` — `:778,964,1146`

JS and TS are separate milestones (`docs/roadmap.md:27-29`). As written, that milestone adds six
more near-identical functions plus schema paths. Collapse entry detection and edge resolution
behind one per-language trait/table **before** the JS/TS burst, or the "convoluted" cost
compounds exactly when the language count grows. This is independent of the map-idiom decision.

## 7. Open questions for the founder

1. Is §2's founder signal a formal **V6 FAIL**, or does the django/postgres timed test run first
   as written in `map-intelligibility.md:17`?
2. Which object should be on the first screen: **treemap (A)**, **DSM (B)**, or **brief (C)**?
3. If A or B: does `maintainer-map.md:55-57` get amended (one object serving three questions), or
   does each render stay single-metric with the brief as the only multi-question surface?
4. Is file-level DSM out of scope permanently (quadratic), or acceptable behind the 50-node cap?

## 8. Suggested next action

1. Record the founder signal in `state/session-state.json` + `active_context` (owner: Perfpal
   orchestrator — not this file).
2. If A or B: prototype the first screen as static SVG on cJSON + hyperfine (~1 session, no core
   changes), then run the same blind `@vision` A/B used in V3 (`docs/evidence/map-idiom-ab.md`).
3. If C: apply §5 and re-run the V6 timed test unchanged.
4. Either way: do §6 before the JS/TS milestone.
