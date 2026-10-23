# Step 9 — founder decision record: Atlas v2 visual system + first-page composition

**Status: SIGNED 2026-10-23 — Option A; step 10 authorized.** The founder selected:

- **A — Port G's visual system** (warm paper/charcoal/burnt-orange; sans chrome + mono data; masthead).
- **Step 10 start:** yes.
- **First-page capture:** `report.html#map`.
- **Detail rail:** G-style right rail (implemented in step 11).
- **F treemap:** candidate; decide at step 12.

No overrides were recorded; the proposed contract below stands, including dropping the pastel module
bands while keeping one light hull. The machine-readable mirror is `decision.json`.

**Inputs:** `docs/atlas/AGENT-BRIEF.md`, `docs/evidence/atlas-audit/AUDIT.md`,
`docs/evidence/map-vision/v7/g/g-hyperfine.html` + `g-hyperfine-final.png`,
`docs/evidence/map-vision/v7/COMPARISON.md`, `docs/specs/atlas-layout.md`, `docs/plans/atlas-build.md`
(steps 9–10), and the fresh `@vision` read in `VISION-READ.md`.

## 1. Decision required

The founder agrees (a) the **visual system** and (b) the **first-page composition**, including the
detail rail, the below-graph chart set, and F's status. This is the gate that prevents another
eight-step drift.

## 2. Options

### Option A — Port G's visual system onto Atlas (recommended)

- Warm paper/charcoal/burnt-orange tokens; sans chrome + mono identifiers/figures; a named masthead.
- First page = masthead + small key-metrics strip + the main graph; below it, a few designed charts
  revealed progressively. G-style right rail returns as the detail layer (step 11).
- Keeps Atlas's genuine improvements: node-level top N%, numbered path badges, column counts,
  unreached column, eight-state coverage, units/anchors, answer-first line.
- **Why:** the founder's 2026-10-23 model explicitly says restore G's warm/sans/masthead visual
  language; the audit and the fresh vision read both identify the missing designed-page system as the
  regression. It fixes composition without discarding the model/data.
- **Cost:** a real renderer rework (tokens, type roles, masthead, strip, chrome), plus goldens and
  DOM assertions. Step 10 is the first half; rail/charts are steps 11–12.

### Option B — Keep Atlas's cool/slate system; fix composition only

- Retain white/slate/all-monospace; fix the fold, duplicate chrome, truncation, edge labels, and add
  the key-metrics strip.
- **Why it might win:** faster; less risk of disturbing the legibility work; the audit's own
  "alternative" recommendation.
- **Why it is not recommended:** the founder's instruction is to restore G's visual language; the
  audit's alternative is a fallback only if the founder dislikes G. It risks a second "looks like
  shit" round.

### Option C — Hybrid: G composition + a new palette/type system

- Keep G's masthead/rail/fit discipline but design a new warm-neutral palette rather than copying G.
- **Why it might win:** a chance to improve on G rather than revert.
- **Cost/risk:** needs a second design pass and a new founder checkpoint before step 10; slower, and
  it re-opens the exact drift the audit diagnosed ("re-implemented from a text spec").

**Recommendation: A.** It is the only option that satisfies the founder's explicit directive and the
audit's primary recommendation, and it preserves every Atlas improvement worth keeping.

## 3. Proposed visual contract (Option A)

### 3.1 Tokens

- Paper `#f4f1ea`; panel `#fbf9f4`; ink `#202b35`; entry `#1f2d3a`; muted `#6f6a61`;
  hairline `#d6d0c4`; hot/path `#c2410c`.
- Production `#3f6d6a` accent; test `#5c7391`; bench `#7a4f7d`; unreached `#b3aca1`; node fills as
  in G (`#fdfbf6` production, `#eef2f7` test, `#f7f0f7` bench, `#e8e5df` unreached).
