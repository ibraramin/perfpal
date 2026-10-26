---
plan name: atlas-build
plan description: Ship the Atlas map page
plan status: done
---

> **DONE ON MAIN 2026-10-26 (`D-pf-258`) — this file is a pre-merge branch snapshot.** Main closed
> steps 11–15: step 11 `189df91`, step 12 `9a94905`, step 13 `86c0537` (`D-pf-253`), step 14
> `9631137` (`D-pf-255`), step 15 `57d91a2` (`D-pf-256/257/258`). `R-pf-109` is closed as a record
> on main. **Do not execute steps 11–15 on `feat/atlas-sim`; do not treat the historical status
> lines below as current.** The story track is `atlas-s4` (done); the branch reconciliation is
> `R-pf-112` / `docs/adr/ADR-0012-atlas-branch-reconciliation.md`. Main's copy of this plan is
> authoritative.
> **New agents: read `docs/atlas/AGENT-BRIEF.md` first.** The founder's 2026-10-23 model supersedes
> the "100%-graph canvas" wording in this plan: the first page is the **main graph + some key
> metrics**; below it, reveal more key charts slowly; restore G's visual language (warm palette,
> sans chrome + mono data, masthead); F stays undecided. Do not run A2 until the founder visual gate
> passes.
>
> **v2 composition amendment (2026-10-23):** the founder's model supersedes the v1 composition
> clauses (the "100%-graph canvas" + fixed seven-block brief wording in the Idea). The data,
> honesty, entry, metric, no-schema, no-JS and determinism contracts are unchanged. **Easy to
> understand beats exhaustive metrics:** the first page is the **main graph + a small key-metrics
> strip** (entry, hottest file, 3 actions, one caveat); below it, a few **designed charts** are
> revealed progressively (weight, blast radius, file detail; F treemap candidate). No 3,000px text
> brief, no duplicate definitions, no metric wall. Restore G's visual language (warm
> paper/charcoal/burnt-orange, sans chrome + mono data, masthead, detail rail). **Steps 9–13 are the
> visual rework; A2/ship move to 14–15.** Steps 1–8 remain green.

## Idea
Build and ship Perfpal Atlas: the entry-flow core graph (G) as a full-bleed 100%-graph canvas, with the fixed seven-block below-graph brief (answer line, KV1 actions, weight strip, blast radius, file cards, noise footer, drill links), the F weight mass as an always-on below-graph strip, the F treemap as an Explorer-only stem, and no mode switch. This plan implements the decisions of ADR-0009 (Atlas core) and ADR-0010 (declarative animation), the metric contract docs/specs/atlas-metrics.md, the composition docs/specs/atlas-layout.md, and the V8 amendments to map-intelligibility/maintainer-map/map-animation. Data decision: NO schema change in v1 — every element is an existing v1.7 field or a deterministic chart-side derivation (docs/research/atlas-data-gap-audit.md); I/O sinks are downscoped; loop-I/O findings stay in KV1. Honesty: the canvas draws the default-tier structural call graph (LOW name-based + MEDIUM semantic; 0 resolved edges exist in the default tier), labels it "structural (LOW name-based + MEDIUM semantic)", prints the MEDIUM/LOW mix, never prints "resolved"/"reachable_from_entry" for that set, and never claims runtime truth. Entry policy: production main, then production script (Python `__main__`), else highest-fan-in production lib-root labelled "library root (no production main)"; else a NO ENTRY stub — never abort. Findings ride as text annotations on the canvas nodes (M-18, `HOT ×N`/`WARN ×N`/`INFO ×N`), satisfying the north-star's "findings as annotations" without a second encoding. All blocks are state-unconditional; the page shape is identical for application, library, no-entry, empty, all-test, capped, partial-parse and single-file scans. The report's existing single fixed enhancement script (`REPORT_SCRIPT`, pan/zoom/filter) is unchanged; the Atlas adds no new script (A12). The holdout (django/postgres) stays vaulted until the founder A2 timed test; a library-shaped repo is pre-registered (A-pf-116) as the second A2 class. This is a build plan: it starts only after founder ratification of ADR-0009/ADR-0010, and it is done only when the Definition of Done below passes with evidence committed.

