# ADR-0008: Map Idiom Decision (R4 gate)

- **Status:** Proposed
- **Date:** 2026-10-17
- **Deciders:** Founder + orchestrator
- **Gates:** Map-build increments I1–I5 (entry/reachability, cost-v2, map
  layer, annotations, proof-plus-demo; birdseye-map.md:11-24); amends
  birdseye-map.md F3 if the third-layer idiom is rejected
  (birdseye-map.md:43-45).

## Context

- The product direction is map-first: one maintainer birdseye map answering
  look-first, blast-radius, and plausible-cost questions, with findings
  riding as annotations on the map, never as bare lists
  (maintainer-map.md:7-13).
- Research lanes R1 (layout families plus fit matrix), R2 (tool teardown
  plus verdict matrix), and R3 (cost idioms, caps, papers, stealable
  legends, gaps) are done; all evidence below cites R1/R2/R3 sections or
  local file:line pointers, with zero new external claims
  [R1 §Fit/misfit matrix] [R2 §Verdict matrix] [R3 §5].
- Build increments I1–I5 are gated on this decision: research plus ADR gate
  must land before F1 starts (birdseye-map.md:43-45).
- Non-negotiables binding every idiom: static-only with candidates never
  certainties (perfpal-core-spec.md:24,42); byte-identical double-scan
  (ADR-0002:91-97; birdseye-map.md:30,A3); offline zero-JS SVG plus
  musl-static with no chart/graph/layout library in core
  (ADR-0002:24,40-57); pre-layout top-50 caps with ties path-ascending and
  marked truncation (maintainer-map.md:22,25); spec-Omit list including
  cross-component file edges, LOW-confidence cross-file call edges, flame
  graphs, history/diff, and JS layout (maintainer-map.md:23;
  birdseye-map.md:40-41); schema-additive-only (ADR-0002:97)
  [R1 §Non-negotiables].

## Decision

- L0 workspace: DSM/matrix with pinned row/column order, overflow
  row/column for the top-50 cap, and rule-overlay markers [R1 §Family-6].
- L1 file: Sugiyama-native layered DAG reusing the break-cycles,
  topo-layers, and barycenter 3-pass pipeline with rank-prefix caps
  (ADR-0002:34; birdseye-map.md:18) [R1 §Family-1].
- L2 component pages: treemap for in-component look-first ranking with HEB
  drawn only for intra-component resolved edges at a pinned bundling
  strength [R1 §Family-3].
- Third layer (entry-stratified, birdseye-map.md F3): Sugiyama-native DAG
  stratified by entry reachability over resolved call edges only,
  LOW-confidence edges excluded and captioned (birdseye-map.md:11-13)
  [R1 §Family-1]; depth caps and focus-plus-neighbours follow the
  documented cap idioms [R3 §2]; phase-view framing follows the maintainer
  literature [R3 §P7].
- Legends and captions reuse the stolen strings verbatim (estimate banner,
  model-quality disclaimer, sorted-axis and width honesty, relevance and
  severity keys, caps-rule, truncation marker, empty-state stub) [R3 §5].
- Remainder conventions (`showing top S of M`, `…+N more`, aggregated edge
  counts) are marked NOVEL SYNTHESIS per the gap log [R3 §7].

## Alternatives Considered

- Unseeded force variants (FR/FA2 raw) — rejected: explicitly
  non-deterministic with no auto-stop, failing byte-identical double-scan
  unmodified [R1 §Family-2].
- d3-recipe force — rejected for all levels: clusters imply false truth and
  musl-static still requires a native port of the force loop [R1 §Family-2];
  full-corpus force layout is explicitly omitted (maintainer-map.md:33)
  [R1 §Family-2].
- Voronoi treemaps — holdout stands: float relaxation plus seed sensitivity
  breaks byte-identical output, with cost scaling in iterations times nodes
  [R1 §Family-7].
- Code-city 2.5D for L0/L1 — deferred: less information-dense per pixel
  than matrix/treemap for coupling questions, with occlusion needing a
  viewpoint choice [R1 §Family-5].
- Flame graphs — held out: dynamic call profiles rather than the static
  hierarchy, matching the spec omission (maintainer-map.md:23)
  [R1 §Family-4].
- Animated history maps — rejected: recency is not importance, no edges,
  playback speed misleads as cost; maps stay static, quantified, capped
  [R2 §tool-3].
- Picture-without-numbers maps — rejected: diagrams without quantified
  governance gates repeat the vitamin-not-painkiller failure; maps carry
  numbers and CI gates [R2 §tool-7].
- Commercial DSM-plus-rules affordances (triangles, queryable rules,
  hotspot-times-severity gates) are adopted as patterns only, not
  products [R2 §tool-4] [R2 §tool-6].

## Consequences

Positive:

- Every level idiom satisfies byte-identical, offline, musl, and cap-safe
  columns of the decision matrix [R1 §Fit/misfit matrix].
- L0 answers blast-radius with the most-honest surveyed family [R1 §Family-6].
- L1 reuses the shipped file-DAG pipeline instead of inventing a new one
  [R2 §Overlap].
- L2 bounds the HEB cap-distortion weakness to one component [R1 §Family-3].
- The third layer directly serves the daily-hard reachability class
  [R3 §4].

Negative:

- DSM loses click-to-expand interactivity; mitigated with pre-rendered
  per-subsystem sections, and monochrome/print-safe markers need a legend
  that survives printing [R1 §Family-6].
- HEB bundle thickness stays cap-sensitive even bounded to one component;
  every L2 chart carries the caps-rule caption [R3 §5].
- Isometric and force idioms remain unbuilt; a future milestone may revisit
  them only through a new ADR with fresh evidence.
- Reversal trigger: the birdseye-map.md A5 demo test governs — if a
  maintainer unfamiliar with the repo cannot name 3 concrete actions from
  the map alone in under 10 minutes, the idiom is judged wrong, not the
  data (birdseye-map.md:35-37).

## Gates

- Binds map-build I1–I5: no F1 entry/reachability work starts until this
  ADR leaves Proposed (birdseye-map.md:43-45).
- If the third-layer idiom is rejected during build, birdseye-map.md F3 is
  amended by a new ADR, never silently reinterpreted (birdseye-map.md:43-45).
- Amendments to level idioms after acceptance require a superseding ADR
  citing the A5 demo outcome or new R-lane evidence.
