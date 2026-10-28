# H3 vision gate — final — 2026-10-28 @ `8544706`

Report: hyperfine fixture, H3 final artifacts. Reviewer: `@vision`. Captures (not committed;
regenerable with the recorded Playwright setup):
`/var/tmp/opencode/h3-vision-final/{report,detail,profile}-*.png`.

## Verdicts

| Capture | Verdict | Reason |
|---|---|---|
| `report-1440-canvas-js-off.png` (`#atlas-canvas` element, JS off) | **PASS** | All 19 node cards painted with borders/filenames/costs; groups, ①②③ badges, arrows, UNREACHED box and stats footer present. |
| `report-3072-canvas-js-off.png` (`#atlas-canvas` element, JS off) | **PASS** | Same 19 cards at width; no ghosts. |
| `report-1440-map-js-off.png` (`#map`, JS off) | **PASS** | Painted map + FILE DETAIL panel + full chrome. |
| `report-1440-map-full-js-off.png` (full `#map`, JS off) | **PASS** | Whole static map (Weight → Drills) renders; no blank regions. |
| `report-1440-map.png` (JS on) | **PASS** | Final frame identical to JS-off; the `<noscript>` rule matches the live end state. |
| `report-1440-top.png` | **PASS-WITH-NITS** | Atlas embed caught mid-entrance-animation (only `main.rs` painted); final frame proven painted in the map capture. |
| `detail-1440-top.png` | **PASS** | Title/location/function cells wrap; table readable. |
| `detail-3072-top.png` | **PASS-WITH-NITS** | Wrap confirmed; the trailing column after `confidence` is cut at the 72rem/table edge (pre-existing horizontal overflow, scrollable table). |
| `detail-1440-table.png` (`#ranked-actions`) | **PASS** | 2–3-line wraps in title/location/function; no clipping in those cells. |
| `profile-1440-top.png` | **PASS** | Cost-ranked table fits; `rank` header single-line; badges wrap cleanly. |

**Overall: PASS.** No blocking visual defect.

## Closed

- **JS-off static canvas.** The reveal animation starts columns at `opacity:0` with a 1.2s+ delay.
  The H3 `<noscript>` final-frame rule (`animation:none; opacity:1; transform:none;
  stroke-dashoffset:0` on `#map #atlas-canvas *`) makes the canonical static frame paint immediately.
  The element captures show 19/19 node cards at 1440 and 3072; the earlier "scaffolding-only / ghost
  main.rs" observation is closed as an animation-timing defect, now fixed.
- **Ranked-action wrapping.** The `#ranked-actions td.wrap` scope (not `#findings`) lets long
  function/title/location tokens wrap instead of clipping.
- **Profile `rank` header.** Nowrap + fixed first-column width; no "ran k" break.
- **Dead filter UI removal.** No filter bar/controls remain; no gaps or orphaned chrome; the detail
  page reads as a complete static list.

## Non-blocking nits

- `report-1440-top.png` is a mid-animation capture (entrance reveal in flight); a settle-wait in a
  future capture script would make the top proof match the final frame.
- The detail table's trailing column after `confidence` is cut at the table edge (pre-existing
  horizontal overflow; `.table-scroll` is the escape).
- The known inert `Pause` control still looks enabled in JS-off captures (cosmetic, pre-existing).
- Full-width note-card measure at 3072 and dense `unresolved` edge labels in the story remain the
  H2-approved map-interior nits.

## Gate basis

- H2 acceptance: `report-check.txt` — 5 widths × DPR 1/2 interactive + JS-off, auto and pinned 1440;
  no horizontal scroll, canvas fill, no clipped labels, story final, 3 scripts; PASS.
- Budget: `budget.txt` — hyperfine 338,050 B / 124–132 ms, cJSON 269,313 / 112–136, 3k 514,788 /
  128–132; all PASS.
- Story gate: `story-gate.txt` — 304/304 + 3k 44/44, verdict PASS.