## Implementation
- Ratification gate — **DONE 2026-10-23**: founder ratified ADR-0009 (Atlas core) and ADR-0010 (declarative animation) plus the V8 amendments to map-intelligibility/maintainer-map/map-animation; D-pf-239/D-pf-240 confirmed; ADR statuses set to Accepted; ratification recorded in state/session-state.json and the handoff. The library-shaped A2 holdout is pre-registered (`docs/evidence/map-vision/v8/PREREGISTRATION.md`). Next: execute steps 2–4 in a fresh session. No implementation before this step completed.
- **DONE 2026-10-23** (commit `fe58299`; 29 unit tests) — View model in src/reporter/atlas.rs: pure deterministic functions over MetricsDocument — entry election (production main, then production script (Python `__main__`), else highest-fan-in production lib-root, else NO ENTRY stub) with alternatives, structural adjacency from graphs.call_graph.edges with a MEDIUM/LOW confidence split, BFS depth/parent/reachable over the full pre-cap structural edge set, node and edge caps with aggregated boxes, file cost (sum function_cost_v2), component grouping/hulls and production-first cost shares, dominant finding rule + per-file finding annotation count, top functions, blast-radius counts from graphs.include_graph reverse edges, coverage/noise counts from parse_health/parse_health_summary and confidence, and the representative path trace (BFS-tree leaf by cost↓/depth↓/path asc, then parent-chain). Reuse src/reporter/views.rs ScaleOrdering for the median value and ranks (the shipped median is lower-middle for even N); sort every traversal; unit tests for entry matrix, caps, path determinism and confidence split.
- **DONE 2026-10-23** (commit `f76a63b`; 20 unit tests) — Metric formatting module in src/reporter/atlas_metrics.rs: implement docs/specs/atlas-metrics.md — units (static cost units (v2)/(v1)), '× median function' + 'top P%' anchors, '% of scanned static cost' shares, dependent-file blast-radius anchors, thousands separators, <10× one decimal vs ≥10× integer, stub strings, and the no-time-unit/no-runtime-claim rule. Unit tests: huge multiple (83,354×), zero median, zero total cost, v1 fallback, percentile ties, stub text, and no emitted 'ms'/'µs'/'runtime' claim in any string.
- **DONE 2026-10-23** (commits `3fa7c07`, `5e6b45e`, `6c0f0f3`; 27 unit tests) — Canvas renderer in src/reporter/atlas_render.rs: deterministic SVG/HTML for the 100%-graph canvas — depth columns, file nodes (name, cost, × median function, HOTTEST, M-18 finding annotation), forward/back structural edges with distinct-edge labels, component hulls with the approximate-prefix disclosure, unreached column, numbered representative path trace, legend with units and the structural caption, caps chip and canvas footer. Graph chrome only (no cards/tables/bars/metric selectors). CSS/SMIL animation (entry pulse, depth reveal, edge draw-on, path trace, HOTTEST pulse), CSS-only pause, prefers-reduced-motion and print final frames. Zero new <script>/event handlers/network; the report's existing fixed REPORT_SCRIPT is unchanged. Byte-identical double render.
- Steps 2–8 are **green** (2026-10-23; steps 2–6 code commits `672e4b6` + `a2a4c99` + `d31f283`; step 7/8 V8 fix series; full gate + review rounds 1–2 + V8 panel). Steps 9–10 (founder A2, ship) remain. **Deviation recorded:** the `weight-treemap` Explorer stem is **deferred** — the session constraint kept the Explorer section unchanged; the weight detail links to `#explorer` and no treemap exists in v1 yet. Component mass rows use the chart-side approximate prefix groups; per-component page links use the **schema component index** (so they resolve). The forbidden-claim scan is scoped to the Atlas brief/canvas; legacy drill pages are unchanged. M-17 (functions in the page header) and M-02 (answer-line reachability) remain a doc reconciliation follow-up. **Step 7 golden reconciliation:** no cJSON/hyperfine/tmux `report.html` goldens exist in the repo (those repos are V7 prototype HTML only); the committed golden set is the four fixture `report.html` trees, all regenerated. **V8 fixes:** the gate found and fixed legibility/collision defects (except the disclosed capped framing, deferred to A2) (body width, SVG text floors, two-line headers, compact edge pills, hull clearance, first-screen key line, capture hygiene); only the four `report.html` goldens changed.
- **DONE 2026-10-23** (commits `672e4b6`, `a2a4c99`, `d31f283`; 17 unit tests) — Brief and drills renderer in src/reporter/atlas_brief.rs: the fixed seven below-graph blocks (answer line; KV1 actions; weight strip with production-first top files/components + labelled non-production (test/bench/other) band; blast radius with include dependents + structural caller/callee counts + coverage; file cards with top functions and dominant finding; noise/confidence footer; drill links) plus the Q2 reverse drill (1-hop dependents, component DSM incl. headers, structural callers/callees, include coverage), complete action list + complete profile queue links, complete weight detail, and per-component page links. Uses `AtlasView`/`atlas_metrics`; M-08 prints `blast.dependent_top_pct` verbatim; additive `AtlasFile`/`AtlasFunction` view fields (no schema). Rule messages/titles are sanitized so no forbidden time claim is emitted, while identifiers are preserved. The treemap was **deferred** (see the deviation above).
- **DONE 2026-10-23** (commit `672e4b6`) — Wired report.html#map in src/reporter/mod.rs: canvas first, brief below, first answer-bearing block = answer line; nav/breadcrumbs labelled Atlas; the V4 list-first first screen removed (KV1/KV2/KV3 content now rides in the brief or the existing drill artifacts). Explorer section and the single fixed `REPORT_SCRIPT` unchanged. Integration tests landed early: `tests/atlas_render.rs` (A9 canvas purity, A12 no-new-JS, double render) and `tests/atlas_states.rs` (A10 eight-state stack, A8 canvas→answer first screen incl. canvas entry/HOTTEST/path); the V4 `#map` tests in `key_views.rs`/`i4_annotations.rs` were migrated. The four `report.html` goldens were regenerated (only report.html changed).
- **DONE 2026-10-23 (step 7)** — Tests and goldens: the full plan list is covered by 106 Atlas unit tests + 5 integration locks + the 8-state fixture matrix + double-render determinism + A9/A12/A4. `tests/atlas_vision.rs` (ignored) renders the eight full report pages on demand for the V8 matrix. The four committed `report.html` goldens are current (`PERFPAL_REGEN_GOLDEN=1 ... regenerate_goldens` is a no-op); no cJSON/hyperfine/tmux goldens exist in the repo (plan wording reconciled). `src/schema/` and `docs/schema/` are unchanged; report.html has exactly one existing `REPORT_SCRIPT`; the canvas/SVG/animation artifacts have zero scripts/handlers/network.
- **DONE 2026-10-23 (step 8)** — Vision gate (V8): the eight states were rendered at 1440×900 final frames (`docs/evidence/map-vision/v8/html`, `*-final.png`, `manifest.json`), with two fresh blind `@vision` reads per state (16/16 A1 pass) and two strip-specific reads per state (16/16 pass; the weight-strip inclusion rule holds). Evidence: `docs/evidence/map-vision/v8/reads.json` + `NOTES.md`. Blockers found by the gate were fixed except the disclosed capped framing (the depth-1 column is taller than the 1440×900 viewport; 20 of 26 shown/6 aggregated; deferred to A2) and the affected states re-read; > library and capped are included. A2 moves to step 14 after the visual rework (steps 9–13).
- **Step 9 — G-baseline audit + founder composition decision (design first; no implementation).** Read `docs/evidence/map-vision/v7/g/g-hyperfine-final.png` + `g-hyperfine.html`, `docs/evidence/map-vision/v7/COMPARISON.md`, and `docs/evidence/atlas-audit/AUDIT.md`. Put G and the current Atlas side by side at 1440×900. The founder agrees the visual system (warm paper/charcoal/burnt-orange; sans chrome + mono data; masthead/wordmark; detail rail) and the first-page composition (main graph + a **small** key-metrics strip: entry, hottest file, 3 actions, one caveat; no full brief). Decide the below-graph chart set and whether the F treemap is a candidate. Record the decision + G-baseline diff under `docs/evidence/atlas-visual/v2/`. **Do not start pixel work before this sign-off.**
- **Step 10 — First-page fit + visual language + chrome hygiene.** At 1440×900 the main graph is fully visible: `#atlas-canvas` bottom ≤ 900 including header, legend and footer; no clipping, no horizontal scroll; the answer/action line in the first scroll. Restore G's palette/typography/masthead; add the key-metrics strip; keep the canvas free of tables/cards/selectors. One definition each: one unit definition, one structural caveat, one legend, one footer. Remove duplicate caveat/median lines; fix the clipped footer, leading-dot truncation (`..tity/statistics.rs`), the amber/orange edge-label hue collision (path only on the path/selected edges; counts in `<title>`), and the unreached label ambiguity. **Easy to understand beats exhaustive metrics:** no 3,000px brief, no metric wall.
  - **Step 10 — IMPLEMENTED 2026-10-23; step-10 founder check APPROVED 2026-10-24; step-13 final gate pending.** First-page fit, G
    palette/type/masthead, the key-metrics strip and chrome hygiene landed; evidence
    `docs/evidence/atlas-visual/v2/` (decision, G diff, 1440×900 renders, DoD). The "no 3,000px
    brief" sentence in step 10 above is **step 12** (the progressive-chart rework); step 10 keeps the
    brief and does not increase page height. Fit evidence covers hyperfine/cJSON; capped/deep states
    exceed 900px by design (V8-disclosed) and are deferred to step 13/A2. Committed 2026-10-24
    (`468d772` + docs/state); no push.
