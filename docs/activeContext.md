# Active Context

## Current Task
I3 (map-build) DONE — commit `d78bb85`. Two new deterministic stems:
`layer-entry.svg` (entry-first capped files in BFS-depth columns from non-LOW
call pairs; real entries only; depth cap 8 + fold; unreached sub-layered by
`topo_layers`; no-entry topo fallback; cost heat + single entry-reachable hot
outline; LOW edges excluded/captioned) and `layer-bands.svg` (A-pf-077:
capped component bands, per-band file cap 10, cycle-broken intra-band edges,
cross-band file edges aggregated into counted ribbons, LOW/outside captions,
stubs). 679 tests pass; dual review APPROVED (rev-1 blockers B1–B3/F1 fixed).
Default scans have zero non-LOW call edges, so entry strata honestly degrade
to entry/not-reached (R-pf-096); the bands layer carries today's value.

## Recent Decisions
- **D-pf-221 (live):** I3 DONE (`d78bb85`) — entry-stratified + bands stems;
  LOW call edges excluded, goldens byte-stable, dual APPROVED.
- **D-pf-216 (live):** R4 idioms — L0 DSM, L1 Sugiyama-native, L2 treemap+HEB
  (intra-component only), 3rd entry-stratified Sugiyama; A5 demo is reversal trigger.
- **D-pf-215 (live):** map-first direction LOCKED — maintainer birdseye map is the core feature;
  exhaustive sessionized research (R1 algorithms → R2 precedents → R3 honesty → R4 ADR gate) before
  any implementation (I1 entry-points → I2 cost-v2 → I3 map layer → I4 annotations → I5 proof+demo).
- **D-pf-214 (live):** revamp graded — correctness APPROVED, plan BLOCKED (paper track missing), depth C-.
- **D-pf-213 (live):** license marker `LicenseRef-PolyForm-Strict-1.0.0` (SPDX custom-ref); distributions activation still needs founder accept.
- **D-pf-211 (live):** first alignment audit (counter still 0).
- *(Cold `state/archive/ledger.jsonl`: D-pf-195–207 batches, A-pf-062/075/076/079/082/089/093/094/077/084, D-pf-200–205 — append-only.)*

## In Progress
- Committing I3 state closeout. Nothing else open.

## Next
- **NEXT: I4 annotations** — finding dots on nodes (severity+confidence glued,
  never bare red), `report.html` drill path header → workspace/file/component →
  `findings-detail` rows, output-contract registry +1 stem; django humans bounded,
  detail byte-identical. Signpost: plan `map-build`, spec `birdseye-map` F4,
  commit `d78bb85` + handoff.
- Then I5 proof+demo (incl. A-pf-095 true GENUINE triage + semantic-tier check
  that makes `layer-entry` depth strata non-trivial).
- Carried nits (non-blocking): C octal bound `i < 010` parses decimal; IR/schema
  prose still at v1.3; old `layer-file` reserve cost-desc branch partly untested;
  static-SVG label/uncertainty thresholds need the I5 eyeball; pixel glance re-owed.
- M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080). A-pf-084 retired (stale).

## Blockers
- **M5 publish:** `B-pf-001` + `B-pf-002` — unchanged, founder-owned.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map, per founder order).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice
  (divergence instrument, then map-first core — both founder-permitted) · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Grading burst:** rev-1 APPROVED / req-1 BLOCKED / ora-2 C- → D-pf-214 → EOL (`e4c2993`).
- **Map research:** des-1/exp-2 map research (lib-2 errored, superseded by R1) → D-pf-215 → planning commit.
- **I1+I2 burst:** entries C/Rust/Python (`e758681`/`e62184e`/`27b2967`) → cost-v2
  (`daffa62`/`e8faa33`/`a7eb78d`) → `e0df47a` state closeout.
- **This burst:** I3 map layer (`d78bb85`) — entry-stratified stem + bands, 679 tests, dual APPROVED.