- **One deliberate deviation from G:** back-edges are neutral `#8d8577` thin-dashed, not amber, so
  the burnt-orange path never collides with another edge type. Forward edges `#8d8577` solid at 0.34
  opacity; path `#c2410c` solid 2.3px.
- Drop the pastel module **bands**; keep one light module **hull** outline. Depth columns stay.

### 3.2 Typography

- `ui-sans-serif` stack for chrome, labels, legend, masthead; `ui-monospace` only for paths,
  identifiers and figures.
- Wordmark 21px/800; kicker 8px/800 with `.16em`; entry path 12.5px/750 mono; strip 9–10.5px;
  column headers 10px/800; node name 10–11.5px/700 mono; node meta ≥8.5px; footer 8.5–9px.
- **Floor:** no new text below 8.5px; node names stay ≥10px. Hierarchy comes from weight, color and
  spacing, not from shrinking text.

### 3.3 First-page composition (end-state, after step 11)

```
┌ 1440×900 ───────────────────────────────────────────────────────────────┐
│ MASTHEAD 58px: PERFPAL · ATLAS · <repo>   ENTRY · <policy> · <path>      │
│                confidence · alternatives        [Pause] [Explorer →]    │
├ KEY-METRICS STRIP ≤56px ────────────────────────────────────────────────┤
│ ENTRY · HOTTEST · PATH · CAPS · 3 START actions (path:line + rule)       │
│ one caveat: structural (LOW name-based + MEDIUM semantic) · legend swatches│
├ GRAPH STAGE ~720–745px ──────────────────────────────────────────────────┤
│ depth columns · file nodes · forward/back edges · light hulls · path     │
│ unreached column · numbered path badges · top N% · no panels/tables      │
├ FOOTER 24px ────────────────────────────────────────────────────────────┤
│ reachability · file pairs · MEDIUM/LOW mix · unreached production/non-   │
│ production (one label) · parse · one unit definition                    │
└─────────────────────────────────────────────────────────────────────────┘
```

- Everything (masthead, strip, graph, legend, footer) ends **≤900px** at 1440×900.
- Step 10 renders this **full-width** (no rail yet); step 11 restores the G-style **right rail**
  (≈868px map + ≈538px rail, 2-column file cards: role, cost, top functions, finding ID). The founder
  signs the end-state composition now; step 11 implements the rail.
- At <1360px the rail stacks below the graph or becomes an overlay; **no horizontal scroll**.
- Canonical capture: `report.html#map` (the global report header sits above the anchor and is not
  part of the 900px Atlas first page). If the founder wants the global header counted, the graph
  stage must lose another ~150px; state that explicitly.

### 3.4 Chrome hygiene

- **One** unit definition, **one** structural caveat, **one** legend, **one** footer.
- No duplicate caveat/median lines; no clipped footer; no leading-dot truncation (mid-ellipsis + full
  path in `title`/rail); edge labels only on the path/selected edges, full counts in `<title>`.
- Unreached is one unambiguous label: `unreached production: N · non-production: M`.
- Canvas stays free of tables, cards, bar lists, selectors and a second map (A9).

### 3.5 Below-graph set (step 12; founder chooses F there)

1. **Weight** — production-first share; designed visual, not a text table. **F treemap is a
   candidate** here; prototype both and decide with the founder at step 12.
2. **Blast radius** — dependent files, top dependents, include coverage.
3. **File detail** — top files with functions + finding IDs (the rail is the first-page detail layer).
- Progressive disclosure; Q2/weight drills move to separate pages; `#map` ≤~5,000px; no 3,000px
  brief; no metric wall.
- F data is **not** deleted. F is undecided, not rejected. No text-only strip may masquerade as F.

### 3.6 Keep from Atlas

Node-level top N%, numbered path badges, column counts, unreached column, eight-state coverage,
units/anchors, answer-first line, legible node typography. Light hulls replace pastel module bands.

## 4. Step 10 acceptance (runnable, before the founder visual check)

At 1440×900, reduced motion, `report.html#map`:

