# Session handoff — ADR-0011 (Atlas Story: companion vs in-report)

- **Date:** 2026-10-26 · **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (`feat/atlas-sim`)
- **HEAD:** `8f0249e` (S4 close; gate `71d0d04`, evidence `2ebeabd`) · **not pushed, not merged**
- **Track:** Atlas Run · **Status:** **ADR-0011 accepted — the Atlas Story remains a standalone
  companion; A12 unchanged; no code.** S4 remains the gated story baseline (`71d0d04`).
- **ADR:** `docs/adr/ADR-0011-atlas-story-companion.md` · **Requirement:** `docs/specs/atlas-story.md`
  (sealed) · **Evidence:** `docs/evidence/atlas-sim/` · **Detail:** `state/tracks/atlas-sim/handoff.md`

## Objective

Decide, from S4 evidence, whether the Atlas Story remains a standalone companion or graduates into
the static report. If in-report with controls, write the A12 amendment and a scoped implementation
plan; if companion, record the decision and stop. No pixel work.

## Decision (D-pf-256 / ADR-0011)

1. **The Atlas Story remains a standalone companion** for v1; it does not graduate into
   `report.html`.
2. **A12 is unchanged.** `report.html` keeps exactly one fixed `REPORT_SCRIPT`; the Atlas
   canvas/SVG/animation artifacts keep zero `<script>`, zero handlers, zero network references. No
   CSS/SMIL story film is added to the report either.
3. **No code, schema, layout or dependency change.** `layout_version`/`flow_version` stay 1; the
   fixed artifact set, `metrics.json`, `report.html` and the `--story` output are untouched.
4. **Graduation is a separate, founder-ratified decision** requiring all of: (a) a founder D2
   session on a **self-contained** story artifact (the `docs/research/atlas-run.md` §5 protocol,
   including the F8/A2-style action read); (b) the report's step-13 founder visual gate passed
   (`R-pf-109` closed); (c) a deterministic, offline, no-network player that is byte-identical on
   double render with a complete final frame and pause/reduced-motion/print; (d) an explicit A12
   amendment naming the second script, its fixed source, data channel and gates.
5. **The companion's packaging is a recorded follow-up, not this session's work.** A precondition
   for D2 is a self-contained artifact (data inlined; no `fetch`, no server) reachable from the CLI.
   This ADR makes **no ship claim** for the companion.
6. **The story stays the temporal companion to the Atlas map**, not a second map: no second metric
   encoding, schema field, time unit or runtime claim.

## Why (evidence)

- **The mechanism is proven.** S4 gate `71d0d04` (tree `51548f3`, dirty 0): 897 tests, `sim/verify.py`
  **304/304**, 3k scale **44/44**, page **463,490 B**, first frame **1,706 ms**, `verdict=PASS`
  (`docs/evidence/atlas-sim/GATE.txt`); F1–F7 pass; F6 keeps `report.html` byte-identical story-off/on;
  F5 keeps `atlas-layout.json` byte-identical on a double scan.
- **The reads pass but are not the founder gate.** F1/scale reads are **PASS-WITH-NITS, no blockers**
  (`VISION.md` Read 8). The pre-registered D2 protocol is a **founder** session; it has not run, and
  F8 is deferred, not claimed (`atlas-story.md:117`; `NOTES.md:186`). V8 showed a comprehension-only
  gate can pass a page the founder rejects (`AGENT-BRIEF.md:49-51`).
- **The report is mid-rework.** `R-pf-109` is open, the step-13 founder visual gate is pending, and
  A2/ship is blocked (`docs/plans/atlas-build.md:7-22`). Graduating now would add a surface to an
  unapproved composition.
- **A12 is load-bearing.** In-report controls need a second script + a data channel + new
  deterministic/offline gates, and a port of the `sim.js` player into the Rust emitter (`sim.js` is
  58,261 B). That is a build project after the value is proven, not before.
- **The companion is not shipped.** It loads data with `fetch('data-index.json')` /
  `fetch(entry.file)` (`sim/sim.js:1328,1426`) and therefore needs an HTTP server;
  `perfpal <repo> --story` adds only `atlas-layout.json` (`docs/report/output-contract.md:90-113`);
  the `perfpal story` subcommand is deferred (`atlas-story.md:157-160`).
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
- **State:** `state/session-state.json` (D-pf-256; A-pf-125 done; phase), `docs/activeContext.md`,
  `state/tracks/atlas-sim/handoff.md`, this handoff.

## Not changed

`report.html`, `metrics.json`, `atlas-layout.json` output, `sim/`, `src/`, `docs/schema/`,
`Cargo.toml`/`Cargo.lock`, `layout_version` 1, the holdouts (vaulted), A2/ship (still blocked).

## Verification

| Claim | Command / artifact | Result |
|---|---|---|
| ADR committed | `git show --stat HEAD` | ADR-0011 + spec notes + CHANGELOG |
| No code/schema change | `git status --porcelain src/ docs/schema/ Cargo.toml Cargo.lock` | empty |
| State JSON valid | `jq empty state/session-state.json` | exit 0 |
| A12 gate text intact | `git diff -U0 docs/specs/atlas-layout.md docs/specs/atlas-feature.md` | dated notes only; gate sentences unchanged |
| Citations resolve | `rg`/`sed` spot-checks on ADR refs | lines match (story §11 shifted to 157-160; corrected) |
| Review panel | R1/R2/Q1/Q2 parallel lanes | see close record |

## Next

1. **Fresh session: Atlas v2 step 11** (G-style detail rail), then step 12 (progressive charts) and
   the step-13 founder/V8 gate. No A2/ship until step 13; `R-pf-109` stays open.
2. **Story follow-up (not active):** self-contained companion artifact + CLI entry point → founder
   D2/F8 session → graduation only per ADR-0011 Decision 4.
3. **Uncommitted:** none after the close commit.
