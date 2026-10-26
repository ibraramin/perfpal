# Active Context

## Current Task
**Merge Train M1 DONE 2026-10-27.** `feat/js-ts` was merged into `main` with `--no-ff` (merge
`cda9d5f`); `Cargo.toml`/`Cargo.lock` are **0.3.0** and the metrics schema is **1.8**. The js-ts
plan is done; the B6 DoD + full gate are green on the merged tree
(`docs/evidence/merge-m1/gate.txt`). **Next: M2 — merge `feat/atlas-sim` → v0.4.0.**
- **Semantic resolution:** README, `docs/report/output-contract.md`, `docs/tracks/README.md`,
  `src/reporter/mod.rs`, `state/archive/ledger.jsonl` (append-only union, 724 lines) and
  `tests/p6_hardening.rs`; every conflict read on both sides and combined. Evidence:
  `docs/evidence/merge-m1/resolutions.md` + the merge commit body.
- **Integration fix:** main-side `agent_report` hardcoded the old 67-rule catalog. Added 11 curated
  `PERF-JS-*` hints (catalog lock 78), updated spec §8/§12 (`derive_sample.py` too), re-measured the
  synthetic envelope at **77,713 B** (4,207 B / 5.1% headroom), and regenerated the six goldens + the
  agent-report sample at 0.3.0. The golden delta is `tool.version` plus main's agent-report nav links
  restored in the two new JS/TS `report.html` files; finding counts are unchanged (15/9/32/29/11/11).
- **Repo state:** 78 rules, six language tokens (`c`, `js`, `python`, `rust`, `ts`, `tsx`), six golden
  trees, schema 1.8 byte-identical in `src/schema/` and `docs/schema/`; v1.7 retained as history.

## Recent Decisions
- **D-pf-259 (live, 2026-10-27):** M1 merge done (`cda9d5f`); six semantic resolutions; v0.3.0 /
  schema 1.8; js-ts plan done; B6 DoD + full gate green; no push/tag.
- **D-pf-260 (live, 2026-10-27):** M1 integration fix — `agent_report` catalog lock 67 → 78 with 11
  `PERF-JS-*` hints; sample/spec/goldens regenerated; gate not weakened.
- **D-pf-251 (live):** F treemap remains **undecided**; keep W1; no F code shipped.
- **D-pf-249 (pointer):** archived decisions are in `state/archive/ledger.jsonl`.
- **D-pf-255/256/257/258 (archived 2026-10-27):** Atlas v2 step-14/15 records moved to the cold ledger
  at M1 EOL; the phase is now merge-train.
- **R-pf-104 / R-pf-109 (closed):** A2 pass and the visual-regression record; full text cold.

## In Progress
- M1 is committed: merge `cda9d5f` plus the v0.3.0/state/evidence commit. **No push, no tag.**

## Next
1. **M2:** `git merge --no-ff feat/atlas-sim` on `main`; resolve `README`,
   `docs/report/output-contract.md`, `src/reporter/mod.rs`, `src/reporter/atlas_render.rs`, `state/*`,
   `state/archive/ledger.jsonl` semantically; keep main's canonical state; preserve story-only code;
   restore `strip_entry_line` to `pub(crate)`.
2. **M2 ADR renumber:** Atlas `ADR-0011` → `ADR-0012`, `ADR-0012` → `ADR-0013`; update every reference;
   add the direction-C override addendum (merge now; no A12 change; no graduation claim).
3. **M2 close:** bump 0.4.0, `CHANGELOG [0.4.0]`, run the story gate (897 tests, `sim/gate.sh`
   304/304, 3k 44/44, F5/F6) + full gate, mark the story track done, update state/handoff; no push/tag.
4. **M3:** verify all four branches are ancestors; remove the agent-report worktree; prune merged
   branches; full CI gate; verify 0.4.0/schema 1.8, CHANGELOG, README, hot state ≤8 KB, clean status;
   EOL; no push/tag.

## Blockers
- **Publish gate:** B-pf-001 (contact) + B-pf-002 (minisign secrets) — founder-owned.
- **Founder V6 gold standard:** the founder's own timed A2 run was not applied; a fresh holdout is
  needed for an untainted re-run. Recorded, not waived.
