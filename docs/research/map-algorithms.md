# Map Algorithms — Layout Survey (R1, evidence-backed redo)

Scope: read-only research feeding the maintainer birdseye map. Surveys layout
families, judges fit/misfit against Perfpal non-negotiables. Recommends nothing
final — idiom choice is R4's job (`docs/research/map-decision.md` + ADR).

Provenance: this is a REDO (2026-10-17). The first version cited sources without
opening them; every factual claim below traces to a page actually fetched on
2026-10-17 by three parallel research lanes. Pages that could not be opened are
listed under Dead ends and support no claim.

## Non-negotiables (every layout must satisfy)

Sourced from local harvest (exp-1). Citations are file:line pointers.

- **Static-only / no-build.** Static analysis only; outputs are optimization
  candidates, never certainties (`perfpal-core-spec.md:24,42`; `NORTH-STAR.md:32`).
  No build flags / compile DB / macro expansion; non-compiling input is a core
  case (`ADR-0001:45-50`). Heuristics consume IR only; LOW-confidence cross-file
  edges stay out (`ADR-0001:21,25,65`).
- **Determinism / byte-identical.** Sort every traversal; identical inputs ⇒
  identical bytes (`AGENTS.md:19`; `ADR-0002:91-97`). Sorted iteration,
  integer/fixed-point coords, C-locale numbers, no timestamps/IDs; colors pure
  fn of JSON (`ADR-0002:91-97`). Charts are pure functions of `metrics.json`
  only (`ADR-0002:91`); double-scan byte-identical goldens required
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

## Family 1 — Sugiyama layered (hierarchical, Graphviz `dot`)

- **Mechanism.** `dot` renders directed graphs as hierarchies in four phases:
  "break any cycles… assigns nodes to discrete ranks… orders nodes within ranks
  to avoid crossings… sets X coordinates… routes edge splines. This is the same
  general approach as… Sugiyama [STT81]"
  [graphviz.org/pdf/dotguide.pdf, 2026-10-17]. It "aims edges in the same
  direction (top to bottom, or left to right) and then attempts to avoid edge
  crossings and reduce edge length"
  [graphviz.org/docs/layouts/dot/, 2026-10-17].
- **Scale ceiling.** No hard node-count ceiling is published. The documented
  practice is to shrink the graph first ("sccmap… tred… gvpr… unflatten") when
  labels become unreadable, plus cost knobs `mclimit / nslimit / searchsize`
  [dotguide.pdf]. Only concrete datapoint fetched: the SML-NJ module graph
  rendered in "0.23 seconds… on a 3 GHz Intel Xeon" [dotguide.pdf].
- **Determinism.** Input-order deterministic, not randomized: "The underlying
  library keeps nodes and edges in their input order… scan (in order) to find
  all nodes with indegree 0 and enqueue them (maintaining that order)" and "We
  don't randomize initial node placement because we see the input order of
  nodes as being very useful information"
  [forum.graphviz.org/t/dot-implementation-questions/1951, 2026-10-17].
  User-visible control: 'If `ordering="out"`, then the outedges… must appear
  left-to-right in the same order in which they are defined in the input'
  [graphviz.org/docs/attrs/ordering/, 2026-10-17]. Caveat: "layouts are not
  necessarily stable with respect to changes in the input graph. One last
  adjustment can invalidate all previous changes" [dotguide.pdf].
- **Offline static-SVG.** Native, zero-JS: `-Tsvg`, `svgz`, `plain`, `canon`
  output formats, filter-mode CLI [dotguide.pdf].
- **Fit.** Static-only ✅, offline SVG ✅, byte-identical given sorted input +
  fixed flags ✅, top-50 cap collapses crossing-minimizer cost ✅.
- **Misfit.** musl-static with no shelled-out Graphviz ❌: `dot` is an external
  C binary, so use requires a native Rust reimplementation of the
  rank/crossing/position pipeline, not `dot` itself.

## Family 2 — Force-directed (FR 1991, ForceAtlas2, d3-force)

- **Mechanism (FR).** A modification of the Eades spring-embedder: "replace the
  vertices by steel rings and replace each edge with a spring… let go so
  that… forces… move the system to a minimal energy state"; repulsion all-pairs,
  attraction neighbours only; per iteration attract, repulse, "limit the maximum
  displacement by the temperature" then cool
  [courses.engr.illinois.edu/cs519/fa2017/rf.pdf (open FR 1991 copy),
  2026-10-17]. Forces fa(d)=d²/k, fr(d)=−k²/d, cancelling at ideal distance k.
