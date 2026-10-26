# Feature spec — merge-train (mainline integration)

- **Status:** active
- **Scope:** feature
- **Plan:** `docs/plans/merge-train.md`
- **Requirement:** R-pf-112 follow-on (founder chose merge-now)
- **Related:** `docs/plans/js-ts.md`, `docs/plans/atlas-s4.md`,
  `docs/adr/ADR-0011-js-ts-backend.md`, `docs/adr/ADR-0011-atlas-story-companion.md`,
  `docs/adr/ADR-0012-atlas-branch-reconciliation.md`, `docs/specs/atlas-story.md`

## Purpose

Integrate all diverged feature lines into `main` as one versioned line **without losing or
corrupting any feature**. This is a **semantic integration**, not a mechanical merge: every conflict
is resolved by reading both sides and combining their behavior, and every merge is validated by the
combined gates.

## Scope

**In scope**

- Verify `agent-report` is already merged; no merge for it.
- Merge `feat/js-ts` into `main` as **v0.3.0** (JS/TS backends, metrics schema `1.8`).
- Merge `feat/atlas-sim` into `main` as **v0.4.0** (Atlas Story companion S1–S4; layout/flow `1`,
  no schema change).
- ADR-number reconciliation (js-ts `ADR-0011`; Atlas renumbers to `0012`/`0013`).
- State/handoff/activeContext reconciliation, version bumps, CHANGELOG/README, gates.

**Out of scope**

- New features, beta-release nomenclature/publishing, tags/pushes, in-report story graduation,
  `map-build`/A1b, runtime/profiling claims.

## Requirements

**R1 — Main is the integration point.** All merges happen in `/home/shafin/Opencode/Perfpal` on
`main`. Feature worktrees are archives; they are not synced or merged in place.

**R2 — Semantic conflict resolution only.** Never use `--force`, `-X ours`, `-X theirs`,
`--no-verify`, or delete a side to "resolve" a conflict. Read `git show main:<path>` and
`git show <feature>:<path>`, combine the features, and record the decision.

**R3 — Merge order.** `agent-report` (already an ancestor) → `feat/js-ts` → `feat/atlas-sim`.
Never merge the two features in parallel; never merge `feat/atlas-sim` before `feat/js-ts`.

**R4 — Preserve every feature.** The merged main must retain: agent-report artifacts/links; Atlas v2
map (rail, progressive charts, key views); JS/TS backends, six language tokens, schema `1.8`;
Atlas Story S1–S4 (`--story`, `atlas-layout.json`, story-only probes, `sim/` companion).
No duplicate modules, no removed feature, no lost test.

**R5 — ADR numbering.** `js-ts` keeps `ADR-0011`. The Atlas documents renumber:
`ADR-0011-atlas-story-companion` → `ADR-0012`, `ADR-0012-atlas-branch-reconciliation` → `ADR-0013`.
Every reference in docs/state/handoff is updated; no two ADRs share a number.

**R6 — Canonical state.** Main's `state/session-state.json`, `state/handoff/latest.md` and
`docs/activeContext.md` remain canonical. Do not merge stale branch state over them; add the branch's
track handoff/plan and update the hot state after the merge. Hot state stays ≤8 KB with only
live/open/active entries; the ledger is append-only.

**R7 — Versions.** `v0.3.0` after js-ts; `v0.4.0` after atlas-sim. Update `Cargo.toml`,
`Cargo.lock`, `CHANGELOG.md`, `README.md`; `perfpal --version` reports `0.4.0` with metrics schema
`1.8`. No new dependency beyond the js-ts pins.

**R8 — Gates.** After each merge, run the full gate on main: `cargo fmt --all -- --check`,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`,
`cargo deny check licenses`, semantic clippy/test. Run the js-ts B6 DoD for M1 and the story gate
(897 tests, `sim/gate.sh` 304/304, 3k 44/44, F5/F6) for M2. A failing gate blocks the merge; fix
the integration, never weaken the gate.

**R9 — Determinism and contracts.** `metrics.json`/`report.html`/`atlas-layout.json` remain
byte-identical where their contracts require it; story-off/on `report.html` byte-identity holds;
A12 is unchanged (one fixed `REPORT_SCRIPT`; zero scripts/handlers/network in canvas/SVG/animation);
`layout_version`/`flow_version` stay `1`.

**R10 — No release action.** No push, tag, or publish without explicit founder go. No A2/ship claim;
no in-report story graduation claim; F8 remains deferred.

## Acceptance

1. `git merge-base --is-ancestor feat/agent-report main` and the same for
   `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim` all exit 0.
2. Full gate + js-ts B6 + story gate exit 0 on main.
3. `perfpal --version` = `0.4.0` (metrics schema `1.8`); CHANGELOG has `[0.3.0]` and `[0.4.0]`.
4. No duplicate ADR numbers/IDs; no stale branch state; hot state ≤8 KB; `git status` clean.
5. Every conflict has a recorded semantic resolution; no feature is missing.

## Risks

- `src/reporter/mod.rs` and `src/reporter/atlas_render.rs` are the real integration points (Atlas v2
  + js-ts + story projection).
- Goldens: js-ts changes them (v1.8); the story must not change `report.html` when `--story` is off.
- State/ledger: both branches append; main's canonical state must win; ledger union must not reuse IDs.
- ADR numbering: js-ts lands `0011`; Atlas must renumber before its merge.
- `strip_entry_line` visibility: main's function is private; the story projection needs `pub(crate)`.

## Non-goals

- Beta-release nomenclature, publishing, signing, tag discipline.
- In-report story graduation or an A12 amendment.
- `map-build`/A1b ranking work; new heuristics; runtime/profiling ingestion.
