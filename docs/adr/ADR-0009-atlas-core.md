# ADR-0009: Perfpal Atlas — entry-flow core, fixed page, supporting weight strip

- **Status:** Proposed (founder ratification required before build)
- **Date:** 2026-10-23
- **Deciders:** Founder + orchestrator
- **Gates:** `atlas-build`; supersedes `ADR-0008` level idioms where they conflict; amends the
  ADR-0002 call-graph policy for the Atlas canvas; amends the `map-intelligibility` first-screen
  rule (A8) for the Atlas page.

## Context

- The north-star says the **maintainer birdseye map is the core feature**: one visual answering
  where to look first, what breaks if touched, and where time might plausibly go
  (`docs/NORTH-STAR.md:17–21`; `docs/specs/maintainer-map.md:7–13`).
- The I5 A5 human test failed ("I can't understand anything"); `map-intelligibility` locked three
  answer-first slots (KV1/KV2/KV3) and a vision gate. V4 shipped a list-first page with KV1 as the
  first screen; V5 validated it (18/18 reads); V6a fixed KV2 production-first.
- V7 built two prototypes and ran 12/12 blind reads (`docs/evidence/map-vision/v7/COMPARISON.md`).
  G (entry-flow) reads strongly on hyperfine/tmux (entry → HOTTEST → action); cJSON is an honest
  dead end (library, one reachable node, no forward path). F's brief is reliable everywhere; its
  treemap is hijacked by test cost on cJSON (93.5% of area; production 0.2% unreadable).
- **Founder decision D-pf-238 (2026-10-22):** G is the **core graph**; F is **not a fallback and
  not discarded** — F-derived components (weight/treemap mass, production-first area, component
  cells) are candidate supporting elements, included only if developer-needs research justifies
  them. Graph page = 100% graph; metrics/brief below or elsewhere; no mode switch; standardize
  metrics; feature name **Perfpal Atlas**.
- Planning evidence: `docs/research/atlas-developer-needs.md` (needs model, six-test inclusion
  rule), `docs/research/atlas-data-gap-audit.md` (no schema change required),
  `docs/specs/atlas-metrics.md` (metric contract), `docs/specs/atlas-layout.md` (page composition).
- Conflicts this ADR must resolve explicitly:
  1. **ADR-0002:41** forbids "a standalone call-graph chart in v1" because LOW name-based edges
     are noisy and misleading.
  2. **ADR-0008:47–49** requires the third layer to use "resolved call edges only, LOW-confidence
     edges excluded".
  3. The default tier exports **0 resolved call edges** (`docs/evidence/map-data-coverage.md:39–43`)
     — a resolved-only canvas would be empty on every default scan.
  4. **`map-intelligibility` A8** requires `#map`'s first content block to be KV1; the founder now
     requires the graph first.
- Non-negotiables: static-only, offline, no new JavaScript (the report's fixed enhancement script
  is unchanged), byte-identical double scan, additive schema only,
  no copyleft, candidates never certainties (`ADR-0002`; `perfpal-core-spec.md:24,42`).

## Decision

1. **Core graph = G entry-flow.** One elected entry: production `main`, then production `script`
   (Python `__main__` guard), otherwise the highest fan-in production `lib-root`, labelled
   `library root (no production main)`; otherwise a `NO ENTRY` stub (never abort). File nodes on BFS depth columns from the elected entry; component
   hulls as approximate prefix groups; unreached tail counted and drawn grey; a numbered
   representative entry-to-leaf path trace (static, not runtime).
2. **The canvas is 100% graph.** No cards, tables, bars, metric panels or metric selectors on the
   canvas. Graph chrome only: entry badge, node labels (name, cost, `× median function`, HOTTEST, finding
   annotation `HOT ×N`/`WARN ×N`/`INFO ×N`, M-18), edge labels (distinct structural-edge count), hulls,
   unreached column, path trace, legend, caps chip, canvas footer.
3. **Below the canvas: a fixed seven-block brief**, state-unconditional — answer line, KV1 actions,
   weight strip, blast radius, file cards, noise footer, drill links
   (`docs/specs/atlas-layout.md` §4). The first block is the canvas; the first answer-bearing block
   is the answer line directly below it. This **amends A8** for the Atlas page.
4. **F support is a component, not a mode.** The weight mass ships as an **always-on below-graph
   weight strip** (`% of scanned static cost`; production-first; non-production (test/bench/other) mass in a labelled
   band). The **treemap is Explorer-only** (`weight-treemap`), never on the Atlas page. Component
   cells become hulls (structure) + component rows (mass); production-first is an **ordering rule**,
   not an area encoding. The six-test inclusion rule in
   `docs/research/atlas-developer-needs.md` §7 is the standing rule for any future supporting
   component. **No mode switch:** every scan renders the same canvas + seven blocks; data changes
   content and labels, never page shape.
5. **Structural call edges are permitted on the Atlas canvas, with disclosure.** Because the
   default tier has 0 resolved edges, the canvas draws the default-tier call graph (LOW + MEDIUM)
   and labels it `structural (LOW name-based + MEDIUM semantic), may over/under-approximate`; it prints the
   `N MEDIUM / M LOW` mix; it never prints `resolved` or `reachable_from_entry` for that set and
   never implies runtime truth. Reachability on the canvas is **estimated structural reachability**.
   This supersedes ADR-0002's blanket ban for the Atlas canvas only (the ban on generic standalone
   call-graph charts remains for Explorer stems) and supersedes ADR-0008's resolved-only line for
   the Atlas canvas.