- **Scale (FR).** Basic iteration Θ(V²) (repulsion dominates); grid-variant
  Θ(V)+|E| "assuming… uniformly distributed". Authors ran a fixed "50
  iterations every time"; "Many… drawn in less than a second, and all… under 10
  seconds on a SPARC station 1… 'interactive' speed" for moderate graphs — no
  10k-node guarantee [same FR PDF].
- **Determinism (FR).** NOT seeded: coincident vertices separate in a "randomly
  chosen orientation" [same FR PDF]. Byte-identical output needs sorted
  traversal + seeded RNG + fixed iterations added on top — none in the paper.
- **Mechanism (ForceAtlas2, Jacomy et al. 2014).** "Nodes repulse each other
  like charged particles, while edges attract their nodes, like springs…
  converges to a balanced state"; combines "Barnes Hut simulation,
  degree-dependent repulsive force, and local and global adaptive temperatures".
  Deliberately "continuous (no phases, no auto-stop)" for live view; rejects
  simulated annealing, auto-stop, phased/multilevel coarsening
  [journals.plos.org/plosone/article?id=10.1371/journal.pone.0098679,
  2026-10-17].
- **Scale (FA2).** "All-around solution to Gephi users' typical networks
  (scale-free, 10 to 10,000 nodes)"; "good performances for network of fewer
  than 100000 nodes, while keeping it a continuous layout" [same PLOS page].
  Continuous cost means Perfpal must impose its own stop (fixed ticks).
- **Determinism (FA2).** Explicitly "not deterministic, and… varies depending on
  the initial state… can get stuck in a local minimum"; "stops exclusively at
  the user's request" [same PLOS page]. Fails byte-identical double-scan
  unmodified; needs fixed init + fixed ticks + seeded jitter.
- **Mechanism (d3-force).** Velocity-Verlet integrator, "constant unit time
  step Δ=1… constant unit mass m=1". Each tick: interpolate alpha toward target
  by alphaDecay, run forces, apply velocityDecay, advance positions; natural run
  is "⌈log(alphaMin)/log(1−alphaDecay)⌉; by default, this is 300"
  [d3js.org/d3-force/simulation, 2026-10-17].
- **Determinism (d3 — strongest of the three).** Missing positions init "in a
  phyllotaxis arrangement, so chosen to ensure a deterministic, uniform
  distribution"; randomSource "defaults to a fixed-seed linear congruential
  generator" [same d3 page]; "Two generators instanced with the same seed
  generate the same sequence" with "long period (up to 1 billion numbers)"
  [d3js.org/d3-random, 2026-10-17]. Manual static run is `stop()` + fixed-count
  `tick()` loop.
- **Static-SVG pattern (proven).** Bostock's recipe: `.stop()`, then run "a
  fixed number of times, and then display it once" —
  "Rather than updating the graph with each tick, we run the graph a fixed
  number of times, and then display it once"
  [gist.github.com/mbostock/1667139, 2026-10-17]. Large graphs "should be
  computed in a web worker" (100-node gist vs 1000-node worker; no absolute
  ceiling published) [d3 simulation docs].
- **Fit.** Static-SVG serializable (final x/y only) ✅; top-50 cap makes O(n²)
  trivial ✅; offline zero-JS output ✅.
- **Misfit.** musl-static forbids shelling to Node/Gephi/Graphviz — requires a
  native Rust port of the force loop ❌; only d3-force ships a ready
  determinism recipe (phyllotaxis + fixed-seed LCG + fixed 300 ticks + sorted
  input) — FR/FA2 need that harness built ❌. Explicitly omitted full-corpus
  per `maintainer-map.md:33`.

## Family 3 — Treemap + hierarchical edge bundling (HEB)

