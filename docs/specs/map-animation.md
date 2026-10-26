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

- **No new JavaScript.** The Atlas canvas/SVG/animation artifacts contain no `<script>` and no
  event handlers. Animation is CSS keyframes and/or SVG SMIL only. The static SVG remains the
  canonical chart artifact. The report already ships exactly one fixed enhancement script
  (`REPORT_SCRIPT`, pan/zoom/filter; fixed source, no interpolated data, no network); it is
  unchanged and outside the animation clause.
- **Deterministic timeline.** Delays/durations are a deterministic function of the view model
  (BFS depth, rank, path position) with fixed constants. No randomness, no wall-clock input.
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

ADR-0002 currently says static SVG, zero-JS for artifacts. This spec requires a **clarifying
amendment**:
artifacts stay static, offline, zero-new-JS and byte-deterministic; the HTML report may include
declarative CSS/SMIL animation over the static SVG with pause, reduced-motion and print fallbacks.
The static SVG remains the canonical artifact. The amendment is recorded before the winner ships;
prototypes may proceed under this spec.

## Acceptance

- A1: final-frame blind `@vision` read states the slot question + one concrete `path:line`/file
  action, no blockers.
- A2: founder timed test passes with animation; pause + reduced-motion + print show the complete
  final frame.
- A3: **no new JavaScript.** The Atlas canvas/SVG/animation artifacts contain no `<script>` and no
  event-handler attributes; no network refs. The report already ships exactly one fixed
  enhancement script (`REPORT_SCRIPT`, fixed source, no interpolated document data, no network);
  it is unchanged and outside the animation clause. The gate asserts exactly one block in
  `report.html` and zero in the canvas/SVG/animation artifacts.
- A4: double-scan byte-identical for the emitted HTML/SVG (excluding run provenance).
- A5: no information exists only mid-animation; the final frame is complete.

## Atlas application (2026-10-23)

**Ratified 2026-10-23** (founder; ADR-0010 Accepted; A12 scope confirmed).

The animation applies to the Atlas entry-flow canvas; the standalone drill pages are static (no
animation). The timeline is a deterministic function of the view model (fixed constants; no
randomness, no wall-clock input):
entry pulse 0–1.2s → depth-column reveal 1.2–2.8s →
edge draw-on 2.8–4.2s → path-trace numbering 4.2–5.4s → HOTTEST pulse 5.4s+. The CSS-only pause
control sits in the masthead above the canvas (outside `#atlas-canvas`);
`prefers-reduced-motion: reduce` and `@media print` show the
complete final frame. The final frame includes every node, label, number, caption and edge, the
key-metrics strip, the detail rail, and the four below-graph charts (`docs/specs/atlas-layout.md`
step-12 amendment; the v1 seven-block brief is retired).

Animation is presentation only: the canvas is readable with animation disabled, and no metric or
answer exists only mid-animation. The metric contract (`docs/specs/atlas-metrics.md`) applies to
every frame: no time units, units + anchors present, one measured encoding (cost). ADR-0010 records
the ADR-0002 amendment for the shipped renderer. The report's existing single fixed
`REPORT_SCRIPT` enhancement (pan/zoom + click-to-filter) is unchanged and is not part of the
animation; no new script is added for the Atlas.