# Handoff — Atlas ratification (docs/state only)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. This session converted
the founder's ratification into binding status: **ADR-0009 (Atlas core) and ADR-0010 (declarative
animation) are Accepted**, the V8 amendments are ratified, and D-pf-239/D-pf-240 are confirmed.
The next session builds the ratified contract. **No drift; no implementation.**

**Phase:** Atlas **ratified 2026-10-23**. Next: `atlas-build` steps **2–4** (view model, metric
formatting, canvas renderer + unit tests) in a fresh session. The founder A2 remains the final gate.

**Commit base:** `74a948b`. This session: docs/state/evidence only; **no `src/` change, no schema
change**.

## This session — ratification (A-pf-114) + holdout pre-registration (A-pf-116)

- **ADR-0009 → Accepted** (`docs/adr/ADR-0009-atlas-core.md`). Added a Ratification section recording:
  - **Structural edges = option A.** The canvas draws the default-tier set (LOW name-based + MEDIUM
    semantic), labelled `structural (LOW name-based + MEDIUM semantic), may over/under-approximate`,
    prints the `N MEDIUM / M LOW` mix, and never prints `resolved`/`reachable_from_entry` for that
    set. Resolved-only is rejected because the default tier has 0 resolved edges. Reversal trigger
    stays live.
  - **A8 amendment.** First block = canvas carrying **entry + HOTTEST + path trace**; the one
    `path:line` action is in the first scroll (answer line + KV1). Legacy list-first A8 applies only
    to the old report.
  - Decision 3 now states the A8 placement explicitly; no schema change confirmed.
- **ADR-0010 → Accepted** (`docs/adr/ADR-0010-declarative-animation.md`). Added a Ratification section
  recording **A12**: the report keeps **exactly one fixed existing `REPORT_SCRIPT`** (unchanged); the
  Atlas canvas/SVG/animation add **zero scripts**, zero event handlers, zero network; CSS keyframes/
  SMIL only. Presentation-layer, deterministic-timeline and scope clauses confirmed.
- **V8 amendments ratified** — `docs/specs/map-intelligibility.md` now carries a Ratified 2026-10-23
  note for the A8 first-screen amendment, the structural-edge disclosure and A12. `atlas-layout.md`
  status line updated; `atlas-feature.md` names the pre-registered library holdout.
- **Plan activated** — `docs/plans/atlas-build.md` status = `active`; step 1 (ratification gate)
  marked **DONE 2026-10-23**. Step 2 is the next unit of work.
- **Library-shaped A2 holdout pre-registered** (A-pf-116): the **published `memchr` crate payload**
  at `bd6068c30e9074a90c285e47912fa0b047d07597` (root crate has no `[[bin]]`, no `src/bin`, no
  `build.rs`, no workspace; 45 `src/` `.rs` files; `Unlicense OR MIT`; **0 design-corpus mentions**).
  The repo's dev `fuzz/` + `benchmarks/` subtrees (16 bins / 10 workspaces / 1 bench build.rs) are
  **excluded from the A2 scan**; the protocol extracts the crate payload. Evidence:
  `docs/evidence/map-vision/v8/PREREGISTRATION.md` (structural preflight only; no Perfpal scan).
  Vaulted until A2.
- **State hygiene:** D-pf-239/240 marked RATIFIED; new D-pf-241 (structural edges), D-pf-242 (A8),
  D-pf-243 (A12); A-pf-114/116 completed; A-pf-110 superseded; R-pf-107 resolved; D-pf-236/237,
  A-pf-107/112/113 demoted to the cold ledger; artifacts moved cold. Hot file **5,761 B** (≤ 8 KB).

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| Both ADRs Accepted | `rg -l 'Status:.*Accepted' docs/adr/ADR-0009-atlas-core.md docs/adr/ADR-0010-declarative-animation.md \| wc -l` | 2 |
| V8 ratification recorded (3 specs) | `rg -l 'Ratified 2026-10-23' docs/specs/map-intelligibility.md docs/specs/maintainer-map.md docs/specs/map-animation.md \| wc -l` | 3 |
| Plan active | `rg -o 'plan status: \w+' docs/plans/atlas-build.md` | `plan status: active` |
| Holdout pinned, crate payload scoped | `rg -c 'bd6068c30e9074a90c285e47912fa0b047d07597' docs/evidence/map-vision/v8/PREREGISTRATION.md` (2); `rg -c 'crate payload at the pin' …` (2) | pass |
| Dev subtrees excluded | `rg -c '16 .*targets' docs/evidence/map-vision/v8/PREREGISTRATION.md` | 1 |
| Change set confined | `git diff --name-only 74a948b` | docs/state/evidence only |
| A12 scope recorded | `rg -c 'zero scripts' docs/adr/ADR-0010-declarative-animation.md` | 2 |
| A8 amendment recorded | `rg -c 'entry \+ HOTTEST \+ path' docs/specs/map-intelligibility.md` | 2 |
| No code/schema change | `git status --short src/ docs/schema/ src/schema/` | 0 lines |
| Whitespace clean | `git diff --check` | clean |
| State JSON valid | `jq -e '.schema_version==2 and (.decisions\|length)==6 and (.next_actions\|length)==7' state/session-state.json` | 6 decisions / 7 active actions |
| Hot budget | `wc -c < state/session-state.json` | 5,761 B |

