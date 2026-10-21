# Handoff — Latest

## Objective
**Map-intelligibility V0+V1 are DONE and dual-APPROVED.** The founder's I5 A5 failure
("I can't understand anything") is now diagnosed with evidence instead of opinion: **H2
(visualisation/ranking) dominates Q1 and Q3**; **Q2 is H2 for includes and H1 for calls in
the default tier**. The next node is **V2 (methodology + spec lock)**, then V3 A/B → V4 →
V5 vision loop → V6 founder timed holdout test. `map-build` remains **not done** and has no
I6. M5 still waits on `B-pf-001`/`B-pf-002`.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1–I5 ✅ but A5 ❌; successor
`map-intelligibility`: **V0 ✅ V1 ✅** (this burst); V2–V8 queued. JS/TS backends + paper
queue behind the map.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI emitting heuristics + the maintainer birdseye map
  (core feature) + LLM-consumable metrics (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; the map exists and is deterministic but unintelligible; the
  core-feature claim is blocked until a maintainer can act on it.
- **How this session serves the north-star:** it converts the founder's FAIL into a
  repeatable measurement (pre-registered answers → blind vision reads → known-answer recall)
  and identifies the exact failure surface (selection keys, caps, labels, missing answer
  view) instead of adding charts. **Drift: NONE.**

## This session — V0 + V1
### V0 — pre-registration → rendered views → blind reads
- **Pre-registration committed first (`3982b03`)** before any chart was opened: 3 expected
  actions per repo (tmux/hyperfine/cJSON = where / not-touch / profile), source + I5 triage
  only; `i5-map-proof.md` §7.2/§7.3 deliberately excluded.
- **30 PNGs** (`docs/evidence/map-vision/v0/`): report first screen + `report.html#map` at
  **1440×900 / 100% zoom**, and all 8 overview stems at 100% (6 stems wider than 1440 px were
  captured at natural width so no chart was cropped — disclosed in `manifest.json`).
  Render scripts committed under `v0/render/`; recall script under `v0/recall/`.
- **28 blind `@vision` reads** (`v0/reads.json`): 27 views + a second independent read of
  tmux#map. Blind = image only, no source, no sibling views, describe-then-judge.
- **Diagnosis:** `docs/evidence/map-vision-diagnosis.md` (`3a32b01`). Score: **4 answers /
  22 partial / 1 none**. Q1's first screen points at test/fuzzer noise (tmux `fuzz` 171.17;
  cJSON `tests` 151.43 / `example_2/test` 214.29); Q2 is answered only by `include-graph`
  (hubs `tmux.h`, `cJSON.h`) and never labelled as blast radius; Q3 is cost heat whose top
  node is an FP/test/startup path (`format.c`, `src/command.rs`, `testunity.c`) with no units
  or anchors. Labels truncate; 8/28 reads flag the 8 px dot encoding; the report clips wide
  charts (37% of 3110 px hyperfine stems visible at 1440).

### V1 — data coverage + known-answer recall
- **Data recall** (`docs/evidence/map-data-coverage.md`, `map-recall-v1.json`, `5ba3740`):
  - **9/9 pre-registered files are drawn somewhere, but 0/9 actions are stated at
    function/line granularity**; 2/9 "do not touch" hubs are visible only as include hubs;
    1/9 (`src/command.rs`) is framed as the *hot path*, the opposite decision; 2/9 get a raw
    finding link on `#map` (hyperfine `benchmark/mod.rs:333`, same `run` function as the
    pre-registered loop).
  - **9 GENUINE findings: 0/9 IDs appear in any chart, 0/9 reach `#map`; 7/9 are reduced to a
    file label; 2/9 (curl `lib/file.c:387/582`) are invisible in every overview stem** —
    degree #51/923, weight #80/923, component `lib` rank 9 whose top-3 #map links are other
    findings. The cleanest H2 proof.
  - **H1/H2:** Q1 = H2; Q3 = H2 (+ H1 only for measured runtime, by design); Q2 includes = H2,
    calls = **H1 in the default tier** (all six default scans export 0 non-LOW call edges;
    semantic tier has cJSON 115 / curl 1,548 resolved edges, opt-in). Missing fields are
    additive only: `files[].path_class`, trip provenance, `stats.cost_summary`,
    `components[].metrics.coupling_in/out`, `graph_root`, density guard, uniform edge tier.
