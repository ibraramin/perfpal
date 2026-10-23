# Spec: atlas-layout

Scope: feature

# Spec: atlas-layout (feature)

**Status:** planning lock, Atlas v1. **Ratified 2026-10-23** (founder; ADR-0009/0010 Accepted).
**No implementation in this spec.**
**Date:** 2026-10-23. **Owner:** orchestrator.
**Founder constraints (D-pf-238):** the graph page is 100% graph — no metric panels on the canvas;
metrics/brief live below or elsewhere; no mode switch; G is the core graph; F components are
supporting elements only if justified (`docs/research/atlas-developer-needs.md` §7).
**Inputs:** `docs/research/atlas-developer-needs.md`; `docs/research/atlas-data-gap-audit.md`;
`docs/specs/atlas-metrics.md`; `docs/specs/map-animation.md`;
`docs/evidence/map-vision/v7/{g,f}/reads.json`, `COMPARISON.md`, `NOTES.md`.

> **v2 amendment (2026-10-23, signed):** the founder's 2026-10-23 model supersedes the
> canvas-internal header/legend wording below. The masthead + key-metrics strip (entry, hottest,
> path, caps, 3 actions, one caveat, one legend/unit definition) live **outside** `#atlas-canvas`;
> the canvas is the graph + one footer. The detail rail is step 11; the progressive charts/F
> candidate are step 12. See `docs/evidence/atlas-visual/v2/DECISION.md`.

## 1. Purpose

One full-page composition for Perfpal Atlas: the entry-flow canvas first, a fixed seven-block brief
below it, drills for depth, and the legacy stems + F treemap in Explorer. The page shape is
identical for every scan; only content and labels change.

## 2. Page map

```
┌ header ─────────────────────────────────────────────────────────────┐
│ Perfpal · Atlas · <repo> · <files> files · <findings> findings      │
│ [Pause animation] [Reduced motion] [Explorer →]                     │
├ canvas (first screen, full-bleed, 100% graph) ──────────────────────┤
│ entry badge · HOTTEST · path summary · caps chip · legend           │
│ depth columns 0..N · file nodes · call edges · component hulls      │
│ unreached column · numbered path trace                              │
│ canvas footer: reachability · edge coverage · structural caption    │
├ brief (below the fold) ─────────────────────────────────────────────┤
│ 1 answer line                                                        │
│ 2 KV1 actions (3–5)                                                  │
│ 3 weight strip                                                       │
│ 4 blast radius                                                       │
│ 5 file cards                                                         │
│ 6 noise & confidence footer                                          │
│ 7 drill links                                                        │
└─────────────────────────────────────────────────────────────────────┘
```

- **First block = the canvas.** This amends the `map-intelligibility` A8 rule for the Atlas page
  (recorded in the spec amendment): the first *answer-bearing* block is the answer line directly
  below the canvas, within the first scroll.