## Risks / open

- **R-pf-108:** ratification resolved; **A2 on application + library classes is the decisive gate**.
  The structural LOW/MEDIUM hero canvas remains the main honesty risk (option A; reversal trigger
  recorded in ADR-0009).
- **R-pf-104/105/106** remain open: A1b recall is carried by the brief's complete-list guarantee,
  but the KV1 rank-key / call-weighted Q3 question moves into `atlas-build`.
- **Recorded falsifiers:** first-screen (Q1 + one `path:line` without scrolling), library (canvas
  dead-end), weight-strip, anchor/denominator, noise threshold
  (`docs/research/atlas-developer-needs.md` §9).
- `map-build` remains **NOT done**; A1b not closed; django/postgres + `memchr` holdouts vaulted;
  blockers B-pf-001 (contact) + B-pf-002 (signing secrets) unchanged.

## Next actions — new-session signposts

1. **`/new` → `atlas-build` session 1: steps 2–4 ONLY.**
   - Read `docs/plans/atlas-build.md`, `docs/specs/atlas-metrics.md`, `docs/specs/atlas-layout.md`,
     `docs/adr/ADR-0009-atlas-core.md`, `docs/adr/ADR-0010-declarative-animation.md`.
   - **Step 2:** view model `src/reporter/atlas.rs` — entry election, confidence split, BFS
     depth/parent/reachable, caps, file cost, component hulls, production-first shares, dominant
     finding, top functions, blast radius, coverage/noise, representative path; reuse
     `views.rs` scale ordering; deterministic sorts; unit tests.
   - **Step 3:** metric formatting `src/reporter/atlas_metrics.rs` — `atlas-metrics.md` M-01…M-18,
     anchors, separators, stubs, no-time rule; unit tests.
   - **Step 4:** canvas renderer `src/reporter/atlas_render.rs` — deterministic SVG/HTML, depth
     columns, nodes/edges, hulls, unreached, path trace, legend, caps, structural caption, CSS/SMIL
     + pause/reduced-motion/print; zero new scripts; double-render byte-identical; unit tests.
   - Do **not** start steps 5–6 (brief/drills, `#map` wiring) until 2–4 are green. No schema change.
2. **Keep the holdouts vaulted:** django/postgres + `BurntSushi/memchr` until the founder A2; no
   agent reads of their scans/renders; no fixtures copied from them.
3. **Do not mark `map-build` done.** A1b stays open.
4. A2 (application + library, animation on) is the final gate; a failure supersedes the ADR — never
   tune silently.

## Change set (uncommitted)

- **Amended:** `docs/adr/ADR-0009-atlas-core.md`, `docs/adr/ADR-0010-declarative-animation.md`,
  `docs/adr/ADR-0002-visualization.md`, `docs/specs/map-intelligibility.md` (V8 ratification note),
  `docs/specs/maintainer-map.md`, `docs/specs/map-animation.md`, `docs/specs/atlas-layout.md`,
  `docs/specs/atlas-feature.md`, `docs/plans/atlas-build.md`, `CHANGELOG.md`,
  `docs/activeContext.md`, `state/session-state.json`, `state/archive/ledger.jsonl`.
- **New:** `docs/evidence/map-vision/v8/PREREGISTRATION.md`.
- **No `src/`, schema, test, golden or chart change.**
