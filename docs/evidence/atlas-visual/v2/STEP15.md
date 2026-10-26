# Step 15 — ship / docs / EOL (Atlas v2)

**Status:** docs pass complete; **full DoD 1–13 PASS**; parallel panel **APPROVED** (round 1: 4/4
CHANGES_REQUESTED → fixes; round 2: R1/R2/Q1/Q2 APPROVED, R3 transcript-reproducibility finding;
round 3: R4 APPROVED + R3 re-review APPROVED). **`atlas-build` marked done 2026-10-26 (D-pf-258).**
No code, schema, JS, dependency or golden change; no visual tuning. **Committed `57d91a2`; no push.** **Date:** 2026-10-26. **Owner:** orchestrator.
**Tree:** `main` HEAD `07f557c` at the gate; committed `57d91a2` (docs/state/evidence).

## 1. Gate decisions

### D-pf-256 — memchr library-class first-page-fit residual **ACCEPTED** (beta)

On the published `memchr` crate payload (`bd6068c3`), the Atlas canvas bottom is **1,282.2 px** at
1440×900 (canvas height 1,162.7; 23 production + 7 non-production files in the UNREACHED cluster) —
382 px below the fold. The application class (postgres `856a659`) fits at **871.2 px**. The residual
is the real instance of the step-13 capped-state disclosure.

**Decision:** accept as a **documented beta residual**, not a silent omission and not a layout
contract pass. A2 was a delegated PASS on both classes (4/4 blind lanes named three concrete actions
in 87–120 s and answered understandable YES; pause/reduced-motion/print PASS). The strip, rail and
first scroll carry the actions even when the canvas runs past the fold. The fix (cap/aggregate the
UNREACHED column) would reopen the approved step-13 composition and requires code + tests/goldens +
a fresh vision/A2 gate — out of scope for step 15 under the no-tuning constraint.

**Recorded in:** `docs/evidence/accepted-limitations.md` §D; `docs/specs/atlas-layout.md` §8 beta
exception; `README.md` (Beta status); `CHANGELOG.md`; this file. **Fix trigger:** first post-beta
layout pass or a real user report of the fold as a blocker.

### D-pf-257 — the delegated A2 PASS closes the A2 gate for this beta

The A2 run (postgres + memchr, 4/4 lanes, pause/reduced/print PASS) was executed under the founder's
Session A4 instruction and recorded as a delegated, evidence-based proxy (`docs/evidence/map-vision/v8/a2.md`;
`D-pf-255`). It closes the A2 gate for this beta. **The founder's own timed eye-test remains the V6
gold standard and is an open record:** the holdout is released and its three actions are published,
so an untainted founder re-run would need a fresh holdout. This is recorded, not waived.

## 2. Docs pass

| File | Change |
|---|---|
| `README.md` | Atlas page section; full artifact layout incl. `atlas-weight.html`, `atlas-q2.html`, `agent-report.md`, `agent-actions.json`; schema 1.7; beta limitation D-pf-256; Atlas/agent doc pointers; Rust catalog counts corrected to 22 (67 total) |
| `docs/report/output-contract.md` | `report.html#map` Atlas-page contract; `atlas-weight.html` / `atlas-q2.html` stability rows; agent-report links; consumption entry points; schema examples 1.7; canvas is inline SVG with no `<script>`/handlers/network |
| `docs/specs/atlas-feature.md` | Surfaces + goals reflect the two-layer page; A2 execution + D-pf-257 beta closure; A8/A10 re-baselined; A1b traceability vs open severity ranking; **F treemap candidate is deferred** (D-pf-251) |
| `docs/specs/atlas-layout.md` | §1/§5/§7/§8/§9 updated to the shipped strip + canvas + rail + four charts; explicit ≤900 first-page-fit clause + D-pf-256 beta exception; v1 seven-block record retained as history; F treemap deferred |
| `docs/specs/maintainer-map.md` | Atlas composition table + question placement updated; first block = strip + canvas; seven-block brief retired; F treemap deferred |
| `docs/specs/map-intelligibility.md` | Step-12 placement amendment; A8 re-baseline; rail/no-mode-switch wording; F treemap deferred |
| `docs/specs/atlas-metrics.md` | Explorer row / cross-scale anchor wording now says the treemap candidate is deferred |
| `docs/specs/map-animation.md` | Final frame = strip + canvas + rail + four charts; pause sits in the masthead outside the canvas; standalone drills are static |
| `docs/evidence/map-vision/v7/README.md` | V7 prototypes marked **archived evidence** (G shipped, F undecided, not current output) |
| `docs/evidence/accepted-limitations.md` | §D Atlas library-class first-page-fit limitation + fix trigger |
| `CHANGELOG.md` | Step-15 entry (docs, decisions, DoD 1–13, panel approved, plan done, commit `57d91a2`, no push) |

**"Whole-tenth" vs "one decimal":** no shipped doc carries "whole-tenth" as a rule; the stale phrase
existed only in the step-13 evidence records (history, left intact). The binding wording is
`atlas-metrics.md` §5 ("one decimal", including the paired shares from the same integer-tenths
split). **No doc change was required.**

## 3. Definition of Done

Transcript: `docs/evidence/atlas-visual/v2/gate-step15.txt` (cargo 1.92.0; each command's exit code
recorded). **Every command exited 0; OVERALL: PASS; SUPPLEMENTAL: PASS.** The transcript was
**regenerated 2026-10-26** after the R3 confirmation lane found that the first recorder
(`printf '%s' "$*"`) dropped shell quoting, so the displayed `bash -c` lines were not copy-paste
reproducible. The recorder now uses `printf '%q'`, the full gate was re-run, and the results were
identical; the transcript also carries a scope snapshot (`git status --short`, `git diff --stat`).

