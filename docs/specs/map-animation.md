# Spec: map-animation

Scope: feature

# Spec: map-animation (feature)

## Purpose

Give the human map candidates (F weight-map, G entry-flow map) a **mandatory progression
animation** without adding JavaScript, network access, a new dependency, or breaking the
byte-determinism of the generated artifacts.

The animation is a presentation layer over a static, complete final frame. It must never be the
only carrier of information: everything visible mid-animation is visible in the final frame.

## Contract

- **No JavaScript.** The report/HTML contains no `<script>` and no event handlers. Animation is
  CSS keyframes and/or SVG SMIL only. The static SVG remains the canonical chart artifact.
- **Deterministic timeline.** Delays/durations are fixed constants derived from the view model
  (BFS depth, rank, path position). No randomness, no wall-clock input, no data-dependent timing.
  The generated HTML/SVG bytes are byte-identical across runs; the animation runtime is not part of
  the bytes.
- **Static final frame.** The final frame is the complete view (all nodes/cells, labels, numbers,
  captions). `prefers-reduced-motion: reduce` and `@media print` show the final frame with no
  motion. A vision/Playwright capture mode shows the final frame.
- **Pause/step without JS.** A visible CSS-only pause control (checkbox/label) freezes the
  timeline; optional radio/`:target` step controls may reveal the progression step by step. The
  control must be keyboard-accessible and labelled.
- **Animation content.** The timeline may reveal, in order: entry/root emphasis, BFS depth rings,
  edge draw-on, file cards, the numbered path trace, and the hottest node pulse. It must not move
  labels in a way that prevents reading; no element may obscure another after it appears.
- **One primary metric.** Animation is not a second encoding. The view keeps its one primary
  measured metric (cost); severity is an outline/marker; call edges are structural.
- **Vision gate.** Every animated view is vision-gated on its final frame (and optionally 3
  keyframes: start/mid/end). The transcript must state the question + one concrete action with no
  blocker; mid-animation frames are evidence, not the acceptance frame.
- **A2.** The founder timed test runs with animation enabled; the pause control and the static
  fallback must work. A founder who pauses at the final frame must still be able to name 3 actions.
- **Scope.** Animation applies to the human map's HTML report (and its candidate prototypes). It
  does not apply to the agent report, `metrics.json`, or the Explorer static SVG stems by default.
- **No new dependencies.** The renderer emits CSS/SMIL strings; no animation library.

## ADR consequence

ADR-0002 currently says static SVG, zero-JS. This spec requires a **clarifying amendment**:
artifacts stay static, offline, zero-JS and byte-deterministic; the HTML report may include
declarative CSS/SMIL animation over the static SVG with pause, reduced-motion and print fallbacks.
The static SVG remains the canonical artifact. The amendment is recorded before the winner ships;
prototypes may proceed under this spec.

## Acceptance

- A1: final-frame blind `@vision` read states the slot question + one concrete `path:line`/file
  action, no blockers.
- A2: founder timed test passes with animation; pause + reduced-motion + print show the complete
  final frame.
- A3: `grep -c '<script'` = 0 in the emitted report; no event-handler attributes; no network refs.
- A4: double-scan byte-identical for the emitted HTML/SVG (excluding run provenance).
- A5: no information exists only mid-animation; the final frame is complete.