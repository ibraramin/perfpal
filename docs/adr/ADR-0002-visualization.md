# ADR-0002: Visualization Approach — hand-rolled deterministic SVG, zero chart dependencies

- **Status:** Accepted (founder-approved 2026-08-23)
- **Amended by:** [ADR-0010](ADR-0010-declarative-animation.md) (2026-10-23, proposed) — the HTML
  report may carry declarative CSS/SMIL animation over the static SVG; artifacts stay static,
  offline, zero-JS and byte-deterministic, and the static SVG remains canonical. The call-graph
  policy ("no standalone call-graph chart in v1") is amended for the Atlas canvas only by
  [ADR-0009](ADR-0009-atlas-core.md).
- **Date:** 2026-08-23
- **Deciders:** Founder + orchestrator (evidence: @librarian survey of chart tech, licenses verified against primary sources)
- **Gates:** All reporter/chart implementation in `mvp-architecture` and beyond; owns the chart set deferred by [output-contract §3](../report/output-contract.md).

## Context

Pillar 1b requires **architecture-level** bottleneck visualization — dependency structure, component coupling, hot components — not per-function metric heatmaps. The output contract fixed chart naming/stability (`charts/<name>.svg`, kebab-case, stable across runs) but explicitly deferred the chart set and content-determinism mechanics to this ADR.

Binding constraints:

1. **No copyleft (GPL-family) dependencies** in the core tool (license policy).
2. **Byte-determinism**: identical `metrics.json` ⇒ identical SVG bytes (output-contract Principle 5; directories must be diffable).
3. **Self-contained CLI**: end users install Perfpal, nothing else — no Node, no browser, no Graphviz (ADR-0001 footprint precedent).
4. **Charts are advisory**: consumers never parse data out of SVGs; `metrics.json` is the source of truth.
5. Implementation language is **deliberately unpinned** until the module architecture doc (step 6); plausible candidates: C, Rust, Go, Python.

Data available to charts (schema v1.0): `graphs.include_graph` (file nodes; quoted/angle edges with resolution status), `graphs.call_graph` (cross-file edges only, confidence locked LOW), `components[]` (path_prefix, rank, aggregated metrics incl. `finding_density`). `stats.distributions` exists but no v1 chart consumes it — reserved for future minor-bump additions.

## Decision

**Perfpal renders all charts through its own zero-dependency deterministic SVG emitter. No external charting, graph-layout, or plotting library enters the MVP core.**

Chart set v1 (names frozen, kebab-case per output-contract §3):

| Chart | Source | Form |
|---|---|---|
| `include-graph.svg` | `graphs.include_graph` | File-level DAG, simplified layered layout. Resolved internal edges only (quoted includes plus angle-bracket includes resolving to in-repo files); unresolvable targets are excluded here — they surface as ARCH-001 findings; external angle-bracket includes stay bucketed out per ADR-0001. |
| `component-coupling.svg` | include edges + call edges aggregated by `path_prefix`; `components[]` axis | Components×components matrix heatmap. Call-edge contribution styled distinctly (LOW-confidence advisory). |
| `hot-components.svg` | `components[]` | Horizontal bars in `rank` order, bar length ∝ `finding_density`. |

Layout policy: **simplified deterministic layered layout** for DAGs — topological layering from sources, within-layer order via barycenter heuristic with sorted tie-breaks, exactly 3 passes, positions rounded half-up to fixed-point at a stated scale. Cycles (expected input per [ADR-0001](ADR-0001-parser-strategy.md)) are broken deterministically before layering: sort edges by `(from, to)` and drop the lowest-sorted edge that closes a cycle; self-loops are dropped — broken edges remain visible as ARCH-001 findings, never silently. Full Sugiyama crossing minimization is NOT built now (YAGNI; see Alternatives).

Call-graph policy: **no standalone call-graph chart in v1.** Function-level rendering of LOW-confidence name-based edges would be noisy and misleading; call edges instead aggregate into the coupling matrix with distinct styling plus the standing caption caveat already mandated by output-contract §5.

