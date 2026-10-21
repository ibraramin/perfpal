# Active Context

## Current Task
I5 (map-build) proof + demo **executed; review rounds 1–2 fixed** — `docs/evidence/i5-map-proof.md`.
- Gate: 50 suites / **688** tests (feature: 691), fmt+clippy+deny green.
- Determinism: committed-golden double regeneration identical (`749162…`); 9/9 demo repos
  double-scan byte-identical modulo provenance; transcript `docs/evidence/i5-determinism.txt`.
- Semantic tier: libclang 23.1 verified; cJSON 837→1,162, curl 10,985→11,807; ARCH-003
  LOW→MEDIUM; 115/1,548 resolved C edges. Report boilerplate tier-aware, including the
  resolver-Ok / zero-semantic-finding edge case.
- A-pf-095: true GENUINE 9/172 = 5.2% (`i5-prodhot.json`); supplementary top-3 calibration
  1/27 GENUINE.
- Demo: 9/9 repos all stems + 4–50 pages; two blind map-only maintainers named 3 actions
  each → A5 PASS on action-naming, **timing + founder quote still pending**.
- A4 captions fixed; feature-golden comparator default-only; goldens regenerated.

## Recent Decisions
- **D-pf-225 (live):** I5 review round 2 — both seats CHANGES_REQUESTED → tier predicate also
  honors `parse_health.resolver == Ok` (+zero-finding test), HTML explicit tier sentence,
  `hot-components` link caption candidate, output-contract caveat, timing JSON fields,
  `i5-prodhot.json`, dossier B1/B2 citations. Round 3 pending.
- **D-pf-224 (live):** I5 review round 1 — code APPROVED; requirements CHANGES_REQUESTED →
  G4 tier-aware boilerplate, G5/G6/G7 docs, G8/G9 evidence persistence, G2 timing disclosure.
- **D-pf-223 (live):** I5 evidence — gate 687 at that commit, 9-repo double-scan, semantic
  tier, true GENUINE 9/172 + top-node 1/27; map-quality R-pf-098/099/100.
- *(Cold `state/archive/ledger.jsonl`: D-pf-217/218–222, R-pf-007/094–096, done
  A-pf-095/097 — append-only.)*

## In Progress
- I5 closeout: code+goldens+evidence committed (`dbb9bbe`, `2b90954` + this round-2 fix);
  **round-3 review** and the **founder A5 quote** remain.

## Next
- **I5 closeout:** founder A5 verbatim (timed) → dossier §7.3; round-3 dual review APPROVED;
  `markPlanDone map-build`.
- **Then map-quality burst (new session):** A-pf-098 entry-detection strata (R-pf-098,
  high), A-pf-099 production-path ranking + known-trip guard (R-pf-099), A-pf-100
  density denominator guard (R-pf-100), A-pf-103 semantic goldens (R-pf-103, low).
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
