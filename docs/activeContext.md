# Active Context

## Current Task
**map-intelligibility V0 + V1 DONE and dual-APPROVED.**
- **V0** (`3982b03` → `3a32b01`): 9 pre-registered actions committed **before any chart**;
  30 PNGs (report `#map` at 1440×900/100%, 8 stems/repo); **28 blind @vision reads** (27 views)
  = **4 answers / 22 partial / 1 none** → `docs/evidence/map-vision-diagnosis.md`.
- **V1** (`5ba3740`): field coverage + known-answer recall → `docs/evidence/map-data-coverage.md`
  + `map-recall-v1.json`. **0/9 pre-registered actions at function/line; 0/9 I5 GENUINE IDs in
  any chart; 7/9 file-only; curl `lib/file.c` invisible in every overview stem.**
- **Verdicts:** Q1 = H2, Q3 = H2, Q2 = includes-H2 / calls-H1 (default tier has 0 non-LOW call
  edges). Missing fields are additive only. Wide charts clip in the report (37% visible).
- **Reviews:** code APPROVED (3 rounds; B1–B4 fixed in `bb5b2f9`/`54a4586`/`2be3523`),
  requirements APPROVED; task gate 8/8.

## Recent Decisions
- **D-pf-228 (live):** V0/V1 dual-APPROVED; H2 dominates Q1/Q3; Q2 includes-H2/calls-H1;
  0/9 actions and 0/9 GENUINE IDs surfaced; `map-build` not done; V2 next.
- **D-pf-227 (live):** I5 A5 FAIL (unintelligible) → successor `map-intelligibility`;
  two tiers + mandatory `@vision` gate; no I6.
- **D-pf-226 (live):** I5 proof dual APPROVED; resolver-Ok tier C-only; gate 688/691.

## In Progress
- Session closed. Nothing in flight; working tree committed.

## Next
- **NEW SESSION — V2 methodology + spec lock:** ≤3 answer-first views (one question each,
  one primary encoding), first screen answers Q1 in plain language, 8 stems under an explicit
  `Explorer` divider, units + anchors (`× median`, `top 1%`), no orphan views, text-first
  ranked brief as a candidate. Then V3 idiom A/B (tmux/hyperfine/cJSON; keep django/postgres
  as holdout) → V4 implement → V5 two blind reads/key view → V6 founder timed holdout →
  V7 re-triage R-pf-098/099/100 → V8 closeout.
- **Signpost:** plan `map-intelligibility` (V2), specs `map-intelligibility` +
  `maintainer-map`, commits `54a4586` / `5ba3740` / `3a32b01`, handoff
  `state/handoff/latest.md`.
- **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Map legibility:** R-pf-104 — cause isolated; founder timed A2 test is the remaining gate.
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
