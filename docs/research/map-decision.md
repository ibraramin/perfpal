# Map Idiom Decision (R4 gate)

Scope: choose one layout idiom per map level for the map-first build, using
only evidence already gathered in R1–R3 [R1 §Fit/misfit matrix] [R2 §Verdict matrix] [R3 §4].
This document recommends; the binding choice lands in ADR-0008.

## 0. Scope and provenance (zero new external claims)

- This is gate R4 of the map-research track; R1 surveyed layout families,
  R2 tore down tools, R3 fixed honesty wording [R1 §Fit/misfit matrix] [R2 §Verdict matrix] [R3 §5].
- No page was fetched for R4; no new tool, algorithm, or paper claim appears
  below — every external fact cites the R1/R2/R3 section where it was
  already evidenced [R3 §8-dead-ends-convention].
- Cells in §2 cite R1 family sections or R2 tool sections only; local
  constraints cite repo files with file:line pointers [R1 §Non-negotiables].
- Anything not traceable to R1/R2/R3 or a local file is marked
  NOVEL SYNTHESIS inline; remainder conventions (`showing top S of M`,
  `…+N more`, aggregated edge counts) are novel synthesis per the R3 gap
  log, not fetched conventions [R3 §7].

## 1. Non-negotiables recap (all must hold for every idiom)

- Static-only, no-build: outputs are optimization candidates, never
  certainties; no build flags, compile DB, or macro expansion
  (perfpal-core-spec.md:24,42; NORTH-STAR.md:32; ADR-0001:45-50) [R1 §Non-negotiables].
- Heuristics consume IR only; LOW-confidence cross-file edges stay out of
  layout-affecting paths (ADR-0001:21,25,65) [R1 §Non-negotiables].
- Byte-identical: sort every traversal; integer/fixed-point coords, C-locale
  numbers, no timestamps or IDs; colors pure functions of JSON; charts are
  pure functions of metrics JSON only; double-scan byte-identical goldens
  required (AGENTS.md:19; ADR-0002:91-97; birdseye-map.md:30,A3) [R1 §Non-negotiables].
- Offline SVG/HTML plus musl-static: hand-rolled zero-dependency deterministic
  SVG emitter, no chart/graph/layout library in core; no Node, browser,
  Graphviz, or Chromium at user sites; DOT export deferred post-MVP
  (ADR-0002:24,40-57,13-18,38); HTML zero-JS/offline with relative img links
  only [R1 §Non-negotiables].
- `semantic-clang` default OFF; graceful absence of libclang is a
  first-class path (AGENTS.md:23) [R1 §Non-negotiables].
- Caps and stubs pre-layout: top-50 with ties broken path-ascending, top-10
  report rows, labels tail-kept to 32 characters (maintainer-map.md:22)
  [R1 §Non-negotiables].
- Every truncation is marked `showing top S of M` or `…+N more`, never
  silent (maintainer-map.md:25; birdseye-map.md:A4) [R1 §Non-negotiables].
- Empty scope renders a stub, never a blank [R1 §Non-negotiables].
- Omitted by spec: cross-component file edges (aggregated plus captioned),
  LOW-confidence cross-file call edges, flame graphs, history/diff views,
  JS-driven layout (maintainer-map.md:23; birdseye-map.md:40-41) [R1 §Non-negotiables].
- No force-directed full-corpus layout beyond roughly 1k entities; layered
  DAG plus barycenter 3-pass only (maintainer-map.md:33; ADR-0002:34,75) [R1 §Non-negotiables].
- Every heat legend reads candidate-density or candidate-cost with a
  profile-to-confirm instruction (maintainer-map.md:26-27) [R1 §Non-negotiables].
- Schema-additive-only: minor bumps add fields, never rename or remove;
  new charts arrive as minor bumps (ADR-0002:97) [R1 §Non-negotiables].

## 2. Decision matrix (candidate idioms × non-negotiables)

Each cell cites the R1 family section or R2 tool section that evidenced it.
Ordering signal only; assignment to levels is §3 (NOVEL SYNTHESIS built on
the cited cells).

