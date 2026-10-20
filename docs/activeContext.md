# Active Context

## Current Task
I4 (map-build) DONE — `f56d072`. Findings-as-annotations:
- `FindingIndex`/`FindingDot` (per-file + exact-`parent_dir` component roll-up)
  built once in `charts::render`; dots on `layer-workspace`, `layer-file`,
  `layer-entry`, `layer-bands`, and component pages — worst-severity fill +
  **lowest-confidence** ring (never bare severity), count numeral, `<title>`,
  one pinned legend line per chart.
- New 8th overview stem `layer-findings.svg`: include nodes ∪ finding paths,
  top-50 by `(3*hot + 2*warn + info, path)`, weighted heat, single hottest
  outline, honest caps, zero-findings stub.
- `report.html`: breadcrumb `Map → Components → Files → Findings detail`;
  `#map` section (workspace figure + legend + top-10 component drill table with
  ≤3 detail anchors); `#files` component links; zero per-finding rows.
  `findings-detail.html/.md` byte-identical.
- Gate: 687 tests / 50 suites, fmt+clippy+deny green; rev-1 final APPROVED
  (B1 ring honesty fixed); req-1 F1 (stale CHANGELOG) fixed.

## Recent Decisions
- **D-pf-222 (live):** I4 DONE — dots on 5 map stems (worst-severity fill +
  lowest-confidence ring), `layer-findings.svg` (+1 stem), report.html drill
  path; findings-detail byte-identical; 687 tests; rev-1 B1 fixed.
- **D-pf-221 (live):** I3 DONE (`d78bb85`) — entry-stratified + bands stems.
- **D-pf-216 (live):** R4 idioms — L0 DSM, L1 Sugiyama-native, L2 treemap+HEB
  (intra-component only), 3rd entry-stratified Sugiyama; A5 demo is reversal trigger.
- **D-pf-215 (live):** map-first direction LOCKED — maintainer birdseye map is the core feature.
- **D-pf-214 (live):** revamp graded — correctness APPROVED, plan BLOCKED (paper track missing), depth C-.
- *(Cold `state/archive/ledger.jsonl`: D-pf-195–207, A-pf-062/075/076/077/079/082/084/089/093/094/096 — append-only.)*

## In Progress
- I4 closeout: code committed (`f56d072`); this state commit closes the burst. Nothing else open.

## Next
- **NEXT: I5 proof + demo** — full gate, double-scan byte-identical goldens,
  9-repo demo eyeball with the 3-action maintainer test, A-pf-095 true GENUINE
  manual triage, semantic-tier check that makes `layer-entry` depth non-trivial.
  Signpost: plan `map-build`, spec `birdseye-map` F5, commits `d78bb85`/I4 commit.
- Carried residuals (R-pf-097, low): no django corpus → “django humans bounded”
  is structural only; pre-I4 chart-set prose drift (README:94, website
  scope-and-content:69, ADR-0002:32); 4 layer heat captions + hot-components lack
  the word “candidate”; workspace figure duplicated in `report.html`
  (`#map` + `#architecture`).
- M5 needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Blockers
- **M5 publish:** `B-pf-001` + `B-pf-002` — unchanged, founder-owned.
- **Paper track:** prered + scans + stats + Zenodo PENDING (PARKED behind the map, per founder order).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice
  (divergence instrument, then map-first core — both founder-permitted) · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Grading burst:** rev-1 APPROVED / req-1 BLOCKED / ora-2 C- → D-pf-214 → EOL (`e4c2993`).
- **Map research:** des-1/exp-2 map research (lib-2 errored, superseded by R1) → D-pf-215 → planning commit.
- **I1+I2 burst:** entries C/Rust/Python → cost-v2 → `e0df47a` state closeout.
- **I3 burst:** entry-stratified map + bands (`d78bb85`), 679 tests, dual APPROVED.
- **This burst:** I4 annotations + `layer-findings.svg` + drill path (`f56d072`), 687 tests,
  rev-1 APPROVED / req-1 F1 fixed.
