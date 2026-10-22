# Active Context

## Current Task
**map-intelligibility V5 evidence complete; dual review APPROVED; EOL commit pending; next V6.**
- **18/18 valid blind reads PASS** — two fresh reads per key view (tmux/hyperfine/cJSON ×
  KV1/KV2/KV3), one image per task, no source/sibling views, describe-first + strict JSON
  `{question_answered, action, blockers, confidence}`. Three empty agent outputs were
  retried once. Transcripts: `docs/evidence/map-vision/v5/reads.json`.
- **Renders are byte-identical to V4** (9/9 PNG sha256; fresh HEAD scans reproduce the
  schema-1.7 V4 metrics modulo run metadata). 9 PNGs + `manifest.json` + `render_v5.py`.
- **KV3 no-loop falsifier: PARTIALLY FALSIFIED.** All three Q3 targets
  (`screen_write_collect_add`, `execute_and_measure`, `cJSON_New_Item`) have
  `max_loop_depth == 0` and are demoted, but removing only the no-loop term still leaves
  them outside the inline top 8 (820→325, 58→46, 60→45). The static-cost key cannot surface
  per-byte/per-call leaves; **V7 must adopt call-weighted/per-iteration ranking** (or a
  call-frequency view). Reproducer: `no_loop_falsifier_v5.py/.json`.
- **No code change.** Holdout (django/postgres) untouched. **`map-build` NOT done.**
  **A1b NOT claimed closed** (KV1 Q1 ranks 209/172/37 stay out of line; V7).

## Recent Decisions
- **D-pf-232 (live):** V5 evidence complete — 18/18 reads PASS; renders byte-identical;
  no-loop falsifier partial; no code change; O1 KV2 `top N%` wording + O2 cJSON KV2
  production-first evidence feed the founder/V6.
- **D-pf-231 (live):** V4 shipped + dual-APPROVED; KV1 brief, KV2/KV3 bars, schema 1.7,
  Explorer tier, drill lists, vision gate APPROVED.

## In Progress
- Dual review **APPROVED** (both; non-blocking findings reconciled). EOL state/handoff/CHANGELOG
  commit is the only remaining step. No code in flight.

## Next
- **Founder before V6:** decide the KV2 production-first amendment (V5 O2 supports it) and
  whether to run the randomized KV1 pair re-run (V3 was B/B/A).
- **V6:** founder timed eye-test on an unfamiliar repo **including the django/postgres
  holdout** (never rendered/read in V5); 3 actions in <10 min + "understandable".
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