- **Step 11 — Detail rail / reachable detail.** Restore a per-file detail layer (rail, overlay, or a designed below-chart) so functions and finding IDs are reachable without leaving the page. The canvas alone is not enough; the current 3,367/3,219px brief is not the answer. Keep the canvas graph-only; the rail carries related easy-to-read detail.
- **Step 12 — Progressive key charts.** Below the graph, add a few **designed charts** (not text tables), revealed progressively: weight (production-first share; F candidate), blast radius, file detail; evaluate the F treemap candidate. Move Q2/weight drills to separate pages. Budget `#map` at ~4–5k px (down from 8,982/14,685) and keep the page focused. Do not delete F data; F remains undecided.
- **Step 13 — Founder visual gate + G-baseline diff + V8 rerun.** Render 1440×900; the founder signs off the composition. Add DOM/render assertions: canvas bottom ≤ 900 including header/legend/footer; no clipped text; no duplicate definitions; `#map` height ≤ ~5,000px; no leading-dot truncation; no horizontal scroll. Re-run the V8 A1 + strip reads on the reworked page. Only then A2.
- **Step 14 — Founder A2 (blocked until step 13):** pre-register the library-shaped holdout repo (A-pf-116; single-file C lib or Rust lib crate with no bin) alongside the vaulted django/postgres holdout; the registered holdout is the **published `memchr` crate payload** at `bd6068c3` (dev `benchmarks/` + `fuzz/` excluded; see `docs/evidence/map-vision/v8/PREREGISTRATION.md`); run the timed test with animation enabled (3 actions in under 10 minutes, understandable, pause/reduced-motion/print work), record the transcript and the verdict in docs/evidence/map-vision/v8/. If A2 fails on both application and library classes, stop and escalate a superseding ADR instead of tuning.
- **Step 15 — Ship/docs/EOL (blocked until step 14):** update the output contract and README for the Atlas page, add the CHANGELOG entry, archive the V7 prototypes' role, run the Definition of Done below, run @reviewer + @requirements-reviewer on the final tree, mark the plan done, update state/session-state.json + state/handoff/latest.md + docs/activeContext.md, and commit.

