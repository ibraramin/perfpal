# H1 final vision gate — 1440 / 3072 / JS-off

Date: 2026-10-27 · Tree: `a9ddf61` + `8cf425d` (final H1) · Reviewer: `@vision`

## Verdicts

| Capture | Verdict |
|---|---|
| 1440×900, reduced-motion final frame | **PASS-WITH-NITS** |
| 3072×900, reduced-motion final frame | **PASS-WITH-NITS** |
| 1440×900, JavaScript disabled, reduced motion | **PASS** |

No blocking visual issues. The earlier 3072 FAIL and JS-off FAIL were both
capture-timing artifacts: the first screenshots were taken 400–500 ms into the
CSS column reveal (`atlas-column-reveal .4s 1.2s … both`), so the columns were
still at opacity 0. The final-frame captures disable motion; a DOM probe with
JavaScript disabled reports 19/19 nodes visible and column opacity 1.

## Confirmed

- 3072 canvas fully populated: depth 0–4 + UNREACHED headers, 20 node cards,
  group boxes, badges, edges; rail below the full-bleed canvas.
- 1440: full-width hero, rail beside (908/490), answer line in the first scroll,
  seven restored blocks and four charts, no clipping/overlap.
- Story: own title `The inferred run, beat by beat`, no duplicated masthead,
  closed `<details>` whole-repo disclosure, controls present; 996–1,032 px tall.
- JS-off: identical populated canvas, player shell hidden (`display:none`),
  `<noscript>` narrative visible, static canvas remains the final frame.

## Nits (non-blocking, deferred to H2/H3)

- 3072 hero remains a centered ~1,450 px column while the canvas/rail are
  full-bleed; H2 owns the responsive width/type ramp.
- `×2`/`×6` group-count badges sit flush against group labels at the canvas top.
- Dense 8–10 px edge micro-labels and story-graph edge density (labelled, allowed).
- Text blocks below the map are narrower than the full-bleed canvas at 3072.

## Screenshots

`/var/tmp/opencode/h1/shots/final-first-{1440,3072}.png`,
`final-map-{1440,3072}.png`, `final-story-{1440,3072}.png`,
`final-js-off-reduced-1440.png`.
