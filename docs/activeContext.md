# Active Context

## Current Task
**I5 proof DONE, A5 FAILED (founder intelligibility), `map-build` not closed.**
- Gate: 50 suites / **688** default, **691** feature; fmt+clippy+deny green.
- Determinism: goldens double-regen identical (`749162…`); 9/9 demo repos double-scan
  identical; transcript `docs/evidence/i5-determinism.txt`.
- Semantic tier: libclang 23.1 verified; cJSON 837→1,162, curl 10,985→11,807; report
  boilerplate tier-aware (incl. resolver-Ok / non-C guard).
- A-pf-095: true GENUINE **9/172 = 5.2%** (`i5-prodhot.json`); supplementary top-3
  calibration 1/27 GENUINE.
- Dual review: code + requirements **APPROVED** (proof deliverable, round 3).
- **Founder A5: FAILED** — "I can't understand anything, I doubt anyone will"
  (`docs/evidence/i5-map-proof.md` §7.3). Per spec the idiom is judged wrong, not the data.
  I5 was the last node; **there is no I6**.

## Recent Decisions
- **D-pf-227 (live):** A5 FAIL → successor spec/plan **`map-intelligibility`** (vision-loop
  diagnosis H1 data vs H2 visuals; answer-first key views vs labelled info-dumps; mandatory
  `@vision` gate; founder timed retest). Visual rule added to `AGENTS.md` +
  `maintainer-map` spec. `map-build` stays un-done.
- **D-pf-226 (live):** proof deliverable dual APPROVED; resolver-Ok tier signal C-only.
- *(Cold `state/archive/ledger.jsonl`: D-pf-217…225 review rounds/I1–I4, R-pf-007/094–096/
  101/102, done A-pf-095/097 — append-only.)*

## In Progress
- Session closed. Nothing in flight; working tree committed.

## Next
- **NEW SESSION:** plan **`map-intelligibility`** V0 — run `@vision` over the 8 overview
  stems (tmux, hyperfine, cJSON), record what each communicates/cannot answer → 
  `docs/evidence/map-vision-diagnosis.md`; then V1 data-coverage audit (H1 vs H2), V2
  methodology lock, V3 idiom A/B, V4 implement, V5 vision loop, V6 founder timed test,
  V7 re-triage R-pf-098/099/100, V8 closeout.
- **Signpost:** plan `map-intelligibility`, specs `map-intelligibility` + `maintainer-map`,
  commits `6f31a4f` / `dbb9bbe`, handoff `state/handoff/latest.md`.
- M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Blockers
- **Map legibility:** R-pf-104 (high) — the core feature is unreadable; new plan chartered.
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Grading burst:** rev-1 APPROVED / req-1 BLOCKED / ora-2 C- → D-pf-214 → EOL (`e4c2993`).
- **Map research:** R1–R4 → D-pf-215 → planning commit.
- **I1+I2:** entries C/Rust/Python + cost-v2 (`e0df47a`); **I3:** entry map + bands (`d78bb85`);
  **I4:** annotations + findings stem (`f56d072`); **I5:** proof + demo + **A5 FAIL**
  (commits `dbb9bbe`→`6f31a4f`), successor plan created.
