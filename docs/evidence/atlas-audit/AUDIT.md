# Atlas visual regression audit — 2026-10-23

**Verdict: yes, we went wrong — not in the model or the data, in the visual layer and the gate.**
The G prototype was a *designed page*; Atlas ships an *instrument panel*. The V8 gate passed because
it measures comprehension ("can a reader extract an answer?"), not composition ("does this look
like the thing the founder approved?"). A2 is not ready to run.

## Evidence

- **Real render:** current release binary scanned hyperfine + cJSON into
  `perfpal-test/out-atlas-{hyperfine,cJSON}`; `report.html#map` captured at 1440×900, reduced motion:
  `docs/evidence/atlas-audit/atlas-{hyperfine,cJSON}-final.png` (first viewport) and
  `-full.png` (full page). DOM/geometry: `audit.json`, reproduced by `audit_render.py`.
- **Comparison:** a fresh comparative `@vision` read of the G hyperfine final frame vs the real Atlas
  hyperfine first viewport (two images, no labels, no source).
- **Data:** metrics are identical to the V7 design scans (53 files · 353 functions · 348 findings ·
  342 reachable). The apparent 21-vs-11 unreached discrepancy is a **label ambiguity**, not a data
  change (see F9).
- **Timeline:** V7 `60dac49` (G/F prototypes, 12/12 reads) → Atlas research/specs → steps 1–8
  (`fe58299`…`996fb35`) → V8 16/16 A1 + 16/16 strip → founder: *"looks like shit."* Eight
  implementation steps accumulated without a founder visual checkpoint.

## Findings

### F1 — The first screen clips the canvas (objective defect)

- `#atlas-canvas` is **980px tall** (bottom **981px**); the fold is 900px. The bottom 81px —
  legend/footer — is cut, and the answer line/brief starts at **981px**, below the fold.
- G: a fixed **1440×900** artboard (`body{width:1440px;height:900px;overflow:hidden}`), everything
  fits.
- The V8 preflight checked "the key line/first row above the 900px fold"; it did not assert the
  **canvas bottom** or the first answer block. Its own NOTES record "capped's lower depth-1 rows are
  below the 1440×900 first screen" — the gate knew about below-fold content but not the composition.
- This alone makes the page feel unfinished: a clipped diagram with no answer line.

### F2 — The visual language was replaced, not ported

| | G (approved) | Atlas (shipped) |
|---|---|---|
| palette | warm paper `#fbf9f4`, charcoal `#1f2d3a`, burnt orange `#c2410c`, muted browns | cool white `#ffffff`, slate `#0f172a`/`#334155`, `#cbd5e1`, amber `#b45309` |
| type | `ui-sans-serif` chrome + mono identifiers/figures | `ui-monospace` everywhere (body is `monospace`) |
| masthead | wordmark + tagline + entry breadcrumb + two aligned buttons | no masthead; one orphan `Pause` button + six raw mono lines |
| hierarchy | one stats line, one legend row, one footer | six header lines, units/keys/legend/footer, repeated definitions |
| detail | right rail of 19 file cards (role, cost, top functions, finding ID) | full-width canvas; detail moved below; nodes carry only `INFO ×N`/`WARN ×N` |

The founder's "amazing" was largely this system. The spec described content and acceptance, not the
visual system, so the implementation re-invented it.

### F3 — The right rail was removed; the map lost its action layer

- G: the rail lets a maintainer decide **which function in which file** without leaving the image
  (role, cost, top functions, `PERF-RS-ARCH-003`-style ID).
- Atlas: full-width canvas; the cards live in the below-graph seven-block brief. The canvas answers
  "`src/command.rs` is hot"; it does not answer "which function, which finding?" The vision read:
  *"B tells me `src/command.rs` is hot and expensive — I already suspected that — and then stops."*
- This is the "100% graph" rule over-applied: the rail was **detail**, not a metric panel. Removing
  it bought width and lost the information hierarchy.

### F4 — The page became a report, not a map

- First viewport 1440×900; **full page 22,942px** (hyperfine) / **34,831px** (cJSON).
- `#map` alone is **8,982px** / **14,685px**: canvas 980 + brief **3,367/3,219** + inline
  `#atlas-drill-q2` and `#atlas-drill-weight` sections. Then Explorer/components/files/findings/
  appendix.
- G was one 900px artboard. The founder asked for the graph page to be the graph; the default
  experience is a long scroll with the graph at the top.

### F5 — Chrome hygiene broke

- The structural caveat (`LOW name-based + MEDIUM semantic`) appears **6×**; the
  `× median function = all functions in the scan` definition **2×**; `static cost units (v2)`
  **121×**. The comparative read saw the same caveat twice verbatim in the header.
- The footer legend is **physically clipped** at the 900px frame edge (F1).
- Six monospace header lines with no visual grouping; the colour/size hierarchy that made G scannable
  is gone.

### F6 — Truncation destroys path identity

- G: mid-ellipsis (`src/benchma…cheduler.rs`) and the full path in the rail.
- Atlas: leading-dot truncation (`..tity/statistics.rs`, `..hmark/scheduler.rs`,
  `..ut/progress_bar.rs`, `__ress_bars`) — the reader cannot tell `src/` from `tests/` from `crates/`,
  and there is no rail to recover the full path.

### F7 — Edge/path hairball

- Atlas: **49 edge labels** on 49 edges; amber back-edges and orange path dashes in the same hue
  family; labels land on node borders in the depth 1–3 corridor; the path fights the noise.
- G: thin recessive back-edges, one solid high-contrast red route, one shaded reachable hull.
- Atlas's node-level `top N%` and numbered path badges are **improvements** worth keeping; the
  edge-label density and hue collision are not.

### F8 — Two background systems fight

- Atlas: depth columns **plus** pastel module bands **plus** a first-class unreached column.
- G: depth columns **plus** one reachable hull.
- More information, less figure/ground: direction is harder to read because nodes stack vertically
  inside columns and the module bands cut across the depth reading.

### F9 — Unreached labeling is ambiguous (not a data bug)

- Footer: `21 unreached production · 11 unreached non-production`; the column header reads
  `UNREACHED` and a reader reports "11 files." The comparative read inferred a 21-vs-11 data
  disagreement; the metrics are identical and the footer is correct.
- Fix the label (e.g. `unreached production: 21 · non-production: 11`), not the data.

### F10 — F's visual is absent from the page

- The F treemap is Explorer-only; the Atlas page's "weight strip" is text rows.
- The founder loved F's look and explicitly said "don't throw F away." "Not discarded" is true in
  the Explorer, but the Atlas page has no F visual; the page's visual identity is G-only.

### F11 — Small text everywhere

- **132** canvas text elements under 10px (hyperfine); **38** (cJSON). The V8 floors are 7.9–9.9px.
- More labels + more chrome = more small text; G had small labels too, but the composition carried
  the hierarchy.

### F12 — The V8 gate's blind spot (process finding)

- V8: **16/16 A1 reads + 16/16 strip reads pass** — all true. A1 asks "which question + one
  concrete action"; it does not judge palette, typography, hierarchy, clipping, duplicate chrome,
  page height, or the founder's taste. The strip reads are on a **crop** of the weight block.
- The gate compared Atlas states to each other, never to **G**. There was no founder visual veto
  before the implementation locked.
- Comprehension passed; composition failed. The gate is necessary but not sufficient.

## Root causes

1. **The visual system was re-implemented from a text spec.** The specs encoded content, states and
   acceptance; the G palette/typography/layout lived only in the prototype. The build had no
   "match G" constraint, so it invented a different look.
2. **"100% graph" was over-applied.** The rail was removed; detail moved below; the canvas became a
   full-width hairball and the page a text wall. The founder's intent was "no metric panels on the
   canvas", not "remove the detail that makes the map usable."
3. **Feature accumulation without a composition budget.** Seven-block brief + canvas header/units/
   keys/legend/footer + module bands + edge pills + caps chip + inline drills. Each was locally
   justified; nobody judged the whole page.
4. **The gate substituted for the founder's eye.** A1 reads test comprehension; the founder tests
   design and trust. A2 was deferred to the very end, so the visual drift accumulated over 8 steps.
5. **Legibility fixes optimized text size and collisions, not hierarchy.** 90rem body, text floors,
   four-line nodes, 49 edge labels: more legible per element, denser and more instrument-like as a
   page.
6. **F's visual was demoted to a strip.** The treemap moved to Explorer; the "not discarded" promise
   is technically met, visually absent.

## What to do (recommended; no implementation yet)

1. **Freeze Atlas steps 9–10 / A2.** Do not run the founder timed test on this page; it would test a
   regression.
2. **Make G the visual baseline, not just the content baseline.** Put G and Atlas side by side and
   write a one-page visual contract:
   - first screen: canvas **fully visible** at 1440×900 (canvas bottom ≤ 900 including header,
     legend, footer); no clipped text; answer line in the first scroll.
   - one masthead/wordmark; one stats line; one legend; one footer; no duplicate definitions.
   - palette/typography: restore G's warm/cream/charcoal/burnt-orange + sans-for-chrome/mono-for-data,
     or get explicit founder approval for a new system.
   - detail: restore a right-rail file-card layer (or an equivalent in-canvas detail layer) with
     role, cost, top functions and finding IDs. The rail is detail, not a metric panel; decide with
     the founder how it coexists with "100% graph."
   - path: one solid high-contrast route; back-edges thin/recessive; no amber/orange collision;
     edge labels only on the path/selected edges (full counts in `<title>`).
   - truncation: mid-ellipsis + full path in title/rail; never leading-dot.
   - page budget: `#map` ≤ ~4–5k px; drills on separate pages; brief concise; no 22k–34k page.
   - F: show the treemap vs strip to the founder and decide; if the strip stays, design it as a
     visual, not a text table.
3. **Fix the objective defects first:** canvas fit, duplicate chrome, clipped footer, truncation,
   edge labels/path contrast, unreached label, page height.
4. **Add a founder visual gate + a G-baseline diff to the V8 gate.** The founder must say "this looks
   good" on a 1440×900 render before A2. A1 reads remain; a visual regression against G blocks.
5. **Re-run the gate after fixes; then A2.** No new feature blocks until the composition is
   approved.
6. **Process:** one change at a time; render; founder check. The gate should assert composition
   (fold fit, no clipping, no duplicate definitions, page-height budget), not only comprehension.

## Weakest assumption / alternative

**Weakest assumption:** that restoring G's rail and palette is the right fix. The founder may prefer
a **new** visual system that keeps Atlas's genuine improvements (node-level `top N%`, numbered path
badges, module bands, unreached column) and G's composition discipline. The audit recommends a
founder side-by-side decision, not a unilateral revert. **Alternative:** keep the Atlas graph but
fix the composition — fit the fold, restore a detail rail/overlay, restore a warm palette and
sans/mono hierarchy, reduce edge labels, and cut the page height. That is likely the best of both.

## Reproduce

```bash
# current release binary already in target/release
./target/release/perfpal perfpal-test/hyperfine -o perfpal-test/out-atlas-hyperfine
./target/release/perfpal perfpal-test/cJSON -o perfpal-test/out-atlas-cJSON
python3 docs/evidence/atlas-audit/audit_render.py
```

Evidence: `atlas-{hyperfine,cJSON}-{final,full}.png`, `audit.json`. Comparative vision read:
the G hyperfine final vs the Atlas hyperfine first viewport (2026-10-23).
