---
plan name: merge-train
plan description: Three-branch mainline merge
plan status: active
---

## Idea

Integrate the diverged feature lines into `main` as one versioned line, **on main**, with semantic
conflict resolution — not a mechanical merge. Fact base (2026-10-26):

- **agent-report is already merged.** `feat/agent-report` (`c7f0009`) and `fix/agent-report-regen`
  (`c73d6ff`) are ancestors of `main` (`c797004`, `0 ahead`). Verify and remove the worktree; no merge.
- **`feat/js-ts`** (`2e483b1`): 15 ahead / 20 behind main, base `9a94905`; plan `js-ts` active, B5 done,
  **B6 merge gate next**; owns schema `1.8`, JS/TS backends, `ADR-0011-js-ts-backend`.
- **`feat/atlas-sim`** (`9e14507`): 65 ahead / 23 behind main, base `821b984`; story S1–S4 gated
  (`71d0d04`); `ADR-0012-atlas-story-companion` + `ADR-0013-atlas-branch-reconciliation`; ADR-0013
  recorded direction C (defer), now **overridden by the founder: merge the companion into main**. No
  A12 change, no in-report graduation, no ship claim.

**Merge order:** agent-report (already merged) → **js-ts (v0.3.0)** → **atlas-sim (v0.4.0)**. js-ts
has the newer base, the defined B6 gate, and owns `src/reporter/mod.rs` + goldens; atlas-sim resolves
against the post-js-ts renderer. Never merge in parallel, never atlas-sim first.

**Versions:** v0.2.0 = current main (agent-report + Atlas v2 map; Cargo currently `0.2.0-beta.1`);
v0.3.0 = + JS/TS backends (schema `1.8`); v0.4.0 = + Atlas Story companion (layout/flow `1`, no schema
change). The old beta nomenclature is superseded; update `docs/plans/beta-release.md` separately.

**Where:** all merge sessions run in `/home/shafin/Opencode/Perfpal` on `main`. Feature worktrees are
read-only archives; do not sync/merge inside them.

## Implementation

- **M1 — main worktree: `feat/js-ts` → v0.3.0.** `git merge --no-ff feat/js-ts`. Resolve the 6
  conflicts semantically: `README`, `docs/report/output-contract.md`, `docs/tracks/README.md`,
  `src/reporter/mod.rs`, `state/archive/ledger.jsonl`, `tests/p6_hardening.rs`. Keep main's canonical
  `state/handoff/latest.md`/`activeContext`/`session-state`; add the js-ts track docs. Bump
  `Cargo.toml`/`Cargo.lock` to **0.3.0**; add `CHANGELOG [0.3.0]`; confirm `VERSION_SUFFIX` is
  metrics schema **1.8**; run the js-ts B6 DoD on the merged main tree + the full gate; mark plan
  `js-ts` done; update state/handoff; commit. **No push/tag.**

  **M1 result (2026-10-27): DONE.** Merge `cda9d5f`; six conflicts resolved semantically;
  `Cargo.toml`/`Cargo.lock` **0.3.0**, schema **1.8**; `feat/js-ts` is an ancestor of `main`; the
  js-ts B6 DoD + full gate are green on the merged tree (`docs/evidence/merge-m1/gate.txt`).
  Integration fix: `agent_report` catalog lock 67 → 78 with 11 curated `PERF-JS-*` hints; spec
  §8/§12 and `derive_sample.py` updated; synthetic envelope 77,713 B; six goldens + the agent-report
  sample regenerated at 0.3.0 (version delta + the two new JS/TS agent-report nav links). `js-ts` plan done; state/handoff updated;
  no push/tag.
- **M2 — main worktree: `feat/atlas-sim` → v0.4.0.** `git merge --no-ff feat/atlas-sim`. Resolve
  semantically against the post-js-ts main: `README`, `docs/report/output-contract.md`,
  `src/reporter/mod.rs`, `src/reporter/atlas_render.rs`, `state/*`, `state/archive/ledger.jsonl`.
  Keep main's canonical state; preserve story-only code; restore `strip_entry_line` to `pub(crate)`.
  **Renumber Atlas `ADR-0011` → `ADR-0012` and `ADR-0012` → `ADR-0013`** (js-ts owns `ADR-0011`);
  update every reference. Add a dated addendum recording the founder override of R-pf-112 direction
  **C** (merge now; **no A12 change**; **no graduation claim**; §5 still required for in-report
  graduation). Bump `Cargo.toml`/`Cargo.lock` to **0.4.0**; add `CHANGELOG [0.4.0]`; run the story
  gate (897 tests, `sim/gate.sh` 304/304, 3k 44/44, F5/F6) + the full gate; mark the story track done;
  update state/handoff; commit. **No push/tag.**
