# Session handoff — ADR-0011 (Atlas Story: companion vs in-report)

- **Date:** 2026-10-26 · **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (`feat/atlas-sim`)
- **HEAD:** base `8f0249e` (S4 close; gate `71d0d04`, evidence `2ebeabd`) → `daf3bcb` (ADR) →
  `00da6fd` (state) → the panel-fix commit containing this file. **Not pushed, not merged.**
- **Track:** Atlas Run · **Status:** **ADR-0011 accepted — the Atlas Story remains a standalone
  companion; A12 unchanged; no code.** S4 remains the gated story baseline (`71d0d04`).
- **Branch snapshot:** this branch is a **pre-merge story snapshot**. `main` has since recorded
  Atlas v2 done (`c797004`; step 13 approved, A2 delegated pass, `R-pf-109` closed:
  D-pf-255/257/258); this branch still shows `R-pf-109` open. **Reconcile before any merge.**
- **ADR:** `docs/adr/ADR-0011-atlas-story-companion.md` · **Requirement:** `docs/specs/atlas-story.md`
  (sealed) · **Evidence:** `docs/evidence/atlas-sim/` · **Detail:** `state/tracks/atlas-sim/handoff.md`

## Objective

Decide, from S4 evidence, whether the Atlas Story remains a standalone companion or graduates into
the static report. If in-report with controls, write the A12 amendment and a scoped implementation
plan; if companion, record the decision and stop. No pixel work.

## Decision (D-pf-259 / ADR-0011)

1. **The Atlas Story remains a standalone companion** for v1; it does not graduate into
   `report.html`.
2. **A12 is unchanged.** `report.html` keeps exactly one fixed `REPORT_SCRIPT`; the Atlas
   canvas/SVG/animation artifacts keep zero `<script>`, zero handlers, zero network references. No
   CSS/SMIL story film is added to the report either.
3. **No code, schema, layout or dependency change.** `layout_version`/`flow_version` stay 1; the
   fixed artifact set, `metrics.json`, `report.html` and the `--story` output are untouched.
4. **Graduation is a separate, founder-ratified decision** requiring all of: (a) a founder **§5
   graduation session on the post-S4 story** (distinct from the 2026-10-24 D1 review) on a
   **self-contained** artifact, including the F8/A2-style action read; (b) the report's visual gate
   (`R-pf-109` closed; recorded on main 2026-10-26 — confirm at rebase); (c) a deterministic,
   offline, no-network player that is byte-identical on double render with a complete final frame
   and pause/reduced-motion/print; (d) an explicit A12 amendment naming the second script, its fixed
   source, data channel and gates.
5. **The companion's packaging is a recorded follow-up, not this session's work.** A precondition
   for the §5 session is a self-contained artifact (data inlined; no `fetch`, no server) reachable
   from the CLI. This ADR makes **no ship claim** for the companion.
6. **The story stays the temporal companion to the Atlas map**, not a second map: no second metric
   encoding, no core `metrics.json` schema field/version bump (the existing story-only additive
   `atlas-layout.json` blocks are unchanged), no time unit or runtime claim.

## Why (evidence)

- **The mechanism is proven.** S4 gate `71d0d04` (tree `51548f3`; 0 non-evidence dirty paths):
  **897 tests (896 passed, 1 ignored)**, `sim/verify.py` **304/304**, 3k scale **44/44**, page
  **463,490 B**, first frame **1,706 ms**, `verdict=PASS` (`docs/evidence/atlas-sim/GATE.txt`);
  F1–F7 pass; F6 keeps `report.html` byte-identical story-off/on; F5 keeps `atlas-layout.json`
  byte-identical on a double scan.
- **The founder has not used the post-S4 story.** The 2026-10-24 D2 session reviewed **D1** (the
  founder accepted presentation/animation but said the information was not insightful: D-pf-251),
  and that verdict drove S1–S4. The pre-registered §5 protocol on the **post-S4** artifact has not
  run; F8 is deferred, not claimed (`atlas-story.md:117`; `NOTES.md:186`). V8 passed 16/16
  comprehension reads while the founder rejected the page (`AUDIT.md:1-20`;
  `AGENT-BRIEF.md:49-51`).
