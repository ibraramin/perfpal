# Handoff — map prototypes V7 (F/G) + founder correction

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. V7 built the two
candidate idioms; the founder chose **G as the core graph** and kept **F's components as
supporting elements**. No drift.

**Phase:** V7 prototype burst complete. Next is **research + planning only** (`map-shipped`):
developer-needs research → metrics standardization → data-gap audit → full-page combined layout →
component-inclusion rule → naming → spec/ADR → final build plan. **No implementation.**

**Commit base:** `0e24c30` (V6a). This session: plans/evidence/docs/state only; **no `src/` change**.

## This session — V7

- Formal plans created and linked: `map-f-treemap`, `map-g-flow`, `map-shipped`;
  feature spec `map-animation` (mandatory declarative animation, no JS).
- Built two deterministic Python prototypes from `out-v6-{cJSON,hyperfine,tmux}`:
  - **F — weight map:** component treemap (area = static cost, one ramp, production-first,
    test/bench hatched), KV1 brief, Q2 1-hop drill. CSS rank-reveal animation.
  - **G — entry-flow map:** entrypoint (production `main`; library root otherwise) at depth 0,
    file nodes on BFS call-edge rings, per-file key-function cards, component hulls, unreached
    tail, numbered path trace. CSS/SMIL progression animation.
- Both: 1440×900 start/mid/final frames, CSS-only pause, reduced-motion/print final frames,
  zero `<script>`/event handlers, units + `× median`/`top P%` anchors, byte-identical HTML on
  double render.
- **12/12 blind reads valid** (2 per candidate per repo; one F cJSON read retried). Evidence:
  `docs/evidence/map-vision/v7/{f,g}/reads.json`, `COMPARISON.md`.
- Failure modes recorded: **F on cJSON** = tests 93.5% of area, production 0.2% unreadable (the
  brief carries the answer); **G on cJSON** = library, no production main, one reachable node,
  no forward path. Hyperfine/tmux read strongly for both.
- **Founder decision (D-pf-238):** G is the **core graph**; F is **not a fallback and not
  discarded** — F-derived components (weight/treemap mass, production-first area, component cells)
  are candidate supporting elements, included only if developer-needs research justifies them.
  Graph page = 100% graph; metrics/brief below or elsewhere. Next sessions = research/planning.
- The founder timed A/B was **not run**; the decision came from the prototypes + 12/12 blind
  reads. The A2 timed test moves to the shipped feature on the holdout.

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| F prototype renders | `python3 docs/evidence/map-vision/v7/f/render_f.py` | exit 0; 3 maps + 3 drills + 9 PNGs |
| G prototype renders | `python3 docs/evidence/map-vision/v7/g/render_g.py` | exit 0; 3 maps + 3 drills + 9 PNGs |
| No JS | `grep -c '<script'` on all emitted HTML | **0** each; no event handlers |
| Units/anchors | grep + Playwright cell/anchor assertions | present; no bare cost |
| HTML determinism | double render + `cmp` | 6/6 F identical; 6/6 G identical |
| PNG size | PNG header check | all 18 frames 1440×900 |
| Blind reads | `jq` on `v7/{f,g}/reads.json` | **12/12 valid** (one retry) |
| Holdout | v7 evidence + scans | **vaulted**; no agent reads |
| No code change | `git status --short src/` | empty |

## Risks / open

- **G is fragile on libraries/test-heavy scans** (cJSON: no forward path). F's width/area can be
  hijacked by test cost (cJSON: 93.5%). Both are recorded, not tuned away.
- Call edges are all **LOW confidence** (static, name-based); component labels are directory-prefix
  based and often test-dominated. These are the main data gaps for the shipped graph.
- `map-build` remains NOT done; A1b is NOT claimed closed (KV1/Q3 recall remains V7/agent-report
  data work).
- Naming: **Perfpal Atlas** (founder, 2026-10-22) — G core flow map; F components as weight-map
  support. The name is recorded in the plan/state.
- Blockers unchanged: B-pf-001 (contact), B-pf-002 (signing secrets).

## Next actions — new-session signposts

1. Start a fresh session; read `map-shipped`, `map-animation`, `map-intelligibility`,
   `maintainer-map`, and `docs/evidence/map-vision/v7/COMPARISON.md`.
2. **First research task:** what a developer actually wants to see **alongside** the main graph
   (actions, metrics, weight mass, blast radius, noise). This defines the metric set.
3. Then metrics standardization, data-gap audit, full-page combined layout, component-inclusion
   rule, naming, spec/ADR amendments, final build plan. **Research/planning only — no code.**
4. Do **not** mark `map-build` done; do **not** claim A1b closed; holdout stays vaulted until the
   shipped feature's A2 test.

## Change set (uncommitted)

- **Evidence:** `docs/evidence/map-vision/v7/` — F/G renderers, 18 PNGs, drills, manifests,
  NOTES, reads, `README.md`, `COMPARISON.md`.
- **Plans:** `docs/plans/map-f-treemap.md`, `docs/plans/map-g-flow.md`, `docs/plans/map-shipped.md`.
- **Spec:** `docs/specs/map-animation.md`.
- **Research:** `docs/research/map-external-review-reconciliation-2026-10-22.md`,
  `docs/research/product-consolidation-options-2026-10-22.md`.
- **State/docs:** `state/session-state.json`, `docs/activeContext.md`, `CHANGELOG.md`.
- **No `src/`, schema, test, or golden change.**
