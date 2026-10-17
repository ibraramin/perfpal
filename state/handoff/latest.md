# Handoff — Latest

## Objective
I1b DONE: Rust entry-point detection + honest-empty reachability (founder-approved
option 1) — `main` HIGH + handler MEDIUM + lib-root MEDIUM (`main.rs`
bin-exclusion) + honest `unknown-entry` LOW stub; `resolved_rust_edges`
semantic-fact-only incl. intra-file, honestly EMPTY (Rust syntactic-only,
no LOW fallback — rejected option 3); no new IR/schema fields (v1.5 reused);
7 new `i1b_entry_rust` locks; C goldens untouched. Commit `e62184e` (code)
+ state closeout.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1a ✅ (C) + I1b ✅
(Rust), I1c (Python) queued next. Paper + JS+TS parked behind map;
M5 parked behind B-pf-001/002.

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
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| tests | `cargo test --locked` | 641 passed / 0 failed (634 + 7 new) |
| licenses | `cargo deny check licenses` | ok (exit 0) |
| ratchets | `cargo test --locked --test benchmark --test precision_kpi` + `git diff --name-only` | 1+1 pass, NO_RATCHET_FILES_TOUCHED |
| rev-1 | read-only review | APPROVED, no blocking findings |
| rev-2 | requirements check (7 clauses) | code 7/7 PASS; procedural gaps closed by gate evidence above |
| commit | `git log --oneline -1` | `e62184e` I1b code, independent |

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
1. **NEXT SESSION (`/new`): I1c Python entry-points + reachability** — same
   contract (`if __name__` + entry scripts), additive only, `cargo test`
   green, independent commit.
2. Then I2 cost-v2 → I3 map layer → I4 annotations → I5 proof+demo.
   M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Change set (this session)
- **Code:** Rust detector trio + widened C+Rust gate, `tests/i1b_entry_rust.rs`
  (7 locks), C goldens untouched, findings byte-identical — commit `e62184e`.
- **State/docs:** `state/session-state.json` (A-pf-091 done, A-pf-092 queues
  I1c; D-pf-218 covers I1a, I1b decision in handoff+commit msg), this handoff,
  `docs/activeContext.md`, `CHANGELOG.md` (state commit follows code commit).

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted);
first periodic audit 2026-10-16 (counter 0).
