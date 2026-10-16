---
plan name: map-build
plan description: Cost-weighted map build sessions
plan status: active
---

## Idea
Five implementation sessions building the maintainer birdseye map after the R4 ADR gate, each committing independently: I1 entry-point detection plus reachability sets over resolved call edges as additive IR/schema fields; I2 cost model v2 with trip-count gating and reachability-aware cap election that preserves entry-reachable tails; I3 the entry-point-stratified cost map as third layer plus the deferred middle-bands layout reusing the existing layout pipeline; I4 findings-as-annotations with report.html drill path and output-contract registry plus one stem; I5 determinism proofs, dual review, and 9-repo demo eyeball where maintainers must name 3 actions from the map or the idiom is judged wrong.

## Implementation
- I1 entry-points: per-language main/handler/lib-entry detection + reachability sets, additive fields, reachability locks, cargo test green
- I2 cost-v2: trip-count gating + per-iteration weights + reachability-aware caps; GENUINE share measured up, ratchets unlowered
- I3 map layer: entry-stratified cost map + middle-bands layout; goldens regen byte-stable on double-scan
- I4 annotations: finding dots on nodes + html drill path + contract registry; django humans bounded, detail byte-identical
- I5 proof: full gate exit 0, dual review APPROVED, 9-repo demo eyeball with 3-action maintainer test

## Required Specs
<!-- SPECS_START -->
- maintainer-map
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
<!-- SPECS_END -->