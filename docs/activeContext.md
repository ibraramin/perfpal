# Active Context

## Current Task
**map-intelligibility V4 DONE, dual-APPROVED + vision-gated (implementation shipped).**
- **Key views:** KV1 text-first brief, KV2 horizontal dependents bars, KV3 horizontal cost
  bars; first screen = KV1; all 8 legacy stems under `<section id="explorer">` with a
  `Key views` nav entry; units/anchors + counted demotions per the V2 lock.
- **Schema 1.7:** additive `files[].path_class` + `stats.functions[].trip_count_source` /
  `known_trip_value` (optional on the wire; docs + embedded copies byte-identical).
- **Drill lists:** `findings-detail.html#ranked-actions` (complete KV1 action list) and the new
  `profile-queue.md/.html` (complete cost list); output contract + README updated.
- **V4 fix list:** only `ARCH-002` suppressed from KV1 (`ARCH-001`/`ARCH-003` stay actions);
  full rationale (no `…`); one KV2 action line; `scripts/` → `bench`; 900px card rule recorded.
- **Vision gate APPROVED:** 9 key views + 3 first screens; `docs/evidence/map-vision/v4/`
  (`reads.json`, `manifest.json`, 12 PNGs).
- **Recall:** shipped-key ranks tmux 209/7543 (Q1) / 820/3189 (Q3); hyperfine 172/346 / 58/353;
  cJSON 37/835 / 60/1055 — still not inline. **A1b is NOT closed on severity**; V7 owns the KV1
  rank-key re-triage; V5 tests the KV3 no-loop guard.

## Recent Decisions
- **D-pf-231 (live):** V4 shipped + dual-APPROVED (round 2). Brief/bars/bars; schema 1.7;
  profile-queue + ranked findings-detail; only ARCH-002 suppressed; KV2 production-first still
  pending founder; vision evidence committed.
- **D-pf-230 (live):** V3 idiom A/B dual-APPROVED; brief/bars/bars; V2 lock intact; KV2
  production-first proposal pending; V3 prototype recall ranks recorded.
- **D-pf-229 (live):** V2 locked; 3 slots/encodings; first screen = Q1; Explorer divider;
  units/anchors; KV1/KV3 complete lists; V4 fields `path_class` + trip provenance.

## In Progress
- **V4 complete; nothing in flight.** EOL state/handoff/CHANGELOG committed.

## Next
- **V5:** two fresh blind `@vision` reads per key view on tmux/hyperfine/cJSON (no source, no
  sibling views), strict `{question_answered, action, blockers, confidence}`; iterate until both
  name the question + one concrete action; commit transcripts. Then **V6** founder timed test
  including django/postgres (holdout) → V7 rank-key re-triage/open findings → V8 closeout.
- **Before V6:** founder decides the KV2 production-first amendment and whether to run the
  randomized pair re-run (KV1 was B/B/A, partly confirmatory).
- **Signpost:** plan `map-intelligibility` (V5), specs `map-intelligibility` +
  `maintainer-map`, evidence `map-idiom-ab.md` + `map-data-coverage.md` §8 +
  `docs/evidence/map-vision/v4/reads.json`, handoff `state/handoff/latest.md`.
- **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002.
- **Do not claim A1b closed on severity.**

## Blockers
- **Map legibility:** R-pf-104/R-pf-105 — views shipped, ranking open (V7); founder timed A2
  remains the gate (V6).
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prereg + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Map research:** R1–R4 → D-pf-215.
- **I1–I4:** entries C/Rust/Python + cost-v2 (`e0df47a`); entry map + bands (`d78bb85`);
  annotations + findings stem (`f56d072`).
- **I5:** proof + demo + **A5 FAIL** (`dbb9bbe`→`6f31a4f`), successor plan created.
- **V0/V1:** prereg `3982b03` → diagnosis `3a32b01` → recall `5ba3740` → review fixes
  `bb5b2f9`/`54a4586`/`2be3523`.
- **V2:** methodology + spec lock `45fa514`; EOL `423de20`.
- **V3:** prereg `1f042af` → 18 prototypes + 27 blind reads → decisions `81b685e`; EOL `e0ded77`.
- **V4:** schema 1.7 `bc887ef` → key views `de93292` → ARCH scope `0bd6d08` → rank key/nav
  `ae50e61` → dual bar scale `f896631` → review round 2 `882ddce` → vision/recall evidence
  `2ec547c`; EOL this session.