| Candidate | Byte-identical | Offline (zero-JS SVG) | musl / zero-dep | Cap-safe (top-50) | Honesty |
|---|---|---|---|---|---|
| Sugiyama-native (hand-rolled rank/crossing/position) | pass: sorted input plus fixed flags is input-order deterministic [R1 §Family-1] | pass: static SVG output pattern [R1 §Family-1] | pass: hand-rolled rank/crossing/position, no shelled-out binary [R1 §Family-1] | pass: cap collapses crossing-minimizer cost [R1 §Family-1] | pass: edge direction is honest about hierarchy [R1 §Family-1] |
| d3-recipe force (phyllotaxis + fixed-seed + fixed ticks) | pass with harness: deterministic init plus fixed-seed generator plus fixed tick count [R1 §Family-2] | pass: baked final coordinates render statically [R1 §Family-2] | fail: requires a native Rust port of the force loop [R1 §Family-2] | pass: quadratic cost is trivial at 50 nodes [R1 §Family-2] | weak: clusters imply false grouping truth [R1 §Family-2] |
| Treemap + HEB | pass: pinned sort order plus bundling strength plus layout [R1 §Family-3] | pass: rects plus sampled spline polylines, interaction secondary [R1 §Family-3] | pass: pure integer geometry, no new deps [R1 §Family-3] | weak: dropping edges changes bundle thickness, which is itself the signal [R1 §Family-3] | pass: density read is honest with a cap caveat [R1 §Family-3] |
| DSM / matrix | pass: pinned row/column order plus hierarchy plus rule set (order must be pinned; matrix is order-dependent, not order-invariant) [R1 §Family-6] | pass: grid of rects plus text plus markers, natively static [R1 §Family-6] | pass: easiest of all families to hand-roll [R1 §Family-6] | pass: top-N subsystems plus overflow row/column keeps the matrix well-defined [R1 §Family-6] | pass: most honest of the surveyed families [R1 §Family-6] |
| Code-city 2.5D (isometric projection) | pass: sorted traversal plus fixed property mapping [R1 §Family-5] | pass: isometric 2.5D projection with zero JS [R1 §Family-5] | pass: pure geometry with painter order [R1 §Family-5] | pass: filter before layout keeps orientation [R1 §Family-5] | weak: building height implies a truth the static model cannot certify [R1 §Family-5] |
| Flame (held out) | pass with conditions: sorted input plus hashed colors plus fixed minwidth [R1 §Family-4] | pass with conditions: static image conveys the big picture only if zoom/search are dropped [R1 §Family-4] | pass: cheap emit, no libclang [R1 §Family-4] | pass: minwidth threshold plus documented pre-filter [R1 §Family-4] | held out: shows dynamic call profiles, not the static hierarchy, so spec omits it pending an exception with cost-not-time framing [R1 §Family-4] |
| Voronoi treemap (holdout) | fail: floating-point relaxation plus seed sensitivity [R1 §Family-7] | pass: polygons render statically [R1 §Family-7] | fail: iterative float geometry with cost scaling in iterations times nodes [R1 §Family-7] | weak: polygonal caps complicate truncation [R1 §Family-7] | weak: boundaries imply a precision the relaxation cannot certify [R1 §Family-7] |

Net fit-order carried from R1 (ordering, not assignment):
DSM ≈ treemap+HEB ≈ flame > Sugiyama > isometric > d3-recipe force >
unseeded force variants > Voronoi [R1 §Fit/misfit matrix].

## 3. Recommended idiom per map level (NOVEL SYNTHESIS from cited cells)

- L0 workspace (components as nodes, aggregated inter-component edges,
  heat = finding density; maintainer-map.md:16): DSM/matrix [R1 §Family-6].
  Coupling and blast-radius questions are answered by partitioned matrices
  with rule overlays, matching the two strongest precedent tools
  [R2 §tool-4] [R2 §tool-6]; overflow row/column absorbs the top-50 cap
  without distorting any signal [R1 §Family-6].
- L1 file (top files by degree, heat = per-file function cost, single
  outlined hottest file; maintainer-map.md:17): Sugiyama-native layered DAG
  (topological layering, barycenter 3-pass, deterministic cycle-break),
  reusing the existing file-DAG pipeline (ADR-0002:34) [R1 §Family-1].
  Matches what the shipped file-level artifacts already render
  [R2 §Overlap]; the cap collapses minimizer cost [R1 §Family-1].
- L2 component pages (intra-component files plus edges;
  maintainer-map.md:18): treemap for look-first ranking inside the component
  (size = cost, color = severity) with HEB drawn only for intra-component
  resolved edges at a pinned bundling strength [R1 §Family-3]. Bounding HEB
  to one component bounds the cap-distortion weakness, since dropped edges
  cannot leak across components [R1 §Family-3].
- Third layer, entry-stratified (birdseye-map.md F3 stem reusing resolved
  pairs and the break-cycles/topo-layers/barycenter pipeline with rank-prefix
  caps and symmetric stubs; birdseye-map.md:18): Sugiyama-native DAG
  stratified by entry reachability over resolved call edges only,
  LOW-confidence edges excluded and captioned (birdseye-map.md:11-13)
  [R1 §Family-1]. Depth caps follow the documented depth-limit behavior
  (include only modules within N steps) [R3 §2]; focus-matches plus direct
  neighbours with a depth control answer the daily-hard reachability class
  [R3 §4]; the phase view (input → calculation → output) correctly revealed
  architecture in the maintainer literature [R3 §P7].

