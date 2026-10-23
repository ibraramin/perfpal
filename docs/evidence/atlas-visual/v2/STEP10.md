# Step 10 — first-page fit, G visual language, chrome hygiene

**Date:** 2026-10-23 · **Scope:** `report.html#map` at 1440×900, reduced motion.
**Contract:** `DECISION.md` (Option A), `G-BASELINE-DIFF.md` §4, `docs/plans/atlas-build.md` step 10.
**Evidence:** `step10.json`, `gate.txt`, `VISION-STEP10.md`,
`atlas-v2-{hyperfine,cJSON}-{final,full}.png`; reproduce with
`python3 docs/evidence/atlas-visual/v2/render_step10.py`.

Step 10 renders progress (not the end-state): the detail rail is step 11, progressive charts/F are
step 12, and the founder visual gate is step 13. The step-10 render was read by `@vision`; the
PASS-WITH-NITS nits were fixed (full-bleed paper, opaque non-production nodes, restored production
unreached, non-additive unreached chip, chip/edge-pill 8.5px, contrast, subline, entry hierarchy),
and the final read is **PASS**. The record is `VISION-STEP10.md`; that read is a proxy, **not** the
founder gate.

## Before → after (shipped Atlas v1 → step 10)

| Metric (1440×900, `#map`) | v1 (audit) | Step 10 |
|---|---|---|
| `#map` band | 1,408px inset (body padding) | **1,440px full-bleed**; canvas content 1,408px |
| `#atlas-canvas` bottom | **981** (hyperfine/cJSON) | **862** (both) |
| `#atlas-canvas` height | 980 | **742** |
| Masthead | none (orphan mono chrome) | **58px** |
| Key-metrics strip | 5 head spans + a keys row + a 9-item legend + footer | **53px** (≤56): entry · hottest · path · caps, 3 START chips, one legend/unit/caveat |
| Canvas footer | clipped below 900 | one **19px** line, inside 900 |
| `#map` height | 8,982 / 14,685 | **8,887 / 14,591** |
| Page height (document scroll) | 22,942 / 34,831 | **22,911 / 34,800** |
| `body.scrollWidth` | 1,440 | **1,440** |
| `#atlas-brief` top | 981 | **862** (= canvas bottom) |
| `.atlas-legend` | 1 (inside canvas) + keys/units duplicates | **1** (strip, outside canvas) |
| `.atlas-footer` | 1 (clipped) | **1** (inside canvas) |
| Structural caveat occurrences | 6 | **1** |
| Median-definition occurrences | 2 | **1** |
| Visible edge labels | 49 of 49 edges (hyperfine) | **2** of 49 (path only; every edge keeps its `<title>`) |
| Leading-dot truncation | yes (`..tity/statistics.rs`) | **0** (mid-ellipsis + full path in `title`) |
| Canvas panels (table/ul/li/card/bar) | 0 | **0** |
| `REPORT_SCRIPT` blocks | 1 | **1** |

## Acceptance booleans (`step10.json`)

| Check | hyperfine | cJSON |
|---|---|---|
| `#atlas-canvas` bottom ≤ 900 | PASS 862 | PASS 862 |
| `scrollingElement.scrollWidth` ≤ `clientWidth` + 1 | PASS 1440 ≤ 1440 | PASS 1440 ≤ 1440 |
| `#atlas-brief` top ≥ canvas bottom | PASS 862 ≥ 862 | PASS 862 ≥ 862 |
| `#map` height ≤ audit baseline (8,982 / 14,685) | PASS 8,887 | PASS 14,591 |

Chrome counts (both repos): `.atlas-legend` **1**, `.atlas-footer` **1**, structural caveat **1**,
median definition **1**, leading-dot truncations **0**, canvas panels **0**, `REPORT_SCRIPT` **1**.
Visible edge labels: **2** of 49 edges (hyperfine), **1** of 5 (cJSON) — exactly the on-path edges.

## Verification

`gate.txt` is the command + exit + artifact-hash transcript for this tree. All commands exit **0**
(fmt, clippy, `cargo test --locked` 55 suites, deny, semantic clippy/tests, the four targeted Atlas
locks, schema-clean, and the evidence script). `render_step10.py` now fails closed on any false
acceptance boolean and asserts the chip/footer text checks. The final PNG/report hashes in
`step10.json` match the files on disk; both final PNGs are 1440×900.

The **step-10 founder visual check (DECISION §4.9)** was **APPROVED 2026-10-24**; the founder authorized the commit. The **step-13 final composition/V8 gate remains PENDING**. `VISION-STEP10.md` records the
`@vision` read and the fixes; it is not the founder gate. Fit evidence covers hyperfine/cJSON at
1440×900; capped/deep states exceed 900px by design (V8-disclosed) and are deferred to step 13/A2.

## Rendered text floors (computed px)

| Element | Rendered |
|---|---|
| node name | **11.5** |
| node meta | **9.0** |
| column header | **10.5** |
| node badge | **8.5** |
| strip | **9.0** |
| START chip | **9.5** |
| visible edge pill | **8.5** |
| ENTRY value | **10.0** |

## Unreached accounting

The chip and footer both use the **global** unreached totals, so they always agree.

- Hyperfine chip: `production unreached: 21` + `non-production unreached: 11`; footer:
  `unreached production: 21 · non-production: 11`.
- cJSON chip: `production unreached: 4` + `non-production unreached: 75`; footer:
  `unreached production: 4 · non-production: 75`.
- The chip is **non-additive** (no `+` prefix) even when some unreached cards are drawn: the
  library state draws both callers and still shows `production unreached: 2`, matching its footer.
- The chip title is `P production unreached · N non-production unreached · no estimated
  reachability`; `data-prod-unreached` / `data-nonprod-unreached` carry the same globals.
  The text checks are recorded in `step10.json` and asserted by the evidence script.