- **M2 result (2026-10-27): DONE.** Merge `646597c`; twelve conflicts resolved semantically;
  `Cargo.toml`/`Cargo.lock` **0.4.0**; layout/flow **1**, schema **1.8**; `feat/atlas-sim` is an
  ancestor of `main`. Atlas ADRs renumbered (`ADR-0012-atlas-story-companion`,
  `ADR-0013-atlas-branch-reconciliation`) with the founder-override addendum (direction C → merge
  now; **no A12 change**; **no graduation claim**). Story state IDs final at `D-pf-267..274` /
  `A-pf-131..132`; the ledger keeps the story records (no ID reuse; two-hop remap documented in
  ADR-0013). Story gate on merged main: **1,089 tests** (897 at the branch snapshot + the JS/TS
  suites), `sim/gate.sh` **304/304**, 3k **44/44**, F5/F6 PASS, page 463,490 B, first frame max
  1,616 ms (`docs/evidence/atlas-sim/GATE.txt`); full gate green (`docs/evidence/merge-m2/gate.txt`).
  Integration fix: clippy `let_and_return` in `footer_parts` (single footer source); story data
  regenerated (metrics content hash only). Story track done; state/handoff updated; no push/tag.
- **M3 — main finalization.** Verify `feat/agent-report`, `fix/agent-report-regen`, `feat/js-ts`,
  `feat/atlas-sim` are ancestors of main; remove the agent-report worktree; prune merged branches;
  full CI gate; verify `--version` (**0.4.0**, schema **1.8**), CHANGELOG, README, hot state ≤8 KB,
  `git status` clean; update `state/handoff/latest.md` + `docs/activeContext.md` +
  `state/session-state.json` (EOL). No push/tag without founder go.

## Semantic integration rules (binding)

1. **No blind merging.** Never use `--force`, `-X ours`, `-X theirs`, `--no-verify`, or delete a
   side to "resolve" a conflict. Every conflict gets a semantic resolution.
2. **Read both sides, combine features.** For each conflict, inspect `git show main:path` and
   `git show <feature>:path`, then write the union that preserves both features. Record the decision
   in the merge commit body (and the merge-train resolution log).
3. **Canonical state stays main's.** Do not merge stale branch `state/session-state.json`,
   `state/handoff/latest.md` or `docs/activeContext.md`; keep main's, add the branch's track handoff
   and plan, and update the hot state after the merge.
4. **One ADR number per document.** js-ts keeps `ADR-0011`; Atlas renumbers to `0012`/`0013` in M2.
   No two ADRs may share a number.
5. **Gates are the arbiter.** If the combined tree fails a gate, fix the integration — do not weaken
   the gate. Re-run the full gate after every merge.

## Expected conflict decisions

| Path | Main side | Feature side | Required combination |
|---|---|---|---|
| `src/reporter/mod.rs` | Atlas v2 renderer/rail/charts | js-ts language wiring; Atlas story `atlas_layout_json` | Keep all modules and wiring; no duplicate `pub mod`; story JSON remains opt-in |
| `src/reporter/atlas_render.rs` | step 11/12 rail/charts | story geometry + `strip_entry_line` | Keep main's composition; `strip_entry_line` `pub(crate)` |
| `src/cli/mod.rs` | current flags | `--story`, language/backend flags | Union of flags; `VERSION_SUFFIX` 1.8 |
| `docs/report/output-contract.md` | Atlas v2 + agent-report | JS/TS v1.8 + story artifacts | All sections, no duplicate headings |
| `README.md` | Atlas/agent-report | JS/TS + story | All feature sections |
| `state/archive/ledger.jsonl` | main tail | branch tail | Append-only union; no ID reuse |
| `tests/*` | main tests | js-ts/story tests | Keep both; no deletions |

## Version train

