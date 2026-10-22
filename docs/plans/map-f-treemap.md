---
plan name: map-f-treemap
plan description: Weight-map treemap candidate
plan status: active
---

## Idea
Build the F candidate for Perfpal's human map: a weight map of the whole repository. A deterministic component-level treemap (top-50 components, area = share of static cost, one cost ramp, hottest cells outlined, production-first) is the hero; a KV1 brief strip (3-5 ranked actions with path:line, why, severity/confidence, anchors) is the annotation layer; Q2 ("what breaks") is a component DSM plus focus+1-hop dependents drill, not a first-screen encoding. Add the mandatory declarative animation (spec map-animation): cells reveal in cost-rank order, the hottest cell pulses, and the component-to-file drill transitions; CSS-only pause plus prefers-reduced-motion/print fallbacks, no JS. The static final frame is complete and vision-gated. Prototype from the existing out-v6 metrics on cJSON/hyperfine/tmux (design repos only; holdout vaulted), run two blind @vision reads per repo, A/B against G, then implement the winner in Rust with tests/goldens, the ADR-0002 animation amendment, a fresh vision gate and the founder A2 timed test on the holdout. If F loses, archive the prototype and record the decision; keep the brief/DSM pieces if G wins.

## Implementation
- Freeze the F contract: component-level treemap (top-50, production-first, area = static cost, one cost ramp, severity outlines), KV1 brief strip, component DSM + focus+1-hop drill, Explorer opt-in, and the map-animation timeline (rank reveal + hot-cell pulse + drill transition). Write the spec/ADR-0008 amendment draft.
- Build the deterministic treemap layout/animation core in a Python prototype: integer squarified layout with sorted components, labels, hottest-cell outline, brief strip from the KV1 view model, static SVG final frame, and the map-animation CSS timeline (CSS-only pause, reduced-motion/print fallbacks; no JS). Read only out-v6-{cJSON,hyperfine,tmux}/metrics.json.
- Render cJSON + hyperfine (tmux as the density stress case) at 1440x900: start/mid/final frames plus the animated HTML. Run two fresh blind @vision reads per repo on the final frame (one image per task, no source/siblings/expected answer), record transcripts and blockers (labels, scale, treemap legibility, animation value).
- Build the Q2 drill prototype: the component DSM plus a focus+1-hop dependents list/DSM for one selected component/file, linked from the treemap; render and run two blind reads. Confirm Q2 is answered without adding a second first-screen encoding.
- Run the F-vs-G A/B on the same repos: blind comprehension reads, the stated criteria (Q1/Q2/Q3, whole-codebase grasp, data honesty, animation value), and the founder timed test. Record the verdict, the evidence, and what would change it.
- If F wins, implement it in Rust under src/reporter (view model + SVG/HTML renderer + animation emitter), add unit/integration tests and goldens, record the ADR-0002 animation amendment and the map-intelligibility/maintainer-map spec updates, run cargo fmt/clippy/test, byte-determinism, the vision gate, and the founder A2 test on the holdout. If F loses, archive the prototype, record the decision in state/spec, and retain only the reusable brief/DSM pieces.

## Required Specs
<!-- SPECS_START -->
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
- maintainer-map
- map-intelligibility
- map-animation
<!-- SPECS_END -->