# H2 vision gate — 1440 / 3072 / JS-off

Date: 2026-10-28 · Report: hyperfine fixture, final H2 artifact (run4, code
`43dfeb9`) · Reviewer: `@vision`

## Verdicts (final)

| Capture | Verdict |
|---|---|
| 1440×900, first page | PASS |
| 3072×900, first page | PASS-WITH-NITS |
| 1440×900, Atlas map | PASS |
| 3072×900, Atlas map | PASS-WITH-NITS |
| 1440×900, story | PASS |
| 3072×900, story | PASS-WITH-NITS |
| 1440×900, story full section | PASS |
| 3072×900, story full section | PASS-WITH-NITS |
| 1440×900, JS-off map | PASS |
| 3072×900, JS-off map | PASS-WITH-NITS |
| 1440×900, JS-off story / Explorer | PASS-WITH-NITS |
| 3072×900, JS-off story / Explorer | PASS |
| Full Explorer section, 1440 | PASS-WITH-NITS |
| Full Explorer section, 3072 | PASS |
| Atlas brief, 1440 | PASS |
| Atlas brief, 3072 | PASS-WITH-NITS |

**Overall: APPROVED-WITH-NITS.** No blocking visual defect. The 1440 anchor is
unchanged; all four H1 nits are resolved or substantially resolved at 3072; the
JS-off static canvas is complete at both widths.

## H1 nits at 3072

| H1 nit | Status |
|---|---|
| Hero chrome / answer column narrower than the canvas | **RESOLVED** — header bar, `Pause`/`Explorer`, and the answer callout run to the full-bleed width. |
| Story 1,400 px vs 2,880 px canvas | **RESOLVED (container)** — section, controls, status rows, summary and disclosure span the full width; the artwork keeps its own letterbox (pre-existing). |
| Text blocks in a narrower column | **RESOLVED** — full width; lines can reach ~200+ characters (see nits). |
| Type fixed 8–10 px | **RETRACTED as a defect** — computed sizes at 3072 are 12–18 px; the 8–10 px appearance was the downscaled screenshot. |
| Explorer clipped at the right edge | **RESOLVED** — the legacy Explorer SVGs now fit their figure at 1440 (full 13-column graph visible, ~25 px right margin) and sit at intrinsic width, centred, at 3072 (~70 px margins); no upscale, no clip. |
| FILE DETAIL rail unverified at 3072 | **CLOSED** — DOM: rail top 2,854 > canvas bottom 2,844, 12 cards, four `.atlas-chart` blocks (weight, blast, file, coverage). |

## Nits (non-blocking)

- Full-width body copy at 3072 reaches ~200+ characters per line; a reading
  measure would be kinder (the founder chose full-width fluid).
- The story graph artwork is letterboxed inside its full-width canvas (same
  proportion as 1440; pre-existing).
- The hero text block still occupies only the left portion of the 3072 page
  (natural short lines, not a width cap).
- Dense Atlas/story micro-strips remain the smallest text on the page.
- At 1440 the fitted legacy Explorer labels are ~6 px (the price of showing the
  whole figure instead of clipping it); the `.pz` zoom is the intended escape.
- The Atlas brief file-card row at 3072 leaves an empty grid slot at the right
  (6 cards in a 7-column auto-grid).
- The inert `Pause` control is still visible in JS-off captures (cosmetic).

## Screenshots

`/var/tmp/opencode/h2/shots-final/` — `first-{1440,3072}.png`,
`map-{1440,3072}.png`, `story-{1440,3072}.png`,
`story-full-{1440,3072}.png`, `js-off-map-{1440,3072}.png`,
`js-off-story-{1440,3072}.png`, `explorer-full-{1440,3072}.png`,
`atlas-brief-{1440,3072}.png`.
