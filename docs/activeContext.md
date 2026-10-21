# Active Context

## Current Task
I5 (map-build) proof + demo **executed** — `docs/evidence/i5-map-proof.md`.
- Gate: 50 suites / 687 tests, fmt+clippy+deny green; feature build green (clippy + tests).
- Determinism: committed-golden double regeneration identical; 9/9 demo repos double-scan
  byte-identical modulo provenance (`metrics.json`, `report.md`).
- Semantic tier: libclang 23.1 verified; cJSON 837→1,162 findings, curl 10,985→11,807;
  ARCH-003 LOW→MEDIUM upgrades; 115/1,548 resolved C edges.
- A-pf-095: true GENUINE 9/172 = 5.2% (100% of current hot ids already hand-triaged);
  new top-3 map-node calibration = 1 GENUINE / 11 minor / 11 test-noise / 4 FP of 27.
- Demo: all 9 repos emit all 9 stems + 4–50 component pages; two independent map-only
  maintainers named 3 actions each (tmux, hyperfine) → A5 PASS with caveats.
- A4 captions fixed (`candidate … — profile to confirm`); feature-build golden comparator
  gated to default builds. 33 goldens regenerated.

## Recent Decisions
- **D-pf-223 (live):** I5 evidence — gate 687, 9-repo double-scan identical, semantic tier
  verified, true GENUINE 9/172 + top-node 1/27; A4 captions fixed; feature-golden gate fixed;
  map-quality findings R-pf-098/099/100.
- *(Cold `state/archive/ledger.jsonl`: D-pf-218–222 (I1a–I4), R-pf-094–096, done
  A-pf-095/097 — append-only.)*

## In Progress
- I5 closeout: **founder A5 quote** (3 actions from the tmux map) + dual review pending.
  Evidence + state are written; code/goldens/docs are uncommitted.

## Next
- **I5 closeout:** founder A5 verbatim → append to dossier §7.3; `@reviewer` +
  `@requirements-reviewer`; fix blockers; commit; mark `map-build` done.
- **Then map-quality burst (new session):** A-pf-098 entry-detection strata (R-pf-098,
  high), A-pf-099 production-path ranking + known-trip guard (R-pf-099), A-pf-100
  density denominator guard (R-pf-100).
- M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Blockers
- **M5 publish:** `B-pf-001` + `B-pf-002` — unchanged, founder-owned.
- **I5 closeout:** founder A5 quote (human input) — not a code blocker.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice
  (divergence instrument, then map-first core) · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Grading burst:** rev-1 APPROVED / req-1 BLOCKED / ora-2 C- → D-pf-214 → EOL (`e4c2993`).
- **Map research:** des-1/exp-2 map research (lib-2 errored, superseded by R1) → D-pf-215 → planning commit.
- **I1+I2 burst:** entries C/Rust/Python → cost-v2 → `e0df47a` state closeout.
- **I3 burst:** entry-stratified map + bands (`d78bb85`), 679 tests, dual APPROVED.
- **I4 burst:** annotations + `layer-findings.svg` + drill path (`f56d072`), 687 tests.
- **This burst:** I5 proof + 9-repo demo — negative results recorded (entry strata,
  cost rank, density rank); A4 captions + feature-golden gate fixed.
