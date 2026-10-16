# Active Context

## Current Task
Mega-burst sessions 47+ COMPLETE and committed (`e05c2bc → 7ac3e22`, 9 commits + 1 uncommitted
demo sweep). Paper track PARKED per founder; usefulness focus delivered. EOL done; awaiting
founder's demo-report feedback for the next burst.

## Recent Decisions
- **D-pf-205 (live):** 9-repo demo sweep in `../perfpal-test` (C/Rust/Python × small/med/large,
  all permissive, all scans rc=0); founder reviewing raw `out-*/report.html` directly, no summary.
- **D-pf-204 (live):** ablation apparatus COMPLETE `f1c43f4` (N=26 tasks, pilot N=2 shakedown,
  non-vacuity guard, append-merge); full campaign unrun (spend decision, A-pf-078).
- **D-pf-203 (live):** bird's-eye viz COMPLETE (`96c72ba` component pages + `5f7f524` layers +
  `9581575`/`7ac3e22` interactive report.html); zero new deps, offline, deterministic.
- **D-pf-202 (live):** high-assurance mode adopted (founder-ordered): adversarial review +
  finding-diff audit + real-repo eyeball per major change; caught 6+3+1+2 BLOCKs pre-commit.
- **D-pf-201 (live):** corpus pinned `24021cc` (10+4 cells, M4 anchors, scan script, PENDING slots).
- **D-pf-200 (live):** IR v1.4 instrument COMPLETE (`f899c11` + `37d0981` + `fb8fe8d` precision fixes);
  67-rule ratchets held; 526 → 606 tests.
- *(D-pf-197/198/199 demoted to cold `state/archive/ledger.jsonl` — append-only.)*

## In Progress
- Nothing — burst complete. Open threads: founder demo feedback (A-pf-079), ablation campaign
  spend (A-pf-078), owed pixel glance (A-pf-076).

## Next
- **NEXT SESSION (`/new`)** — signpost: plans `divergence-revamp` (instrument ✅, corpus docs ✅,
  viz ✅, ablation apparatus ✅) + specs `divergence-study`/`divergence-paper` (parked); commits
  `e05c2bc`..`7ac3e22`; demo feedback = scope for next burst. Read hot state + this file first.
- Candidate bursts: ablation campaign | bands layer | demo-feedback fixes | Option B polish.
- M5 un-park still needs B-pf-001 (contact) + B-pf-002 (secrets) — founder action (A-pf-080).

## Blockers
- **M5 publish:** `B-pf-001` + `B-pf-002` — unchanged, founder-owned.
- **Hygiene:** hot state must stay ≤ 8192 B (now 6206 B).

## History
- **L2 state split (2026-09-29):** hot/cold split — hot compact schema v2 (≤ 8 KB), full history
  in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in
  hot state, alignment-audit counter fields (now 8/10; audit not due).
- **Sessions 47+ (2026-10-16):** divergence-revamp instrument+usefulness mega-burst; paper parked;
  high-assurance mode adopted; 9-repo demo sweep to `../perfpal-test`.