## What changed

- **DOM:** `#map` = `.atlas-page-chrome` (masthead + key-metrics strip) → `#atlas-canvas`
  (graph + one footer) → `#atlas-brief`. The strip is the first answer-bearing block; the canvas is
  graph-only (A9).
- **Full-bleed `#map`:** `margin:0 -1rem; padding:8px 1rem; border-radius:0` against the body's
  `padding:0 1rem`; the paper band reaches the viewport edges while the canvas stays 1,408px.
- **Masthead (58px):** `PERFPAL · ATLAS`, wordmark `Atlas`, subline on its own line
  (`entry-flow map · static, profile to confirm`, no ellipsis); CSS-only `Pause` + `Explorer →`.
  The repo name is omitted: the HTML deliberately omits `scan.root_path` (no provenance surface);
  adding it needs a separate provenance decision (DECISION §4b).
- **Strip (53px):** line 1 entry/hottest/path/caps (ENTRY value 10px/600 mono); line 2 `START` +
  top-3 KV1 chips (9.5px/600 `#202b35`, `#b8b2a7` border, visible `path:line — rule`, full title,
  existing `findings-detail.html#…` anchors); line 3 the single swatch legend + one unit definition +
  one structural caveat.
- **Tokens:** warm `#map` band (`#f4f1ea` / `#fbf9f4` / `#202b35` / `#6f6a61` / `#d6d0c4` /
  `#c2410c`); global `body{background:#fff}` untouched; sans chrome + mono identifiers/figures.
- **Non-production nodes:** opaque G fills — test/fixture/vendor/generated `#eef2f7` / `#7f93aa`,
  bench `#f7f0f7` / `#8a6f9e`, both `stroke-dasharray="3 2"`; the hatch pattern was removed.
- **Edges:** forward `#8d8577` solid at 0.34; back neutral `#8d8577` thin-dashed (not amber); path
  `#c2410c` solid 2.3px ending in the burnt-orange `atlas-arrow-path` marker; visible `×N` pills
  only on `edge.on_path` at the 8.5px floor.
- **Hulls:** pastel bands removed; one dashed neutral outline + label + title.
- **Truncation:** mid-ellipsis (first ~half + `…` + last ~half); full path stays in the node/column
  `<title>`.
- **Unreached:** column line 2 is `no forward path`; one combined chip carries the global
  `production unreached: P` / `non-production unreached: N` with `data-prod-unreached` /
  `data-nonprod-unreached`; footer reads `unreached production: P · non-production: N`.
  The chip is non-additive even when unreached cards are drawn.
- **Caveat de-duplication:** the full structural caveat is only in the strip; brief captions now
  point to the Atlas legend.
- **Contrast/a11y:** kicker and hull labels use `#6f6a61`; column headers use `#202b35` /
  `#6f6a61`; the paused label gets `#ece8df` / `#c2410c`.
- **Standalone helper:** `render_canvas_page` wraps the fragment in `<section id="map">` so the
  scoped CSS applies.

## PNGs

- hyperfine: [`atlas-v2-hyperfine-final.png`](atlas-v2-hyperfine-final.png) ·
  [`atlas-v2-hyperfine-full.png`](atlas-v2-hyperfine-full.png)
- cJSON: [`atlas-v2-cJSON-final.png`](atlas-v2-cJSON-final.png) ·
  [`atlas-v2-cJSON-full.png`](atlas-v2-cJSON-full.png)

## Deviations / risks

- The strip is 53px on both repos because line 1 wraps once at 1,408px content width (the ENTRY value
  is 10px and the HOTTEST anchor is long, especially on cJSON). It is ≤56px and nothing is clipped.
- 1280×800 pass: `clientWidth` 1280, `scrollWidth` 1280, canvas bottom 780 — no horizontal scroll.
- `noClippedChrome` is true for `.atlas-keymetrics`, `.atlas-footer`, and chip titles on both repos.
- The brief begins at 862, so ~38px of it is visible in the 900px viewport. All step-10 assertions
  pass; pushing the brief fully below the fold is step 11/12 composition.
- Page height does **not** increase (22,911 / 34,800 vs 22,942 / 34,831), but it is still a long
  report. `#map` ≈ 8.9k/14.6k is unchanged in kind (only ~95/94px below the v1 baseline); the ~5k
  budget and progressive charts are step 12.
- **Step-10 founder visual check APPROVED 2026-10-24**; the step-13 final composition/V8 gate remains
  PENDING. `VISION-STEP10.md` records the final `@vision` pass on the final render, including the
  chip/font/contrast edits; that read is a proxy, **not** the founder gate.
- **Fit scope:** hyperfine/cJSON only. Capped/deep states exceed 900px by design (V8-disclosed) and
  are deferred to step 13/A2; do not claim state-unconditional 900px fit.
- **Disclosed deviations** (also in `DECISION.md` §4b): masthead wordmark is `Atlas` (the `<repo>`
  label is omitted because the HTML deliberately omits `scan.root_path`); entry value is
  10px/600 rather than the sketch's 12.5px/750 to keep the strip at 53px. Both go to the step-13
  founder check.
- **Measurement note:** the evidence uses overlay scrollbars. A classic-scrollbar Chromium gives
  canvas bottom 854 and `#map` 8,780/14,353 — still passing; this is a probe-only observation (not
  recorded in `step10.json`), and step 13 pins the scrollbar mode.
- The step-9 contract is signed; the rail (step 11) and F/charts (step 12) remain open. Holdouts stay
  vaulted; A2 (14) and ship (15) are untouched.
- No schema change, no new dependency, no new report script. Founder approved the step-10 first page
  and authorized the commit on 2026-10-24 (see git log).
