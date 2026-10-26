# Active Context

## Current Task
**Merge Train M2 DONE 2026-10-27.** `feat/atlas-sim` was merged into `main` with `--no-ff` (merge
`646597c`); `Cargo.toml`/`Cargo.lock` are **0.4.0**, metrics schema **1.8**, Atlas Story layout/flow
`1`. The Atlas ADRs renumbered to **ADR-0012** (companion) and **ADR-0013** (branch reconciliation),
with the founder-override addendum (direction C → merge now; **no A12 change**; **no graduation
claim**). The story track is done; the story gate + full gate are green on the merged tree
(`docs/evidence/atlas-sim/GATE.txt`, `docs/evidence/merge-m2/gate.txt`). **Next: M3 — finalization.**
- **Merge:** twelve conflicts resolved semantically (code: `atlas_render.rs` keeps main's
  `PAGE_GRAPH_W`/width-parameterised layout, crate-visible `Layout` fields, main's collision-aware
  labels, `strip_entry_line` `pub(crate)`, one `footer_parts` source; `mod.rs` keeps both
  `atlas_charts` and `atlas_layout` + `atlas_layout_json`). Docs/state: main's canonical state,
  append-only ledger with the story IDs final at `D-pf-267..274` / `A-pf-131..132`, README/CHANGELOG
  union at 0.4.0, ADR reference sweep.
- **Gate fix:** the first story-gate run failed on clippy `let_and_return` (`footer_parts`) and the
  committed story data's `source.metricsContentSha256` (version bump only); both fixed, then the
  gate passed on a clean tree. Gate not weakened.
- **Repo state:** 78 rules, six language tokens (`c`, `js`, `python`, `rust`, `ts`, `tsx`), six
  golden trees regenerated at 0.4.0 (version-only delta, counts 15/9/32/29/11/11), agent-report
  sample regenerated (78 hints, 8,882 B), schema 1.8 byte-identical in `src/schema/` and
  `docs/schema/`.

## Recent Decisions
- **D-pf-275 (live, 2026-10-27):** M2 merge done (`646597c`); v0.4.0 / schema 1.8 / layout+flow 1;
  ADR-0012/0013 + founder override; story gate + full gate green; no push/tag.
- **D-pf-251 (live):** F treemap remains **undecided**; keep W1; no F code shipped.
- **D-pf-267..274 / A-pf-131..132 (cold):** the story-line decision/action records, final IDs at
  merge; ADR-0013 documents the two-hop remap (251..255 → 260..264 → 267..274).
- **D-pf-259/260 (archived 2026-10-27):** M1 merge + integration fix moved to the cold ledger at the
  M2 close.
- **D-pf-249 (pointer):** archived decisions are in `state/archive/ledger.jsonl`.

## In Progress
- M2 is committed: merge `646597c`, v0.4.0 prep `e1b4f1b`, gate fix/data `ff276fd`, evidence/state
  commit. **No push, no tag.**

## Next
1. **M3:** verify all four branches are ancestors of `main`; remove the agent-report worktree; prune
   merged branches; full CI gate; verify 0.4.0/schema 1.8, CHANGELOG, README, hot state ≤8 KB, clean
   status; update `state/handoff/latest.md` + this file + `state/session-state.json` (EOL); no
   push/tag.
2. **Story follow-up (`A-pf-133`):** self-contained companion artifact + `perfpal story <repo>`
   entry before the §5 founder session; F8/A2 story read deferred; no in-report graduation.
3. **Post-beta map/ranking:** F treemap decision (D-pf-251), memchr fold trigger (D-pf-256),
   `map-build`/A1b recall (R-pf-105/106/108).

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
- **V7:** F/G prototypes + 12/12 blind reads; founder decision D-pf-238 (G core, F supporting);
  Atlas named. V7 archived 2026-10-26.
- **Atlas planning/build/ratification (2026-10-23):** ADR-0009/0010 Accepted; V8 ratified; steps
  9–15 (v2) complete; step 13 approved; A2 delegated PASS; `atlas-build` done (D-pf-255/257/258).
- **Atlas v2 Sessions A1–A5 (2026-10-24/26):** rail, progressive charts, gate evidence, A2 holdout,
  docs/EOL; commits `189df91`, `9a94905`, `86c0537`, `9631137`, `57d91a2`; no push.
- **Atlas Story D1–D2 + S1–S4 (2026-10-24/26):** core-fed story, edges/IO/purpose/queue/scenes;
  final gate `71d0d04` (897 tests, 304/304, 3k 44/44); ADR-0012 companion; no graduation.
- **Merge Train M1 (2026-10-27):** `feat/js-ts` merged (`cda9d5f`); v0.3.0 / schema 1.8;
  agent_report catalog lock 67 → 78; goldens regenerated; js-ts plan done; no push/tag.
- **Merge Train M2 (2026-10-27):** `feat/atlas-sim` merged (`646597c`); v0.4.0 / layout+flow 1;
  ADR-0012/0013 + founder override; story gate 1,089 tests / 304/304 / 3k 44/44 / F5/F6; full gate
  green; story track done; **no push/tag**.
