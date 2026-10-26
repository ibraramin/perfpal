# Handoff — Merge Train M1 (`feat/js-ts` → `main` v0.3.0; 2026-10-27)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. M1 integrates the JS/TS
backend line into `main` as **v0.3.0** without losing any feature. **No drift:** the Atlas v2 map and
the agent-report artifacts stay intact; the only product addition is the JS/TS tier; no A2/ship/
graduation claim, no push/tag. **M2 (`feat/atlas-sim` → v0.4.0) is next.**

**Phase:** **Merge Train M1 DONE 2026-10-27.** `feat/js-ts` is an ancestor of `main` (merge
`cda9d5f`); `Cargo.toml`/`Cargo.lock` are **0.3.0**; metrics schema **1.8**; the js-ts plan is done;
the B6 DoD + full gate are green on the merged tree (`docs/evidence/merge-m1/gate.txt`).

**Commits:** `cda9d5f` (merge `feat/js-ts`) + `158c7b0` (v0.3.0 integration/goldens/state) + `8f7c5c4`
(`.gitignore` for the pre-existing local Playwright scratch); the docs/state reconciliation commits
`228b19c`/`ea06d9a` carry the handoff/state corrections. **No push/tag.**

## This session — M1

- **Merge:** `git merge --no-ff feat/js-ts` in `/home/shafin/Opencode/Perfpal` on `main`. Six
  conflicts, each resolved by reading both sides and combining behavior (no `--force`,
  `-X ours/theirs`, `--no-verify`, no side deleted):
  - `README.md` — five-language/78-rule intro + 1.8 references, keeping main's Atlas v2,
    agent-report sections and beta limitation.
  - `docs/report/output-contract.md` — js-ts discovery/language-token rule + 1.8 schema names,
    keeping the Atlas page contract.
  - `docs/tracks/README.md` — main's C row (C1–C4 merged) plus B's B5 row advanced to B6 done;
    start message/track-state updated.
  - `src/reporter/mod.rs` — `pub(crate) HEURISTIC_CATALOG_V1` (agent_report needs it) with the
    78-rule text and 11 JS entries; Atlas v2 modules and JS label wiring both present.
  - `state/archive/ledger.jsonl` — append-only union (main tail then branch tail); 724 lines,
    all valid JSON, no cross-tail ID reuse (append-only status pairs are intentional).
  - `tests/p6_hardening.rs` — six golden trees + the agent-report prune block; 19-artifact set
    with both Atlas pages; stale 17 count corrected.
  Full log: `docs/evidence/merge-m1/resolutions.md`.
- **Version:** `Cargo.toml`/`Cargo.lock` **0.2.0-beta.1 → 0.3.0**; `--version` verified
  `perfpal 0.3.0 (metrics schema 1.8)`; `CHANGELOG [0.3.0]` added (JS/TS, schema 1.8, B5
  precision fixes, no tag/publish); README/roadmap live version markers updated (the published
  v0.2.0-beta.1 download reference stays, because no 0.3.0 release exists).
- **Gate failure (first run) and fix — not a gate weakening:** the first `cargo test --locked` run
  failed 5 `agent_report` unit tests; the first rerun then exposed 3 integration-test failures from
  the same root cause — main's Track C (landed after the js-ts base) hardcoded the old 67-rule
  catalog and a schema-1.7 sample. Fix: 11 curated
  `PERF-JS-*` hints + 78-id completeness lock; spec §8/§12 and `derive_sample.py` updated; the
  synthetic envelope re-measured **77,713 B** (4,207 B / 5.1% headroom); six golden trees and the
  agent-report sample regenerated at **0.3.0**. Golden delta: `tool.version` in every tree plus main's agent-report nav links restored in the two
  new JS/TS `report.html` files; finding counts are unchanged (15/9/32/29/11/11).
- **Stop-on-first-failure:** the first gate command (`cargo test --locked`) failed; work stopped and
  the failure was reported before any fix. The fix follows merge-train binding rule 5 (fix the
  integration, never weaken the gate); the full gate was then rerun from the top.
- **Plan:** `js-ts` marked done (`markPlanDone`) with the B6 result recorded in the plan.

## Measured evidence

