# Atlas sim — vision record (Track D, Session D1)

**Date:** 2026-10-24 · **Viewport:** 1440×900 · **Seat:** `@vision` (independent reads)
**Artifacts:** `hyperfine-{start,mid,end}.png`, `cjson-{start,mid,end}.png` (this directory)
**Context:** standalone companion prototype, static inferred run story over the Atlas map.

## Read 1 — first keyframes (pre-fix)

Verdict: **changes required**, four blockers.

| # | Blocker | Evidence |
|---|---|---|
| B1 | cJSON unreached cards floated outside any depth column, under no header | `cjson-mid` |
| B2 | Key strip truncated ENTRY confidence, HOTTEST ratio, and the 2nd/3rd START entries | all frames |
| B3 | Small badge text above the ENTRY card read as a clipped fragment | hyperfine frames |
| B4 | CAPS segment adjacent to ENTRY read ambiguously (`19 of 10…`) | hyperfine frames |

Nits: path order lost (plain dots instead of numbered badges); cJSON hottest was dimmed to
invisibility; START row truncation; `Reset` focus ring competed with the honesty banner.

## Read 2 — after the extraction + layout fixes

Verdict: **PASS-WITH-NITS** — no blockers.

- **B1 fixed:** cJSON now has `UNREACHED / no forward path` with five cards inside a labelled lane.
  Residual: the left side of a two-reachable-file graph is empty by construction (accepted).
- **B2 fixed:** ENTRY, HOTTEST and all three START entries are full in the hyperfine frames; the
  three-row strip is not crowded. Residual: the cJSON HOTTEST value (`1,000,234× median`) overflowed
  row 1.
- **B3 fixed:** `ENTRY · INFO ×8` is legible above the card.
- **B4 fixed:** `CAPS 19 of 19 reachable files shown · 0 aggregated` is unambiguous.
- **N1 fixed:** numbered path badges `1/2/3` and `1/2` are readable.
- New nits: the hyperfine `UNREACHED` header had no content beneath it (all unreached files are
  capped out of the drawn set); cJSON node text sat against card edges; adjacent badges in the
  depth-1/unreached corridor nearly touched.

## Read 3 — final confirmation (post-nit fixes)

Fixes: HOTTEST split into a mid-ellipsised path + compact metrics (`2.0M cost · 1.0M× median`, exact
string in the element title); node text compressed with `textLength`/`lengthAdjust` and names capped
at 24 characters; the `UNREACHED` header is suppressed when the lane has no drawn nodes.

Verdict: **PASS** — see below.

**Verdict: PASS.** No blockers.

- cJSON HOTTEST now reads `tests/unity/tes…ts/testunity.c · 2.0M cost · 1.0M× median` and fits before
  `PATH 2 steps`; it reads as the same Atlas strip, only compacted (exact value in the title).
- cJSON cards: no overflow, no edge-hugging; all seven cards fit their 150px boxes.
- hyperfine: no `UNREACHED` header over empty canvas; depth 0–4 only.
- Fold intact (narration y≈845–870, footer y≈885); honesty banner visible; no new overlap or
  clipping.

**Carried-forward nits (non-blocking):** the cJSON `WARN ×11 · partial` and `WARN ×6 · unreached ·
partial` badges sit ~3px apart across the depth-1/unreached boundary; a two-reachable-file graph
leaves the left canvas empty by construction; mid-ellipsis can hide the directory/file boundary in
long paths (tails stay distinguishable).


## Read 4 — synthetic state demos

Frames: `no-forward-path.png`, `empty.png`, `capped.png`.

Verdict: **PASS-WITH-NITS** — no blockers.

- All three are legible and labelled: `SYNTHETIC STATE DEMO`, a named title, a body, and the note
  that nothing is fabricated; the honesty strip and the `synthetic state demo` chip are present.
- No fake numbers: `no hotspot recorded`, `0 steps`, `no ranked actions recorded`, `no canvas nodes`;
  no time units, no scan percentages.
- The empty canvas is explained by the stub, not read as a render failure.

Nits (both fixed after the read): the capped body opened with "This scan exceeds…" (it could imply a
real scan — now "In a capped scan… It is a state demo, not a scan"); the capped stop said "ends at a
static leaf" while `PATH` was 0 steps (now "the story stops at the canvas cap, not at a leaf").

## Read 5 — post-fix function frames and badge definition

Frames: `hyperfine-loop.png`, `cjson-loop.png`, `hyperfine-mid.png`, `capped.png`.

Verdict: **PASS.**

- **Function highlight:** the focused node's bottom line becomes `▶ run · L34` (hyperfine) and
  `▶ main · L85` (cJSON) in bold type, inside the node border. Both loop targets are ENTRY nodes,
  so the entry text rule out-specifies the orange and the label renders cream (#e7e1d6, measured);
  it is highly legible, and the `▶` marker carries the highlight. On paper nodes the label is orange. It reads as that file's function,
  not a separate card or annotation; legible at node size, no wrap or clip. Branch-beat frames
  correctly keep `top N%` (highlight is loop-beat-only).
- **Badge definition:** `badge ×N = dominant rule annotation, not a file total` sits on the
  definition row without crowding or collision.
- **Capped copy:** "It is a state demo, not a scan" is explicit; the stop line is "the story stops at
  the canvas cap, not at a leaf"; no reader can infer a real scan.
- No blocker. The only carried nit is the pre-existing tight cJSON badge cluster at the
  depth-1/UNREACHED boundary.
