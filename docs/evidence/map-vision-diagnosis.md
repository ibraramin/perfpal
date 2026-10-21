# Map-Intelligibility V0 — vision diagnosis (H1 data vs H2 visuals)

> Plan `map-intelligibility` V0 · spec `map-intelligibility` (two hypotheses) · tree `337ed93`
> + this burst · date 2026-10-21. Pre-registration committed first at `3982b03`
> (`docs/evidence/map-vision-prereg.md`). This document is the **diagnosis**, not a redesign:
> V1 (`docs/evidence/map-data-coverage.md`) does the recall test that decides H1 vs H2 per
> question.

## 1. Protocol actually run

| Item | Value |
|---|---|
| Corpora | tmux `c251547e` (C, 216 files), hyperfine `c516152` (Rust, 53), cJSON `6d9f244` (C, 101) |
| Renderer | Playwright (chromium 1243), viewport **1440×900**, `device_scale_factor=1` (100% zoom) |
| Artifacts | `report.html` first screen, `report.html#map`, and all 8 overview stems per repo = 30 PNGs in `docs/evidence/map-vision/v0/<repo>/` |
| Standalone stems | captured at natural size when the SVG exceeds 1440 px wide, so no chart is cropped for the vision read (`manifest.json`) |
| Reads | **28 blind reads** in `docs/evidence/map-vision/v0/reads.json`: one per view, two independent reads of `tmux#map` |
| Blind rule | image path only; no source, no sibling views, no expected answer; describe-then-judge |
| Model | `@vision` (image-only) |

### 1.1 The reading surface itself (measured, not inferred)

The report body is `max-width:72rem` (1152 px); charts sit in `figure.pz` with
`max-width:none; overflow-x:auto`. They are **not scaled down** — they are clipped to a
1152 px window with a horizontal scrollbar. Measured visible fraction:

| Repo | Stem | Natural width | Visible in report | Clipped |
|---|---|---:|---:|---|
| hyperfine | include-graph, layer-file, layer-findings | 3110 px | **37%** | 63% off-screen right |
| cJSON | layer-bands | 1994 px | **58%** | 42% |
| tmux | layer-bands | 1874 px | **62%** | 38% |
| hyperfine | layer-bands | 1634 px | 70.5% | 29.5% |
| hyperfine | layer-workspace | 1350 px | 85.3% | 14.7% |
| tmux/cJSON | most 1100 px stems | 1100 px | 100% | — |

So at the founder's canonical 1440×900, three of hyperfine's eight stems show barely more
than a third of the chart. The founder's "I can't understand anything" is at least partly
literal: **most of the chart is not on screen**.

## 2. Per-view result (28 blind reads)

`Q` = the maintainer question the read judged the view to answer; `action` = whether a
concrete action was named; `conf` = reader confidence. Full descriptions in `reads.json`.

### tmux

| View | Q served | Named action | Cannot answer | Verdict | conf |
|---|---|---|---|---|---|
| report#map (p1) | Q1 | open `fuzz/cmd-parse-fuzzer.c:50/67/68` | rule codes, `.` identity, runtime, edges, true positives | partial | 0.78 |
| report#map (p2) | Q1 | open `fuzz/cmd-parse-fuzzer.c:50` | where time goes in `.` (7326 findings), edge weights | partial | 0.70 |
| include-graph | Q1 | inspect `tmux.h` (single include hub) | edge labels/counts, legend, 156 hidden files | partial | 0.65 |
| layer-file | Q3 | profile `format.c` (max cost 30k) | time units, grouping, function drill-down | partial | 0.78 |
| layer-entry | Q3 | profile `format.c` | call graph (0 resolved), per-fn detail, strata (single flat column) | partial | 0.55 |
| component-coupling | Q2 | identify component `.` (only lit cell) | scale, values, axes | partial | 0.45 |
| hot-components | Q1 | profile `fuzz` (171.17) | `'.'` row name, file paths, runtime | partial | 0.90 |
| layer-workspace | Q1 | profile `fuzz` | file:line, `.` identity, ring encoding | partial | 0.75 |
| layer-bands | Q3 | profile `format.c` | edges never drawn, function detail, 187 omitted files | partial | 0.72 |
| layer-findings | Q1 | profile `format.c`, `window-copy.c` | heuristic/line, runtime, why hot | **answers** | 0.72 |

