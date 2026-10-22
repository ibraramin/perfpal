# F prototype — weight-map treemap (cJSON · hyperfine · tmux)

Status: **prototype built and mechanically verified**; blind `@vision` reads, the F-vs-G A/B,
and the founder A2 timed test are **not** part of this session. No `src/` changes, no commit.

## What this is

The F human-map candidate from `docs/plans/map-f-treemap.md`: a deterministic whole-repository
weight map. One first screen carries the component treemap, the KV1 brief strip and a one-line
answer; the Q2 dependency question moved to a separate drill page (component DSM + focus + 1-hop
dependents). Animation is declarative CSS over a complete final frame (spec `map-animation`).

Inputs are the three design-repo scans only:

| repo | input | files | functions | findings | parse ok/partial/failed |
|---|---|---|---:|---:|---|
| cJSON | `perfpal-test/out-v6-cJSON/metrics.json` | 101 | 1055 | 837 | 63 / 38 / 0 |
| hyperfine | `perfpal-test/out-v6-hyperfine/metrics.json` | 53 | 353 | 348 | 53 / 0 / 0 |
| tmux | `perfpal-test/out-v6-tmux/metrics.json` | 216 | 3189 | 7545 | 100 / 116 / 0 |

The django/postgres holdout is never read or rendered. The renderer hard-codes the design inputs
and refuses holdout paths.

## Artifacts

- `render_f.py` — self-contained; `python3 docs/evidence/map-vision/v7/f/render_f.py`.
- `f-<repo>.html` — 1440×900 first screen (treemap + brief + answer + Q2 link), CSS keyframes.
- `f-<repo>-drill.html` — component DSM + focus + 1-hop dependents; CSS `:target`/`:hover` only.
- `f-<repo>-{start,mid,final}.png` — 1440×900 viewport frames. Final uses reduced-motion state.
- `manifest.json` — image dimensions + sha256, counts, caps, input hashes, caveats.
- `NOTES.md` — this file.

## F contract → implementation

| Contract | Implementation |
|---|---|
| Deterministic | Every traversal sorted (`cost desc`, then id; findings by severity/confidence/cost/path/line); integer treemap coordinates via squarify + rounding; fixed palette; no random, clock, environment, or input paths in the HTML. |
| Component treemap | Longest matching `components[].path_prefix`; `.` root fallback. Area = component share of summed `function_cost_v2`. Top-50 production-first selection; residual becomes an `other` cell so areas still sum to the repo total. |
| One cost ramp | Single blue ramp for **production** cells. Test/bench/other cells are neutral gray + diagonal hatch (demoted, class-labelled), not a second metric. |
| Hot cells | Top-3 by cost get 1–3 rank badges and a dark outline; rank 1 also pulses. |
| Labels | Cell label + share/cost when area allows; `<title>` always carries label, class, cost, share and file count. |
| Production-first | Brief actions are production-first; test/bench fill only if needed (never in these three runs). The answer calls out when the map itself is test-dominated. |
| KV1 brief | 3–5 cards: rank, `path:line`, rule title, one-line why, severity/confidence, file cost + share. Distinct rules, max 5, `-ARCH-002` excluded. |
| Q2 drill | Component-level DSM of resolved **file-to-file** include edges; deterministic production-first order; 1-hop dependents and uses for the hottest production component. Clicking a treemap cell targets its component row. |
| Animation | Cells reveal in cost-rank order (`--i × 40 ms`), brief fades after the map, hot cell pulses; CSS checkbox + label pauses; `prefers-reduced-motion` and `@media print` show the complete final frame. No `<script>`, no event handlers, no network. |
| Final frame complete | Every cell, label, brief card, answer and the Q2 link are visible in the final/reduced-motion/print frame. Start/mid are evidence only. |

## Cost units and anchors (added before the A/B)

Every measured cost is printed as **`N static cost units (v2)`** (the canonical unit for
`function_cost_v2`, which is what the schema calls the metric). Percentages of the map and the
`93.5% of static weight is test code` line are shares, not performance metrics, and stay as shares.