| Claim | Artifact | Result |
|---|---|---|
| Merge commit | `git log --oneline -1` at merge | `cda9d5f merge: feat/js-ts into main` |
| Branch merged | `git merge-base --is-ancestor feat/js-ts main` | exit 0 |
| Version | `cargo run --locked --quiet -- --version` | `perfpal 0.3.0 (metrics schema 1.8)` |
| Schema copy | `cmp src/schema/metrics.schema.v1.8.json docs/schema/metrics.schema.v1.8.json` | byte-identical |
| B6 DoD + full gate | `docs/evidence/merge-m1/gate.txt` | final rerun: every command exit 0; the first run failed (agent-report lock) and was fixed |
| JS/TS determinism | double scan `examples-js` / `examples-ts` (masked provenance) | charts/schema byte-equal; metrics equal modulo provenance |
| Agent-report lock | `cargo test --locked --lib reporter::agent_report::tests::` | 24 passed, 0 failed |
| Goldens | `cargo test --locked --test p6_hardening` | 4 passed, 0 failed |
| Hot state | `wc -c state/session-state.json` | 5,614 B (≤ 8,192) |

## Verification

| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | exit 0 (all suites) |
| Licenses | `cargo deny check licenses` | exit 0 |
| Semantic gate | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` + `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | exit 0 |
| Branch ancestor | `git merge-base --is-ancestor feat/js-ts main` | exit 0 (observed post-commit) |
| Clean status | `git status --porcelain` | empty (observed; `.playwright-mcp/` is ignored by `8f7c5c4`) |
| No tag | `git tag --points-at HEAD` | empty |
| Merge body | `git log -1 --format=%B cda9d5f` | 19 lines; lists the six conflict resolutions |
| Plan done | `docs/plans/js-ts.md` frontmatter | `plan status: done` |
| State valid | JSON parse of `state/session-state.json` + all `state/archive/ledger.jsonl` lines | all valid |

## Risks / open

- **R-pf-105/106/108** and **A1b** remain open; `map-build` not done; F undecided (D-pf-251).
- **Publish gate** B-pf-001 (contact) / B-pf-002 (minisign secrets) — founder-owned.
- **Founder V6 gold standard** for A2 still needs a fresh holdout; recorded, not waived.
- **JS/TS accepted limitations** remain as published (no types, no path aliases, declaration files/
  minified bundles skipped, syntactic tier only); `R-tb-021` (ALLOC-001 noise) and `R-tb-022`..`024`
  stay live in the js-ts track handoff.
- `.playwright-mcp/` is pre-existing local scratch, now ignored by `8f7c5c4`; **not** part of the
  merge change set.

## Next

1. **M2:** `git merge --no-ff feat/atlas-sim` on `main` → **v0.4.0**. Resolve `README`,
   `output-contract`, `src/reporter/mod.rs`, `src/reporter/atlas_render.rs`, `state/*`, ledger
   semantically. Preserve story-only code; `strip_entry_line` → `pub(crate)`. Renumber Atlas
   `ADR-0011` → `0012` and `ADR-0012` → `0013`, update every reference, add the direction-C
   override addendum (merge now; no A12 change; no graduation claim). Bump 0.4.0; `CHANGELOG
   [0.4.0]`; story gate (897 tests, `sim/gate.sh` 304/304, 3k 44/44, F5/F6) + full gate; mark the
   story track done; no push/tag.
2. **M3:** verify all four branches are ancestors; remove the agent-report worktree; prune merged
   branches; full CI gate; verify 0.4.0/schema 1.8, CHANGELOG, README, hot state ≤8 KB, clean
   status; EOL; no push/tag.

## Change set (this commit, after the merge commit)

- **Version/docs:** `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `README.md`, `docs/roadmap.md`,
  `docs/tracks/README.md` (merge resolution), `docs/plans/js-ts.md` (B6 result + done),
  `docs/specs/agent-report.md` (§8/§12).
- **Integration:** `src/reporter/agent_report.rs` (11 hints, count/size locks),
  `tests/agent_report.rs` (1.8 schema + six languages),
  `docs/evidence/agent-report/sample/derive_sample.py` (78 rows),
  `docs/evidence/agent-report/sample/agent-{actions,report}.sample.*` (regenerated at 0.3.0).
- **Goldens:** 24 files across `tests/golden/report*` (`tool.version` in every tree plus main's
  agent-report nav links restored in the two new JS/TS `report.html` files; counts unchanged
  15/9/32/29/11/11).
- **State/evidence:** `state/session-state.json` (compacted, 5,614 B),
  `state/archive/ledger.jsonl` (11 append-only archive records; 735 lines total),
  `state/handoff/latest.md`, `docs/activeContext.md`,
  `docs/evidence/merge-m1/{gate.txt,resolutions.md}`.
- **Merge commit:** `cda9d5f` carries the six conflict resolutions and the js-ts feature tree.
- **Hygiene:** `8f7c5c4` adds `/.playwright-mcp/` to `.gitignore` so the pre-existing local scratch
  cannot block the requested clean `git status`; no product effect.
- **No** push/tag; no A2/ship/graduation claim.