| Version | Content | Session |
|---|---|---|
| **v0.2.0** | agent-report + Atlas v2 map (current main; Cargo `0.2.0-beta.1`) | already in main |
| **v0.3.0** | JS/TS backends, schema `1.8` | M1 |
| **v0.4.0** | Atlas Story companion, layout/flow `1`, no schema change | M2 |

## Session signposts

### M1 — JS-TS → v0.3.0 (main worktree)

> **Worktree:** `/home/shafin/Opencode/Perfpal` · **Branch:** `main` · **Base:** `c797004` (+ this plan)
> **Track:** Merge Train M1 · **Task:** merge `feat/js-ts` into main → **v0.3.0**.
> **Read:** `docs/plans/merge-train.md`, `docs/specs/merge-train.md`, the js-ts plan/spec/ADR/evidence
> via `git show feat/js-ts:...`.
> **Merge:** `git merge --no-ff feat/js-ts`. Resolve `README`, `docs/report/output-contract.md`,
> `docs/tracks/README.md`, `src/reporter/mod.rs`, `state/archive/ledger.jsonl`, `tests/p6_hardening.rs`
> semantically (rules above). Keep main's state/handoff/activeContext; add the js-ts track docs.
> **Then:** bump `Cargo.toml`/`Cargo.lock` to **0.3.0**; `CHANGELOG [0.3.0]`; confirm `VERSION_SUFFIX`
> is metrics schema **1.8**; run the js-ts B6 DoD on the merged main tree + full gate; mark `js-ts`
> done; update state/handoff; commit. **No push/tag.**
> **DoD:** main v0.3.0; `feat/js-ts` ancestor; full gate green; state/handoff updated; clean status.

### M2 — Atlas Story → v0.4.0 (main worktree)

> **Worktree:** `/home/shafin/Opencode/Perfpal` · **Branch:** `main` · **Base:** M1 close
> **Track:** Merge Train M2 · **Task:** merge `feat/atlas-sim` into main → **v0.4.0**.
> **Read:** `docs/plans/merge-train.md`, `docs/specs/merge-train.md`, Atlas ADR-0011/0012, story spec,
> atlas-s4 plan, track handoff/evidence via `git show feat/atlas-sim:...`.
> **Merge:** `git merge --no-ff feat/atlas-sim`. Resolve `README`, `output-contract`,
> `src/reporter/mod.rs`, `src/reporter/atlas_render.rs`, `state/*`, ledger semantically. Keep main's
> state; preserve story code; `strip_entry_line` `pub(crate)`.
> **Renumber:** Atlas ADR-0011 → **0012**, ADR-0012 → **0013**; update every reference; add the
> direction-C override addendum (merge now; no A12 change; no graduation claim).
> **Then:** bump **0.4.0**; `CHANGELOG [0.4.0]`; run the story gate (897 tests, `sim/gate.sh` 304/304,
> 3k 44/44, F5/F6) + full gate; mark the story track done; update state/handoff; commit. **No push/tag.**
> **DoD:** main v0.4.0; story companion in main; no A2/ship/graduation claim; clean status.

### M3 — finalization (main worktree)

> **Worktree:** `/home/shafin/Opencode/Perfpal` · **Task:** verify all four branches are ancestors of
> main; remove the agent-report worktree; prune merged branches; full CI gate; verify `--version`
> (**0.4.0**, schema **1.8**), CHANGELOG, README, hot state ≤8 KB, `git status` clean; update
> `state/handoff/latest.md` + `docs/activeContext.md` + `state/session-state.json` (EOL). No push/tag.

## Definition of Done

1. `git merge-base --is-ancestor <branch> main` exits 0 for `feat/agent-report`,
   `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim`.
2. `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`,
   `cargo test --locked`, `cargo deny check licenses`, semantic clippy/test — all exit 0 on main.
3. js-ts B6 DoD and the story gate (897 tests, `sim/gate.sh` 304/304, 3k 44/44, F5/F6) exit 0.
4. `perfpal --version` reports **0.4.0** with metrics schema **1.8**; CHANGELOG has `[0.3.0]` and
   `[0.4.0]`; README describes JS/TS and the Atlas Story.
5. No duplicate ADR numbers; no duplicate decision/risk/action IDs in hot state; no stale branch
   state; hot state ≤8 KB; `git status` clean.
6. No push/tag; no A2/ship/in-report-graduation claim.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- divergence-study
- evidence-beta-spec
- maintainer-map
- merge-train
<!-- SPECS_END -->