## Definition of Done

All commands run from the repo root. **Every command must exit 0** (the task is NOT done otherwise).

1. `cargo fmt --all -- --check` — exit 0.
2. `cargo clippy --locked --all-targets -- -D warnings` — exit 0.
3. `cargo test --locked` — exit 0 (includes the Atlas unit/integration suites).
4. `cargo deny check licenses` — exit 0.
5. `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` — exit 0.
6. `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` — exit 0.
7. `cargo test --locked --test atlas_render atlas_double_render_is_byte_identical` — exit 0 (double scan → byte-identical HTML/SVG, modulo run provenance).
8. `cargo test --locked --test atlas_render atlas_adds_no_new_script` — exit 0 (report.html has exactly one existing `REPORT_SCRIPT` block; the canvas/SVG/animation artifacts have zero; no event-handler attributes).
9. `cargo test --locked --test atlas_render atlas_canvas_has_no_metric_panels` — exit 0 (A9: no table/bar/card list/metric selector inside the canvas element).
10. `cargo test --locked --test atlas_states atlas_states_render_same_stack` — exit 0 (A10: all eight states render the same canvas + seven blocks).
11. `test -z "$(git status --short src/schema/ docs/schema/)"` — exit 0 (no schema change).
12. `test -f docs/evidence/map-vision/v8/reads.json && test -f docs/evidence/map-vision/v8/a2.md` — exit 0 (vision + A2 evidence committed).
13. `@reviewer` and `@requirements-reviewer` both APPROVED on the final tree.

### v2 visual-gate DoD (before A2)

14. `docs/evidence/atlas-audit/` is committed; the founder composition sign-off + G-baseline
    side-by-side are recorded under `docs/evidence/atlas-visual/v2/` at 1440×900.
15. First-page assertions pass: `#atlas-canvas` bottom ≤ 900 including header/legend/footer; no
    clipped text; no duplicate definitions; `#map` height ≤ ~5,000px; no leading-dot truncation; no
    horizontal scroll.
16. The founder visual gate is APPROVED and the V8 A1 + strip reads are re-run on the reworked page.
    Only then may A2 (step 14) run.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- evidence-beta-spec
- divergence-study
- maintainer-map
- map-intelligibility
- map-animation
- atlas-metrics
- atlas-layout
- atlas-feature
<!-- SPECS_END -->
