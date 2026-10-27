# Spec: atlas-feature

Scope: feature

# Spec: atlas-feature (feature)

**Status:** planning lock, Atlas v1; **implemented through step 14** (A2 delegated PASS, D-pf-255;
beta closure D-pf-257); step 15 documents the product contract. **No implementation in this spec.**
**Date:** 2026-10-23 (status 2026-10-26). **Owner:** orchestrator.
**Plan:** `atlas-build`. **Core decision:** ADR-0009. **Animation:** ADR-0010.

## 1. What the feature is

Perfpal Atlas is the product's human map (beta artifacts; publish gate founder-owned): one page per
scan whose first page is an **entry-flow graph** (G) with a small key-metrics strip and a detail
rail, and whose four designed below-graph charts answer the three maintainer questions in plain
language. The F weight mass ships as an
always-on chart; the F treemap candidate remains **deferred** (Explorer-only when built; F
undecided, D-pf-251). The page shape is identical for every scan — no mode switch. **The v1 seven-block brief was retired in step 12.**

Surfaces:

- `report.html#map` — the Atlas page: key-metrics strip + graph-only canvas + detail rail, then
  weight, blast-radius, file-detail and coverage/links charts. The page also links the LLM-facing
  `agent-report.md` / `agent-actions.json` (Track C) below the fold.
- Drills — `atlas-q2.html` (Q2 reverse), `atlas-weight.html` (complete weight detail), complete
  action list, complete profile queue, per-component pages.
- `#explorer` — the 8 legacy stems; the F treemap (`weight-treemap`) is a deferred candidate
  (Explorer-only when built; F undecided, D-pf-251).
- Static SVG/HTML artifacts; no new JavaScript (the report keeps one fixed existing enhancement
  script); byte-deterministic.

## 2. Goals

1. A maintainer unfamiliar with a repo names 3 concrete actions from the Atlas page in under
   10 minutes (A2), on an application **and** a library-shaped repo.
2. The first screen answers Q1 without scrolling: the strip carries entry, HOTTEST file and three
   START actions; the canvas carries entry, HOTTEST and the representative path.
3. Every number carries a unit and a denominator-adjacent anchor; no time units
   (`docs/specs/atlas-metrics.md`).
4. The library/test-heavy failure modes are handled by content, not by a hidden mode: the START
   strip and the weight/blast charts carry the answer when the canvas is a dead end.
5. Structural call edges are disclosed as structural; no runtime claim; the MEDIUM/LOW mix is
   printed.
6. No schema change: the Atlas ships on schema v1.7 fields with deterministic chart-side
   derivations (`docs/research/atlas-data-gap-audit.md`).

## 3. Non-goals

- Runtime truth, profiling data, time estimates, churn/history, ownership, cross-repo or
  cross-language views.
- Metric panels, tables, bars, metric selectors or a second map on the canvas.
- A data-keyed mode switch or an explicit fallback mode.
- A schema change (additive candidates are recorded with triggers, not added).
- New dependencies; JavaScript; network access; copyleft.

## 4. Acceptance (all must hold)

- **A1** — blind `@vision` reads: for every state (application, library, no-entry, empty,
  all-test, capped, partial-parse, single-file) the final frame states its question and one
  concrete `path:line`/file action; evidence committed under `docs/evidence/map-vision/v8/`.
- **A2** — founder timed test with animation enabled: 3 actions in under 10 minutes and
  "understandable" on the holdout (django/postgres) **plus** a pre-registered library-shaped
  repo; pause/reduced-motion/print work. **Execution record:** run 2026-10-24/26 as a delegated
  proxy on postgres + the `memchr` crate payload (4/4 blind lanes PASS; D-pf-255). **D-pf-257
  records that this delegated PASS closes the A2 gate for this beta;** the founder's own timed run
  remains the V6 gold standard and would need a fresh holdout because the holdout is now released.
- **A12 (as amended by ADR-0014)** — no new JavaScript in the Atlas artifacts: the Atlas canvas and
  animation blocks contain no `<script>` and no event handlers; they still add zero scripts.
  `report.html` has exactly three script elements (ADR-0014): the unchanged fixed `REPORT_SCRIPT`
  (fixed source, no interpolated data, no network), one fixed `STORY_SCRIPT`, and one inline
  `<script type="application/json">` data block for the always-embedded Atlas story. The story
  script/data are the only additions. **Status 2026-10-27:** amended by `ADR-0014`; the story is
  in-report and always on, `--story` only emits `atlas-layout.json`, and the canvas/SVG/animation
  artifacts are unchanged.
- **A4** — double scan → byte-identical HTML/SVG (excluding run provenance).
- **A7/A11** — no bare measured number; every `×`/`%` states its denominator; no time units.
- **A8 (amended; step-12 re-baselined)** — first block = key-metrics strip + canvas; the strip is
  the first answer-bearing surface (entry, HOTTEST, path, three START actions); the canvas carries
  entry + HOTTEST + path; the first below-canvas block is the weight chart.
- **A9** — canvas purity: no table/bar/card list/metric selector inside the canvas element.
- **A10** — state-unconditional: every state renders the same first page (strip + canvas + rail)
  and the same four below-graph charts.
- **A1b (retained)** — the known-answer **traceability** guarantee of `map-intelligibility` holds
  for KV1/KV2/KV3 through the START strip, the charts and the drills; the **severity-ranking recall
  remains open** (A1b is not claimed closed).
- **Dual review** — `@reviewer` + `@requirements-reviewer` APPROVED on the final tree.

## 5. Dependencies and gates

- ADR-0009 and ADR-0010 are **Accepted (founder ratification 2026-10-23)**; implementation proceeds
  under them and the V8 amendment set.
- `docs/specs/atlas-metrics.md` and `docs/specs/atlas-layout.md` are the binding contracts.
- The V8 amendments in `map-intelligibility` / `maintainer-map` / `map-animation` apply.
- The django/postgres holdout was vaulted until A2; A2 is done (delegated, D-pf-255; beta closure
  D-pf-257) and the holdout is released for that evidence only — do not tune against it. The library-shaped second
  A2 class was pre-registered as the **published `memchr` crate payload** at commit
  `bd6068c30e9074a90c285e47912fa0b047d07597` (no root bin/build.rs/workspace; dev
  `benchmarks/`+`fuzz/` excluded from the scan; never used during design);
  see `docs/evidence/map-vision/v8/PREREGISTRATION.md`. Only a structural preflight may be recorded
  before A2.
- No schema change without a separate additive plan.

## 6. Rollback

- **A2 fails on both classes** → stop tuning; the feature is superseded by a new ADR (candidate:
  text-first brief or a different core view). The V4 list-first page remains in git history and can
  be restored for the report; no schema rollback is needed because v1 changes no schema.
- **Weight-chart falsifier fires** (no read uses it) → remove the weight chart; the rest of the
  page remains valid.
- **Canvas purity or determinism regressions** → block the release; the DoD gate must pass before
  shipping.

## 7. Evidence

- Vision reads, state matrix renders, DOM assertions, determinism logs and the A2 transcript are
  committed under `docs/evidence/map-vision/v8/`.
- Claims in the handoff pair with the runnable command and its result (evidence gates L4).