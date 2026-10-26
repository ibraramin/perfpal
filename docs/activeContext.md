# Active Context

## Current Task
**Merge Train M3 DONE 2026-10-27 — the merge train is complete.** Finalization of the three-line
merge into `main` at **v0.4.0** (metrics schema **1.8**, Atlas layout/flow `1`). All four feature
lines (`feat/agent-report`, `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim`) are verified
ancestors of `main`. The agent-report worktree was archived to
`/home/shafin/Opencode/.trash/Perfpal-agent-report-2026-10-27` and its git registration pruned;
**branch refs were retained** (founder instruction: no permanent deletion). Full CI gate on `main` at
`2c01c17`: fmt, clippy, tests, `cargo deny check licenses`, semantic clippy/tests all exit 0 —
**2,180 passed / 0 failed / 2 ignored** (`docs/evidence/merge-m3/gate.txt`). Story gate re-run on the
final tree at `978a623`: **PASS**, 1,089 tests (1 ignored), **304/304** checks, 3k scale PASS
(`docs/evidence/atlas-sim/GATE.txt`). `--version` = `perfpal 0.4.0 (metrics schema 1.8)`; CHANGELOG
`[0.3.0]` + `[0.4.0]`; README at 0.4.0 with JS/TS and the Atlas Story; hot state 4,884 B ≤ 8 KB; ADR
numbers unique (0001–0013); clean status; no tag. **No push, no tag.**

## Recent Decisions
- **D-pf-276 (live, 2026-10-27):** M3 done — all four lines merged and verified; worktree archived
  (reversible, branch refs kept); full CI gate + story-gate re-run green; v0.4.0 / schema 1.8;
  clean status; no push/tag.
- **D-pf-251 (live):** F treemap remains **undecided**; keep W1; no F code shipped.
- **D-pf-267..274 / A-pf-131..132 (cold):** the story-line decision/action records, final IDs at
  merge; ADR-0013 documents the two-hop remap (251..255 → 260..264 → 267..274).
- **D-pf-275 (archived 2026-10-27):** M2 merge moved to the cold ledger at the M3 close.
- **D-pf-259/260 (archived 2026-10-27):** M1 merge + integration fix moved to the cold ledger at the
  M2 close.
- **D-pf-249 (pointer):** archived decisions are in `state/archive/ledger.jsonl`.

## In Progress
- Nothing in flight. The merge train (M1 → M2 → M3) is closed on `main`; the working tree is clean at
  the M3 close. **No push, no tag** — both remain founder-gated.

## Next
1. **Story follow-up (`A-pf-133`):** self-contained companion artifact (inline data; no `fetch`/HTTP
   server) + a `perfpal story <repo>` entry before the §5 founder session; F8/A2 story read deferred;
   no in-report graduation (A12 unchanged).
2. **Post-beta map/ranking:** F treemap decision (D-pf-251), memchr fold trigger (D-pf-256),
   `map-build`/A1b recall (R-pf-105/106/108).
3. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets, then tag/publish.

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
  green; story track done; no push/tag.
- **Merge Train M3 (2026-10-27):** finalization — four lines ancestor-verified; worktree archived +
  registration pruned (branch refs kept); full CI gate 2,180 passed / 0 failed / 2 ignored; story
  gate re-run PASS at `978a623`; v0.4.0 / schema 1.8; hot state 4,884 B; clean status; no push/tag.