Anchors are scan-relative and deterministic:

- **Treemap cell** — `N static cost units (v2) · M× median component · top P% of cells`.
  `median component` is the median cost over the scan components with cost > 0 (the base
  `components[]` set; tmux root files are not components in this median).
  `P = round(100 × cost rank / rendered cells)`, rank 1 = hottest.
- **Brief row / drill member file** — `N static cost units (v2) · M× median function · top P% of files`.
  `median function` is the median of all `function_cost_v2` values in the scan;
  `P` is the file's cost rank among files with cost > 0.
- **Residual `other` cell** — no median comparison; labelled `residual · top P% of cells` because it is
  an aggregate of omitted cells, not a component.

Observed medians: cJSON median component 24 / median function 2; hyperfine 202 / 7; tmux 2,528 / 20
(static cost units (v2)). These are intentionally tiny because most scan components/functions are
small; the huge cJSON multipliers are the test-cost outlier, not a display bug.

The treemap caption now states `area = static cost units (v2); colour = the same scale` and names the
anchor; cell `<title>`/aria carry the full unit + `× median component` + `top P%` even when the cell is
too small for a visible cost line. The manifest records `cost_unit` and `anchor_method`.

## Caps and thresholds

- `TOP_CELLS = 50` (production-first selection), plus one residual `other` cell.
- `MAX_ACTIONS = 5`; at most one action per rule; info findings stop selection once 3 stronger actions exist.
- Label threshold 4,200 px²; rank badge only when the cell is ≥40×34 px.
- Treemap 1360×404 inside a 1440×900 viewport; drill page is taller than 900 by design.
- Root expansion: if `.` holds >55% of cost and >40 files (tmux: 94.5%, 154 files), root files become
  cells so the map is legible. The DSM/drill stays component-level.

## Counts (current run)

| repo | components (scan) | rendered cells | actions | hottest cell | hottest production | prod share |
|---|---:|---:|---:|---|---|---:|
| cJSON | 21 | 18 | 4 | `tests/unity/test/tests` 93.53% (test) | `core (root)` 0.17% | 0.18% |
| hyperfine | 12 | 12 | 5 | `src` 35.66% | `src` 35.66% | 61.87% |
| tmux | 4 | 50 + `other` | 5 | `format.c` 6.84% | `format.c` 6.84% | 99.45% |

Include edges: cJSON 229 total / 105 resolved / 96 file-to-file aggregated / 124 unresolved / 9 to
non-repo modules; hyperfine 238 / 234 / 205 / 4 / 29; tmux 234 / 234 / 214 / 0 / 20.

Q2 focus answers: cJSON `core (root)` → tests, fuzzing; hyperfine `src` → 4 components
(src/benchmark, src/export, src/output, …); tmux `core (root)` → compat, fuzz.

## Data caveats (also in `manifest.json`)

1. **cJSON is dominated by test static cost.** One Unity test function
   (`tests/unity/test/tests/testunity.c:4557`, `function_cost_v2 = 2,000,008`) makes that component
   93.53% of the map; all production is 0.18%. This is honest to the metric and is the reason the
   answer/brief are explicitly production-first, but the production cells are visually tiny. F
   cannot make production legible on cJSON without changing the area contract.
2. **Component derivation is scan-derived path prefixes, not a true architecture.** Overlapping
   prefixes resolve to the longest match; `.` is the fallback. tmux's flat tree collapses to four
   components, hence the documented root-file expansion for the treemap only.
3. **Top-50 cap.** tmux's 107 omitted root files are folded into `other (107 cells)` at 17.2% of
   cost; the cell is hatched like other demoted cells. Other repos are uncapped.
4. **Include edges are partial.** Only resolved edges with both endpoints present in `files[]` are
   aggregated. Unresolved edges (`to = null`) and resolved imports naming non-repo modules (Python
   stdlib, numpy, `plot_utils`, etc.) are excluded from the DSM. Schema 1.7 has **no per-edge
   confidence field**, so there is no LOW-confidence edge tier to filter; the LOW/MEDIUM/HIGH tier
   exists only on findings.
