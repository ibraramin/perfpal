# Handoff — Latest

## Objective
R1 algorithms research burst: exhaustive layout survey for the maintainer
birdseye map → `docs/research/map-algorithms.md`, dual-reviewed and committed.
Docs only; no product code changed.

## Phase
POST-MVP map-first. `map-research` R1 ✅ (dual APPROVED); R2 precedents queued;
R3 → R4-gate → `map-build` I1–I5 per plan. Paper + JS+TS parked behind map;
M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, with findings as map
  annotations (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; map-research session 1 of 4 done, implementation gated
  on R4 + dual review.
- **How this session serves the north-star:** R1 locks the layout evidence base
  (fit/misfit vs static-only deterministic constraints) before any map code —
  exactly the research-before-implementation order D-pf-215 mandates.
- **Drift flag: NONE.** Survey only; no idiom chosen (R4's job). Audit counter 0.

## This session — R1 result
- **Deliverable:** `docs/research/map-algorithms.md` (202 lines, ~1500 words) —
  7 families (Sugiyama, force-directed, treemap+HEB, static flame, code-city,
  DSM/matrix + held Voronoi extra), each with mechanism/ceiling/determinism/
  offline/fit/misfit/sources; non-negotiables block with `file:line` citations;
  fit/misfit matrix + net fit-order; R4-deferral + lib-2-supersedes markers.
- **Lanes:** lib-1 (external survey, librarian) + exp-1 (local constraints,
  explorer) in parallel; main-window synthesis; rev-1 (reviewer) + rev-2
  (requirements) dual APPROVED, no blocking findings.
- **DoD note:** `scripts/gates/task-gate.sh` does not exist (assumed path was the
  plan's weakest assumption, caught at runtime); DoD evidence fell back to
  `test -f` + family-`grep` (31 hits) + dual APPROVED. Follow-up: add or correct
  the gate path before R2 (one line, no behavior change).

## Verification
| Claim | Command | Result |
|---|---|---|
| R1 doc exists, 7 families | `test -f docs/research/map-algorithms.md && grep -c -i -e sugiyama -e force-directed -e treemap -e flame -e code-city -e DSM docs/research/map-algorithms.md` | file present, 31 hits |
| Dual review | rev-1 + rev-2 terminal results | APPROVED / APPROVED |
| No product code changed | `git status --short` (pre-commit) | `?? docs/research/map-algorithms.md` only (+ state/docs EOL set) |
| Hot-state hygiene | byte count post-demotion | **8176 B / 8192 B** (was 8414; A-pf-008 → cold ledger) |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact placeholder), **`R-pf-082`** (medium;
  synthetic-corpus credibility), **`R-pf-007`** (medium; differentiation claims),
  **`R-pf-094`** (low), **`R-pf-095`** (low; Windows residuals).
- **Blockers (unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Hot-state hygiene:** **8176 B / 8192 B** — 16 B margin; next session should
  demote another settled item BEFORE appending (candidates: done A-pf-085,
  stale M5-parked A-pf-051/073).
- **Follow-ups:** gate-path fix (task-gate.sh missing); R4 carries flame-graph
  exception burden (spec currently omits flame).

## Next Actions
1. **NEXT SESSION (`/new`): R2 precedents** → `docs/research/map-precedents.md`
   (Sourcetrail, code2flow, NDepend/Structure101/Lattix, CodeSee/Gource,
   madge/depcruise — helpful vs eye-candy, sourced). Read state + this handoff first.
2. Then R3 → R4 (ADR gate, dual review) → I1–I5. M5 needs B-pf-001 + B-pf-002
   (founder-owned, A-pf-080).

## Change set (this session)
- **Research:** `docs/research/map-algorithms.md` (new).
- **State:** `state/session-state.json` (A-pf-085 done, A-pf-086 queued R2,
  A-pf-008 demoted, phase advanced; 8176 B),
  `state/archive/ledger.jsonl` (+1 demotion record), this handoff,
  `docs/activeContext.md`, `CHANGELOG.md`.
- **Excluded:** nothing untracked remains.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted);
first periodic audit 2026-10-16 (counter 0).
