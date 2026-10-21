# Active Context

## Current Task
**map-intelligibility V3 DONE and dual-APPROVED (docs + evidence; no implementation).**
- **Idioms chosen** (`docs/evidence/map-idiom-ab.md`): **KV1 = text-first ranked brief** (2/3
  pairs), **KV2 = horizontal dependents bars** (3/3), **KV3 = horizontal cost bars** (3/3).
  18 blind single + 9 blind pairwise `@vision` reads over tmux/hyperfine/cJSON
  (`docs/evidence/map-vision/v3/reads.json`); prototypes and repro scripts committed.
- **The sharp finding:** the idioms are legible, but the **ranking still points at noise**.
  The locked KV1 severity key puts the pre-registered Q1 targets at rank **210/197/35** (none
  inline); the KV3 no-loop guard demotes all three Q3 targets (**811/66/58**, incl. the
  per-byte leaf `screen_write_collect_add`). V7 owns the KV1 rank-key re-triage; V5 tests the
  no-loop guard. V4 must not claim A1b closed on severity alone.
- **V2 lock intact.** KV2 production-first is recorded as a **proposed V2 amendment pending
  founder ratification** (cJSON `cJSON.h` is otherwise buried by 22-dependency unity headers).
  KV2 median-in-degree=0 uses `% of scanned files` + `top P%`, not `× median`.
- **V4 fix list:** ARCH-002 directory-aggregate message vs file:line; truncated rationale text;
  duplicate KV2 caption; `scripts/` classification question.

## Recent Decisions
- **D-pf-230 (live):** V3 dual-APPROVED; brief/bars/bars; V2 lock intact; KV2 amendment
  pending founder; recall ranks recorded in `recall_ranks.json`.
- **D-pf-229 (live):** V2 locked + dual-APPROVED; 3 slots/encodings; first screen = Q1;
  Explorer divider; units/anchors; KV1/KV3 complete lists; V4 fields `path_class` + trip
  provenance; `map-build` not done.
- **D-pf-228 (live):** V0/V1 dual-APPROVED; H2 dominates Q1/Q3; Q2 includes-H2/calls-H1.

## In Progress
- V3 complete; nothing else in flight. V3 results + EOL state committed this session.

## Next
- **V4:** implement the chosen key views (KV1 brief, KV2 bars, KV3 bars) + the two additive
  fields (`files[].path_class`, `stats.functions[].trip_count_source` /
  `known_trip_value`; schema 1.6→1.7), output contract, and the V4 fix list. Then **V5** (two
  blind reads per key view) → **V6** founder timed holdout → V7 (R-pf-098/099/100 + KV1 rank
  key) → V8 closeout.
- **Before V6:** consider the randomized pair re-run flagged in `map-idiom-ab.md` §7 (KV1
  B/B/A order + brief was V2's default = partly confirmatory).
- **Signpost:** plan `map-intelligibility` (V4), specs `map-intelligibility` +
  `maintainer-map`, evidence `map-idiom-ab.md`, handoff `state/handoff/latest.md`.
- **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Map legibility:** R-pf-104/R-pf-105 — idioms chosen, ranking open (V7); founder timed A2
  test remains the gate (V6).
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map).

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
- **V3:** prereg `1f042af` → 18 prototypes + 27 blind reads → decisions; results commit this session.
