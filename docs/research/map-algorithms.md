# Map Algorithms — Layout Survey (R1)

Scope: read-only research feeding the maintainer birdseye map. Surveys layout
families, judges fit/misfit against Perfpal non-negotiables. Recommends nothing
final — idiom choice is R4's job (`docs/research/map-decision.md` + ADR).
Supersedes errored lib-2: redone exhaustively, not resumed from its partial.

## Non-negotiables (every layout must satisfy)

Sourced from local harvest (exp-1). Citations are file:line pointers.

- **Static-only / no-build.** Static analysis only; outputs are optimization
  candidates, never certainties (`perfpal-core-spec.md:24,42`; `NORTH-STAR.md:32`).
  No build flags / compile DB / macro expansion; non-compiling input is a core
  case (`ADR-0001:45-46`). Heuristics consume IR only; LOW-confidence cross-file
  edges stay out (`ADR-0001:21,25,65`).
- **Determinism / byte-identical.** Sort every traversal; identical inputs ⇒
  identical bytes (`AGENTS.md:19`; `ADR-0002:91-97`). Sorted iteration
  (path / (from,to) / (rank,prefix)), integer/fixed-point coords, C-locale
  numbers, no timestamps/IDs; colors pure fn of JSON (`ADR-0002:91-97`).
  Charts are pure functions of `metrics.json` only
  (`ADR-0002:91`); double-scan byte-identical goldens required
  (`birdseye-map.md:30,A3`).
- **Offline SVG/HTML + musl-static.** Hand-rolled zero-dependency deterministic
  SVG emitter; no chart/graph/layout lib in core (`ADR-0002:24,40-57`).
  Self-contained CLI: no Node/browser/Graphviz/Chromium; DOT export deferred
  post-MVP (`ADR-0002:13-18,38`). HTML zero-JS/offline, relative `img` links
  only. musl-static; `semantic-clang` default OFF, graceful absence first-class
  (`AGENTS.md:23`; `Cargo.toml:25`).
- **Caps / stubs / honesty markers.** Pre-layout caps: top-50 (ties path-asc),
  top-10 report rows, labels tail-kept 32ch (`maintainer-map.md:22`;
  `src/charts/mod.rs:73`; `src/reporter/mod.rs:46`). Every truncation marked
  `showing top S of M` / `…+N more`, never silent (`maintainer-map.md:25`;
  `birdseye-map.md:A4`). Empty scope = stub, never blank
  (`src/charts/layers.rs:500,520`). Omit: cross-component file edges
  (aggregated+captioned), LOW-confidence cross-file call edges, flame graphs,
  history/diff, JS layout (`maintainer-map.md:23`; `birdseye-map.md:40-41`).
  No force-directed full-corpus beyond ~1k entities; layered DAG + barycenter
  3-pass only (`maintainer-map.md:33`; `ADR-0002:34,75`). Every heat legend =
  candidate-density/cost — profile to confirm (`maintainer-map.md:26-27`).
- **Schema-additive-only.** Minor bumps additive only
  (`docs/schema/metrics-json-v1.md:46,54`). New charts = minor bump; rename /
  remove = major (`ADR-0002:97`; `output-contract.md:64,70`).

## Family 1 — Sugiyama layered (hierarchical, dot/Graphviz)

- **How it works:** cycle removal (reverse edges → DAG); rank assignment
  (network simplex / longest-path) sets Y; crossing minimization via
  layer-sweep + barycenter/median + transpose; x-assignment + spline routing
  (per `dotguide.pdf`).
- **Ceiling:** crossing minimization NP-hard; readable to ~50–200 nodes/edges;
  degrades into tall thin ranks beyond. Graphviz `dot` handles 1000s
  technically but illegibly.
- **Determinism:** deterministic given same sorted input — Graphviz uses input
  order as tie-break, no RNG by default. Byte-identical iff input traversal sorted.
- **Offline:** excellent — `dot -Tsvg` emits static SVG, no JS. Patterns:
  shell-out (rejected: no Graphviz in musl core) or re-implemented simple rank
  layout in Rust (allowed: hand-rolled emitter).
- **Fit:** STRONG FIT for call-graph / map skeleton under top-50 cap; TB/LR
  direction matches caller→callee intuition.
- **Misfit:** dense cycles need feedback-edge handling; tall-rank blowup without
  caps; shell-out violates self-contained CLI.
- **Sources:** https://graphviz.org/docs/layouts/dot/ ;
  https://graphviz.org/pdf/dotguide.pdf (Gansner et al. lineage: Sugiyama et
  al. [STT81], Warfield, Carpano).

## Family 2 — Force-directed (FR, ForceAtlas2, d3-force)

