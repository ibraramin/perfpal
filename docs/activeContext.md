# Active Context

## Current Task
**map-intelligibility V6a complete (KV2 production-first, 6/6 blind reads); V6 founder timed
test pending on the released holdout.**
- **V6a implemented + vision-gated.** KV2 orders production first, then dependents desc, then
  path; `top P%` follows that key; median stays over all files; inline = up to 8 production + up
  to 2 greyed demoted rows; HTML/Markdown print `N non-production files demoted`. `cJSON.h` now
  ranks #1 and both fresh blind readers name it; the 22-dependent unity `TEST` headers and
  hyperfine `BENCH` rows are correctly read as demoted-but-never-excluded. Evidence:
  `docs/evidence/map-vision/v6/kv2-amendment/` (3 renders, `reads.json`, `manifest.json`).
- Tests: 3 unit + 1 integration KV2 locks; four golden trees regenerated (KV2 caption-only diff)
  and stable on a second regeneration. No schema change.
- **Holdout released only for V6.** Fresh HEAD scans: django (2932 files, 39 s, 0 partial) and
  postgres (2566 files, 299 s, **1165 partial**). Structural preflight PASS: one `.kv1/.kv2/.kv3`
  each, rows present, KV2 demotion disclosed, no page errors (`docs/evidence/map-vision/v6/
  preflight.py` + `.json`). No agent reads of holdout content.
- The V5 "byte-identical to V4" claim is historical: KV2 changed by design; KV1/KV3 unchanged.

## Recent Decisions
- **D-pf-234 (live):** V6a KV2 production-first amendment implemented + 6/6 blind reads PASS;
  no schema change; holdout scanned for V6 only; `map-build` NOT done; A1b NOT closed.
- **D-pf-233 (live):** founder pre-V6 decisions — KV2 amendment APPROVED; randomized KV1 pair
  re-run SKIPPED; holdout scan both, timed test on postgres, django untimed smoke.
- **D-pf-232 (live):** V5 evidence complete (18/18 reads; no-loop falsifier partial).

## In Progress
- **V6 timed test.** Protocol: `docs/evidence/map-vision/v6/protocol.md`; result template:
  `result.template.json`. Founder opens `out-v6-postgres/report.html#map` at 1440×900/100%,
  names 3 actions in <10 min + understandable verdict, verbatim. No agent substitute.

## Next
- **Founder:** run the timed postgres test (then optional django smoke). Record result verbatim.
- **If PASS:** close A2/A2b evidence, close R-pf-104, keep R-pf-105/106 + A1b open, EOL commit.
- **If FAIL:** re-open V3 with a **distinct** idiom; two distinct idiom failures → ADR-0008
  amendment. Record the defect, fix, re-test.
- **V7:** KV1 rank-key re-triage + call-weighted/per-iteration Q3 ranking; fix or explicitly
  accept R-pf-098/099/100. A1b stays open.
- **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Map legibility:** R-pf-104/R-pf-105/R-pf-106 — views validated (A1), but ranking recall
  (A1b) is open; founder A2 is the V6 gate.
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prereg + scans + stats + Zenodo PENDING (PARKED behind the map).

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
- **V3:** prereg `1f042af` → 18 prototypes + 27 blind reads → decisions `81b685e`; EOL `e0ded77`.
- **V4:** schema 1.7 `bc887ef` → key views `de93292` → ARCH scope `0bd6d08` → rank key/nav
  `ae50e61` → dual bar scale `f896631` → review round 2 `882ddce` → vision/recall evidence
  `2ec547c`; EOL `758f492`.
- **V5:** fresh scans + 9 byte-identical renders + 18 blind reads + no-loop falsifier;
  evidence `docs/evidence/map-vision/v5/`; EOL this commit.
