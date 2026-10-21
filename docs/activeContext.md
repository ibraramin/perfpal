# Active Context

## Current Task
**map-intelligibility V2 DONE and dual-APPROVED (docs lock; no implementation).**
- **Locked view set** (`docs/specs/map-intelligibility.md` + `maintainer-map.md`): 3 slots,
  one question + one encoding each — **KV1 Start here** (Q1, severity/rank), **KV2 Blast
  radius** (Q2, coupling; includes-only in the default tier), **KV3 Profile queue** (Q3, cost).
- **Rules:** `report.html#map` opens with KV1 in plain language; the 8 stems move under a
  labelled `Explorer` section (`#architecture` renamed, not duplicated); units + scan anchors
  are mandatory (`× median`, `top 1%`, `% of scanned files`); no silent caps (KV1 →
  `findings-detail`, KV3 → `profile-queue`); production-first with counted demotions; KV3
  no-loop demotion + known-literal-trip flag + unknown-trip badge.
- **Fields** (`docs/evidence/map-data-coverage.md` §8): V4 additive = `files[].path_class` +
  `stats.functions[].trip_count_source`/`known_trip_value` (schema 1.7);
  `cost_summary`/`coupling_in/out`/`graph_root`/density guard are chart-side or deferred, each
  with a "what would change it" trigger.
- **Reviews:** code round 1 CHANGES_REQUESTED (MAJOR: KV3 known FPs; KV3 A1b gap) → founder
  approved the trip-provenance guard → round 2 **APPROVED**; requirements **APPROVED** (8/8).
  Task gate **9/9**.

## Recent Decisions
- **D-pf-229 (live):** V2 locked + dual-APPROVED; 3 slots/encodings; first screen = Q1; Explorer
  divider; units/anchors; KV1/KV3 complete lists; V4 fields `path_class` + trip provenance;
  `map-build` not done.
- **D-pf-228 (live):** V0/V1 dual-APPROVED; H2 dominates Q1/Q3; Q2 includes-H2/calls-H1;
  0/9 actions and 0/9 GENUINE IDs surfaced.
- **D-pf-227 (live):** I5 A5 FAIL (unintelligible) → successor `map-intelligibility`; two tiers
  + mandatory `@vision` gate; no I6.

## In Progress
- V2 docs complete; commit pending explicit founder approval. Nothing else in flight.

## Next
- **V3 idiom A/B:** 2–3 candidate idioms per locked slot (KV1 text-first brief vs ranked cards;
  KV2 blast-radius bars vs matrix; KV3 cost bars vs treemap) + blind `@vision` A/B on
  tmux/hyperfine/cJSON; choose and record "what would change it". Keep django/postgres unseen
  for V6.
- **Then V4 → V5 → V6 → V7 → V8:** implement the two additive fields + views; two blind reads
  per key view; founder timed holdout; re-triage R-pf-098/099/100; closeout.
- **Signpost:** plan `map-intelligibility` (V3), specs `map-intelligibility` +
  `maintainer-map`, handoff `state/handoff/latest.md`.
- **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Map legibility:** R-pf-104 — cause isolated and V2 lock applied; founder timed A2 test is
  still the remaining gate (V6), with the documented KV3 FPs as the V5 falsifier.
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
- **V2:** methodology + spec lock; review round 2 APPROVED; commit pending.
