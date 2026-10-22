---
plan name: map-shipped
plan description: Shipped combined map research
plan status: done
---

## Idea
Turn the V7 F/G prototypes into one shippable human-map feature. The founder's model: **G (entry-flow) is the core graph**; **F (weight map) is not a fallback and is not thrown away** — its components (weight/treemap mass, production-first area, component cells) are candidate **supporting elements** around the G core, included only where research shows a developer actually needs them. This plan is research + planning ONLY — no code. It must (1) research what a developer actually wants to see **alongside** the main graph (actions, metrics, weight mass, blast radius, etc.); (2) standardize the metrics shown (what, where, units, anchors), under the founder constraint that the graph page is 100% graph and metrics/brief live below or elsewhere; (3) audit the data needed for the combined graph (entry policy, call edges, file cards, component hulls, unreached tail, path trace, cost/severity) and decide additive fields vs downscope; (4) define the full-page combined layout, animation, pause/reduced-motion/print; (5) define which F-derived components join the G core and under what conditions — not a mode switch, a component-inclusion rule; (6) feature name is **Perfpal Atlas** (chosen 2026-10-22) — record it in the spec/ADR; (7) update the specs/ADRs and produce a final build plan with tests/goldens/vision/A2 gates. The holdout stays vaulted until the shipped feature is ready for the A2 timed test.

## Implementation
- Read the V7 comparison, both plans, the F/G reads and manifests; reconcile the combined feature against map-intelligibility, maintainer-map, map-animation and ADR-0008. Record what G keeps, which F components are candidates, and what is deferred.
- Developer-needs research (first): what does a developer actually want to see alongside the main graph? Synthesize the 12 blind reads, the F/G failures (cJSON library/test-heavy; tmux density), prior art (Sourcetree/Sourcetrail/Structure101/CodeCharta/IDE call hierarchies), and the product audience (AI-generated codebases). Produce a needs brief: questions asked, actions taken, supporting views, where metrics belong, what is noise.
- Metrics standardization audit: inventory every field used by the G core, the F-derived components, the KV1 brief, the Q2 drill and the animation; define the canonical display set, units, anchors and formats; decide what appears on the graph page (nothing but the graph), what moves below it, and what stays drill-only. Produce a metrics contract doc.
- Data-gap audit: for every graph element (entry, file nodes, call edges, function cards, component hulls, unreached tail, path trace, cost/severity), name the existing field and any missing field; recommend additive schema changes or an explicit downscope. Include entry semantics (main vs lib-root vs multi-entry), call-edge confidence/coverage, component labels, I/O sinks, and card ranking.
- Full-page combined layout spec: G core as a full-bleed graph (no metric panels on the canvas), its legend/controls, the animation timeline and static fallbacks, the F-derived supporting components (only if the needs research justifies them), the Q2 drill, and where the brief/metrics live (below the graph or a separate page). Include the 1440x900 and print/reduced-motion cases.
- Component-inclusion rule + naming: define which F-derived components join the G core and when (e.g. weight mass as a secondary panel/drill, production-first area for test-heavy scans), the user-facing disclosure, and record the chosen feature name **Perfpal Atlas**. No fallback-mode switch unless the research proves it necessary.
- Spec/ADR amendments + final build plan: update map-intelligibility, maintainer-map, map-animation, ADR-0008 and ADR-0002; produce a phased build plan with file paths, tests/goldens, vision gate, A2 holdout and a runnable Definition of Done. No implementation in this plan.

## Required Specs
<!-- SPECS_START -->
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
- maintainer-map
- map-intelligibility
- map-animation
<!-- SPECS_END -->