6. **Metrics are standardized** per `docs/specs/atlas-metrics.md`: one measured encoding per
   surface (cost), no time units, denominator-adjacent anchors, production-first with counted
   demotions, caps/coverage printed, structural edges named structural.
7. **No schema change in Atlas v1.** All Atlas elements are existing fields or deterministic
   chart-side derivations (`docs/research/atlas-data-gap-audit.md` §2, §5). A general I/O-sink
   classification is **downscoped**; additive candidates (`components[].member_paths[]`,
   `entry_id`, `io_sink`, `call_site_count`, `cost_summary`) are recorded with triggers, not added.

## Options considered

### Edge policy

- **A — draw the structural set (LOW + MEDIUM) with disclosure (chosen).** Keeps the default-tier
  product story alive; the cost is that the hero graph rests on name-based edges. Mitigation:
  caption, confidence mix, no runtime language, and the ADR-0002 reversal trigger stays.
- **B — resolved-only.** Honest but empty on every default scan (0 resolved edges); the core
  feature would not render. Rejected.
- **C — require the semantic tier.** Makes the graph opt-in and unavailable to default users;
  kills the feature's first-run value. Rejected.
- **What would change the choice:** semantic-tier coverage becomes broad (prefer resolved edges and
  demote LOW to a caption); or blind reads show readers mistake structural edges for runtime
  behaviour (strengthen labelling or move to B).

### F treatment

- **A — weight strip always + treemap in Explorer (chosen).** Keeps the mass answer (Q3) without
  the cJSON area hijack; respects "not discarded"; state-unconditional.
- **B — full treemap below the graph.** Fails on cJSON (93.5% test wall), duplicates cost and adds
  a second encoding. Rejected.
- **C — drop F entirely.** Loses the "where is the mass" answer and contradicts D-pf-238.
  Rejected.
- **What would change the choice:** the weight-strip falsifier (no blind read uses it to answer
  mass or to choose an action); or a production-only area contract that passes the cJSON case.

### Page model

- **A — fixed stack, no mode switch (chosen).** Same canvas + seven blocks for every scan; library
  and test-heavy states change labels/content. Best disclosure; matches D-pf-238.
- **B — explicit fallback mode for libraries.** A data-keyed page shape is a worse-disclosed mode
  switch; the founder rejected it. Rejected.
- **C — keep the list-first KV1 page.** Contradicts "graph page is 100% graph" and the founder's
  model. Rejected.
- **What would change the choice:** a library-shaped holdout where the fixed stack cannot produce
  an action even from the brief (then a founder decision on a reverse-context state).

## Consequences

**Positive**

- One coherent page; the library and test-heavy failure modes are handled by content, not by a
  hidden mode.
- The mass answer survives without the treemap's area hijack; the treemap remains available in
  Explorer.
- The metric contract removes the V7 unit/multiple defects (misread `83,354×`, invented `ms`/`µs`,
  mixed denominators).
- No schema churn: the Atlas ships on v1.7 fields with chart-side derivations.

**Negative / mitigated**

- The hero graph uses LOW-confidence edges → disclosed caption + confidence mix; ADR-0002's
  reversal trigger remains; the semantic tier can upgrade the canvas later.
- Caps fragment large repos (tmux 20/106) → aggregated boxes with `+N more`, complete lists in
  drills, caps printed.
- Component hulls are approximate prefix groups, not canonical components → disclosed on the hull
  legend and in the weight strip.
- The A8 first-screen rule is amended for the Atlas page → recorded explicitly in
  `map-intelligibility`; the KV slot questions/encodings are retained, only placements move.
- The weight strip is a new below-graph cost view → bounded by one encoding (share), no area, and
  the strip falsifier.

## Gates

- `atlas-build` may not start until this ADR is ratified by the founder.
- The build must pass A1 (blind vision, all states), A2 (founder timed test on the holdout,
  application + library class), A4 (byte-determinism), A7/A11 (units/anchors/denominators), A9
  (canvas purity), A10 (state-unconditional) and A12 (no new JS).
- No schema change without a separate additive plan; the holdout stays vaulted until the A2 test.
- If A2 fails on both application and library classes, this ADR is superseded by a new one
  (candidate: text-first brief or a different core view); it is never silently reinterpreted.

## References

- `docs/NORTH-STAR.md` · `docs/specs/maintainer-map.md` · `docs/specs/map-intelligibility.md` ·
  `docs/specs/map-animation.md` · `docs/specs/atlas-metrics.md` · `docs/specs/atlas-layout.md`.
- `docs/evidence/map-vision/v7/COMPARISON.md`, `f/reads.json`, `g/reads.json`, `NOTES.md`.
- `docs/research/atlas-developer-needs.md` · `docs/research/atlas-data-gap-audit.md`.
- `docs/adr/ADR-0002-visualization.md` · `docs/adr/ADR-0008-map-idiom-decision.md` ·
  `docs/evidence/map-data-coverage.md` §4–§8.
