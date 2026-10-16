---
plan name: map-research
plan description: Exhaustive map research sessions
plan status: active
---

## Idea
Four exhaustive read-only research sessions feeding the maintainer birdseye map, each with a named doc deliverable so nothing evaporates: R1 layout algorithms for static codebases (Sugiyama limits, force-directed ceiling, treemap+edge-bundling, static flame graphs, code-city, DSM/matrix) plus deterministic-layout constraints; R2 precedent teardown (Sourcetrail ego-graph, code2flow honest estimates, NDepend/Structure101/Lattix, CodeSee/Gource, madge/depcruise) judging maintainer-helpful vs eye-candy with sources; R3 cost+uncertainty visualization and maintainer information-needs literature; R4 constraint-mapping decision matrix against Perfpal non-negotiables (static-only, no-build, musl, byte-identical, offline SVG/HTML, top-50 caps) ending in an ADR that gates all implementation.

## Implementation
- R1 algorithms: survey layouts with fit/misfit vs static-only deterministic constraints -> docs/research/map-algorithms.md
- R2 precedents: teardown 6+ tools, verdict helpful-vs-eyecandy with sources -> docs/research/map-precedents.md
- R3 honesty: cost-without-lying idioms, cap/omission markers, maintainer-needs literature -> docs/research/map-honesty.md
- R4 decision: matrix vs non-negotiables, recommended idiom per map level + rejected alternatives -> ADR + docs/research/map-decision.md
- Gate review: reviewer + requirements-reviewer verdicts on R1-R4 packet before any I-session starts

## Required Specs
<!-- SPECS_START -->
- maintainer-map
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
<!-- SPECS_END -->