## 4. Rejected alternatives (each with fetched reason)

- Unseeded force variants (FR/FA2 raw): explicitly non-deterministic and, for
  the continuous variant, deliberately without auto-stop, so byte-identical
  double-scan fails unmodified [R1 §Family-2].
- d3-recipe force for any level: clusters imply false truth, and musl-static
  still demands a native port of the force loop [R1 §Family-2]; explicitly
  omitted full-corpus per the map spec (maintainer-map.md:33) [R1 §Family-2].
- Voronoi treemaps: holdout stands — float relaxation plus seed sensitivity
  breaks byte-identical output, with cost scaling in iterations times nodes
  [R1 §Family-7]; revisit only if rectangular/isometric maps prove
  insufficient [R1 §Family-7].
- Code-city 2.5D for L0/L1: evocative but less information-dense per pixel
  than matrix/treemap for coupling questions, with occlusion needing a
  viewpoint choice [R1 §Family-5]; kept as a deferred alternative, not a level
  idiom.
- Flame graphs for any static level: held out because flame shows dynamic
  call profiles rather than the static file/call hierarchy, matching the
  spec omission (maintainer-map.md:23) [R1 §Family-4].
- Animated history maps (motion idiom): motion shows recency rather than
  importance, carries no dependency edges, and playback speed misleads as
  cost [R2 §tool-3]; maps stay static, quantified, and capped [R2 §tool-3].
- Picture-without-numbers maps: auto-updating diagrams with no quantified
  governance gate are the vitamin-not-painkiller failure mode; maps must
  carry numbers and CI gates, not pictures [R2 §tool-7].

## 5. Legend and caption strings (stolen verbatim from R3 §5, no new wording)

- Estimate banner: `pretty good estimate — static, profile to confirm` [R3 §5].
- Impossibility footnote for LOW-confidence edge captions: `No algorithm can
  generate a perfect call graph` (adapted to static C/Rust resolution) [R3 §5].
- Runtime-unknown edge label for unresolved calls [R3 §5].
- Loud-skip marker wording for skipped/ambiguous callees [R3 §5].
- Model-quality disclaimer: `cost hints reflect the static model, not
  silicon` [R3 §5].
- Sorted-axis honesty: `sorted by degree, not importance` (pattern stolen
  from the alphabetical-merge axis disclaimer) [R3 §5].
- Width honesty: `call counts unknown statically` [R3 §5].
- Relevance legend (hotspot heat): hotspot priority wording stolen verbatim
  for the heat key [R3 §5].
- Severity legend (finding-density heat): severity wording stolen verbatim
  for the heat key [R3 §5].
- Caps rule: `caps apply to pictures; counts and validation use full data`
  (stolen from the visualize-vs-validate warning) [R3 §5].
- Truncation marker: `…` plus full-string fallback in SVG title elements
  (stolen from the ellipsis-plus-tooltip contract) [R3 §5].
- Empty-state stub: `no files/components recorded — <why> — <next>`
  (stolen from the headline-plus-description-plus-next-step rule) [R3 §5].

## 6. Risks and gaps carried forward

- No fetched graph-viz source showed an in-chart remainder-node convention:
  `…+N more` remains NOVEL SYNTHESIS for the build [R3 §7].
- No fetched graph-viz source showed a `showing top S of M` caption
  convention: NOVEL SYNTHESIS for the build [R3 §7].
- No fetched source captions aggregated edges with counts (e.g. file-count
  times include-count rollups): NOVEL SYNTHESIS for the build [R3 §7].
- Layout idioms from the gallery families are uncovered (one provider
  rate-limited, tutorial redirects unexamined) [R3 §7].
- No fetched source quantifies a readability threshold for label hiding in
  static SVG (the 200-node LOD rule is interactive-only); the tail-kept
  32-character plus tooltip policy is synthesis [R3 §7].
- Uncertainty-encoding evidence is screen/interactive (gradient/violin);
  no fetched source tests transparency/width encodings inside static SVG
  architecture maps [R3 §7].
- R1 dead ends carry zero claims: Holten full-PDF demo counts unverified,
  squarified Big-O uncited, paywalled companions replaced by open mirrors,
  Wettel/Balzer full-PDF bytes unexamined, one graph-layout determinism
  thread unopened [R1 §Dead ends].

## 7. What R4 gates

- The map-build increments (entry/reachability, cost-v2, map layer,
  annotations, proof-plus-demo; birdseye-map.md:11-24) start only on this
  decision plus ADR-0008; if the demo test in birdseye-map.md A5 fails, the
  idiom is judged wrong, not the data (birdseye-map.md:35-37).
- If R4 rejects the third-layer idiom, birdseye-map.md is amended by a new
  ADR, never silently reinterpreted (birdseye-map.md:43-45).