- **Canvas must answer Q1 without scrolling** (falsifier #1 in the needs research): a blind read of
  the first screen names the entry, the HOTTEST file and the representative path from the canvas
  alone; the one `path:line` action is in the first scroll (answer line + KV1).
- **No metric panels on the canvas.** Cards, tables, bars, metric selectors, a second map and any
  data-keyed block are forbidden (metrics spec §4, §9).

## 3. Canvas anatomy

### 3.1 Canvas header strip (inside the canvas frame)

- `ENTRY · production main · confidence HIGH · 0 alternatives`,
  `ENTRY · production script · confidence HIGH · 0 alternatives`, or
  `ENTRY · library root (no production main) · confidence MEDIUM · 2 alternatives`.
- `HOTTEST: <file> · <cost> · <anchor>` (M-04 rank 1).
- `PATH · <N> steps` (M-16).
- Caps chip: `N of M reachable files shown · N aggregated` (M-14).
- Pause control (CSS-only checkbox/label; keyboard accessible).

### 3.2 Graph field

- **Depth columns** `depth 0` … `depth N` (BFS from the elected entry), left→right; the column
  header shows the depth and the count of files in that column, including aggregated files.
- **File nodes:** rounded box per file with the file name (tail-keep 32 chars, full path in
  `title`/aria), `cost` + `× median function` (M-04), a `HOTTEST` text badge on rank 1, and a
  compact finding annotation `HOT ×N` / `WARN ×N` / `INFO ×N` (M-18; text only — findings ride as
  annotations on the map per the north-star, with the full rule list in the file card below).
  Production files use the default fill; test/bench/fixture/vendor/generated files use a
  labelled hatch; unreached production files are grey with `unreached` text.
- **Edges:** forward caller→callee edges solid with an arrowhead; back-edges dashed amber; the
  edge label is the distinct structural-edge count (M-10). The edge style key and the
  `structural (LOW name-based + MEDIUM semantic)` caption are mandatory.
- **Component hulls:** light rounded outline behind nodes sharing a domain prefix, with the hull
  label and a disclosure that hulls are approximate prefix groups. Hulls are structure, not a
  metric channel.
- **Unreached column:** production files with no estimated reachability, grey, sorted cost↓/path,
  with the count; non-production unreached are aggregated into a counted chip.
- **Path trace:** numbered badges `1..K` on the representative entry-to-leaf path (M-16), with the
  label `representative path (static, BFS-derived)`.
- **Caps:** when the node cap is hit, draw the top-cost files per depth and an aggregated box
  `+N more · <calls in>`; never silently drop.

### 3.3 Canvas legend (units) — v2: the one legend lives in the page strip

One compact legend row: cost unit `static cost units (v2)`; depth = BFS from entry; edge styles
(forward / back / aggregated) + structural caption (`LOW name-based + MEDIUM semantic`);
production vs test/bench hatch; unreached;
`× median function` denominator. No colour ramp for severity, no metric dropdown.

### 3.4 Canvas footer

`<R> of <M> source files reachable (P%) · <D> edges drawn of <T> file pairs · <N> MEDIUM / <L> LOW
structural edges · <U> unreached production · <V> unreached non-production`. Counts with
denominators; no anchors.

## 4. The seven-block brief (below the graph)

Every block renders in every state; a zero-state stub replaces missing content.

1. **Answer line.** 2–3 sentences: elected entry and why; HOTTEST file; the one action to open
   first (`path:line` + function + rule title); the one caveat that most affects trust. No topology
   restatement.
2. **KV1 — Start here.** 3–5 ranked actions: `path:line`, function, rule title, severity +
   confidence, production class, cost anchor (M-05), trip provenance (M-12), why, drill link.
   Production-first; complete list linked.
3. **Weight strip.** Top 5 production files + top 3 **production** components by `% of scanned static cost`
   (M-06/M-07), plus the labelled `non-production (test/bench/other) = X%` band (M-15). Always rendered; zero-state stub
   `no production cost recorded`. No area encoding, no treemap.
4. **Blast radius.** For the selected or entry file: `N dependent files` (M-08) with
   `% of source files` and `top P%`; `N caller files · M callee files (structural)` (M-09); top 5
   dependent files; include coverage line; full-list drill. The LOW/structural label is mandatory.
5. **File cards.** For the top selected files (cap 5–8): file cost + anchor (M-04); top 3 functions
   by cost with `top P%` (M-05); dominant finding rule + severity; production class; drill link.
   This is where the V7 G right-rail cards live.
6. **Noise & confidence footer.** M-13: parse `ok/partial/failed`; unresolved imports/includes;
   `MEDIUM/LOW` edge counts; demoted files; caps; unreached production/non-production. One compact
   block, not a table of every file.
7. **Drill links.** Complete ranked action list; complete profile queue; full dependents; weight
   detail; per-component pages. Every link resolves to content that shows the action; no dead
   affordances.

## 5. Scan states (same page shape, different content)

| State | Canvas | Brief |
|---|---|---|
| **Application (production main/script)** | entry at depth 0; forward flow; HOTTEST; path trace | full brief; weight strip shows production share |
| **Library (lib-root entry)** | entry = highest-fan-in production lib-root; forward edges may be empty; callers into the entry drawn grey in the unreached column with `reverse context → Q2 drill`; footer says `no forward path` | full brief; answer line states the library case; KV1 + weight strip carry the action; blast radius prominent |
| **No entry** | `NO ENTRY` stub panel; no fake node | full brief; answer line states why; weight strip + KV1 still render |
| **Empty / no functions** | named stub `no source functions recorded` | named stubs in every block |
| **All-test / test-dominated** | production nodes may be tiny; test/bench hatched; counts printed | weight strip leads with the non-production band; KV1 production-first; noise footer prints the share |
| **Capped (large repo)** | top-cost files per depth + `+N more` aggregated boxes; caps chip | file cards show the shown set; drill links resolve to complete lists |
| **Partial parse** | parse-affected files marked `partial`; no silent omission | noise footer leads with `parse N partial / M failed` |
| **Single-file repo** | one node + entry badge; no edges | full brief; weight strip = 100% one file; blast radius stub |

## 6. Drills

- **Q2 reverse drill:** component DSM + focus file + 1-hop dependents; include coverage; call
  callers/callees with the structural label. Answers "what breaks if I touch it".
- **Complete action list:** the full deterministic KV1 ranking (not a cap).
- **Profile queue:** the complete function-level cost list within the non-demoted set.
- **Weight detail:** file/component share detail; the F treemap (`weight-treemap`) lives in
  **Explorer**, labelled, never on the Atlas page; the weight detail links to it.
- **Per-component pages:** existing component pages, Explorer-tier evidence when reached from a
  key view.

## 7. Animation

Per `map-animation` (mandatory) and the pending ADR-0010:

- CSS keyframes/SMIL only; **no `<script>`, no event handlers, no network**.
- Timeline (deterministic function of the view model; fixed constants): entry pulse (0–1.2s) →
  depth columns reveal (1.2–2.8s) → edge draw-on (2.8–4.2s) → path trace (4.2–5.4s) → HOTTEST
  pulse (5.4s+). No randomness, no wall-clock input.
- The final frame is complete: every node, label, number, caption, edge and the full brief.
- A CSS-only pause control freezes the timeline; keyboard accessible; labelled.
- `prefers-reduced-motion: reduce` and `@media print` show the final frame with no motion.
- Animation is presentation, never information: nothing exists only mid-animation.

## 8. Print and reduced motion

- Print: page 1 = canvas final frame + legend + canvas footer; page 2+ = the seven-block brief;
  drills are linked, not printed by default. No animation; no hover-only affordances.
- Reduced motion: identical to print's final frame; the pause control stays.
- 1440×900 canonical; canvas fills the first viewport (header + canvas ≥ 700px); no horizontal
  scroll at 1280×800 minimum. Long paths wrap or tail-keep with `title`; the brief is single-column.

## 9. Accessibility

- Every SVG node/edge carries a `<title>` with the full `path`, cost, unit and anchor.
- Pause is a real checkbox + label; focus order follows the header → canvas → brief.
- No colour-only encoding: production/test, forward/back, reached/unreached, HOTTEST all carry
  text labels. Contrast: text on fills ≥ 4.5:1 for key-view text.
- The answer line is the first text block; it is plain language (no `DAG`, `BFS`, `barycenter`).

## 10. Acceptance

- **A1 (vision):** for each state in §5, a fresh blind read of the final frame states the question
  it answers and one concrete `path:line`/file action; library and capped states included.
- **A2 (founder):** timed test on the holdout (application + a library-shaped repo), animation
  enabled; 3 actions in under 10 minutes; understandable; pause/reduced-motion/print work.
- **A8 (amended first screen):** first block = canvas; first answer block = answer line directly
  below; the canvas itself carries entry + HOTTEST + path.
- **A9 (canvas purity):** DOM assertion: no table, bar list, card list or metric selector inside
  the canvas element; only nodes/edges/labels/legend/caps.
- **A10 (state-unconditional):** all §5 states render the same canvas + seven blocks.
- **A12 (no new JS):** the Atlas canvas and animation blocks contain no `<script>` and no event
  handlers; the report's existing single fixed enhancement script (`REPORT_SCRIPT`, fixed source,
  no interpolated data, no network) is unchanged and inventoried. `grep` asserts exactly one block
  in `report.html` and zero in the canvas/SVG/animation artifacts.
- **A4 (determinism):** double scan → byte-identical HTML/SVG (excluding run provenance).
- **A7/A11:** metrics spec §10.

## 11. Open items (owner: build plan)

- Whether the canvas uses depth **columns** (V7 G) or **rings**; columns are the current evidence
  winner on 1440×900 and are the default unless a blind read prefers rings.
- The exact aggregated-box wording (`+N more · <calls in>`) and whether aggregated boxes are
  focusable.
- Whether the weight strip's component rows use `components[].path_prefix` or chart-side
  `domain_of` grouping (metrics spec §11).
- The drill page count and nav labels (`Q2 reverse`, `All actions`, `Profile queue`, `Weight
  detail`, `Explorer`).