- **Mechanism.** Treemap = containment hierarchy as nested space-filling
  rectangles; HEB bends "each adjacency edge, modeled as a B-spline curve,
  toward the polyline defined by the path via the inclusion edges from one node
  to another" [doi 10.1109/tvcg.2006.147 (Holten 2006), 2026-10-17]. d3's
  bundle layout: "For each input link, a path is computed that travels through
  the tree, up the parent hierarchy to the least common ancestor, and then back
  down to the destination node"
  [github.com/d3/d3-3.x-api-reference/Bundle-Layout.md, 2026-10-17]; Vega:
  "Dependency curves are routed along the tree path between source and target
  nodes in the package hierarchy" [vega.github.io/vega/examples/edge-bundling/,
  2026-10-17]. Bundling strength β∈[0,1]: "Low bundling strength mainly
  provides low-level, node-to-node connectivity information, whereas high
  bundling strength provides high-level information as well"; "a fairly high
  bundling strength (β = 0.8) can be chosen to retain visual bundles while still
  resolving ambiguity" [cs.jhu.edu Holten06 PDF excerpts, 2026-10-17].
  Squarified subdivision is greedy row-wise: "Either the rectangle is added to
  the current row, or the current row is fixed and a new row is started… This
  decision depends only on whether adding a rectangle to the row will improve
  the layout of the current row or not", motivated because optimal tessellation
  "falls in the category of NP-hard problems" [vanwijk.win.tue.nl/stm.pdf,
  2026-10-17].
- **Scale ceiling.** Demonstrated reuse scale is hundreds of leaf nodes (the
  canonical flare-imports HEB demo shared by the d3 gist and Vega example)
  [mbostock gist 5672200; Vega example]. Holten's own paper-scale node/edge
  counts could NOT be verified (full PDF exceeded fetch limit — dead end).
- **Determinism.** Squarified is deterministic given sorted input — "The order
  in which the rectangles are processed is important. We found that a
  decreasing order usually gives the best results" [stm.pdf]. HEB geometry is
  deterministic given fixed node positions, β, and spline model ("We settled on
  a piecewise cubic B-spline representation"); endpoint positions depend on the
  tree layout, so byte-identical output requires pinning layout + sort order + β
  [Holten06 excerpts].
- **Known limitation (quoted).** "the large number of collinear nodes within
  these layouts… causes bundles to overlap along the collinearity axes" and "we
  currently consider the bundle overlap in case of layouts with a large number
  of collinear nodes to be the biggest problem of hierarchical edge bundles"
  [Holten06 excerpts].
- **Offline static-SVG.** Good: pure geometry (rects + B-splines sampled with
  "M ≥ 50… line segments… to get a smooth spline representation"
  [cs.jhu.edu Holten06 PDF excerpts, 2026-10-17]); the paper's interactivity is
  secondary ("Although our main focus… was on the visualization itself,
  interaction is an important aspect" [cs.jhu.edu Holten06 PDF excerpts,
  2026-10-17]), and the d3/Vega examples degrade to static paths without hover
  [d3 bundle-layout API ref + Vega edge-bundling example, 2026-10-17].
- **Fit.** Static-only/zero-JS/offline SVG ✅ (precompute rects + spline
  polylines in Rust); musl-safe, no new deps ✅; byte-identical given pinned
  sort + β + layout ✅.
