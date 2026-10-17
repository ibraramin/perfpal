# Handoff — Latest

## Objective
R2 precedents burst: teardown of 10 tools → `docs/research/map-precedents.md`,
verdict matrix + real-world CI usage + local overlap table, dual-gated
(strict APPROVED; requirements BLOCKED on B1/B2 → fixed → re-APPROVED),
committed. Docs only; no product code changed.

## Phase
POST-MVP map-first. `map-research` R1 ✅ R2 ✅ (both evidence-backed); R3
honesty queued; R4-gate → `map-build` I1–I5 per hardened plan. Paper + JS+TS
parked behind map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, findings as map
  annotations (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; map-research 2 of 4 done, implementation gated on R4
  + dual review.
- **How this session serves the north-star:** R2 separates maintainer-helpful
  patterns to steal (code2flow honesty template, depcruiser validation-first,
  Lattix/NDepend rule overlays, CodeScene hotspot×severity gates) from
  eye-candy anti-patterns (Gource motion, CodeSee pictures-without-numbers) —
  directly shaping the R4 idiom decision.
- **Drift flag: NONE.** Teardown only; no idiom chosen. Audit counter 0.

## This session — R2 result
- **Lanes (4, hardened-plan discipline):** lib-5 (Sourcetrail archived 2021 +
  postmortem, code2flow README, Gource; 4 dead ends) · lib-6 (NDepend DSM +
  CQLinq, Structure101 tangles + Sonar acquisition 2024-10-15, Lattix triangles
  + Teradyne 15%; 3 dead ends) · lib-7 (CodeSee $10M shutdown via GitKraken
  press + uninstall docs, madge, depcruiser, gh_grep CI usage across 8 repos,
  sourced 7th CodeScene; 0 dead ends) · exp-2 (local artifact table +
  per-tool overlap-or-gap, file:line pinned).
- **Deliverable:** `docs/research/map-precedents.md` (378 lines, 84 dated
  citations, verdict matrix with discounted marks, dead ends, references).
- **Reviews:** rev-6 (strict) APPROVED with 6 non-blocking suggestions
  (verdict-adjacent cites + gh_grep pins — recorded, not applied; judgment
  call, see Risks) · rev-7 (requirements) BLOCKED on B1 (verdict-level cites
  8/10) + B2 (3rd source for code2flow/CodeScene) → fixed (10 verdict edits +
  2 self-fetched sources: code2flow CHANGELOG, CodeScene pricing; 2 new dead
  ends logged) → rev-8 re-check APPROVED 10/10 + 2/2. No disagreement
  suppressed: rev-6's bar (adjacent cites suffice) vs rev-7's literal
  plan-reading (verdict-level cites) — resolved by meeting the stricter bar.
- **Reviewer noted, deliberately deferred:** rev-6's gh_grep per-bullet pins +
  wild-config coverage for dead/commercial tools; rev-7's suggestion to state
  per-tool why no wild configs exist. Fold into R4 only if R4 needs the
  traceability — R2's intent (tools live in CI, not demos) is met.

## Verification
| Claim | Command | Result |
|---|---|---|
| R2 doc: 10 tools, 84 dated cites | `grep -c 2026-10-17` + `grep ^## [0-9]` | 84 hits, 10 sections |
| Reviews | rev-6 + rev-7 + rev-8 terminal | APPROVED / BLOCKED→fixed / APPROVED |
| No product code changed | `git status --short` (pre-commit) | research + state/docs only |
| Hot-state hygiene | byte count | **7780 B / 8192 B** (A-pf-086 done, A-pf-087 queued R3) |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact placeholder), **`R-pf-082`** (medium;
  synthetic-corpus credibility), **`R-pf-007`** (medium; differentiation claims),
  **`R-pf-094`** (low), **`R-pf-095`** (low; Windows residuals).
- **Blockers (unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Follow-ups:** gate-path fix (task-gate.sh missing, carried); R4 carries
  flame-graph exception burden; deferred rev-6/rev-7 suggestions (gh_grep pins)
  only if R4 needs them.

## Next Actions
1. **NEXT SESSION (`/new`): R3 honesty** → `docs/research/map-honesty.md`
   (cost-without-lying idioms, cap/omission markers, ≥5 fetched venue
   papers/examples). Read state + this handoff first.
2. Then R4 (ADR gate, dual review) → I1–I5. M5 needs B-pf-001 + B-pf-002
   (founder-owned, A-pf-080).

## Change set (this session)
- **Research:** `docs/research/map-precedents.md` (new, 378 lines).
- **State:** `state/session-state.json` (A-pf-086 done, A-pf-087 queued R3;
  7780 B), this handoff, `docs/activeContext.md`, `CHANGELOG.md`.
- **Unchanged:** ledger (no demotions needed this session).

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted);
first periodic audit 2026-10-16 (counter 0).
