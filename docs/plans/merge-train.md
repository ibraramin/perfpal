---
plan name: merge-train
plan description: Three-branch mainline merge
plan status: active
---

## Idea
Design and execute the reconciliation of all diverged worktrees into main as a versioned merge train (no beta-release nomenclature). Fact base (2026-10-26): main is c797004 at 0.2.0-beta.1 with agent-report and Atlas v2 already merged. feat/agent-report (c7f0009) and fix/agent-report-regen (c73d6ff) are both ancestors of main (0 ahead): no merge needed; remove the worktree after confirming no unique work. feat/js-ts (2e483b1) is 15 ahead / 20 behind main, base 9a94905, plan js-ts active with B5 done and B6 merge gate next; it owns schema v1.8 and ADR-0011-js-ts-backend. feat/atlas-sim (9e14507) is 65 ahead / 23 behind main, base 821b984, story S1-S4 gated (71d0d04), ADR-0011-atlas-story-companion + ADR-0012-atlas-branch-reconciliation, direction C recorded as Proposed (defer until the section-5 founder session). Founder has now chosen merge-now and feature-based versions: v0.3.0 for JS/TS, v0.4.0 for the Atlas Story companion. Merge order is js-ts first, then atlas-sim: js-ts has the newer base, the defined B6 gate, and owns src/reporter/mod.rs + goldens; atlas-sim must resolve against the post-js-ts renderer. Never merge in parallel, never merge atlas-sim first, and never merge a branch's stale state/handoff/activeContext files: keep main's canonical state and add the branch's track docs. Cross-branch ADR collision: js-ts owns ADR-0011 after M1; renumber Atlas ADR-0011 to ADR-0012 and Atlas ADR-0012 to ADR-0013 at M2, updating all references. Version train: v0.2.0 = current main (agent-report + Atlas v2 map); v0.3.0 = + JS/TS backends (schema 1.8); v0.4.0 = + Atlas Story companion (layout/flow 1, no schema change). No push/tag without explicit founder go; the beta-release plan's 0.2.0-beta.1 nomenclature is superseded and is updated separately.

## Implementation
- M1 (js-ts worktree): merge main into feat/js-ts; resolve README, docs/report/output-contract.md, docs/tracks/README.md, src/reporter/mod.rs, state/archive/ledger.jsonl, tests/p6_hardening.rs; keep main's state/handoff/activeContext and add the js-ts track docs.
- M1: bump Cargo.toml/Cargo.lock to 0.3.0, add the CHANGELOG [0.3.0] JS/TS section, confirm src/cli/mod.rs VERSION_SUFFIX is metrics schema 1.8, mark plan js-ts done, update the js-ts track handoff.
- M1: run the js-ts B6 DoD from docs/plans/js-ts.md (files, fmt, clippy, test, deny, semantic, determinism, cargo check) plus the JS/TS suites/goldens/canary; commit the merge.
- M1: from /home/shafin/Opencode/Perfpal (main worktree) run git merge --ff-only feat/js-ts, then the main gate (fmt/clippy/test/deny/semantic); update main state/handoff/activeContext; commit if needed; no push/tag.
- M2 (atlas-sim worktree): merge main (now v0.3.0) into feat/atlas-sim; resolve README, output-contract, src/reporter/mod.rs, src/reporter/atlas_render.rs, state/activeContext.md, state/handoff/latest.md, state/session-state.json, state/archive/ledger.jsonl; keep main's state docs and preserve story-only code.
- M2: renumber Atlas ADR-0011-atlas-story-companion.md -> ADR-0012, ADR-0012-atlas-branch-reconciliation.md -> ADR-0013 (js-ts claims ADR-0011); update every reference with rg; add a dated addendum recording the founder override of R-pf-112 direction C (merge now; no A12 change; no graduation claim; section-5 still required for in-report graduation).
- M2: bump Cargo.toml/Cargo.lock to 0.4.0, add the CHANGELOG [0.4.0] Atlas Story companion section, update README; run the story gate (897 tests, sim/gate.sh 304/304, 3k 44/44, F5/F6) and the main gate; mark the story track done; commit the merge.
- M2: from /home/shafin/Opencode/Perfpal run git merge --ff-only feat/atlas-sim, run the main gate again, update main state/handoff/activeContext, commit; no push/tag.
- M3 (main worktree): verify feat/agent-report, fix/agent-report-regen, feat/js-ts and feat/atlas-sim are all ancestors of main; remove the agent-report worktree; prune merged branches; run the full CI gate; verify --version (0.4.0, schema 1.8), CHANGELOG, README, hot state <=8 KB, and git status clean.
- M3: update state/handoff/latest.md + docs/activeContext.md + state/session-state.json (EOL), record the version train and the remaining post-merge follow-ups (story packaging + section-5, map-build/A1b, publish gate); no push/tag without explicit founder go.

