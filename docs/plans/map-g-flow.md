---
plan name: map-g-flow
plan description: Entry-flow map candidate
plan status: active
---

## Idea
Build the G candidate for Perfpal's human map: an entry-flow map that tells the program's story. Formalize the founder's sketch into a deterministic, static-first view: the production entrypoint (main; for libraries the highest fan-in production lib-root, labelled) sits at the centre; file nodes branch outward along resolved call edges in BFS depth rings; each file carries a card listing its top functions by function_cost_v2/fan-in plus its top finding; component/domain hulls overlap the files to show general work; unreached files/functions are greyed with a count; a numbered path trace highlights the representative entry-to-sink path. Add the mandatory declarative progression animation (CSS keyframes/SMIL, no JS): entry pulse, depth-ring reveal, edge draw-on, cards, path trace, hot-node pulse, with a CSS-only pause control and prefers-reduced-motion/print fallbacks (spec map-animation). The view must label call edges as static/LOW-confidence and must not imply runtime truth; Q2 ("what breaks") stays a reverse drill (component DSM + focus+1-hop dependents), not a second encoding on the flow map. Prototype from the existing out-v6 metrics on cJSON/hyperfine/tmux (design repos only; holdout vaulted), run two blind @vision reads per repo, A/B against F, then implement the winner in Rust with tests/goldens, the ADR-0002 animation amendment, a fresh vision gate and the founder A2 timed test on the holdout. If G loses, archive the prototype and record the decision; keep the reusable animation/path-trace engine if F wins.

## Implementation
- Formalize the G contract: entry policy (production main vs highest fan-in production lib-root vs multi-entry), file-level call-edge aggregation (distinct from/to pairs + call counts), top-50 node cap with honest aggregation, production-first component hull derivation (replace test-dominated path prefixes), unreached-tail treatment, path-trace selection, and the LOW-confidence static-edge labelling. Write the spec/ADR-0008 amendment draft.
- Build the deterministic layout/animation core in a Python prototype: integer radial/BFS placement (sorted by cost/depth/path), collision-safe node boxes, file cards, component hulls, unreached tail, static SVG final frame, and the map-animation timeline (CSS keyframes/SMIL progression, CSS-only pause, reduced-motion/print fallbacks; no JS). Read only out-v6-{cJSON,hyperfine,tmux}/metrics.json.
- Render cJSON + hyperfine (tmux as the density stress case) at 1440x900: start/mid/final frames plus the animated HTML. Run two fresh blind @vision reads per repo on the final frame (one image per task, no source/siblings/expected answer), record transcripts and blockers (edge density, card legibility, entry ambiguity, LOW-confidence gaps).
- Build the Q2 reverse drill for one selected file/component: component DSM plus focus+1-hop dependents list/DSM, linked from the flow map; render and run two blind reads. Confirm Q2 is answered without overloading the flow map's encoding.
- Run the F-vs-G A/B on the same repos: blind comprehension reads, the stated criteria (Q1/Q2/Q3, whole-codebase grasp, data honesty, animation value), and the founder timed test. Record the verdict, the evidence, and what would change it.
- If G wins, implement it in Rust under src/reporter (view model + SVG/HTML renderer + animation emitter), add unit/integration tests and goldens, record the ADR-0002 animation amendment and the map-intelligibility/maintainer-map spec updates, run cargo fmt/clippy/test, byte-determinism, the vision gate, and the founder A2 test on the holdout. If G loses, archive the prototype, record the decision in state/spec, and retain only the reusable animation/path-trace pieces.

## Required Specs
<!-- SPECS_START -->
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
- maintainer-map
- map-intelligibility
- map-animation
<!-- SPECS_END -->