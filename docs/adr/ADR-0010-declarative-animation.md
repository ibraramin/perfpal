# ADR-0010: Declarative animation over the static SVG (ADR-0002 amendment)

- **Status:** Accepted (founder ratification 2026-10-23)
- **Date:** 2026-10-23
- **Ratified:** 2026-10-23 (founder)
- **Deciders:** Founder + orchestrator
- **Gates:** `atlas-build`; amends `ADR-0002` (static SVG, no new JavaScript) with a clarifying clause; does
  not change ADR-0002's emitter decision or license audit.

## Context

- `docs/specs/map-animation.md` makes a **progression animation mandatory** for the human map:
  the animation is the story on the entry-flow map (BFS progression + path trace), it must work
  without JavaScript, and it must never be the only carrier of information.
- ADR-0002 accepted a **hand-rolled deterministic SVG emitter** with no chart/graph library, no
  external runtime, and byte-identical output; it did not anticipate animation. The V7 prototypes
  proved that CSS keyframes and SVG SMIL satisfy the contract: 3 start/mid/final frames per repo,
  CSS-only pause, reduced-motion/print final frames, zero `<script>`, zero event handlers,
  byte-identical double render (`docs/evidence/map-vision/v7/{f,g}/NOTES.md`).
- The founder wants the progression animation on the shipped Atlas (D-pf-237/D-pf-238); the
  static SVG remains the canonical chart artifact for determinism and offline use.

## Decision

1. **Artifacts stay static, offline and byte-deterministic; no new JavaScript.** The static SVG remains the
   canonical chart artifact; `metrics.json` remains the source of truth.
2. **The HTML report may include declarative animation** over the static SVG: CSS keyframes and/or
   SVG SMIL only. **No new `<script>`** and no event-handler attributes for the animation; no
   network references, no animation library, no new dependency. The report already ships exactly
   one fixed enhancement script (`REPORT_SCRIPT`, pan/zoom + click-to-filter; fixed source, no
   interpolated document data, no network). It is **unchanged and out of scope** of this
   animation clause; the zero-new-JS gate asserts exactly one existing block and zero script in the
   Atlas canvas/SVG/animation artifacts.
3. **The animation is a presentation layer over a complete final frame.** Nothing is visible only
   mid-animation; the final frame contains every node, label, number, caption and the full brief.
4. **Deterministic timeline.** Delays/durations are a deterministic function of the view model
   (BFS depth, rank, path position) with fixed constants; no randomness, no wall-clock input. The
   emitted bytes are identical across runs.
5. **Controls and fallbacks.** A visible CSS-only pause control (checkbox/label) freezes the
   timeline and is keyboard-accessible and labelled. `prefers-reduced-motion: reduce` and
   `@media print` show the final frame with no motion. A capture mode (Playwright) shows the final
   frame for vision gates.
6. **Scope.** The animation applies to the Atlas HTML report and its drills' key views; it does not
   apply to `metrics.json`, the agent report, or the Explorer static SVG stems by default.

## Ratification

Founder ratification **2026-10-23** (Perfpal Atlas ratification session; **D-pf-240 confirmed**).
The decision above is accepted as written, with these recorded confirmations:

- **A12 no-new-JS scope confirmed.** The report keeps exactly **one fixed existing
  `REPORT_SCRIPT`** (unchanged, out of scope); the Atlas canvas/SVG/animation artifacts add
  **zero scripts**, zero event-handler attributes and zero network references. CSS keyframes and/or
  SVG SMIL only.
- **Presentation-layer rule confirmed:** nothing is visible only mid-animation; the final frame is
  complete; pause, `prefers-reduced-motion` and print show the final frame.
- **Deterministic timeline confirmed:** delays/durations are a pure function of the view model;
  double render stays byte-identical.
- **Scope confirmed:** Atlas HTML report and its drills' key views; not `metrics.json`, the agent
  report, or the Explorer static SVG stems by default.

## Options considered

- **JavaScript animation (e.g. requestAnimationFrame) — rejected.** Breaks the no-new-JS rule, offline and
  byte-determinism; adds a runtime surface the product does not have.
- **Pre-rendered GIF/video — rejected.** Large binary artifacts, no pause semantics, no
  reduced-motion/print control, weaker accessibility, no text scaling.
- **No animation — rejected.** The founder made progression animation mandatory and the V7 evidence
  shows the animation *is* the story on the entry-flow map (`COMPARISON.md` "Animation value").
- **CSS keyframes / SVG SMIL over the static SVG — chosen.** Proven in both V7 prototypes; no new
  dependency; pause/reduced-motion/print are CSS features; bytes stay deterministic.
- **What would change the choice:** a required animation effect that CSS/SMIL cannot express
  deterministically and accessibly; or a determinism/print defect the vision gate cannot resolve.

## Consequences

**Positive**

- The core feature carries its progression story without violating the no-new-JS rule, offline or
  determinism.
- The static SVG stays the canonical artifact for diffs, LLM consumption and print.
- No dependency, no license surface, no runtime.

**Negative / mitigated**

- Animation adds CSS/SMIL bytes to the HTML → determinism test covers double-render equality; the
  timeline is a pure function of the view model.
- Mid-animation screenshots can look incomplete → the vision gate judges the **final frame**;
  keyframes are evidence only.
- Pause/step without JS is CSS-only → a checkbox/label control is mandatory and keyboard
  accessible; `:target` stepping is optional.

## Gates

- `map-animation` A1 (final-frame blind read), A2 (founder timed test with animation), A3
  (no new JavaScript: the report keeps exactly one existing fixed `REPORT_SCRIPT`; the
  canvas/SVG/animation artifacts have zero scripts; no handlers; no network), A4 (double-scan
  byte-identical), A5 (no information only mid-animation).
- The Atlas build must pass the same gates plus the layout spec's A9/A10.
- Ratified by the founder on 2026-10-23; the Atlas renderer may ship animation under A1–A5 and the
  layout spec's A9/A10.

## References

- `docs/specs/map-animation.md` · `docs/adr/ADR-0002-visualization.md` ·
  `docs/specs/atlas-layout.md` §7–§8 · `docs/evidence/map-vision/v7/{f,g}/NOTES.md`.
