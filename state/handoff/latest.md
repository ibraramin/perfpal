# Handoff — Latest

## Objective
R4 decision burst: matrix vs non-negotiables → `docs/research/map-decision.md`
+ `docs/adr/ADR-0008-map-idiom-decision.md` (zero new external claims, all
cells cite R1/R2/R3), dual-gated (strict APPROVED; requirements APPROVED).
Docs only; no product code changed.

## Phase
POST-MVP map-first. `map-research` R1 ✅ R2 ✅ R3 ✅ R4 ✅ (all evidence-backed);
`map-build` I1 queued (gated on ADR-0008 leaving Proposed). Paper + JS+TS
parked behind map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, findings as map
  annotations (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; research track complete, implementation gated on ADR.
- **How this session serves the north-star:** R4 locks the per-level idiom
  (L0 DSM, L1 Sugiyama-native, L2 treemap+HEB, 3rd entry-stratified) so I1–I5
  builds the honest map the north-star demands.
- **Drift flag: NONE.** Research synthesis only; no idiom prototyping.

## This session — R4 result
- **Deliverables:** `docs/research/map-decision.md` (183 lines: §0 zero-claim
  provenance, §1 non-negotiables, §2 7×5 matrix all cells `[R1 §Family-N]`,
  §3 4 level idioms NOVEL SYNTHESIS, §4 7 rejections, §5 11 legends `[R3 §5]`,
  §6 gaps `[R3 §7]`, §7 gates) + `docs/adr/ADR-0008` (119 lines, Proposed,
  2026-10-17, gates I1–I5, A5 demo as reversal trigger).
- **Answers to fix-1 open Qs:** (1) L2 treemap+HEB kept — HEB bounded to one
  component, L1+3rd share Sugiyama pipeline, no conflict; (2) I1–I5 maps 1:1
  onto birdseye F1–F5 — confirmed; (3) ADR Date 2026-10-17 is local governance,
  not an external access-date — allowed.
- **Reviews:** rev-1 (strict) APPROVED + 2 non-blocking suggestions (tag one
  deferred-alternative line as synthesis; mark ADR process constraint as local
  — carried to I1, not applied) · req-1 APPROVED 4/4 (matrix, levels,
  zero-claims, signposts). No disagreement suppressed.
- **Synthesis honesty:** `showing top S of M`, `…+N more`, aggregated-edge
  counts marked novel synthesis per R3 §7 — no fetched graph-viz source showed
  them.

## Verification
| Claim | Command | Result |
|---|---|---|
| Zero http in new files | `rg "https?://" docs/research/map-decision.md docs/adr/ADR-0008*` | no matches (exit 1) |
| Sizes | `wc -l` | 183 + 119 lines |
| Reviews | rev-1 + req-1 terminal | APPROVED / APPROVED |
| No product code changed | `git status --short` | research + adr + state/docs only |
| Hot-state hygiene | byte count | **8171 B / 8192 B** (D-pf-216 added, A-pf-088 done, A-pf-089 queues I1) |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact), **`R-pf-082`** (medium; synthetic corpora),
  **`R-pf-007`** (medium; differentiation), **`R-pf-094`** (low), **`R-pf-095`**
  (low; Windows residuals).
- **Blockers (unchanged):** **`B-pf-001`** (contact); **`B-pf-002`** (secrets).
- **Follow-ups:** rev-1 2 nits to I1; D3/Gephi idioms uncovered; static-SVG
  label/uncertainty thresholds remain synthesis to validate in I3; third-layer
  rejection amends birdseye F3 via new ADR, never silent.

## Next Actions
1. **NEXT SESSION (`/new`): I1 entry-points + reachability** per `map-build`
   (gated on ADR-0008 leaving Proposed — founder accept). Read state + this
   handoff first.
2. Then I2–I5. M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Change set (this session)
- **Research:** `docs/research/map-decision.md` (new, 183 lines).
- **ADR:** `docs/adr/ADR-0008-map-idiom-decision.md` (new, 119 lines).
- **State:** `state/session-state.json` (phase R4 DONE, D-pf-216 live, A-pf-088
  done, A-pf-089 queues I1; 8171 B), this handoff, `docs/activeContext.md`,
  `CHANGELOG.md`.
- **Unchanged:** ledger (no demotions needed this session).

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted);
first periodic audit 2026-10-16 (counter 0).
