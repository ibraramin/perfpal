# Atlas — new-agent brief (read this first)

> **2026-10-26 status (authoritative):** main closed Atlas v2 steps 11–15 (`D-pf-258`); step 13
> approved (`D-pf-253`); A2 delegated pass (`D-pf-255/257`); `R-pf-109` closed as a record. This
> brief is the 2026-10-23 audit snapshot. On `feat/atlas-sim`, **do not re-run steps 11–15**; the
> branch reconciliation is `R-pf-112` / `docs/adr/ADR-0012-atlas-branch-reconciliation.md`.

**Status (at the 2026-10-23 audit):** Atlas steps 1–8 implemented; **A2/steps 9–10 BLOCKED** by the
2026-10-23 visual regression audit (`R-pf-109`). Do **not** run A2, ship, or mark the plan done.
**Owner:** orchestrator. **Audience:** every agent touching `src/reporter/atlas_*`, the Atlas specs,
the V8 evidence, or the Atlas plan.

## 1. What Atlas is

**Perfpal Atlas** is the shipped human map. **G (entry-flow) is the core graph.** F (weight
map/treemap) is **not a fallback and not discarded** — F's weight/treemap mass, production-first area
and component cells are candidates for the "more key charts" below the graph, decided slowly with
the founder. No mode switch.

**Founder model (2026-10-23, verbatim):**
> "I want the main graph in the first page, with some key metrics, then we will go down to show some
> more key charts slowly."

This **updates** the earlier "graph page is 100% graph" wording:

- The **canvas** still contains no metric panels, tables, bar lists, selectors, or a second map.
- The **first page** may carry a small **key-metrics** strip beside/below the graph (entry, hottest
  file, a few actions, coverage/caveat) — not a full brief, not a dashboard.
- **Below**, reveal **more key charts progressively** (weight, blast radius, file detail, an F
  treemap candidate). Each is a designed chart, not a text table. No 3,000px text wall.

**F (2026-10-23, verbatim):** "I don't know, we will slowly figure it out." → keep the treemap and
F components as candidates; prototype options and show the founder. Do not ship a text-only weight
strip as "F" without a founder check.

## 2. What went wrong (2026-10-23 audit)

Full evidence: `docs/evidence/atlas-audit/AUDIT.md`, `audit.json`,
`atlas-{hyperfine,cJSON}-{final,full}.png`; comparative vision read of G vs Atlas.

1. **First screen clips the canvas.** `#atlas-canvas` is 980px tall (bottom 981) at a 900px fold;
   legend/footer cut; the answer line/brief starts at 981. G fit 1440×900.
2. **G's visual language was replaced, not ported.** G: warm cream `#fbf9f4`, charcoal `#1f2d3a`,
   burnt orange `#c2410c`, `ui-sans-serif` chrome + mono data, a masthead, a right rail of file
   cards. Atlas: white/slate, `ui-monospace` everywhere, six raw header lines, no masthead, no rail.
3. **The page became a report.** 22,942px (hyperfine) / 34,831px (cJSON); `#map` 8,982/14,685; brief
   3,367/3,219; Q2/weight drills inline.
4. **Detail rail lost.** G's rail answered "which function in which file" without leaving the image;
   Atlas nodes show only `INFO ×N`/`WARN ×N`, with the detail buried below.
5. **Chrome hygiene.** Structural caveat 6×, median definition 2×, footer legend clipped, leading-dot
   truncation (`..tity/statistics.rs`), 49 edge labels with a path/back-edge hue collision, unreached
   label ambiguity (footer 21 production + 11 non-production; column reads 11).
6. **F's visual absent.** The treemap is Explorer-only; the page's weight strip is text.
7. **The gate's blind spot.** V8 passed **16/16 A1 + 16/16 strip** reads — but A1 tests
   comprehension ("which question + one action"), not composition. No founder visual gate, no G
   baseline, no canvas-bottom/clipping/duplication/page-height assertions.