- **Reviews:** `@reviewer` CHANGES_REQUESTED (B1 vision-score arithmetic, B2 dot-count/pixel
  size) → fixed `bb5b2f9`; round-2 B3/B4 (missing 6th wide stem; repro script would clobber
  the curated manifest) → fixed `54a4586` → **APPROVED**; residual wording closed `2be3523`.
  `@requirements-reviewer` **APPROVED** (7/7 requirements).

## Verification
| Claim | Command / evidence | Result |
|---|---|---|
| Pre-reg before charts | `git merge-base --is-ancestor 3982b03 3a32b01` | exit 0 (prereg → V0 evidence) |
| 30 PNGs | `find docs/evidence/map-vision/v0 -name '*.png' \| wc -l` | **30** |
| Reads / views / verdicts | JSON assert on `reads.json` | **28 reads / 27 views / 4 answers, 22 partial, 1 none** |
| 9 GENUINE IDs present | JSON assert on `map-recall-v1.json` | **9/9 present** |
| Evidence JSON parses | `json.load` on reads/manifest/recall | ok |
| No code/golden change | `git diff --quiet 6f31a4f..HEAD -- src tests/golden` | exit 0 |
| `map-build` not done | `docs/plans/map-build.md` status | `active` |
| Task gate | `~/.config/opencode/scripts/gates/task-gate.sh` (8 commands above + coverage checks) | **ALL PASS 8/8** |
| Review (code) | reviewer rounds 1–3 | **APPROVED** (round 3) |
| Review (requirements) | requirements-reviewer | **APPROVED** (7/7) |

## Risks / Open
- **R-pf-104 (high):** cause isolated (H2 Q1/Q3; Q2 includes-H2/calls-H1); founder timed test
  (A2) remains the gate — V6, with a django/postgres holdout.
- **R-pf-098 (high):** entry detection over-marks lib-roots (1,462/628/2,672 functions) —
  V7 founder decision (true roots vs cap reserve vs honest label).
- **R-pf-099/R-pf-100 (medium):** production-path ranking + tiny-scope density — V7.
- **R-pf-103 (low):** no semantic-tier committed-golden lock.
- **Blockers unchanged:** B-pf-001 (contact), B-pf-002 (secrets) for M5.

## Next Actions — NEW SESSION SIGNPOST
1. **New session** with: **plan `map-intelligibility` (V2), specs `map-intelligibility` +
   `maintainer-map`, commits `54a4586` / `5ba3740` / `3a32b01`, handoff
   `state/handoff/latest.md`, evidence `docs/evidence/map-vision-diagnosis.md` +
   `docs/evidence/map-data-coverage.md`.**
2. **V2 first:** lock the methodology — ≤3 answer-first views (one question each, one primary
   encoding), the first screen answers Q1 in plain language, the 8 stems move under an explicit
   `Explorer` divider, every number gets a unit + anchor, no orphan views, the text-first
   ranked brief is a candidate. Add the two-tier + vision-gate rules to the spec (already in
   `maintainer-map` + `AGENTS.md`; make the acceptance view set explicit).
3. **V3:** 2–3 idioms per question + the text-first brief; blind `@vision` A/B; choose and
   record "what would change it". Design on tmux/hyperfine/cJSON; keep django/postgres unseen
   for V6.
4. **V4 → V5 → V6:** implement, two blind reads per key view, then the founder timed holdout
   test (3 actions <10 min). **Stop criterion:** two distinct idioms failing the founder test
   → ADR-0008 amendment, no endless polishing.
5. **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002 (founder-owned).

## Change set (this session)
- **New evidence:** `docs/evidence/map-vision-prereg.md`; `docs/evidence/map-vision-diagnosis.md`;
  `docs/evidence/map-data-coverage.md`; `docs/evidence/map-recall-v1.json`;
  `docs/evidence/map-vision/v0/{reads.json,manifest.json,<repo>/*.png,render/,recall/}`.
- **Commits:** `3982b03` prereg → `3a32b01` V0 → `5ba3740` V1 → `bb5b2f9`/`54a4586`/`2be3523`
  review fixes.
- **State:** `state/session-state.json` (D-pf-228; R-pf-104/A-pf-104 updated),
  `docs/activeContext.md`, this handoff, `CHANGELOG.md`.
- **No code/golden change** in this burst; `map-build` stays active.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to divergence-instrument
then map-first core; first periodic audit 2026-10-16 (counter 0).