- **Misfit.** Top-50 caps distort bundling — dropping edges changes bundle
  thickness, which IS the quantitative signal ("a thick bundle shows the
  presence of two elements at a fairly high level") ⚠️; slice-and-dice/linear
  layouts trigger the collinear-overlap failure — prefer squarified/radial
  placement ⚠️.

## Family 4 — Static flame graphs (Brendan Gregg; spec-OMITTED, surveyed as held-out)

- **Mechanism.** x = stack-profile population sorted alphabetically (not time),
  y = stack depth from zero at bottom; each rectangle = one stack frame, width
  ∝ sample frequency; warm/random colors differentiate adjacent frames
  [brendangregg.com/flamegraphs.html, 2026-10-17]. Pipeline: capture stacks →
  fold stacks → `flamegraph.pl` renders SVG [github.com/brendangregg/FlameGraph,
  2026-10-17]. Folded format `frame1;frame2;… count` via `stackcollapse-*`
  converters; source header: "The order used sorts function names
  alphabetically" to maximize merging
  [raw.githubusercontent.com/brendangregg/FlameGraph/master/flamegraph.pl,
  2026-10-17].
- **Scale ceiling.** No hard cap stated; design point is "only the most frequent
  frames have enough width for labels" in static/print form [flamegraphs.html].
  Filtering via `--minwidth NUM # omit smaller functions… (default 0.1 pixels)`
  or `%` of time [FlameGraph README].
- **Determinism.** Sort-alphabetical + merge is deterministic given sorted folded
  input [flamegraph.pl]; BUT default colors are "selected at random" unless
  `--hash`/`--cp` [flamegraph.pl] — byte-identical double-scan requires hashed
  colors + pre-sorted input + fixed minwidth.
- **Offline static-SVG.** Single `.svg` openable in browser; hover/click-zoom/
  search need embedded JS, but the static image "convey[s] the big picture"
  [flamegraphs.html] — zero-JS viable iff zoom/search are dropped (losing the
  zoom that reveals elided labels).
- **Fit.** Print-safe static ✅; musl-portable (Perl → Rust reimplementation, no
  libclang) ✅; top-50 cap maps to `minwidth%` + documented `grep` pre-filter
  (`grep cpuid … | flamegraph.pl`) ✅.
- **Misfit (why held out).** Flame shows dynamic call profiles, not the static
  file/call hierarchy — hence omission from the static map per
  `maintainer-map.md:23`. Any R4 use needs an exception with cost≠time honesty
  framing.

## Family 5 — Code-city / isometric software maps (Wettel & Lanza)

- **Mechanism.** City metaphor: classes = buildings, packages = districts;
  polymetric mapping onto all three building dimensions, e.g. #attributes →
  width+length, #methods → height (tall = high functionality, wide base = much
  state) [Wettel08 ICSE tool-demo PDF, via fetched excerpts, 2026-10-17].
  Customizable view configurations (property mappings, layouts) and scripting
  [Wett2010a PDF, via fetched excerpts, 2026-10-17].
- **Scale ceiling.** Tool-demo claims "several large-scale industrial systems"
  [Wettel08 ICSE tool-demo PDF, via fetched excerpts, 2026-10-17] (no LOC in
  fetched excerpts); controlled experiment used Medium + Large object
  systems [Wettel11 ICSE PDF, via fetched excerpts, 2026-10-17]. No single
  numeric ceiling quoted — treat as evaluated-to-industrial-scale, not bounded.
- **Controlled experiment (verified numbers).** n=41 analysed (45 recruited,
  exclusions noted), 4 locations / 3 countries, academia+industry. Overall
  means: correctness 4.803→5.968 (+24.26%), time 41.048→36.117 min (≈−12%;
  −14.51% medium, −10.x% large) [Wettel11 excerpts]; "+24%/−12%" is correct as
  overall means, statistically significant per ACM abstract
  [dl.acm.org/doi/10.1145/1985793.1985868 abstract, 2026-10-17].
- **Determinism / offline.** Deterministic placement given metrics + mapping (no
  relaxation randomness in fetched excerpts); original is a "sovereign…
  interactive" 3D app [Wett2010a excerpts]. Isometric 2.5D SVG projection is
  offline-capable with zero JS; consistent locality/orientation aids navigation
  [Wettel08 excerpts].
- **Fit.** Static-only ✅, musl (pure geometry) ✅, byte-identical if traversals
  sorted ✅, offline zero-JS SVG ✅, top-50 caps (filter before layout, keeps
  orientation) ✅.
- **Misfit.** 3D occlusion needs a viewpoint choice; flatten to isometric.
  Evocative but less information-dense per pixel than DSM/treemap for coupling
  questions.

## Family 6 — DSM / adjacency-matrix views (Lattix, Sangal et al.)

