# Handoff — Latest

## Objective
R1-REDO: the first-pass layout survey cited sources without opening them
(founder-caught same session); redone with three fetch-mandated lanes,
rewritten from fetched evidence only, strict-reviewed, committed. Docs only;
no product code changed. LL-012 filed.

## Phase
POST-MVP map-first. `map-research` R1 ✅ (evidence-backed, strict review clean);
R2 precedents queued; R3 → R4-gate → `map-build` I1–I5 per plan. Paper + JS+TS
parked behind map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, with findings as map
  annotations (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; map-research session 1 of 4 done properly (redo),
  implementation gated on R4 + dual review.
- **How this session serves the north-star:** R1 now locks a VERIFIED layout
  evidence base (fit/misfit vs static-only deterministic constraints) before any
  map code — the research-before-implementation order D-pf-215 mandates, with
  citations that actually open.
- **Drift flag: NONE.** Survey only; no idiom chosen (R4's job). Audit counter 0.

## This session — R1-REDO result
- **Failure admitted:** first pass (committed `4a67ba6`) was parametric synthesis
  with decorative citations; reviewers flagged "not fetched" and the commit
  landed anyway. Root causes: prompt specified citation format not procedure;
  review caveats treated as notes not blockers. See LL-012.
- **Redo:** 3 parallel fetch-mandated lanes — lib-2 (Sugiyama: dotguide,
  Graphviz forum/orders; force: FR-1991 Illinois PDF, FA2 PLOS, d3-force sim +
  Bostock gist), lib-3 (treemap+HEB: Holten excerpts, squarified text, d3/Vega;
  DSM: MIT-hosted Sangal OOPSLA, Lattix DSM/DesignRules/enforcement guides),
  lib-4 (flame: Gregg page + README + flamegraph.pl source; city: Wettel08/11
  excerpts + ACM abstract, +24.26%/−12% verified as overall means; Voronoi:
  Berkeley excerpts + NoBr12a + arXiv survey).
- **Deliverable:** `docs/research/map-algorithms.md` (381 lines, 36 dated
  citations, dead-ends section supporting nothing, explicit Holten-counts gap).
- **Reviews:** rev-4 (requirements) APPROVED clean; rev-3 (strict)
  CHANGES_REQUESTED with exactly 2 missing inline citations → both fixed
  (citation-only, no new claims) → counted clean per reviewer's own
  normalize-condition. No reviewer disagreement suppressed: the 2 findings were
  real, fixed, and reported here.
- **Corrections vs first pass worth noting:** FR is random-init (not seedable
  out of the box); FA2 explicitly non-deterministic; only d3-force ships a
  determinism recipe; flame default colors are random unless `--hash`;
  DSM has no published determinism story (pinning required); Sugiyama has no
  published node-count ceiling.

## Verification
| Claim | Command | Result |
|---|---|---|
| Redo doc: 7 families, 36 dated cites | `grep -c 2026-10-17` + `grep ^## Family` | 36 hits, 7 families |
| Reviews | rev-3 + rev-4 terminal | CHANGES_REQUESTED(2 fixed) / APPROVED |
| No product code changed | `git status --short` (pre-commit) | research + state/docs/LL only |
| Hot-state hygiene | byte count post-demotion | **7599 B / 8192 B** (demoted done A-pf-085 + stale A-pf-051/073) |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact placeholder), **`R-pf-082`** (medium;
  synthetic-corpus credibility), **`R-pf-007`** (medium; differentiation claims),
  **`R-pf-094`** (low), **`R-pf-095`** (low; Windows residuals).
- **Blockers (unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Hot-state hygiene:** **7599 B / 8192 B** — comfortable again.
- **Follow-ups:** gate-path fix (task-gate.sh missing, carried); R4 carries
  flame-graph exception burden; R2–R4 dispatches must use fetch-mandated prompts
  (LL-012 fallback).

## Next Actions
1. **NEXT SESSION (`/new`): R2 precedents** → `docs/research/map-precedents.md`
   (Sourcetrail, code2flow, NDepend/Structure101/Lattix, CodeSee/Gource,
   madge/depcruise — helpful vs eye-candy, FETCHED sources per LL-012). Read
   state + this handoff first.
2. Then R3 → R4 (ADR gate, dual review) → I1–I5. M5 needs B-pf-001 + B-pf-002
   (founder-owned, A-pf-080).

## Change set (this session)
- **Research:** `docs/research/map-algorithms.md` (rewritten 202→381 lines).
- **Lessons:** `docs/lessons-learned.md` (+LL-012).
- **State:** `state/session-state.json` (demoted A-pf-085/051/073, phase
  advanced; 7599 B), `state/archive/ledger.jsonl` (+3 demotion records), this
  handoff, `docs/activeContext.md`, `CHANGELOG.md`.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted);
first periodic audit 2026-10-16 (counter 0).