### hyperfine

| View | Q served | Named action | Cannot answer | Verdict | conf |
|---|---|---|---|---|---|
| report#map | Q1 | profile `src/output` (84.81), `src/benchmark` (82.74) | edge direction, runtime | **answers** | 0.82 |
| include-graph | **none** | open ARCH-001 (cycles not shown) | which pairs cycle, direction/weight, performance | **none** | 0.45 |
| layer-file | Q3 | profile `src/command.rs` (max cost) | numeric cost, functions, cycle semantics | partial | 0.75 |
| layer-entry | Q1 | profile `src/command.rs` | why hot, entry structure (0 resolved), absolute cost | partial | 0.85 |
| component-coupling | Q2 | inspect `src ↔ src/benchmark` cells | numeric scale, edge counts | partial | 0.72 |
| hot-components | Q1 | profile `src/output`, `src/benchmark` | runtime share, drill-down | partial | 0.80 |
| layer-workspace | Q1 | profile `src/output`, `src/benchmark` | node `4 .` label, broken cycles not drawn | **answers** | 0.82 |
| layer-bands | Q3 | profile `src/command.rs`, `benchmark/mod.rs` | absolute magnitude, ribbon endpoints, dot meaning | partial | 0.72 |
| layer-findings | Q2 | profile `tests/integration_tests.rs`, `benchmark/measure.rs` | per-finding counts, edge labels | partial | 0.70 |

### cJSON

| View | Q served | Named action | Cannot answer | Verdict | conf |
|---|---|---|---|---|---|
| report#map | Q1 | profile `tests` (151.43) / `example_2/test` (214.29) | per-finding detail, time, overlapping labels | partial | 0.72 |
| include-graph | Q1 | treat `cJSON.h` as highest blast radius | timing, clipped labels, 41 hidden files | partial | 0.62 |
| layer-file | Q3 | regenerate without unity/test files, then profile | truncated labels, hairball, test files crowd heat | partial | 0.60 |
| layer-entry | Q1 | profile `testunity.c`, `tests/parse_hex4.c` | function, call path (0 resolved), runtime | partial | 0.72 |
| component-coupling | **ambiguous** | inspect two dark-red cells | scale, title, clipped axes | partial | 0.45 |
| hot-components | Q1 | profile `example_2/test/test_runners` (214.29) | full paths, KLOC/absolute counts | partial | 0.72 |
| layer-workspace | Q1 | triage `example_2/test` node | time, clipped labels, per-finding detail | partial | 0.72 |
| layer-bands | Q3 | inspect red-hot file in `tests/unity/test/tests` | cost units, edge paths, full filenames | partial | 0.60 |
| layer-findings | Q1 | triage `cJSON_Utils.c` (61), tests nodes | root cause, time, ~20 clipped labels | **answers** | 0.80 |