- **Mechanism.** The module adjacency matrix with rows/columns ordered by a
  hierarchical subsystem decomposition: "The x-axis and the y-axis of the DSM
  are the subsystems… The DSM depicts the dependencies between the various
  subsystems", strength "calculated by aggregating the dependency of leaf
  elements within a column"
  [docs.lattix.com DSM user guide, 2026-10-17]. Sangal et al. define it as "a
  partitioned adjacency matrix" from manufacturing, "the first application of
  DSM for the explicit management of inter-module dependencies"
  [groups.csail.mit.edu/sdg/pubs/2005/oopsla05-dsm.pdf (open OOPSLA copy),
  2026-10-17]. Partitioning reveals architecture: "partitioning helps
  determine: Layering… Strongly (or cyclically) connected components…
  Independent Components within a layer," shown as "square boxes (called virtual
  partitions) within the DSM" [Lattix DSM guide]. Design-rule overlays mark
  intent per cell: "for a can-use rule, the cell has a green triangle in the
  upper left vertex; and, for a cannot-use rule, the cell has a yellow triangle
  in the lower left vertex. If there is a dependency in a cell governed by a
  cannot-use rule, we show it with a red triangle"
  [lattix.com DesignRules PDF, 2026-10-17]. Rules are Can/Cannot/Must Use
  ("Cannot Use… creates a violation if there is a dependency… Can Use… will
  never create a violation… Must Use… if there is no dependency"), inherited
  top-down and "applied in sequence, therefore, inherited rules can be
  overridden" [Lattix architecture-enforcement guide, 2026-10-17]. Drill-down:
  "click on any cell to see the actual dependency, rule and rule violation"
  [DesignRules PDF]; "You can expand any of the subsystems by clicking on the
  expand icon '+'" since "This ability to display hierarchy is one of the
  reasons why DSMs scale so effectively" [Lattix DSM guide].
- **Scale ceiling.** "loading actual open source Java applications to create DSMs
  that can represent systems with thousands of classes"
  [doi 10.1145/1094855.1094915 (ACM companion page), 2026-10-17]; case study
  "application… to a system of about 200,000 lines of code" (Haystack)
  [OOPSLA PDF]. Docs: "It is highly scalable and the problematic dependencies
  are easy to identify" [Lattix DSM guide].
- **Determinism.** NO determinism/ordering-stability quote found in fetched
  sections — recorded gap. The workflow is explicitly iterative and
  human-guided: "establishing the right hierarchical structure… relies on
  guidance from the user… an iterative process seems to work best" [OOPSLA
  PDF]. Byte-identical output requires pinning row/column order + hierarchy +
  rule set; the matrix is order-dependent, not order-invariant.
- **Offline static-SVG.** Excellent: grid of rects + text + triangle markers —
  trivially static SVG, no JS. Drill-down maps to static expansion (one SVG per
  expanded subsystem, or anchor-linked sections).
- **Fit.** Static-only/zero-JS/offline-native ✅; musl-safe, no deps ✅;
  byte-identical via sorted traversal + pinned ordering ✅; top-50 caps map
  cleanly (top-N subsystems by coupling + overflow row/column — matrix stays
  well-defined, unlike bundle thickness) ✅.
- **Misfit.** Interactivity loss (no click-to-expand; mitigate with
  pre-rendered per-subsystem pages) ⚠️; red/green rule triangles need a legend
  that survives monochrome/printing ⚠️.

## Family 7 — Voronoi treemaps (HOLDOUT; Balzer, Deussen & Lewerentz 2005)

- **Mechanism.** Hierarchy treemap with arbitrary polygons (not rectangles) for
  better aspect ratio + boundary identification, via "iterative relaxation of
  Voronoi tessellations" with transparency + interactive zoom
  [Berkeley-hosted Balzer PDF, via fetched excerpts, 2026-10-17]. Follow-up
  work: Lloyd's iterative centroidal-Voronoi method + weight adaptation to meet
  area requirements [uni-konstanz NoBr12a PDF, via fetched excerpts,
  2026-10-17].
- **Cost.** 6-object example needed 104 iterations to area error <0.01 and 217
  to stable <0.001 [Balzer excerpts]; modern reformulation O(n log n) per
  iteration, O(k·n log n) overall vs original Ω(n²) per iteration [NoBr12a].
  No large-system ceiling quoted — cost grows with k×n.
- **Determinism.** Requires init "sites… with unique positions" + small-ε
  weights, iterate to error threshold/imax [NoBr12a]; Lloyd relaxation moves
  each site toward its cell centroid until movement < threshold
  [arxiv.org/html/2508.03445v1, 2026-10-17]. Random init ⇒ non-deterministic
  unless seeded; floating-point relaxation threatens byte-identical double-scan.
- **Verdict: HOLDOUT justified** — violates byte-identical + musl-simplicity
  (iterative float geometry, seed sensitivity, k×n cost). Revisit only if
  rectangular/isometric maps prove insufficient.

## Fit/misfit matrix vs non-negotiables