## Version train

| Version | Content | Session |
|---|---|---|
| **v0.2.0** | agent-report + Atlas v2 map (current main; Cargo currently says `0.2.0-beta.1`) | already in main |
| **v0.3.0** | JS/TS backends, schema `1.8` (js-ts) | M1 |
| **v0.4.0** | Atlas Story companion, `layout_version`/`flow_version` 1, no schema change (atlas-sim) | M2 |

The old `0.2.0-beta.1` beta-release nomenclature is superseded; update `docs/plans/beta-release.md` separately
if the founder wants a formal release note.

## Session signposts

### M1 — JS-TS B6 → v0.3.0

> **Worktree:** `/home/shafin/Opencode/Perfpal-js-ts` · **Branch:** `feat/js-ts` · **HEAD:** `2e483b1`
> **Track:** Merge Train M1 · **Task:** JS-TS B6 merge gate → main **v0.3.0**.
> **Read:** `docs/plans/js-ts.md` (B6), `state/tracks/js-ts/handoff.md`, `docs/specs/js-ts.md`,
> `docs/adr/ADR-0011-js-ts-backend.md`, `docs/evidence/js-b5/gate.txt`.
> **Goal:** merge `main` (`c797004`) into `feat/js-ts`; resolve `README`, `docs/report/output-contract.md`,
> `docs/tracks/README.md`, `src/reporter/mod.rs`, `state/archive/ledger.jsonl`, `tests/p6_hardening.rs`;
> keep main's `state/handoff/latest.md`/`activeContext`/`session-state` (the branch copies are stale) and add
> the js-ts track docs; bump `Cargo.toml`/`Cargo.lock` to **0.3.0**; run the js-ts B6 DoD; mark plan `js-ts`
> done; commit. Then from `/home/shafin/Opencode/Perfpal`: `git merge --ff-only feat/js-ts`, run the main gate,
> update main state/handoff/activeContext, commit. **No push/tag.**
> **DoD:** main v0.3.0; `feat/js-ts` is an ancestor of main; full gate green; state/handoff updated; clean status.

### M2 — Atlas Story → v0.4.0

> **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` · **Branch:** `feat/atlas-sim` · **HEAD:** `9e14507`
> **Track:** Merge Train M2 · **Task:** Atlas Story merge → main **v0.4.0**.
> **Read:** `docs/adr/ADR-0011-atlas-story-companion.md`, `docs/adr/ADR-0012-atlas-branch-reconciliation.md`,
> `docs/specs/atlas-story.md`, `docs/plans/atlas-s4.md`, `state/tracks/atlas-sim/handoff.md`,
> `docs/evidence/atlas-sim/`.
> **Goal:** merge `main` (now v0.3.0) into `feat/atlas-sim`; resolve `README`, `output-contract`,
> `src/reporter/mod.rs`, `src/reporter/atlas_render.rs`, state docs, ledger; keep main's state docs and preserve
> story-only code; **renumber Atlas `ADR-0011` → `ADR-0012` and `ADR-0012` → `ADR-0013`** (js-ts owns ADR-0011),
> update every reference; add a dated addendum recording the founder override of R-pf-112 direction **C**
> (merge now; **no A12 change**; **no graduation claim**; §5 still required for in-report graduation); bump
> `Cargo.toml`/`Cargo.lock` to **0.4.0**; run the story gate (897 tests, `sim/gate.sh` 304/304, 3k 44/44,
> F5/F6) and the main gate; mark the story track done; commit. Then from main:
> `git merge --ff-only feat/atlas-sim`, run the main gate, update main state/handoff/activeContext, commit.
> **No push/tag.**
> **DoD:** main v0.4.0; story companion in main; no A2/ship/graduation claim; clean status.

### M3 — mainline finalization

> **Worktree:** `/home/shafin/Opencode/Perfpal` (main). Verify `feat/agent-report`,
> `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim` are all ancestors of main; remove the
> agent-report worktree; prune merged branches; run the full CI gate; verify `--version` (**0.4.0**, schema
> **1.8**), CHANGELOG, README, hot state ≤8 KB, `git status` clean; update `state/handoff/latest.md` +
> `docs/activeContext.md` + `state/session-state.json` (EOL); no push/tag without founder go.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- divergence-study
- evidence-beta-spec
- maintainer-map
<!-- SPECS_END -->