- **The report gate is snapshot-divergent.** On this branch `R-pf-109` is open and step 13 is
  pending; `main` records it closed 2026-10-26 with A2 delegated pass (`c797004`; D-pf-255/257/258).
  The decision does not rely on the report gate being open; at merge, condition 4(b) is satisfied.
- **A12 is load-bearing.** In-report controls need a second script + a data channel + new
  deterministic/offline gates, and a port of the `sim.js` player into the Rust emitter (`sim.js` is
  58,261 B). That is a build project after the value is proven, not before.
- **The companion is not shipped.** It loads data with `fetch(entry.file)` (`sim/sim.js:1328`) and
  `fetch('data-index.json')` (`:1426`) and therefore needs an HTTP server; `perfpal <repo> --story`
  adds only `atlas-layout.json` (`docs/report/output-contract.md:90-113`); the `perfpal story`
  subcommand is deferred (`atlas-story.md:157-160`).
- **The report contract is singular.** `REPORT_SCRIPT` is one fixed block
  (`src/reporter/mod.rs:1718,2443-2447`); A12 asserts exactly one block in `report.html` and zero in
  the canvas/SVG/animation artifacts (`docs/specs/atlas-layout.md:198-201`; `ADR-0010:25-33,51-54`).

## Changed

- **New:** `docs/adr/ADR-0011-atlas-story-companion.md` — decision, options A–D, consequences,
  gates, graduation triggers.
- **Annotated (no requirement change):** `docs/specs/atlas-layout.md` A12 and
  `docs/specs/atlas-feature.md` A12 (dated ADR-0011 status note: unchanged);
  `docs/specs/atlas-story.md` §11.3 (resolution of the deferred decision).
- **CHANGELOG** `[Unreleased] → Changed` entry.
- **State:** `state/session-state.json` (D-pf-259; R-pf-112 branch divergence; A-pf-125 done;
  phase), `docs/activeContext.md`, `state/tracks/atlas-sim/handoff.md`, this handoff.

## Not changed

`report.html`, `metrics.json`, `atlas-layout.json` output, `sim/`, `src/`, `docs/schema/`,
`Cargo.toml`/`Cargo.lock`, `layout_version` 1, the holdouts (vaulted), A2/ship.

## Verification

| Claim | Command / artifact | Result |
|---|---|---|
| ADR commit | `git show --stat daf3bcb` | ADR-0011 + A12 notes + story §11.3 + CHANGELOG |
| State commit | `git show --stat 00da6fd` | state + activeContext + both handoffs |
| No code/schema change in the commits | `git diff --name-status 8f0249e..HEAD -- src/ sim/ docs/schema/ Cargo.toml Cargo.lock` | empty |
| Worktree clean | `git status --porcelain` | 0 lines after the close commit |
| State JSON valid | `jq empty state/session-state.json` | exit 0 |
| A12 gate text intact | `git diff -U0 8f0249e..HEAD -- docs/specs/atlas-layout.md docs/specs/atlas-feature.md docs/specs/atlas-story.md` | dated notes only; gate sentences unchanged |
| Citations resolve | `rg`/`sed` spot-checks on ADR refs | `sim.js:1328,1426`; `mod.rs:1718,2443-2447`; `output-contract.md:90-113`; `atlas-story.md:117,157-160`; `AGENT-BRIEF.md:49-51`; `atlas-run.md:127-159,275`; `atlas-layout.md:198-201`; `ADR-0010:25-33,51-54` |
| Review panel | R1/R2/Q1/Q2 parallel lanes | R2 **APPROVE**, Q1 **APPROVED**; R1/Q2 **CHANGES_REQUESTED** on record accuracy → fixed; see `state/tracks/atlas-sim/handoff.md` close record |

## Next

1. **Fresh session: Atlas v2 step 11** (G-style detail rail), then step 12 (progressive charts) and
   the step-13 founder/V8 gate — **on this branch**. `main` has already closed steps 13–15; reconcile
   the branch (`R-pf-112`) before starting or merging. No A2/ship from this branch.
2. **Story follow-up (not active):** self-contained companion artifact + CLI entry point → §5
   founder session → graduation only per ADR-0011 Decision 4.
3. **Uncommitted:** none after the panel-fix commit.
