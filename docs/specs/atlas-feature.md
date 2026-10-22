# Spec: atlas-feature

Scope: feature

# Spec: atlas-feature (feature)

**Status:** planning lock, Atlas v1. **No implementation in this spec.**
**Date:** 2026-10-23. **Owner:** orchestrator.
**Plan:** `atlas-build`. **Core decision:** ADR-0009. **Animation:** ADR-0010.

## 1. What the feature is

Perfpal Atlas is the shipped human map: one page per scan whose first screen is an **entry-flow
graph** (G) and whose below-graph brief answers the three maintainer questions in plain language.
The F weight mass ships as an always-on supporting strip; the F treemap is an Explorer-only stem.
The page shape is identical for every scan — no mode switch.

Surfaces:

- `report.html#map` — the Atlas page: 100%-graph canvas + the fixed seven-block brief.
- Drills — Q2 reverse, complete action list, complete profile queue, weight detail, per-component
  pages.
- `#explorer` — the 8 legacy stems + `weight-treemap` (F), labelled Explorer-tier.
- Static SVG/HTML artifacts; no new JavaScript (the report keeps one fixed existing enhancement
  script); byte-deterministic.

## 2. Goals

1. A maintainer unfamiliar with a repo names 3 concrete actions from the Atlas page in under
   10 minutes (A2), on an application **and** a library-shaped repo.
2. The first screen answers Q1 without scrolling: entry, HOTTEST file, representative path.
3. Every number carries a unit and a denominator-adjacent anchor; no time units
   (`docs/specs/atlas-metrics.md`).
4. The library/test-heavy failure modes are handled by content, not by a hidden mode: the brief
   and weight strip carry the answer when the canvas is a dead end.
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
  repo; pause/reduced-motion/print work.
- **A12** — no new JavaScript: the Atlas canvas and animation blocks contain no `<script>` and no
  event handlers; the report's existing single fixed enhancement script (`REPORT_SCRIPT`, fixed
  source, no interpolated data, no network) is unchanged. `report.html` has exactly one script
  block; the canvas/SVG/animation artifacts have zero.
- **A4** — double scan → byte-identical HTML/SVG (excluding run provenance).
- **A7/A11** — no bare measured number; every `×`/`%` states its denominator; no time units.
- **A8 (amended)** — first block = canvas; first answer block = answer line below; the canvas
  carries entry + HOTTEST + path, and the one `path:line` action is in the first scroll.
- **A9** — canvas purity: no table/bar/card list/metric selector inside the canvas element.
- **A10** — state-unconditional: every state renders the same canvas + seven blocks.
- **A1b (retained)** — the known-answer recall guarantee of `map-intelligibility` still holds for
  KV1/KV2/KV3 through the brief and drills.
- **Dual review** — `@reviewer` + `@requirements-reviewer` APPROVED on the final tree.

## 5. Dependencies and gates

- ADR-0009 and ADR-0010 are **Accepted (founder ratification 2026-10-23)**; implementation proceeds
  under them and the V8 amendment set.
- `docs/specs/atlas-metrics.md` and `docs/specs/atlas-layout.md` are the binding contracts.
- The V8 amendments in `map-intelligibility` / `maintainer-map` / `map-animation` apply.
- The holdout (django/postgres) stays vaulted until the A2 test. The library-shaped second A2 class
  is pre-registered as the **published `memchr` crate payload** at commit
  `bd6068c30e9074a90c285e47912fa0b047d07597` (no root bin/build.rs/workspace; dev
  `benchmarks/`+`fuzz/` excluded from the scan; never used during design);
  see `docs/evidence/map-vision/v8/PREREGISTRATION.md`. Only a structural preflight may be recorded
  before A2.
- No schema change without a separate additive plan.

## 6. Rollback

- **A2 fails on both classes** → stop tuning; the feature is superseded by a new ADR (candidate:
  text-first brief or a different core view). The V4 list-first page remains in git history and can
  be restored for the report; no schema rollback is needed because v1 changes no schema.
- **Weight-strip falsifier fires** (no read uses it) → remove the strip; the rest of the page
  remains valid.
- **Canvas purity or determinism regressions** → block the release; the DoD gate must pass before
  shipping.

## 7. Evidence

- Vision reads, state matrix renders, DOM assertions, determinism logs and the A2 transcript are
  committed under `docs/evidence/map-vision/v8/`.
- Claims in the handoff pair with the runnable command and its result (evidence gates L4).