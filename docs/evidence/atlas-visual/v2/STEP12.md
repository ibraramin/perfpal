# Step 12 — progressive below-graph charts

**Status:** implemented 2026-10-24 (Session A2); **step-13 founder visual gate still pending; A2/ship
(steps 14–15) remain blocked** (`R-pf-109`). **No schema/JS/dependency change.** F treemap candidate
prototyped; **F remains undecided** and the Explorer treemap is untouched.

## What was built

`report.html#map` now has exactly two layers:

1. **First page (unchanged from the approved step-10/11 composition):** masthead + key-metrics strip
   (ENTRY / HOTTEST / PATH / CAPS / three START actions / one structural caveat / one legend) above
   the 868px graph canvas and the 530px detail rail.
2. **Below the graph — four designed blocks** in `#atlas-charts`:
   - **Weight** — a 100% production/non-production share ribbon, top-5 production-class file bars,
     top-3 production component bars, and the labelled non-production band. Bars are scaled to the
     largest item in each column; every row prints its `% of scanned static cost`; the caption states
     the bar denominator. The ribbon and band print the same paired values from one integer-tenths
     split (the `atlas-metrics` §5 paired-share exception), so the pair sums to 100.0% and one
     measurement never prints two values. All-test scans lead with the band.
   - **Blast radius** — target (elected entry file), the M-08 dependents anchor, an
     include-coverage bar (`resolved of total include edges`), top dependent bars, and the
     structural caller/callee counts. No-entry states use a named stub. The unresolved-includes
     count lives once, in the canvas footer, not duplicated here.
   - **File detail** — top-6 production-first cost ladder: path, role (the elected entry is `ENTRY`),
     proportional bar, exact `file total … static cost units (v2) · × median function · top P% of
     files` (the shared positive-median anchor, so a zero-cost row never prints `0.0×`), top-3
     function links to `profile-queue.html#fn-…`, and the dominant finding link.
   - **Coverage/links** — one line + links to `report.md`, `metrics.json`, `atlas-weight.html`,
     `atlas-q2.html`, `findings-detail.html#ranked-actions`, `profile-queue.html`.

The seven-block text wall (answer/actions/weight/blast/cards/noise/links) is **retired**; the Q2 and
weight drills moved to standalone warm-paper pages (`atlas-q2.html`, `atlas-weight.html`). Rail file
links target the weight page. A8 is re-baselined: the strip above the canvas is the first
answer-bearing surface; the first below-canvas block is the weight chart (no duplicate answer line).
Every chart renders in all eight states with a named stub; no `<table>` and no `<script>` inside the
chart surface.

## Measured evidence (`step12.json`, canonical `#map`, reduced motion)

| Claim | hyperfine | cJSON |
|---|---|---|
| `#map` height (≤5,000) | **1,848** | **1,848** |
| canvas bottom (≤900) | **869** | **869** |
| brief/chart top ≥ canvas bottom | 879 ≥ 869 | 879 ≥ 869 |
| chart total / weight / blast / file / coverage | 961 / 244 / 115 / 545 / 27 | 961 / 244 / 115 / 545 / 27 |
| page height | 15,872 | 22,058 |
| 1440×900 horizontal scroll | none (1,440/1,440) | none (1,440/1,440) |
| 1280×800 horizontal scroll | none | none |
| rail stacks below graph at 1280 | yes | yes |
| chart text floor | 8.5px | 8.5px |
| weight bars positive | 8/8 (max 90px) | 8/8 |
| chart blocks | 4, exact order | 4, exact order |
| tables/scripts in charts | 0 / 0 | 0 / 0 |
| structural caveat / unit definition | 1 / 1 | 1 / 1 |
| artifact links | all resolve | all resolve |
| rail file links → `atlas-weight.html#…` | all resolve | all resolve |

The evidence script reads the existing real outputs only (no rescan), writes the screenshots
(`atlas-v2-step12-{hyperfine,cJSON}-{final,full,charts,1280x800}.png`), the hashes and every boolean
to `step12.json`, and fails closed. Run:

```bash
python3 docs/evidence/atlas-visual/v2/render_step12.py
```

## F treemap candidate (not shipped)

`step12/F-CANDIDATE.md` compares the shipped weight panel with the F treemap on the same two scans.
The prototype (`step12/f-treemap-{hyperfine,cJSON}.html` + PNGs) is evidence only. Headline: F reads
whole-repo component mass well on hyperfine, but on test-dominated cJSON production collapses to a
sub-1% sliver (the known F failure mode), while the W1 ranked rows keep production readable. **F stays
undecided; the Explorer treemap is untouched.** `W1-vs-F-{hyperfine,cJSON}.png` is the founder
side-by-side.

## Verification

| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | all suites, exit 0 |
| Licenses | `cargo deny check licenses` | `licenses ok`, exit 0 |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0 |
| Charts | `cargo test --locked --test atlas_charts` | 10/10, exit 0 |
| States/A8 | `cargo test --locked --test atlas_states` | 2/2, exit 0 |
| Rail | `cargo test --locked --test atlas_rail` | 9/9, exit 0 |
| A9/A12/A4 | `cargo test --locked --test atlas_render` | 5/5, exit 0 |
| No schema | `test -z "$(git status --short src/schema/ docs/schema/)"` | exit 0 |
| Evidence | `python3 docs/evidence/atlas-visual/v2/render_step12.py` | PASS, exit 0 |

Full command/exit transcript: `gate-step12.txt`. Goldens: the four `report.html` trees were
regenerated; the four `atlas-q2.html` / `atlas-weight.html` pages are new artifacts.

## Vision (step-13 proxy — not the founder gate)

`VISION-STEP12.md`: PASS on the final crops. The first viewport carries ENTRY/HOTTEST/START without
scrolling; the full map + rail + footer fit ≤900; the weight bars are visibly filled; the file cost
anchor is fully readable after moving it below the bar. Residuals are step-11 items (mid-elided canvas
node names, one rail card's long function row wraps/truncates) and the disclosed sparse cJSON state;
the founder gate judges those.

## Residuals / deferred

- **F undecided**; no F code shipped. The Explorer treemap and component data are untouched.
- **Step-11 residuals:** canvas basenames can mid-elide (`executi_.ets.rs`); one rail function row
  (`src/shell_syntax.rs`) wraps mid-identifier and loses a line number in the visible card (full value
  in `title`/profile queue).
- **Page height outside `#map`:** the legacy Explorer/report appendices remain; the step-12 budget
  targets `#map`.
- **Step-13 founder gate** is the next gate: 1440×900 composition + G-baseline diff + V8 A1/strip
  rerun. Only then A2.