- **How it works:** repulsion (Coulomb-like) vs attraction; Barnes-Hut O(n log n);
  adaptive temperature / velocity-Verlet + alpha decay; gravity/scaling to compact.
- **Ceiling:** FR collapses past ~500–1k nodes; ForceAtlas2 targets 10–10k,
  explicitly not >100k without hours; d3-force similar. Interactive ticking
  freezes UI → needs worker.
- **Determinism:** historically random; modern d3-force deterministic by default
  (phyllotaxis init + fixed-seed LCG `randomSource`, seedable). Still
  float-iteration-order sensitive; byte-identical needs fixed tick count +
  sorted nodes + pinned floats.
- **Offline:** possible via bake pattern (`simulation.stop(); N×tick()` then
  serialize SVG — mbostock gist #1667139). But porting physics into musl-static
  Rust for top-50 is wasteful; no layout lib in core per ADR-0002.
- **Fit:** MISFIT as primary — non-deterministic heritage, CPU cost, hairball
  past cap. Only as pre-computed static coordinates baked to SVG, never live sim.
- **Misfit (hard):** violates byte-identical-by-construction and zero-dependency
  emitter rules; explicitly omitted full-corpus per `maintainer-map.md:33`.
- **Sources:** https://pmc.ncbi.nlm.nih.gov/articles/PMC4051631/ (doi
  10.1371/journal.pone.0098679); https://d3js.org/d3-force/simulation ;
  https://gist.github.com/mbostock/1667139

## Family 3 — Treemap + hierarchical edge bundling (HEB)

- **How it works:** space-filling enclosure of hierarchy (slice-dice →
  squarified → cushioned); adjacency edges as B-splines routed via
  least-common-ancestor; β∈[0,1] bundling strength; alpha blending reveals hotspots.
- **Ceiling:** treemap to 1k–10k leaves (squarified demo: 3060 employees); HEB
  demo 284 nodes + 1011 call edges. Past ~500 bundled edges, collinear overlap
  is a noted limitation — exactly what the top-50 cap solves.
- **Determinism:** fully deterministic if siblings sorted by size/name;
  squarify is greedy O(n), no RNG. Byte-identical trivially with integer geometry.
- **Offline:** excellent — rectangles + `<path>` splines are pure SVG,
  computable with integer/fixed geometry in Rust, no JS.
- **Fit:** STRONG FIT — hierarchy (dirs→files→fns) + calls is exactly a compound
  graph; best effort-to-value under musl-static + hand-rolled emitter.
- **Misfit:** none structural; only needs cap discipline + bundling-strength default.
- **Sources:** https://doi.org/10.1109/tvcg.2006.147 (PDF
  https://www.cs.jhu.edu/~misha/ReadingSeminar/Papers/Holten06.pdf);
  squarified https://cgl.ethz.ch/teaching/scivis_common/Literature/squarifiedTreeMaps.pdf

## Family 4 — Static flame graphs (folded stacks, offline)

- **How it works:** collapse stacks to `a;b;c count` folded lines; sort + merge;
  x ∝ sample count, y = depth; `flamegraph.pl folded > svg` with
  palette/minwidth filtering.
- **Ceiling:** 10k+ frames fine; `minwidth` drops sub-pixels; x-order
  alphabetical not temporal (unless flamechart). Depth ~20–50 readable.
- **Determinism:** deterministic — `sort` then merge; consistent palette via
  `palette.map`. Byte-identical iff input sorted.
- **Offline:** canonical static viz — single self-contained SVG readable on
  paper; zoom/search JS is progressive enhancement only.
- **Fit:** STRONG FIT for cost-weighted views (width ∝ static cost hint) but
  models stacks, not general graphs — needs synthetic "stacks" from static call
  chains. Cheap to emit from Rust.
- **Misfit:** current spec OMITS flame graphs (`maintainer-map.md:23`) — any use
  needs an R4 exception with honesty framing (static cost ≠ sampled time).
- **Sources:** https://github.com/brendangregg/FlameGraph ;
  https://www.brendangregg.com/flamegraphs.html

## Family 5 — Code-city / isometric (Wettel/Lanza)

- **How it works:** classes=buildings, packages=districts; polymetric mapping
  (NOM→height, NOA→footprint, LOC→color); rectangle-packing containment;
  disharmony maps color design flaws.
- **Ceiling:** evaluated JDK-scale + industrial (1000s classes); n=41 experiment
  +24% correctness, −12% time vs Eclipse/Excel.
- **Determinism:** deterministic iff packing input sorted; original Smalltalk
  tool is interactive 3D (rotation/occlusion).
