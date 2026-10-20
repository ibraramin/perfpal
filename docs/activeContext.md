# Active Context

## Current Task
I2 COMPLETE all 3 languages — C `daffa62` (trip gating, v2 cost, reachable
caps, schema 1.6), Rust `e8faa33` (literal ranges), Python `a7eb78d`
(canonical `range()`, saturating i128 span). 668 tests pass; each increment
dual-reviewed (two blockers caught and fixed: vacuous GENUINE proxy, stale
schema prose, i128 overflow panic). I3 map layer queued.

## Recent Decisions
- **D-pf-216 (live):** R4 idioms — L0 DSM, L1 Sugiyama-native, L2 treemap+HEB
  (intra-component only), 3rd entry-stratified Sugiyama; A5 demo is reversal trigger.
- **D-pf-215 (live):** map-first direction LOCKED — maintainer birdseye map is the core feature;
  exhaustive sessionized research (R1 algorithms → R2 precedents → R3 honesty → R4 ADR gate) before
  any implementation (I1 entry-points → I2 cost-v2 → I3 map layer → I4 annotations → I5 proof+demo).
- **D-pf-214 (live):** revamp graded — correctness APPROVED, plan BLOCKED (paper track missing), depth C-.
- **D-pf-213 (live):** license marker `LicenseRef-PolyForm-Strict-1.0.0` (SPDX custom-ref); distributions activation still needs founder accept.
- **D-pf-211 (live):** first alignment audit (counter still 0).
- *(Cold `state/archive/ledger.jsonl`: D-pf-195–207 batches, A-pf-062/075/076/079/082, D-pf-200–205, done A-pf-015/047/065/081/083 — append-only.)*

## In Progress
- Committing I1+I2 state closeout now. Nothing else open.

## Next
- **NEXT: I3 map layer** — entry-stratified cost map (third stem) + deferred
  middle-bands layout; goldens byte-stable on double-scan. Signpost: plan
  `map-build`, spec `birdseye-map` F3, I2 commits
  `daffa62`/`e8faa33`/`a7eb78d` + handoff.
- Then I4 annotations → I5 proof+demo (incl. A-pf-095 true GENUINE triage).
- Then I2–I5. FEATURE-spec question for both plans: skipped by
  founder decision (repo-scope sufficient; revisit if paper needs its own map corpus spec).
- M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080). Bands layer (A-pf-077) folds into I3.
  Re-owed pixel glance when vision backend returns.

## Blockers
- **M5 publish:** `B-pf-001` + `B-pf-002` — unchanged, founder-owned.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map, per founder order).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice
  (divergence instrument, then map-first core — both founder-permitted) · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Grading burst:** rev-1 APPROVED / req-1 BLOCKED / ora-2 C- → D-pf-214 → EOL (`e4c2993`).
- **This burst:** des-1/exp-2 map research (lib-2 errored, superseded by R1) → D-pf-215 → planning commit.