- **Map/ranking:** R-pf-105/106/108 open; A1b open; `map-build` not done.
- **F undecided:** treemap candidate deferred; no F code ships.

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Map research:** R1–R4 → D-pf-215.
- **I1–I4:** entries C/Rust/Python + cost-v2 (`e0df47a`); entry map + bands (`d78bb85`);
  annotations + findings stem (`f56d072`).
- **I5:** proof + demo + **A5 FAIL** (`dbb9bbe`→`6f31a4f`), successor plan created.
- **V0–V2:** prereg → diagnosis → recall → review fixes → methodology + spec lock.
- **V3–V6a:** 18 prototypes + 27 blind reads → V4 key views + Explorer → V5 validation + no-loop
  falsifier → V6a KV2 production-first amendment (6/6 reads PASS); V6 timed test scrapped.
- **V7:** F/G prototypes + 12/12 blind reads (`docs/evidence/map-vision/v7/`); founder decision
  D-pf-238 (G core, F supporting); Atlas named. **V7 is now archived evidence (2026-10-26).**
- **Atlas planning:** needs research → metrics contract → data-gap audit → layout + inclusion rule
  → ADR-0009/0010 + V8 amendments → `atlas-build` plan. Docs only.
- **Atlas ratification (2026-10-23):** ADR-0009/0010 **Accepted**; V8 ratified; D-pf-239/240
  confirmed; D-pf-241/242/243 recorded; library A2 holdout pre-registered (`memchr` crate payload).
- **Atlas build sessions 1–3 (2026-10-23):** steps 2–8 green (view model/metrics/canvas, brief/
  drills/#map wiring, test/golden lock, V8 16/16 A1 + 16/16 strip); post-V8 visual audit found the
  composition regression (`R-pf-109`); v2 rework planned (steps 9–13), A2/ship moved to 14–15.
- **Atlas v2 Session 4 (2026-10-23/24):** steps 9–10; signed Option A; first-page fit 862 ≤ 900;
  warm/sans/mono system; key-metrics strip; chrome hygiene; step-10 founder check approved 2026-10-24.
- **Atlas v2 Session A1 (2026-10-24):** step 11 detail rail (868+530 stage, 12-card rail, roles,
  exact function/finding links, eight-state lock, collision-aware canvas labels); vision PASS;
  panel R1/Q1/R2/Q2/R3 approved; committed `189df91`; step 13 pending.
- **Atlas v2 Session A2 (2026-10-24):** step 12 progressive charts; seven-block wall retired;
  weight/blast/file/coverage blocks; `atlas-weight.html`/`atlas-q2.html` drills; `#map` 1,848;
  F treemap candidate prototyped (F undecided); committed `9a94905`.
- **Atlas v2 Session A3 (2026-10-24):** step 13 gate evidence — real-page DOM gate all pass at
  1440×900/1280×800; V8 A1 16/16 + strip 16/16 on the reworked page; G-baseline diff (SAME SYSTEM);
  founder composition sign-off APPROVED 2026-10-24 (delegated); evidence committed `86c0537`.
- **Atlas v2 Session A4 (2026-10-24/26):** step 14 A2 holdout (postgres + memchr payload); 4/4 blind
  lanes, 3 actions 87–120 s, understandable YES; pause/reduced/print PASS; memchr fold residual
  (1,282 > 900) carried to step 15; evidence committed `9631137`.
- **Atlas v2 Session A5 (2026-10-26):** step 15 docs/EOL; D-pf-256 (memchr fold accepted), D-pf-257
  (delegated A2 closes beta gate), D-pf-258 (plan done); DoD 1–13 PASS; parallel panel R1/R2/Q1/Q2 +
  R3/R4 approved after fixes; state/handoff/activeContext/track/brief updated; **committed
  `57d91a2`; no push**.
- **Merge Train M1 (2026-10-27):** `feat/js-ts` merged into `main` (`cda9d5f`); six conflicts
  resolved semantically; v0.3.0 / schema 1.8; `agent_report` catalog lock 67 → 78 (+11 JS hints);
  synthetic envelope 77,713 B; goldens/sample regenerated at 0.3.0; B6 DoD + full gate green
  (`docs/evidence/merge-m1/gate.txt`); js-ts plan done; **no push/tag**.