- **Offline:** MISFIT in true 3D (needs WebGL/interaction). FIT only as 2.5D
  isometric SVG projection (parallelograms, painter's order) — no runtime JS,
  but loses navigation value.
- **Fit:** PARTIAL — evocative "map" framing + metric-mapping precedent, but 3D
  interaction violates static-only; prefer treemap/DSM unless isometric novelty
  is explicitly wanted.
- **Sources:** https://doi.org/10.1145/1370175.1370188 ;
  https://wettel.github.io/download/Wettel11a-icse.pdf ;
  demo https://wettel.github.io/download/Wettel08a-icse-tooldemo.pdf

## Family 6 — DSM / adjacency-matrix views

- **How it works:** rows=cols=subsystems, cell=dependency strength; partitioning
  (layering, SCC/cycle detection, reachability) reorders to block-diagonal;
  design rules overlay (`can-use`/`cannot-use`); in-place drill-down.
- **Ceiling:** highly scalable — matrix avoids edge-crossing blowup; cases to
  1000s files; n² cells tamed by aggregation + collapse. Cycles pop as
  off-diagonal marks.
- **Determinism:** deterministic given ordering algorithm + sorted names;
  partitioning is algorithmic, no RNG.
- **Offline:** excellent — plain `<table>` or SVG rects, zero layout physics,
  trivially byte-identical. Easiest musl-static target.
- **Fit:** STRONGEST FIT for the static constraint set — cycles, layering
  violations, densest couplings map naturally; less "map-like" visually but
  most honest for architecture bottlenecks.
- **Misfit:** visual only — reads as table, not landscape; pair with a
  spatial level (treemap/Sugiyama) for the "where do I look" question.
- **Sources:** https://doi.org/10.1145/1103845.1094824 (Sangal et al., Lattix);
  https://www.lattix.com/wp-content/uploads/2024/03/DesignRules.pdf ;
  https://docs.lattix.com/lattix/userGuide/Working_with_the_Dependency_Structure_Matrix_DSM.html

## Family 7 — Voronoi treemaps (sourced extra; held)

Polygons (not rects) via iterative Voronoi relaxation weighted by metric;
better aspect-ratio + boundary legibility; 1000s entities but "minutes not
seconds" (sampling-based); iterative relaxation is seed/epsilon sensitive —
weak byte-identical story; compute-heavy for a static Rust binary. Verdict:
MISFIT for R1, hold as future; prefer squarified.
Source: https://doi.org/10.1145/1056018.1056041 (also
https://doi.org/10.1109/infvis.2005.1532128).

## Fit/misfit matrix vs non-negotiables

| Family | Byte-identical | Offline SVG, no JS | musl / zero-dep | Top-50 cap solves scale | Honesty fit |
|---|---|---|---|---|---|
| Sugiyama (re-impl) | ✅ sorted input | ✅ static SVG | ✅ hand-rolled | ✅ ≤200 readable | ✅ direction honest |
| Force-directed | ⚠️ fixed ticks+seed | ✅ if baked | ❌ physics port | ⚠️ hairball risk | ⚠️ clusters imply false truth |
| Treemap+HEB | ✅ trivially | ✅ rects+splines | ✅ integer geom | ✅ cap cures overlap | ✅ density honest |
| Static flame | ✅ sorted merge | ✅ self-contained | ✅ cheap emit | ✅ minwidth | ⚠️ spec-omitted; cost≠time |
| Code-city 2.5D | ✅ sorted pack | ✅ projection | ✅ painter order | ✅ aggregation | ⚠️ height implies truth |
| DSM/matrix | ✅ ordering algo | ✅ table/rects | ✅ easiest | ✅ collapse | ✅ most honest |
| Voronoi | ❌ epsilon-seed | ✅ polygons | ❌ heavy | ✅ | ⚠️ boundaries imply precision |

Net rank under top-50 + byte-identical + no-JS:
**DSM ≈ treemap+HEB ≈ flame > Sugiyama > isometric-projection > force-directed > Voronoi.**

R4 (`map-decision.md`) maps these to Level 0/1/2 idioms; no level claims are made here.

## References

- Sugiyama lineage: Sugiyama et al. [STT81]; Gansner et al. [GKNV93] (`dotguide.pdf`).
- ForceAtlas2: Jacomy et al. 2014, PLoS ONE 10.1371/journal.pone.0098679.
- HEB: Holten 2006, IEEE TVCG 10.1109/tvcg.2006.147; squarified: Bruls et al.
- Flame: Gregg, FlameGraph repo + brendangregg.com/flamegraphs.
- CodeCity: Wettel & Lanza 2008/2011, ACM 10.1145/1370175.1370188.
- DSM/LDM: Sangal et al. 2005, ACM 10.1145/1103845.1094824; Lattix design-rules PDF.
- Voronoi treemap: Balzer–Deussen–Lewerewicz 2005, 10.1145/1056018.1056041.
