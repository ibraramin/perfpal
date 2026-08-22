---
plan name: mvp-architecture
plan description: Parser ADR, heuristics catalog, schema design
plan status: active
---

## Idea
Design-phase plan for the Perfpal MVP (C, static-analysis-only, CLI → perfpal-report/). Perfpal scans codebases for performance insight: rudimentary stats, architecture-level bottleneck visualization, and LLM-consumable raw data + human report. This plan produces all technical decisions needed BEFORE implementation: (1) parser strategy ADR — tree-sitter vs libclang vs hybrid for C, with license audit against the no-copyleft constraint; (2) prioritized C performance-heuristic catalog for founder sign-off (complexity, malloc/free density, pointer-chasing, loop nesting, recursion, unsafe-copy hotspots, etc., ranked by signal-value vs cost); (3) internal representation + versioned, LLM-agnostic metrics.json schema v1 with confidence/severity fields; (4) perfpal-report/ output contract (metrics.json + report.md + charts/); (5) visualization approach ADR for architecture-level bottleneck maps (dependency graphs, coupling heatmaps) using no-copyleft static chart tech; (6) module architecture doc: scanner → analyzer → reporter pipeline with explicit extension points for runtime-profile ingestion and additional languages. Zero implementation code in this plan — artifacts are ADRs, catalogs, schemas, and docs.

## Implementation
- Write ADR-0001 (parser choice): evaluate tree-sitter vs libclang vs hybrid for C scanning; include dependency-license audit against the no-copyleft constraint; record decision + consequences.
- Draft heuristic catalog v1 for C: enumerate candidate static performance heuristics (cyclomatic/cognitive complexity, function length, malloc/free density, pointer-chasing patterns, loop nesting depth, recursion depth, memcpy/strcpy-family hotspots, missed-inline candidates, etc.), rank by signal-value vs implementation cost, present ranked list to founder for sign-off.
- Define internal representation (IR) for scan results and versioned JSON Schema v1 for metrics.json — generic/LLM-agnostic, with confidence and severity fields per finding.
- Specify the perfpal-report/ output contract: exact layout (metrics.json, report.md, charts/), file naming, versioning/stability rules, and the human-readable report template outline.
- Write ADR-0002 (visualization approach): compare no-copyleft static chart options (SVG/HTML generation libs) for dependency-graph and coupling/bottleneck maps; choose easiest-to-develop option that satisfies pillar 1b.
- Document module architecture: scanner → analyzer → reporter pipeline, plugin boundaries, explicit extension points for runtime-profile ingestion (perf/valgrind/cProfile) and additional languages (C++, Python next).
- Fold all new artifacts into state/session-state.json (append-only), regenerate state/handoff/latest.md, update docs/activeContext.md + CHANGELOG.md; request founder commit approval.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
<!-- SPECS_END -->