1. `#atlas-canvas` bottom **≤900** (includes masthead, strip, legend, footer).
2. `document.scrollingElement.scrollWidth <= 1440` (no horizontal scroll).
3. `#atlas-brief` top ≥ `#atlas-canvas` bottom (canvas first).
4. Exactly one `.atlas-legend`, one `.atlas-footer`, one unit definition, one structural caveat;
   no duplicate median/caveat strings.
5. No visible `..`-leading path truncation; every truncated path has a full-path `title`.
6. Canvas has no table/bar/card list/metric selector (A9); exactly one `REPORT_SCRIPT` (A12);
   double render byte-identical (A4).
7. Page height does not increase; `#map` height recorded (target cut is step 12).
8. `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`,
   `cargo test --locked`, `cargo deny check licenses` all exit 0; goldens regenerated.
9. **Founder visual check** on the 1440×900 render — the step-9 sign-off is the contract, not the
   final gate (step 13 is the final visual gate).

## 4b. Implementation notes (2026-10-23, post-sign-off)

- **Masthead wordmark is `Atlas`, not `<repo>`.** The HTML report deliberately omits
  `scan.root_path` (no provenance surface); adding the repo name needs a separate provenance
  decision. Carried to the step-13 founder check.
- **Entry path renders at 10px/600 mono** inside the key-metrics strip (the sketch said 12.5px/750)
  to keep the strip at 53px, ≤56px.
- **Full-bleed paper band** with `1rem` inner padding and a **1,408px** canvas content width; the
  graph is 1,406px, so the rendered SVG text floors are preserved.
- **Fit evidence is hyperfine/cJSON at 1440×900.** Capped/deep states exceed 900px by design
  (V8-disclosed, deferred to the step-13 gate/A2); do not claim state-unconditional 900px fit.
- **Deferred polish:** the bottom-right void and the inline ENTRY run-on are step-11 (rail) / step-13
  items; the 3,000px brief cut is step 12.
- **Chrome floors:** chip and edge-pill text are 8.5px; hull labels, kicker and column-header labels
  use `#6f6a61`/`#202b35` for contrast.

## 5. Weakest assumption / alternative

**Weakest assumption:** the founder's "amazing" reaction was to G's palette/type/masthead, not only to
its fit and rail. If it was only fit/composition, Option B would suffice.

**Alternative:** Option B (keep Atlas's cool system, fix composition). It is cheaper and lower-risk
to the existing legibility work, but it contradicts the explicit "restore G's visual language"
directive and the audit's primary recommendation.

## 6. Founder sign-off

Reply with the option letter and any overrides. Suggested reply:

```
A. Visual system: G port (warm paper/charcoal/burnt-orange; sans chrome + mono data; masthead).
B. First page: masthead + small key-metrics strip + main graph, canvas bottom ≤900;
   detail rail in step 11; below-graph charts in step 12; F treemap candidate.
C. Overrides: <none / list>
Signed: <name> · <date>
```

- [x] **Option A** — port G (recommended)
- [ ] **Option B** — keep Atlas cool/slate; fix composition only
- [ ] **Option C** — hybrid; needs a second design pass before step 10
- [x] Detail rail: G-style right rail (recommended) / below-chart / overlay
- [x] First-page capture: `#map` anchor (recommended) / include the global report header
- [x] F: keep as a step-12 candidate (recommended) / on-page treemap / Explorer-only
- [x] Drop pastel module bands, keep hulls (recommended) / keep bands

**Founder:** founder (recorded via the step-9 decision prompt)  **Date:** 2026-10-23
**Notes:** Option A; start step 10; `#map` capture; G-style right rail; F candidate; no overrides.

**Step-10 founder visual check:** **APPROVED 2026-10-24** — the founder approved the step-10 first
page (G visual system + composition) and authorized the commit.
**Step-13 final gate:** **PENDING** — detail rail (step 11), progressive charts/F candidate
(step 12), founder final composition/V8 gate (step 13).