| # | Command (as recorded) | Result |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | exit 0 |
| 2 | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| 3 | `cargo test --locked` | exit 0; lib **402 passed**, all suites 0 failed |
| 4 | `cargo deny check licenses` | exit 0 (five benign `license-not-encountered` warnings, `licenses ok`) |
| 5 | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| 6 | `env PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0; lib 402 passed |
| 7 | `cargo test --locked --test atlas_render atlas_double_render_is_byte_identical` | exit 0 (1 passed) |
| 8 | `cargo test --locked --test atlas_render atlas_adds_no_new_script` | exit 0 (1 passed) |
| 9 | `cargo test --locked --test atlas_render atlas_canvas_has_no_metric_panels` | exit 0 (1 passed) |
| 10 | `cargo test --locked --test atlas_states atlas_states_render_same_stack` | exit 0 (1 passed) |
| 11 | `bash -c 'test -z "$(git status --short src/schema/ docs/schema/)"'` | exit 0 (no schema change) |
| 12 | `bash -c 'test -f …/reads.json && test -f …/a2.md'` | exit 0 (both evidence files present) |
| — | `! grep -q preserve_order Cargo.lock` | exit 0 (byte-determinism guard) |
| — | `test -z "$(git status --short src/ Cargo.toml Cargo.lock)"` | exit 0 (no code/dependency change) |
| — | `test -z "$(git status --short tests/golden/)"` / `tests/` | exit 0 (no golden/test change) |
| — | `test -z "$(git status --short -- "*.js" "*.mjs" "*.ts")"` | exit 0 (no JS/TS change) |
| 13 | `@reviewer` + `@requirements-reviewer` on the final tree | **APPROVED** — R1/R2/Q1/Q2 (round 2) + R3/R4 (round 3) after fixes; zero unresolved blocking findings. Record in §4. |

## 4. Parallel review panel

**Round 1 (2026-10-26) — 4/4 lanes CHANGES_REQUESTED.** R1 (docs-to-code correctness), R2
(contract/honesty), Q1 (verbatim scope), Q2 (acceptance/evidence). Blocking findings and fixes:

| Finding (lanes) | Fix |
|---|---|
| Output contract still used schema `1.2` examples (R1, R2) | Updated to `1.7` in the layout, stability row and consumer mapping |
| Specs claimed the deferred F `weight-treemap` ships in Explorer (R1, R2, Q1) | All touched specs now say the F treemap candidate is **deferred** (F undecided, D-pf-251); `#explorer` is the 8 legacy stems |
| Output contract said the Atlas canvas contains "no SVG" (R1, R2) | Corrected to "inline SVG with no `<script>`/event handlers/network" |
| `maintainer-map` kept a stale canvas-first claim (R1) | Replaced with the key-metrics strip + canvas first block (step-12 A8) |
| `atlas-q2.html` row promised a `% of source files` anchor the drill does not print (R1) | Row now lists the actual drill content (dependents, include coverage, DSM, caller/callee counts) |
| README said 23 Rust rules / 46 total (R1) | Corrected to 22 Rust / 67 total (23 C + 22 Rust + 22 Python) |
| CHANGELOG claimed "panel green / plan done" before the panel (R1, R2, Q1, Q2) | Reworded to DoD 1–12 PASS + panel/plan-done pending |
| EOL state not yet written (R2, Q2) | Written after the confirmation panel approves (required ordering) |
| `atlas-layout` did not record the ≤900 first-page-fit clause the residual cites (R2) | §8 now states the ≤900 clause and the D-pf-256 library-class beta exception |
| A2 beta closure not cited in the feature spec (R2) | Added D-pf-257 |
| V7 banner dated 2026-10-24 (Q1) | Dated 2026-10-26 |
| No golden/no-JS check in the transcript (R1, R2, Q2) | Supplemental checks added: `tests/golden/`, `tests/`, JS/TS — all exit 0 |

**Round 2 (2026-10-26) — R1/R2/Q1/Q2 re-verified the fixes and APPROVED.** The fresh R3 lane found
that the first recorder displayed `bash -c` commands without their quoting, so the recorded lines
were not copy-paste reproducible even though the checks had run correctly. **Fix:** the transcript
was regenerated with `printf '%q'`, the full gate re-run (identical PASS results), and a scope
snapshot added.

**Round 3 (2026-10-26) — R4 fresh confirmation APPROVED; R3 re-review APPROVED after the marker
fix.** R3 found the regenerated transcript lacked `SUPPLEMENTAL: PASS` while `STEP15.md:61` claimed
it. **Fix:** the three supplemental checks + the evidence-file conjunction were re-run and the marker
emitted; the transcript now ends with a `## Final summaries` block carrying both `OVERALL: PASS` and
`SUPPLEMENTAL: PASS`. R4 (independent evidence/reproducibility lane) APPROVED; R3 re-review
APPROVED on the corrected transcript. **Panel result: R1/R2/Q1/Q2 (round 2) + R3/R4 (round 3) all
APPROVED; zero unresolved blocking findings; DoD #13 PASS.**

## 5. Scope guard

No `src/`, schema, JS, dependency or golden change; no page tuning; no feature. **`atlas-build` is
marked done 2026-10-26 (D-pf-258); commit `57d91a2`; no push.** `.playwright-mcp/` is
untracked scratch and is not part of the change set.