5. **Static approximation.** `function_cost_v2` is a heuristic, not measured runtime. Parse health
   is partial on cJSON (38/101) and tmux (116/216); findings there are bounded by parse coverage.
6. **Brief selection.** `-ARCH-002` is excluded; LOW-confidence findings rank last and do not appear
   when stronger production actions exist. Severity is text/outline, not a second area encoding.
7. **No runtime, no cross-language linking, no source snippets.** F shows dependency weight and
   include coupling only; it does not prove a bottleneck or show call-argument context.

## What F cannot answer

- Which component is slow at runtime (static cost ≠ time).
- Whether a hot cell is actually executed (entry/reachability is only an outline in the brief).
- Production legibility on a repo whose static cost is one test outlier (cJSON), without breaking
  "area = cost share".
- True architectural boundaries when the scan's component taxonomy is coarse (tmux root).
- Cross-language coupling (C/Rust/Python analysed separately in schema 1.7).
- Any holdout claim — django/postgres were deliberately not read.

## Verification (exact commands, observed)

1. `python3 docs/evidence/map-vision/v7/f/render_f.py` → exit 0.
   `wrote f-{cJSON,hyperfine,tmux}.html + drill`; `captured …`; `wrote manifest.json (9 images)`.
2. `grep -c '<script' f-*.html` → `0` for all six HTML files (3 maps + 3 drills).
   `grep -l 'onclick' f-*.html` → none; `grep -l 'onload' f-*.html` → none;
   `grep -l 'https\?://' f-*.html` → none.
   Unit coverage: `grep -o 'static cost units (v2)' f-*.html | wc -l` → 48/41/119 in the three maps
   and 6 per drill; `grep -oE '[0-9.]+[kM]? cost|cost ·|cost [0-9]' f-*.html` → no bare cost. Every
   cell `<title>` contains the unit and either `× median component` or `top P% of cells`; residual
   `other` uses `top P% of cells`.
3. Playwright (chromium-1243, 1440×900, reduced motion) assertions, all exit 0:
   - cJSON: treemap=1, cards=4, drill link visible (bottom 878 ≤ 900), hidden cells=0, errors=[].
   - hyperfine: treemap=1, cards=5, drill link visible, hidden cells=0, errors=[].
   - tmux: treemap=1, cards=5, drill link visible, hidden cells=0, errors=[].
   - Drills: `svg.dsm=1`, `.focus-panel=1`, dependents lists present, errors=[].
4. Pause/reduced-motion/print (Playwright): clicking the label leaves 0 running animations and the
   timeline `currentTime` frozen across 350 ms; unchecking resumes; print shows cell opacity 1 and
   hides the pause control; reduced motion shows all cells at opacity 1.
5. Double run: `cmp` on `f-cJSON.html`, `f-cJSON-drill.html`, `f-hyperfine.html`,
   `f-hyperfine-drill.html`, `f-tmux.html`, `f-tmux-drill.html` → **byte-identical** (6/6).
6. PNGs: all nine are 1440×900; `manifest.json` sha256 matches each current file. Manifest now also
   records `cost_unit`, `anchor_method`, and per-repo `median_component` / `median_function`.

**Run-provenance exemption:** the `mid` PNG is a paused raster of a compositor frame; its sha256 can
differ between runs even though the HTML bytes do not. `manifest.json` therefore records raster
hashes that may change per run; the deterministic artifact is the HTML (and the final/start frames
were stable in these runs). No timestamps or absolute paths are embedded in any HTML.

## Blockers / next gates

- Blind `@vision` reads (two per repo, final frame, one image per task, no source/expected answer).
- F-vs-G A/B on the same three repos, then the founder A2 timed test.
- Only after a win: Rust implementation, tests/goldens, ADR-0002 amendment, byte-determinism CI,
  fresh vision gate. This prototype intentionally stops before any `src/` work.
