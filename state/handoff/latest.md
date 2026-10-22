# Handoff — Atlas research/planning (docs only)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. This session turned the
founder's D-pf-238 correction (G core, F supporting, no mode switch, graph page 100% graph) into
contracts: needs model, metric set, data decision, layout, inclusion rule, ADRs and a build plan.
No drift; no implementation.

**Phase:** Atlas research/planning **complete**. Next gate is **founder ratification of ADR-0009 /
ADR-0010 and the V8 spec amendments** (A-pf-114); then execute `atlas-build` (A-pf-115). Holdout
stays vaulted until the A2 test; `map-build` remains NOT done; A1b not claimed closed.

**Commit base:** `60dac49` (V7 prototypes + 12/12 blind reads). This session: docs/state only; **no
`src/` change, no schema change**.

## This session — Atlas planning

- **Developer-needs research** (`docs/research/atlas-developer-needs.md`): needs N1–N6 (start, blast
  radius, cost, mass, trust/noise, next action); the fixed **seven-block below-graph brief** (answer
  line, KV1 actions, weight strip, blast radius, file cards, noise footer, drill links); the noise
  ledger; the **six-test component-inclusion rule** applied to F; falsifiers (§9). Grounded in the
  12/12 V7 blind reads + prior art (Sourcetrail/CodeCharta/NDepend/Understand, Sillito FSE'06,
  LaToza ICSE'10, hairball/split-attention/progressive-disclosure). Records the honest caveat that
  G's cJSON read answers "which entry", not Q1.
- **Inclusion rule outcome:** weight mass **PASS as a strip (provisional pending V8 strip-specific
  reads)**; treemap
  **FAIL** on the page → Explorer-only `weight-treemap`; production-first **PASS** as an ordering
  rule; component cells → hulls/rows. No mode switch; state-unconditional.
- **Metrics contract** (`docs/specs/atlas-metrics.md`): M-01…M-18; one measured encoding (cost) on
  the canvas; no time units (`ms`/`µs`/`runtime` forbidden); unit + denominator-adjacent anchors;
  `× median function` / `top P%` / `% of scanned static cost`; thousands separators for huge
  multiples; stubs never blanks; severity/confidence text-only; forbidden-strings test list. No new
  JavaScript (A12): the report keeps one fixed existing `REPORT_SCRIPT`; the canvas/SVG/animation
  artifacts have zero scripts.
- **Data-gap audit** (`docs/research/atlas-data-gap-audit.md`): element-by-element inventory with
  `file:line` citations; **no schema change required**; all chart-side derivations; I/O sinks
  downscoped; additive candidates (`components[].member_paths[]`, `entry_id`, `io_sink`,
  `call_site_count`, `cost_summary`) recorded with triggers; policy conflicts named (structural vs
  `reachable_from_entry`, component identity, median convention, entry abort).
- **Layout spec** (`docs/specs/atlas-layout.md`): canvas anatomy (graph chrome only), the seven
  blocks, eight scan states with the same page shape, drills, animation timeline, print/reduced
  motion, accessibility, A1/A2/A8-amended/A9/A10 checks.
- **Feature spec** (`docs/specs/atlas-feature.md`): goals, non-goals, acceptance, rollback.
- **ADRs (Proposed):** `ADR-0009-atlas-core.md` — entry-flow core, fixed page, weight strip,
  structural-edge policy (supersedes ADR-0008 in part; amends the ADR-0002 call-graph ban for the
  Atlas canvas); `ADR-0010-declarative-animation.md` — amends ADR-0002 for CSS/SMIL animation.
- **V8 amendments:** `map-intelligibility` (placement table, A8 amendment for the Atlas page, new
  A9/A10/A11), `maintainer-map` (Atlas composition), `map-animation` (Atlas timeline).
- **Build plan** (`docs/plans/atlas-build.md`): 10 steps — ratification gate → view model → metric
  formatting → canvas renderer → brief/drills → `#map` wiring → tests/goldens → vision gate V8 →
  founder A2 (holdout + pre-registered library class) → ship/EOL; runnable DoD.
- **State:** D-pf-239/D-pf-240 appended; D-pf-231…235 demoted to the cold ledger; A-pf-113
  completed; A-pf-114/115/116 queued; R-pf-108 added.

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| All 8 new deliverables exist | `ls` of the 8 paths | present (4.8–17.5 KB each) |
| No code/schema change | `git status --short src/` | 0 lines; `docs/schema/` untouched |
| Whitespace clean | `git diff --check` | clean |
| State JSON valid | `jq -e '.decisions\|length' state/session-state.json` | 5 decisions (D-pf-236…240) |
| Plan links specs | `grep -c 'atlas-' docs/plans/atlas-build.md` | 6 |
| No-time rule present | `grep -c 'No time' docs/specs/atlas-metrics.md` | 1 |
| ADR-0008 pointer | `grep -c 'superseded in part' docs/adr/ADR-0008-map-idiom-decision.md` | 1 |
| A8 amendment recorded | `grep -c 'amends A8' docs/specs/map-intelligibility.md` | 1 |
| Holdout vaulted | v7/v8 evidence + scans | vaulted; no agent reads |
| Dual review | `@reviewer` + `@requirements-reviewer` | Round 3: **both APPROVED** (reviewer: no blocking; requirements: APPROVED for commit) |

## Risks / open

- **R-pf-108 (new):** ADR-0009/0010 are Proposed — founder ratification required before any build;
  A2 must pass on application **and** library classes. Structural LOW edges on the hero canvas
  remain the main honesty risk (ADR-0009 option A; reversal trigger recorded).
- **Recorded falsifiers:** first-screen (Q1 + `path:line` without scrolling), library (canvas
  dead-end), weight-strip (no read uses it), anchor/denominator, noise threshold
  (`docs/research/atlas-developer-needs.md` §9).
- **R-pf-104/105/106** remain open: A1b recall is carried by the brief's complete-list guarantee,
  but the KV1 rank-key / call-weighted Q3 question moves into `atlas-build`.
- `map-build` remains NOT done; A1b not closed; blockers unchanged: B-pf-001 (contact), B-pf-002
  (signing secrets).

## Next actions — new-session signposts

1. **Ratify** ADR-0009 + ADR-0010 and the V8 amendments; confirm D-pf-239/240 (A-pf-114).
2. **Pre-register** the library-shaped A2 holdout repo before the vision gate (A-pf-116).
3. **Execute `atlas-build`** (A-pf-115) — read the plan + `atlas-metrics` + `atlas-layout` first.
4. Do **not** mark `map-build` done; do not claim A1b closed; holdout stays vaulted until A2.

## Change set (uncommitted)

- **New:** `docs/research/atlas-developer-needs.md`, `docs/research/atlas-data-gap-audit.md`,
  `docs/specs/atlas-metrics.md`, `docs/specs/atlas-layout.md`, `docs/specs/atlas-feature.md`,
  `docs/adr/ADR-0009-atlas-core.md`, `docs/adr/ADR-0010-declarative-animation.md`,
  `docs/plans/atlas-build.md`.
- **Amended:** `docs/specs/map-intelligibility.md` (V8), `docs/specs/maintainer-map.md` (Atlas
  composition), `docs/specs/map-animation.md` (Atlas application), `docs/adr/ADR-0002` (amendment
  pointer), `docs/adr/ADR-0008` (superseded-in-part pointer).
- **State/docs:** `state/session-state.json`, `state/archive/ledger.jsonl`, `docs/activeContext.md`,
  `CHANGELOG.md`.
- **No `src/`, schema, test, golden or evidence-tree change.**