DOT export (emit `.dot` for users with Graphviz — the madge/deptrac pattern) is **deferred post-MVP** as the escape hatch; the own-SVG baseline always exists.

## License Audit (no-copyleft constraint)

Survey verified against primary sources (@librarian, this session):

| Option surveyed | SPDX / status | Copyleft? | Disposition |
|---|---|---|---|
| Hand-rolled emitter (chosen) | none — Perfpal's own code | No | No obligations, no notices. |
| Graphviz `dot` binary | `EPL-2.0` (relicensing from CPL-1.0 in progress since 2023; the official license page still listed CPL-1.0 at verification time) | Weak-copyleft family, **not GPL** | Legal to invoke (subprocess avoids linking questions entirely); rejected on footprint + determinism grounds, not license. |
| Graphviz cgraph (linked) | `EPL-2.0` | Not GPL-family | Would carry distribution notice duties; unnecessary. |
| mermaid-cli | MIT wrapper; downloads Chromium (~300 MB) | No | Runtime footprint disqualifying. |
| d3 / viz-js | ISC/MIT (viz-js wraps EPL Graphviz-as-WASM) | No | Require a JS/WASM host. |
| matplotlib | PSF-style (`PSF-2.0 AND MIT AND CC0-1.0`, no single registered SPDX) | No | Defaults embed timestamps + uuid4 IDs (upstream #23693); weak at DAGs. |
| gnuplot | `gnuplot` (permissive, unusual patch-distribution clause) | No | Version-drift prone; weak at DAGs. |
| networkx / gonum | `BSD-3-Clause` | No | Language-locked; force-directed layouts nondeterministic without seeds; gonum no longer ships a layered layout. |
| petgraph | MIT/Apache-2.0 | No | No layout module in 0.8.x — algorithms/DOT export only. |
| OGDF / igraph (C/C++ graph libs) | GPL-family | **Yes** | **Banned outright** by policy. |

**Conclusion:** licensing does not differentiate the viable options — every serious candidate is non-GPL-family. The decision rests on determinism and footprint, exactly as in ADR-0001.

## Rationale

1. **Determinism by construction beats determinism by configuration.** Library landmines are documented and real: matplotlib embeds a creation timestamp and uuid4-salted element IDs unless `svg.hashsalt`/`SOURCE_DATE_EPOCH` are managed; Graphviz output bytes vary across versions, plugins, and fontconfig text measurement. Owning the emitter makes byte-stability a property of our code, not a fight with someone else's defaults.
2. **Self-containment preserved.** No user-installed runtime of any kind — the same footprint philosophy that chose tree-sitter over libclang. mermaid-cli's silent first-run Chromium download is the class of surprise this avoids.
3. **Language freedom for step 6.** The choice is implementable identically in C, Rust, Go, or Python. This matters: there is no mature non-copyleft graph-layout library for C, so any library-based decision today would silently constrain the implementation-language fork.
4. **The primitives are genuinely simple.** A coupling matrix is a `<rect>` grid; ranked bars are `<rect>` rows; the only real work is layered DAG layout (~500–1000 LOC; MIT-licensed prior art such as rust-sugiyama may be consulted or ported with notice retained).
5. **Precedent.** madge and deptrac both treat Graphviz rendering as an optional enhancement over a dependency-free baseline — validating own-baseline + deferred-DOT-export as the industry pattern.

## Alternatives Considered

- **Graphviz-invoke (subprocess) — rejected.** Best layout quality, near-zero dev cost, license-clean via invocation — but requires a Graphviz install at user sites (breaks self-containment) and output bytes drift across dot versions/plugins/fonts (breaks Principle 5 diffability). Retained as the post-MVP escape hatch and reversal trigger.
- **cgraph linked into the binary — rejected.** EPL notice duties plus vendoring weight for capability we do not need.
- **mermaid-cli — rejected.** ~300 MB Chromium fetch, offline violation, font-dependent nondeterministic geometry.
- **d3 / viz-js — rejected.** Require a JS/WASM host; sane only inside a JS-implemented tool.
- **matplotlib / gnuplot — rejected.** Poor directed-graph support; determinism achievable only after disabling hostile defaults; add a runtime dependency.
- **networkx / gonum force-directed layouts — rejected.** Nondeterministic without seeds and float-sensitive even with them; language lock-in.
- **Full Sugiyama now — rejected (YAGNI).** Include graphs are typically shallow; layering + barycenter ordering suffices for MVP legibility. Crossing minimization is a post-MVP quality upgrade inside the same emitter.

## Consequences

**Positive**
- Chart bytes are identical across runs and machines; the directory as a whole remains diffable modulo provenance timestamps (output-contract Principle 5).
- Zero new dependencies, zero license obligations, minimal footprint.
- Implementation-language fork (step 6) stays fully open.
- Charts can never disagree with `metrics.json` structurally — they are pure functions of it.

**Negative / mitigated**
- We own layout quality → fixed char-width label metric (never measure fonts), barycenter ordering, node caps with an explicit in-chart truncation marker; MVP bar is *legible*, not pretty.
- Large graphs need handling → deterministic cap rule (top-N by degree, sorted tie-breaks) applied before layout and documented in each chart's caption.
- LOW-confidence call edges could mislead → distinct styling in the coupling matrix + mandatory caption caveat (output-contract §5 already requires the ADR-0001 caveat line).
- Emitter maintenance is ours → renderer stays isolated behind a narrow interface consuming only schema types, mirroring the IR-seam discipline of ADR-0001.

## Guardrails (enforced from day one)

1. Charts render **only** from `metrics.json` structures (`graphs.*`, `components[]`) — never from source, IR, or scanner state.
2. **Determinism rules:** every iteration order is sorted (nodes by path; edges by `(from, to)`; components by `(rank, path_prefix)`); coordinates are integers or fixed-point decimals; C-locale number formatting; no `<metadata>` element, no creation dates, no generator-specific IDs; colors are pure functions of `metrics.json` values.
3. All label/path text is XML-escaped on emission (SVG analog of the output-contract Markdown-injection rule).
4. Node/edge caps apply deterministically (sorted top-N by degree) and truncation is rendered in-chart, never silent.
5. Chart filenames are frozen to the three names above; adding a chart is a minor bump (output-contract §4), renaming/removing is major. A chart file is emitted iff its source section is present and non-empty; otherwise it is omitted and `charts/` may be empty (output-contract §3 permits this) — mirroring how report.md omits clauses for absent optional fields.
6. CI test: rendering a fixture `metrics.json` twice yields byte-identical SVGs; the assertion runs in the CI matrix across platforms once implementation lands.

## Reversal Triggers (what flips us to Graphviz-invoke / hybrid)

1. Users demand publication-quality graph layouts → ship optional `.dot` export (madge pattern); own SVG remains the deterministic baseline.
2. Chart ambitions grow beyond graphs/matrices/bars (e.g., treemaps) and hand-rolling cost exceeds embedding a permissively licensed renderer.
3. Founder relaxes the no-external-runtime stance for an opt-in "enhanced rendering" mode.

## References

- Survey evidence (@librarian, this session): Graphviz license history https://graphviz.org/license/ · matplotlib determinism issue https://github.com/matplotlib/matplotlib/issues/23693 · rust-sugiyama (MIT, porting reference) https://github.com/paddison/rust-sugiyama · petgraph docs (no layout in 0.8.x) https://docs.rs/petgraph
- Precedent: madge (optional Graphviz) https://github.com/paulmillr/madge · deptrac (DOT emission) https://github.com/qossmic/deptrac · CodeCharta (separate web visualizer) https://codecharta.com
- Internal: [output-contract §3–§5](../report/output-contract.md) · [metrics.schema.v1.json](../schema/metrics.schema.v1.json) (`graphs`, `components`) · [ADR-0001](ADR-0001-parser-strategy.md) (include-bucketing, resolver tiers, footprint precedent) · spec pillar 1b