| Family | Byte-identical | Offline SVG, no JS | musl / zero-dep | Top-50 cap safe | Honesty fit |
|---|---|---|---|---|---|
| Sugiyama (native re-impl) | ✅ sorted input + fixed flags | ✅ `-Tsvg` pattern | ✅ hand-rolled rank/crossing/position | ✅ minimizer cost collapses | ✅ direction honest |
| Force-directed (d3-recipe) | ✅ phyllotaxis + fixed-seed LCG + 300 ticks | ✅ baked coords | ⚠️ native Rust port required | ✅ O(n²) trivial at 50 | ⚠️ clusters imply false truth |
| Force-directed (FR/FA2 raw) | ❌ random init / explicitly non-det. | ✅ baked coords | ⚠️ port + harness | ✅ | ⚠️ same + no auto-stop |
| Treemap+HEB | ✅ pinned sort + β + layout | ✅ rects + spline paths | ✅ integer geometry | ⚠️ caps distort bundle thickness | ✅ density honest |
| Static flame | ✅ sorted input + hashed colors | ✅ static conveys big picture | ✅ cheap emit | ✅ minwidth + grep pre-filter | ⚠️ spec-omitted; cost≠time |
| Code-city 2.5D | ✅ sorted traversal + mapping | ✅ isometric projection | ✅ painter order | ✅ filter pre-layout | ⚠️ height implies truth |
| DSM/matrix | ✅ pinned order + hierarchy + rules | ✅ table/rects | ✅ easiest | ✅ overflow row/col | ✅ most honest |
| Voronoi | ❌ float relaxation + seed | ✅ polygons | ❌ heavy | ⚠️ complicates caps | ⚠️ boundaries imply precision |

Net fit-order under top-50 + byte-identical + no-JS (ordering, NOT idiom
assignment — R4 maps these to Level 0/1/2):
**DSM ≈ treemap+HEB ≈ flame > Sugiyama > isometric-projection > d3-recipe
force-directed > FR/FA2 raw > Voronoi.**

## Dead ends (could not open — support no claim)

- Holten 2006 full PDF body (fetch limit) — paper's own demo node/edge counts
  unverified; claims use DOI landing + fetched excerpts only.
- Squarified treemap PDF binary (not persisted) — no Big-O cited.
- ACM doi 10.1145/1103845.1094824 (paywall redirect) — used open MIT-hosted
  OOPSLA PDF + ACM companion page instead.
- FR original doi 10.1002/spe.4380211102 (paywall) — used open Illinois-hosted
  PDF instead.
- Wettel08/Wettel11/Balzer full PDF bytes (need save_binary) — used fetched
  excerpts + ACM abstract; Wettel +24%/−12% verified as overall means.
- dl.acm.org/doi/10.1145/1985793.1985868 (403) — abstract via search excerpt.
- graphviz/graphviz#1614 (SVG determinism) — surfaced, not opened.

## References (all fetched 2026-10-17)

- Sugiyama lineage: Sugiyama et al. [STT81]; Gansner et al. dotguide
  (`graphviz.org/pdf/dotguide.pdf`); dot layout docs; `ordering` attr docs;
  Graphviz forum implementation thread.
- Force-directed: Fruchterman & Reingold 1991 (Illinois-hosted PDF);
  Jacomy et al. 2014, PLoS ONE 10.1371/journal.pone.0098679; d3-force
  simulation + d3-random docs; Bostock static-render gist #1667139.
- Treemap+HEB: Holten 2006, IEEE TVCG 10.1109/tvcg.2006.147 (+ JHU-hosted
  excerpts); Bruls et al. squarified (van Wijk-hosted text); d3 bundle-layout
  API ref; Vega edge-bundling example; Bostock flare gist #5672200.
- Flame: Gregg `flamegraphs.html`; FlameGraph repo README; `flamegraph.pl`
  source (sort + color-randomness header comments).
- CodeCity: Wettel08 ICSE tool demo; Wettel11 ICSE experiment; Wett2010a;
  ACM abstract 10.1145/1985793.1985868.
- DSM/LDM: Sangal et al. 2005 OOPSLA (MIT-hosted PDF); ACM companion
  10.1145/1094855.1094915; Lattix DSM guide, DesignRules PDF,
  architecture-enforcement guide.
- Voronoi: Balzer et al. 2005 (Berkeley-hosted excerpts); Nobar et al. NoBr12a
  (Konstanz); centroidal-Voronoi survey (arXiv 2508.03445).
