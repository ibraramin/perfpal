# Step 13 — vision reads (V8 rerun + G-baseline comparison)

**Status: A1 16/16 PASS · strip 16/16 PASS (2 fresh blind reads per state).** The G-baseline
comparative read verdict is **SAME SYSTEM** with the step-11 truncation residual as the most visible
defect. This file summarizes `step13-reads.json`; it does **not** replace the founder gate.

## 1. Protocol

- Two fresh blind `@vision` reads per state on the **1440×900 final frame**; one image per task;
  temporary neutral filenames; no source, sibling images, docs or expected answer; describe-first then
  strict JSON `{question_answered, action, blockers, confidence}`.
- Two fresh blind reads per state on the **weight-chart crop** (`#atlas-charts
  [data-atlas-block="weight-chart"]`); strict JSON `{mass_answer, top_item, nonproduction_band,
  action, blockers, confidence}`; no scale comparison, no time/runtime inference.
- Frames captured with `prefers-reduced-motion` + animations disabled, `#map` at the viewport top.

## 2. A1 — 16/16 PASS

Every read names the page's question and one concrete file/path action. Highlights:

- **application:** both reads recover entry `src/main.c:1`, HOTTEST `tests/t.c` (900 units, 3.0×,
  UNREACHED), the 3-step path, and the START action `src/a.c:3`.
- **library:** both read the library-root election (`src/lib.c:1`, MEDIUM), HOTTEST `caller1.c`
  (200 units, 2.0×, unreached) and `src/lib.c:3`.
- **no-entry / empty / all-test:** both reads return the *named* stub/no-production answer
  (`no electable production entry`, `no source functions recorded`, the single non-production test
  file), not a fabricated path.
- **capped:** both reads recover `src/f0.c:3`, the depth-1 node (printed `20 of 26 reachable files shown · 6 aggregated`) and the 12-card rail,
  despite the canvas exceeding the fold (V8-disclosed).
- **partial-parse:** both name `src/broken.c:3` and the partial-parse role.
- **single-file:** both name `src/solo.c:3` and the single ENTRY/HOTTEST node.

No read invented a time/runtime claim; no state confused production with non-production.

## 3. Strip — 16/16 PASS

| State | Printed split | Read (both) |
|---|---|---|
| application | 51.4 / 48.6 | paired split correct; top file `src/main.c` 30% |
| library | 100 / 0 | production 100%; `src/caller1.c` 57% |
| no-entry | 100 / 0 | `src/handler.c` 100% |
| empty | stub | `no scanned static cost recorded` |
| all-test | 0 / 100 | non-production 100% (1 file), no production cost |
| capped | 100 / 0 | component `src` 100% (26 files); five files each 4.2% |
| partial-parse | 100 / 0 | `src/main.c` 71%, `src/broken.c` 29% |
| single-file | 100 / 0 | `src/solo.c` 100% |

0 scale confusions, 0 time claims. Every paired share matches the page's single integer-tenths split
(no 0.1/0.1-style ambiguity).

## 4. G-baseline comparative read

One fresh comparative read of `g-hyperfine-final.png` (A) vs `atlas-v2-step13-hyperfine-final.png`
(B), same repo, same 1440×900 viewport, no labels:

- **Same system — yes.** Shared palette roles, type roles (small-caps labels, mono figures, grey
  meta), depth-column graph + 2-column file-card rail, legend row and footer stats line. The three
  convention swaps: masthead heroes the page name (B) vs the repo (A); rail shows the top 12 of 51 +
  a `+39 more` link (A showed 19 cards); node labels are basenames (B) vs mid-ellipsised paths with
  more `src/` context (A).
- **Fit:** A fits all 19 cards + footer with no clipping. B's rail is capped at 12 and fits with
  slack, but its header stack (title, status strip, 3 START boxes, 2 legend lines) pushes the graph
  start to ~200px and the next heading is sliced by the fold.
- **Detail:** both answer "which function in which file" without scrolling; B is richer per row
  (function + line number + finding link), but 39 files sit behind the weight-detail link.
- **Graph:** B prunes to the representative forward path (3 badges) and is easier to read than A's
  49-pair mesh; B draws only path/selected edge labels (2 vs A's 49).
- **Most visible defect in B:** mid-token filename amputations (`executi_.ets.rs`,
  `macos_c_.ters.rs`) with no `src/` prefix to recover context — the known step-11 residual.

**Verdict: SAME SYSTEM.** The read's colour naming is unreliable (it called both grounds "white");
measured DOM tokens are authoritative: `#map` background `#f4f1ea`, ink `#202b35`, muted `#6f6a61`,
burnt-orange `#c2410c`. One read also misread the canvas accounting as "10 of 19 reachable files
shown"; the page prints **`19 of 19 reachable files shown · 0 aggregated`** (footer: `19 of 51 source
files reachable`). The DOM is authoritative.

## 5. Gate status

- `step13.json` assertions: **all pass** (canvas bottom 869, `#map` 1,848, no h-scroll at 1440/1280,
  no clipping/leading-dot/duplicate definitions, ≤1 of each definition).
- `step13-reads.json`: **A1 16/16, strip 16/16**.
- **Founder visual gate: PENDING.** The founder must judge the 1440×900 composition. If rejected,
  stop and escalate a superseding decision; do not tune.