**Score:** of 27 views, 5 were judged to answer one question (`answers`: 3× layer-findings,
hyperfine workspace + report#map), 21 `partial`, 1 `none` (hyperfine include-graph). Every
read that named an action named a **file or component**, never a function or line — except the
report table's top-finding links, which are plain text the reader has to decode.

## 3. Cross-cutting failures (evidence, not opinion)

1. **Q1's first screen points at noise.** tmux#map ranks `fuzz` first (171.17) and the table
   leads with `fuzz/cmd-parse-fuzzer.c`; cJSON#map ranks `tests` (151.43) and an
   `example_2/test` component (214.29). The pre-registered first look is `input.c`
   (`input_parse`, tmux) and `cJSON.c` (`parse_number`, cJSON). A reader who obeys the first
   screen lands in test/fuzzer code. This is `R-pf-099`/`R-pf-100` expressed visually.
2. **Q3 is answered by cost heat whose top node is not a hot path.** The three Q3 reads all
   named the max-cost file: `format.c` (tmux — I5 triage says FP/dispatcher), `src/command.rs`
   (hyperfine — startup-only), `testunity.c` (cJSON — test noise). The views have no units and
   no anchor (`× median`, `top 1%`), only raw cost and a colour ramp.
3. **Q2 is only answered by the include graph, never by the map.** `include-graph` surfaced the
   pre-registered "do not touch" hubs (`tmux.h`, `cJSON.h`) as convergence nodes; the
   `#map`/`workspace` view and the coupling matrix did not. tmux's coupling matrix has exactly
   one lit cell (`.`×`.`), no legend and no scale; cJSON's is "ambiguous"; hyperfine's has no
   numeric scale. `layer-bands` computes cross-component edges but "never draws them", only
   counted footnotes.
4. **Labels are the weakest link.** Reads flagged truncated/overlapping/garble in 18 of 28
   reads: left-clipped cJSON paths (~20 labels), tmux band 2 `2. …`, hyperfine node `4 .`,
   workspace label overflow, `..ts/unity/...`. Filename truncation makes the ranking
   unactionable even when the heat is right.
5. **The dot encoding does not survive at 100%.** All 10 tmux reads and 7 more flagged the
   severity fill / confidence ring / count badges as too small to decode (6 px dots,
   overlapping counts). The legend explains an encoding the reader cannot resolve.
6. **No view decodes rule IDs.** `PERF-C-ARCH-003`, `PERF-C-LOOP-002`, `PERF-PY-ALLOC-003`
   appear as opaque codes in the only table that carries file:line; the appendix is a separate
   page. A maintainer cannot tell "loop" from "alloc" without leaving the view.
7. **Legends are duplicated; canvas is wasted.** The same finding-dots legend is printed inside
   and below the workspace panel (tmux); hot-components occupies the top ~20–40% of a 900 px
   frame; the coupling matrix sits in the top-left corner of a mostly blank canvas.
8. **The report clips wide charts instead of scaling them.** 37% visible for three hyperfine
   stems, 58–62% for layer-bands (measured §1.1). The founder's first encounter with a
   3110 px chart is its left third.

## 4. Preliminary H1/H2 (V1 decides)

| Question | Data exists? | Surfaced by a view? | Preliminary |
|---|---|---|---|
| Q1 where to look first | Yes — findings + `function_cost_v2` + density per KLOC | Yes, but **mis-ranked** (test/fuzzer first) and file-only | **H2 (visual/ranking)**, with `R-pf-099/100` as the ranking defect |
| Q2 what breaks if touched | Includes: yes. Resolved call edges: **0 in default C** (semantic tier needed); LOW edges excluded by design | Includes: only in `include-graph`; never in the answer-first map. Calls: no legible view | **H2 for includes** (aggregate/legibility); **H1 for call-based blast radius** in the default C tier |
| Q3 where time plausibly goes | Static cost exists (`function_cost_v2`); **no runtime weights** (by design) | Yes, but the top node is a test/startup/FP and there are no units/anchors | **H2 (surface/ranking)**, with a real H1 tail only if "time" is read as measured runtime |

**The pre-registered prediction was confirmed and sharpened:** the cost answer is *surfaced but
mis-ranked* (H2 by ranking), and the blast-radius answer is not H1 for includes — the data is
there and the include graph works — but it is never promoted to the answer-first map. The
missing-field list is therefore short and additive; V1 enumerates it with schema paths.

## 5. What this does **not** prove

- Vision reads are model reads, not the founder's timed test (A2). They are a diagnostic, and
  the sample is 3 repos / 27 views.
- One read per view (two for tmux#map) is enough to see the pattern; it is not the two-read
  A1 gate for the future key views (that is V5).
- No claim is made that the data is sufficient for a *good* map; only that these views fail in
  identifiable, fixable ways rather than failing because the fields are absent. V1 tests that.

## 6. Evidence index

- `docs/evidence/map-vision-prereg.md` — answer key (commit `3982b03`)
- `docs/evidence/map-vision/v0/reads.json` — 28 blind reads (compact, machine-readable)
- `docs/evidence/map-vision/v0/manifest.json` — natural vs rendered sizes, PNG dimensions
- `docs/evidence/map-vision/v0/<repo>/*.png` — 30 rendered views at 1440 width / 100% zoom
- Repro: `python3 /var/tmp/opencode/render_v0.py` + `render_v0_stems.py` (session-local; the
  committed PNGs + manifest are the artifact)
