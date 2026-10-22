# Handoff — map-intelligibility V5 (vision validation)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. V5 is the A1
vision gate and the KV3 no-loop falsifier; no drift. django/postgres remain the V6 holdout.

**Phase:** V5 evidence complete; **dual review APPROVED**; next is V6 (founder timed test).
**Commit base:** `758f492` (V4 EOL). **This session:** evidence/docs/state only — no code change.

## This session — V5

### What ran
- Fresh HEAD build (`758f492`); scanned only tmux/hyperfine/cJSON into
  `perfpal-test/out-v5-*`. The metrics are identical to the V4 scans modulo run metadata
  (files/functions/findings: 216/3189/7545, 53/353/348, 101/1055/837).
- Rendered 9 key views (1440×900, 1152-px element screenshots) to
  `docs/evidence/map-vision/v5/`; all 9 PNGs are **byte-identical** to the V4-approved
  renders (sha256 in `manifest.json`).
- Ran **18 blind `@vision` reads** (2 per key view): one image per task, no source, no
  sibling views, no expected answer; describe-first then strict JSON
  `{question_answered, action, blockers, confidence}`. Three first attempts (04, 13, 15)
  returned empty agent messages and were retried once. **18/18 valid reads PASS** — each
  states the slot question and one concrete `path:line`/function action.
- Recorded six non-blocking observations (O1–O6) in `reads.json`: KV2 `top N%` wording
  (all 6 KV2 reads), cJSON KV2 production-header burial (supports the pending amendment),
  KV3 per-block scale, in-view definitions, a perceived cJSON spacing glitch, and the tmux
  HOT/below-median row.
- Ran the **KV3 no-loop falsifier** (`no_loop_falsifier_v5.py`): all three pre-registered
  Q3 targets have `max_loop_depth == 0` and are demoted, but **partially falsified** —
  removing only the no-loop term still leaves them outside the inline top 8
  (shipped→counterfactual: tmux 820→325, hyperfine 58→46, cJSON 60→45). The static-cost
  key cannot surface per-byte/per-call leaves; V7 must adopt call-weighted/per-iteration
  ranking (or a call-frequency view), not merely drop the guard. cJSON
  `cJSON_New_Item` is also `reachable_from_entry = false`.
- **No code change.** Holdout untouched. `map-build` remains NOT done. **A1b is not
  claimed closed** (KV1 Q1 ranks 209/172/37 remain out of line; V7 owns the re-triage).

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| Fresh scans match V4 | `jq` on `out-v4-*` vs `out-v5-*` metrics (excluding `meta`) | identical files/functions/findings |
| Renders match V4 | `cmp` each v5 PNG against its v4 counterpart | **9/9 byte-identical** |
| 18/18 reads pass | `jq '.per_view' docs/evidence/map-vision/v5/reads.json` | **all `both_pass: true`** |
| Falsifier reproduces ranks | `python3 docs/evidence/map-vision/v5/no_loop_falsifier_v5.py` | **3/3 ranks match** (820/58/60), exit 0 |
| Holdout intact | v5 manifest + reads protocol; no django/postgres render or read | absent |
| No code/schema change | `git status --short` | docs/evidence/state only |
| Dual review | `@reviewer` + `@requirements-reviewer` | **APPROVED (both)** |
| State/handoff/CHANGELOG | this commit | present |

### Review reconciliation (non-blocking)
- The reviewer's "transcripts do not use `question_answered`" finding is a grep artifact: the
  key is JSON-escaped (`\"question_answered\"`) inside the transcript strings. All 18
  transcripts load and contain the four contract keys (verified with Python).
- Counterfactual comparability: a comment now pins why `cf_rank <= 8` is apples-to-apples
  (targets are production/non-startup); behavior/ranks unchanged.
- cJSON KV2 read disagreement (test header vs production ABI header) is folded into O2.
- The stale "rank 811" trigger is annotated V3 prototype 811 / V5 shipped 820.
- Dead `ROOT` constant removed; host-local paths documented in both scripts.
- `reads.json.verdict` renamed to **A1 PASS** so it cannot be misread as the dual-review gate.

## Risks / open
- **R-pf-105 / R-pf-106 (medium):** KV1 Q1 recall (209/172/37) and Q3 recall
  (820/58/60) are still not inline. V5 shows the no-loop guard is a contributing
  demotion, not the sole cause; **V7 owns call-weighted/per-iteration ranking** plus the
  KV1 rank-key re-triage. A1b stays open.
- **Founder decisions before V6:** (a) KV2 production-first amendment — V5 O2 (cJSON
  `cJSON.h` at #4 behind unity test headers) is direct supporting evidence; (b) whether to
  run the randomized KV1 pair re-run (V3 was B/B/A, partly confirmatory).
- **O1 KV2 `top N%` wording:** confirmed by all six KV2 reads; V7/founder follow-up
  (relabel or drop the redundant anchor). Not a V5 code change.
- **Blockers unchanged:** B-pf-001 (contact) + B-pf-002 (secrets) for M5.

## Next actions — new-session signpost
1. **Founder:** decide the KV2 production-first amendment and the randomized pair re-run.
2. **V6:** founder timed eye-test on an unfamiliar repo **including the django/postgres
   holdout**; record verbatim; A2 bar = 3 actions in <10 minutes and "understandable".
3. **V7:** KV1 rank-key re-triage + call-weighted/per-iteration Q3 ranking per the V5
   falsifier; fix or explicitly accept R-pf-098/099/100.
4. **Do not mark `map-build` done.** **Do not claim A1b closed.**

## Change set (this session)
- **Evidence:** `docs/evidence/map-vision/v5/` — 9 PNGs, `manifest.json`, `reads.json`
  (18 transcripts + blind mapping + observations), `render_v5.py`,
  `no_loop_falsifier_v5.py/.json`.
- **Docs:** `docs/specs/map-intelligibility.md` (V5 section),
  `docs/specs/maintainer-map.md` (V5 mirror), `docs/evidence/map-idiom-ab.md`
  (falsifier result), `docs/evidence/map-data-coverage.md` §8 (trigger update),
  `CHANGELOG.md`.
- **State:** `state/session-state.json` (D-pf-232, R-pf-106, A-pf-107/108; settled
  entries demoted), `state/archive/ledger.jsonl`, `docs/activeContext.md`.
- **No `src/`, schema, test, or golden change.**
