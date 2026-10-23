# Step-10 render — `@vision` review record

**Status:** step-10 render reviewed by `@vision` on 2026-10-23/24 (three passes: first render,
post-fix render, final render). The step-10 founder check was **approved 2026-10-24**; the
**step-13 final composition/V8 gate remains pending**. This file is evidence, not that gate.

**Inputs:**
- `atlas-v2-hyperfine-final.png`, `atlas-v2-cJSON-final.png` (step-10 1440×900 first viewports).
- `docs/evidence/map-vision/v7/g/g-hyperfine-final.png` (approved G baseline).
- The step-9 read `VISION-READ.md` (G vs Atlas v1) is a separate, earlier record.

## Pass 1 — first step-10 render

**Verdict: PASS WITH NITS.** The masthead, warm palette, strip and first-page fit were present; the
reader listed:
1. Dashed edges/hatch visually striking through node metric text (`tests/common.rs`,
   `src/bench_rement.rs`).
2. The UNREACHED zone said `0 shown` + only `+11 non-production`, while the footer said 21
   production + 11 non-production.
3. The START actions were faint.
4. The masthead subline ellipsized (`…`).
5. The bottom-right canvas was empty and the ENTRY strip still read as a run-on line.

## Pass 2 — after the fixes (updated PNGs)

**Verdict: PASS WITH NITS.** Closed:
- Test/bench cards use opaque pastel fills; `tests/common.rs` and `src/bench_rement.rs` metric lines
  are fully readable; no hatch strike-through.
- Hyperfine UNREACHED header is `no forward path` (no `0 shown`); the chip shows 21 production + 11
  non-production, agreeing with the footer.
- START is now three bordered chips with bold mono `path:line` + gray rule text; legible, no overflow.
- Masthead subline is fully rendered: `entry-flow map · static, profile to confirm`.

Still open / deferred:
- The cJSON UNREACHED chip disagreed with its footer (it said `+74 non-production` while the footer
  said production 4 / non-production 75). Fixed after the read by using the non-additive global
  totals: `production unreached: 4` / `non-production unreached: 75`, matching the footer.
- The bottom-right canvas void remains in hyperfine (the step-11 right rail fills it).
- The ENTRY strip is still a compact run-on line (accepted for step 10; step 13 judges the final
  composition).
- cJSON is sparse (2 reachable files); the `fuzzing · 3` hull wraps one drawn node. Sparsity/
  cosmetic, not a step-10 regression.
- The reader's `10 of 10` concern was a misread: the rendered strip is `19 of 19 reachable files
  shown` (verified in the HTML; the footer is `19 of 51 source files reachable`).

## Post-read consistency edit

The final chip wording (`production unreached: N` / `non-production unreached: N`, no `+`), the
chip/edge-pill 8.5px floor, the kicker/hull/column-header contrast tokens and the orange path
arrowhead are arithmetic/contrast edits applied after pass 2. They do not change the visual system;
the step-13 founder gate covers the final render.

## Pass 3 — final render (2026-10-24)

**Verdict: PASS for step 10** (not the step-13 founder gate). Confirmed on the final
`atlas-v2-{hyperfine,cJSON}-final.png`:
- (a) cJSON chip `production unreached: 4` / `non-production unreached: 75` matches the footer;
  hyperfine 21/11 matches.
- (b) chip/edge-pill 8.5px; START chips legible, one row, no overflow.
- (c) column headers are clearly darker; hull/`INFO ×N` micro-labels are improved but remain the
  lightest text tier.
- (d) burnt-orange path arrowheads are present on the path (into command.rs/metric.rs; cJSON.c) and
  neutral gray elsewhere.
- (e) no clipping and no horizontal overflow; footer/buttons/chip fully visible.
- (f) no new defects.

Residual nits (none block step 10): 8.5px micro-label contrast is at the floor; the ENTRY strip's
`19 of 19 reachable files shown` reads ambiguously at final size in the raster (the HTML is
`19 of 19` — no data error); the hyperfine bottom-right and cJSON left-half canvas voids are known
sparsity/cosmetic items; path arrowheads are small next to the `×N` pills.