**Root cause:** the visual system was re-implemented from a text spec; "100% graph" was over-applied
(the rail removed); features accumulated without a composition budget; and the gate substituted for
the founder's eye. The data/model are fine.

## 3. Non-negotiable rules for new agents

1. **Read the G prototype before touching Atlas:** `docs/evidence/map-vision/v7/g/g-hyperfine.html`
   and `g-hyperfine-final.png`. It is the visual baseline, not just "content evidence."
2. **Restore G's visual language.** Warm paper/charcoal/burnt-orange tokens; `ui-sans-serif` for
   chrome/labels and monospace only for identifiers/figures; a masthead/wordmark; one stats line,
   one legend, one footer. Get founder sign-off before inventing a new system.
3. **First page fits.** At 1440×900 the main graph is fully visible (canvas bottom ≤ 900 including
   header/legend/footer), with a small key-metrics strip. No clipping, no horizontal scroll; the
   answer/action line is in the first scroll.
4. **Below: progressive charts, slowly.** Weight/blast-radius/file detail/F-treemap candidate as
   designed charts. No 3,000px brief; drills on separate pages. Budget `#map` ≈ 4–5k px.
5. **One definition each.** One unit definition, one structural caveat, one legend, one footer. No
   duplicate lines, no clipped text.
6. **Path discipline.** One solid high-contrast path; back-edges thin/recessive; never amber vs
   orange for two different edge types. Edge labels only on the path/selected edges; full counts in
   `<title>`.
7. **Path identity.** Mid-ellipsis + full path in `title`/detail; never leading-dot truncation.
8. **Detail is reachable.** Per-file functions and finding IDs must be reachable without leaving the
   page (rail, overlay, or a designed below-chart). The canvas alone is not enough.
9. **Keep Atlas's genuine improvements:** node-level `top N%`, numbered path badges, module bands,
   unreached column, eight-state coverage, units/anchors. The fix is composition, not a G revert.
10. **Gate:** A1 blind reads **plus a founder visual gate** on a 1440×900 render before A2/ship. A
    **G-baseline diff** is mandatory. Add DOM assertions: canvas bottom ≤ 900; no clipped text; no
    duplicate definitions; page-height budget; no leading-dot truncation. A1 alone cannot close the
    visual gate.

## 4. What not to do

- Do **not** run A2 or ship until the founder says the first page looks right.
- Do **not** add more blocks/features to fix the composition; remove and simplify first.
- Do **not** spend sessions on pixel-level layout nudges before the visual language is approved —
  the founder says layout is cheap; the palette/typography/hierarchy are the hard part.
- Do **not** treat the V8 16/16 reads as proof the page looks good. They prove comprehension.
- Do **not** delete the F treemap or the weight data; F is undecided, not rejected.
- Do **not** mark `map-build` done; A1b stays open; holdouts stay vaulted.

## 5. Status at the 2026-10-23 audit (historical)

- At the audit, `R-pf-109` (high) was open; `A-pf-118` (steps 9–10) was **blocked**.
- The next session at that time was **visual rework**, not A2:
  1. Side-by-side G vs Atlas; agree the first-page composition with the founder.
  2. Fix the objective defects: fold fit, duplicate chrome, clipped footer, truncation, edge
     labels/path contrast, unreached label, page height.
  3. Restore G's palette/typography/masthead; decide the rail/detail layer.
  4. Prototype the "more key charts" below (weight/blast radius/file detail; F treemap candidate).
  5. Add the founder visual gate + G diff; re-run V8; only then A2.

## 6. Reading order

1. `docs/evidence/atlas-audit/AUDIT.md` (the full audit)
2. `docs/evidence/map-vision/v7/COMPARISON.md`, `v7/g/reads.json`, `v7/g/NOTES.md`
3. `docs/specs/atlas-feature.md`, `atlas-layout.md`, `atlas-metrics.md`
4. `docs/adr/ADR-0009-atlas-core.md`
5. `state/handoff/latest.md`, `docs/activeContext.md`
