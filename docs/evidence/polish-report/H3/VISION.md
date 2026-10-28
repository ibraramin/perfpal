# H3 vision gate — final — 2026-10-28 @ `fbda69b`

Report: hyperfine fixture, H3 final artifacts. Reviewer: `@vision`. Capture method: Playwright
(headless), two passes — `/var/tmp/opencode/h3-vision-final/` (10 captures: report/detail/profile +
JS-off canvas) and `/var/tmp/opencode/h3-vision-close/` (7 captures: drills, detail H1, JS-off map).
Captures are not committed; they are regenerable with the recorded Playwright setup.

## Final verdicts (closing pass)

| Capture | Verdict | Reason |
|---|---|---|
| `report-1440-canvas-js-off.png` (`#atlas-canvas` element, JS off) | **PASS** | 19 node cards painted with borders/filenames/costs; UNREACHED box and stats footer present. |
| `report-1440-map-js-off.png` (`#map`, JS off) | **PASS** | Complete static map + FILE DETAIL; the inert Pause is now hidden by the `<noscript>` rule. |
| `report-1440-map.png` (JS on) | **PASS** | Final frame identical to JS-off; Pause + Explorer visible. |
| `detail-1440-top.png` | **PASS-WITH-NITS** | Visible H1 now `Perfpal Findings Detail`; table cells wrap. Nits: mid-token path wraps and the trailing column at the table edge (pre-existing horizontal scroll). |
| `profile-1440-top.png` | **PASS** | `rank` header single-line; rows/badges unchanged. |
| `weight-1440-top.png` (Atlas weight drill) | **PASS-WITH-NITS** | New H1 `Perfpal Atlas` → H2 `Weight detail` on the shared scale (1.6/1.15rem, top rule); 10px drill density legible. Nit: the H2 echoes the first section title. |
| `q2-1440-top.png` (Atlas blast drill) | **PASS** | H1 → H2 `Blast drill` → DSM tables clean; no overlap or clipping. |

Earlier 10-capture pass (same verdicts, kept for the JS-off closure): both `#atlas-canvas` element
captures show 19/19 cards at 1440/3072; `report-1440-map-full-js-off.png` shows the whole static map
and report body; `report-1440-map.png` matches the JS-off final frame.

**Overall: PASS.** No blocking visual defect.

## Closed

- **JS-off static canvas.** The reveal animation starts columns at `opacity:0` with a 1.2s+ delay.
  The H3 `<noscript>` final-frame rule (`animation:none; opacity:1; transform:none;
  stroke-dashoffset:0` on `#map #atlas-canvas *`, plus `#map .atlas-pause{display:none!important}`)
  paints the canonical static frame immediately. The element captures show 19/19 nodes at 1440 and
  3072; the earlier "scaffolding-only / ghost main.rs" observation is closed.
- **Ranked-action wrapping.** `#ranked-actions td.wrap` (not `#findings`) lets long
  function/title/location tokens wrap instead of clipping.
- **Profile `rank` header.** Nowrap + fixed first-column width; no "ran k" break.
- **Drill pages.** Both drills now open with `<h1>Perfpal Atlas</h1>` + `<h2>Weight detail|Blast
  drill</h2>` on the shared heading scale, keeping the documented 10px drill-density body.
- **Dead filter UI removal.** No filter bar/controls remain; no gaps or orphaned chrome; the detail
  page reads as a complete static list.

## Non-blocking nits

- `detail-1440-top.png`: paths wrap mid-token (`…:144` / `4`); the column after `confidence` is cut
  at the table edge (pre-existing horizontal overflow; `.table-scroll` is the escape).
- `weight-1440-top.png`: the H2 `Weight detail` echoes the first section title.
- The earlier `report-1440-top.png` is a mid-animation capture (entrance reveal in flight); the
  final frame is proven in the map capture.
- Full-width note-card measure at 3072 and dense `unresolved` edge labels in the story remain the
  H2-approved map-interior nits.

## Gate basis

- H3 acceptance: `report-check.txt` — 5 widths × DPR 1/2 interactive + JS-off, auto and pinned 1440;
  no horizontal scroll, canvas fill, no clipped labels, story final, 3 scripts; PASS.
- Budget: `budget.txt` — hyperfine 338,091 B / 140–132 ms, cJSON 269,354 / 144–144, 3k 514,829 /
  156–148; all PASS.
- Story gate: `story-gate.txt` — 304/304 + 3k 44/44, verdict PASS.
