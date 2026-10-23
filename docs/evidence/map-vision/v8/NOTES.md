# V8 vision gate — Atlas 8-state matrix

**Date:** 2026-10-23. **Status:** PASS. **Owner:** orchestrator.
**Scope:** `atlas-build` steps 7–8. Founder A2 (step 9) remains the final gate; the holdout
(django/postgres + the memchr A2 payload) was never scanned, rendered, or read.

## Artifacts

- `html/<state>.html` — the eight full report pages rendered from `tests/atlas_fixtures` by
  `tests/atlas_vision.rs` (ignored writer; `PERFPAL_ATLAS_VISION_DIR`).
- `<state>-final.png` — 1440×900 final frames, `prefers-reduced-motion` + `animations=disabled`,
  mouse parked at (1,1) so native `<title>` tooltips are not captured.
- `<state>-weight-strip.png` — `#atlas-weight` element crops.
- `manifest.json` — viewport, per-state HTML/PNG SHA256, dimensions.
- `reads.json` — every blind read (A1 and strip) with the image hash and pass/fail decision.

The `gate.txt` transcript records the steps 7–8 gate (all exits 0). Its `== a2.md` line is the
negative guard that step 9 has not started; `a2.md` is intentionally absent.

Reproduce:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
PERFPAL_ATLAS_VISION_DIR=/tmp/perfpal-v8 cargo test --locked --test atlas_vision -- --ignored
cp /tmp/perfpal-v8/*.html docs/evidence/map-vision/v8/html/
python3 docs/evidence/map-vision/v8/render_v8.py
```

The script refuses to write screenshots unless the DOM preflight passes: first-screen unit/caption
line, rendered node name ≥9.9px, meta ≥8.4px, badge ≥7.9px, column header ≥10.4px, aggregate ≥8.4px, hull ≥9.0px (viewBox-scale-aware), text containment, compact edge labels ≤32px, no node under the
corner mouse, and the key line/first row above the 900px fold.

## Protocol

- Two fresh blind `@vision` reads per state on the 1440×900 final frame; one image per task; the
  prompt forbids source, sibling images, docs and expected answers.
- Two fresh blind reads per state on the weight-strip crop; JSON `{mass_answer, top_item,
  nonproduction_band, action, blockers, confidence}`; no scale comparison, no time inference.
- `reads.json` is the normalized transcript artifact: each read stores the image + sha256, the describe-first summary, the strict JSON and the evaluator decision; raw agent responses are not stored.
- A read passes A1 when it states the question the page answers and names a concrete file/action
  visible in the image. For the empty and all-test states the correct answer is the named stub /
  `none`; for no-entry it is the hottest file in the header.

## Results

- **A1:** 8/8 states, **16/16 reads pass (all high confidence)**. Application/library/partial-parse/single-file are
  clean; capped's lower depth-1 rows are below the 1440×900 first screen (canvas viewBox 1248,
  rendered 1255px; 20 of 26 shown, 6 aggregated, printed); no-entry/empty/all-test are named stubs
  by design.
- **Weight strip:** 8/8 states, **16/16 reads pass**. Every read answers N4 (where is the mass)
  with a printed share; 11/16 name a top file/component in the `action` field and 14/16 name a
top item/band in `mass_answer`; the five `none` actions are the correct empty/all-test/capped-tie
answers; no read compared a share to a multiple or inferred time. The
  inclusion rule holds.

## Blockers found and fixed during the gate (the capped framing is disclosed, not fixed)

1. The report body was capped at 72rem, scaling the 1400-unit SVG to 0.83× and shrinking text to
   5–8px. Body width is now 90rem (canonical 1440); the canvas renders at 1408px.
2. SVG typography floors: name ≥10px, meta ≥8.5px, badges ≥8px, headers ≥10.5px; four-line nodes,
   46px minimum height, 12px row gap.
3. Column headers are two lines, so `UNREACHED · no forward path · N file(s)` cannot collide with
   the depth-0 header.
4. Edge labels were 128px in a ~30px gap. They are now compact `×N` pills with the full
   `N distinct structural edge` in `<title>` and a legend key; a deterministic resolver moves any
   pill that would overlap a node (the library back edge covered `src/caller1.c`).
5. The component-hull label sat on the first node's badge. It is now 16px above the node
   (13px above the badge).
6. The first screen had no key for units, edge pills or path badges. A compact `.atlas-keys` line
   now carries the unit/denominator/edge/path keys and the structural caption.
7. Native SVG `<title>` tooltips were captured in one frame; the renderer now parks the mouse at
   (1,1) before every capture.
8. **Panel round:** the preflight now measures rendered sizes through the SVG screen CTM and
   asserts all six floors; aggregate text was raised to 9.1/8.6; the unit label is v1-aware; edge
   counts >999 clamp to `×999+`; manifest HTML paths were fixed.

## Minor observations (not blockers)

- The component label `src` was read as `SIC` in 3/16 strip reads; the mass, shares and actions
  were still correct. A2 should watch component-label clarity.
- `top P% of files` relies on the page header's file count for its population; the denominator is
  printed on the page, not repeated on each node.
- The capped state is intentionally taller than the viewport; the first screen is not a truncation
  of the data (caps are printed), and the full canvas is in the committed HTML.
