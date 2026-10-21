# Handoff — Latest

## Objective
**Map-intelligibility V3 is DONE and dual-APPROVED (docs + evidence only, no implementation).**
The three V2 slots now have concrete idioms chosen by a pre-registered blind A/B on
tmux/hyperfine/cJSON: **KV1 = text-first ranked brief**, **KV2 = horizontal dependents bars**,
**KV3 = horizontal cost bars**. The sharp result is that the chosen views are legible but the
**ranking still points at noise** — V4 must implement the idioms *and* not claim recall closed;
V7 re-triages the KV1 rank key and V5 tests the KV3 no-loop guard. Next node is **V4**
(implement + schema 1.7 fields), then V5 → V6 → V7 → V8. `map-build` remains **not done**; M5
still waits on `B-pf-001`/`B-pf-002`.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1–I5 ✅ but A5 ❌; successor
`map-intelligibility`: **V0 ✅ V1 ✅ V2 ✅ V3 ✅**; V4–V8 queued. JS/TS backends + paper queue
behind the map.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI emitting heuristics + the maintainer birdseye map (core
  feature) + LLM-consumable metrics (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; the map exists and is deterministic, but the founder's A5 test failed
  on intelligibility. V0/V1 isolated H2; V2 locked the view contract; V3 chose the idiom inside
  each slot before code.
- **How this session serves the north-star:** it prevents V4 from polishing an untested chart
  idiom, and it records the ranking defect that would otherwise survive into the founder test.
  **Drift: NONE.**
- **Holdout:** django/postgres were not rendered, read, or used as data (V6/A2b intact).

## This session — V3 idiom A/B
### Protocol
- Pre-registration committed first (`1f042af`, `docs/evidence/map-vision/v3/prereg.md`):
  6 candidates, prototype-only data transforms, blind singles/pairs, decision rule.
- **18 data-driven prototypes** (2 idioms × 3 slots × 3 repos) from `out-i5b` schema-1.6
  scans; Playwright 1440 wide; manifest sha256 `63f6e0da…`.
- **18 blind single reads + 9 blind pairwise reads** (`reads.json`), one image per single, no
  source/siblings; pairs compare the two candidates of one slot+repo in pre-registered A/B
  order.
- Post-prereg corrections disclosed: KV3 ranking gained startup demotion (locked spec
  wins over prereg) and scale/row parity; KV2 matrix rows changed to production-first
  includers after the first cJSON render hid the production dependents of `cJSON.h`
  (all 3 matrices re-rendered and re-read; all 9 pairs ran after the correction).

### Decisions (pair vote: winner=1, tie=0.5, ≥2/3)
| Slot | Winner | Vote | Conf (pairs) | What would change it (full text in `map-idiom-ab.md` §5) |
|---|---|---|---|---|
| **KV1 Start here** | text-first ranked brief | **2.0 / 1.0** | 0.70, 0.72, 0.70 | V5 blind reads still fail to name an action after the V4 message/truncation fixes → cards; a ≤900 px card layout wins a re-run 2/3; founder prefers inline code evidence. |
| **KV2 Blast radius** | horizontal dependents bars | **3.0 / 0.0** | 0.85, 0.93, 0.78 | a matrix with full-path column labels + ranked totals row wins a re-run 2/3; Q2 moves to component units; founder wants the dependent list inline. |
| **KV3 Profile queue** | horizontal cost bars | **3.0 / 0.0** | 0.85, 0.82, 0.85 | V5 shows the no-loop guard hides a genuine hot leaf → per-iteration cost / two-list variant *before* an idiom change; a treemap with `path:line` + rank labels wins a re-run. |

### The recall finding (deterministic, not a vote)
Ranks of the 9 pre-registered actions in each candidate's full ordering:
- **KV1 Q1:** tmux `input_parse` **210/7,545**, hyperfine `Benchmark::run` **197/348**, cJSON
  `parse_number` **35/837** — none inline; the views name `format_replace` / `print` /
  `apply_patch` instead. The locked severity key is the problem, not the brief (V7).
- **KV2 Q2:** tmux.h **1/216**, src/command.rs **4/53**, cJSON.h **1/101** — all inline;
  bars satisfy recall.
- **KV3 Q3:** `screen_write_collect_add` **811/3,189**, `execute_and_measure` **66/353**,
  `cJSON_New_Item` **58/1,055** — all no-loop and demoted; tmux's target is a per-byte leaf
  whose call frequency is invisible to static cost (H1 for call weights). V5 falsifier.

### Other findings / V4 fix list
- **ARCH-002 `Hot component` message** describes a directory aggregate while the row anchors a
  file:line (tmux #1, cJSON #1) — fix message selection or suppress component rows from KV1.
- **Rationale truncation** (4/5 brief rows end in `…`); key-view text rows are exempt from the
  32-char cap — show the full why.
- **Duplicate KV2 action caption** (header + caption) — keep one.
- **`scripts/` classification:** hyperfine `scripts/plot_*.py` counted production under the
  prototype proxy — decide the V4 `path_class` rule.
- **KV1 cards** are 1635 px (not one screen); any future card idiom must fit 900 px or be
  labelled explorer-tier.
- **KV2 median-in-degree = 0** (tmux/cJSON) makes `× median dependent count` undefined; use
  `% of scanned files` + `top P%` (hyperfine keeps 6.0×). **KV2 production-first is a proposed
  V2 amendment pending founder ratification**; V2's KV1/KV3 scope stands until then.

## Verification
| Claim | Command / evidence | Result |
|---|---|---|
| Docs/evidence only, no code/golden | `git diff --quiet HEAD -- src tests/golden` | exit 0 |
| V3 artifact gate | `python3 docs/evidence/map-vision/v3/render/check_v3.py` | **PASS** (18 renders 1440px, 18 singles + 9 pairs, 3 decisions + triggers, specs mirrored, no holdout leak) |
| Recall ranks reproducible | `python3 docs/evidence/map-vision/v3/render/recall_ranks.py` | **9/9** match `map-idiom-ab.md` §4 |
| Task gate (full) | `task-gate.sh` (11 commands: diff, gate, recall, spec/decision greps, D-pf-230, holdout, no-src change) | **11/11 PASS** |
| Review (code) | reviewer round 2 (after 2 must-fixes + suggestions) | **APPROVED** |
| Review (requirements) | requirements-reviewer | **APPROVED** (8/8) |
| Holdout | `check_v3.py` + grep: no django/postgres in renders/reads/manifest data | pass |
| V2 lock intact | spec table unchanged; KV2 amendment flagged pending | pass |
| `map-build` not done | `docs/plans/map-build.md:4` | `active` |

## Risks / Open
- **R-pf-105 (medium, new):** KV1 severity rank misses Q1 inline (210/197/35); KV3 no-loop
  guard hides a per-byte leaf (rank 811). **V7** rank-key re-triage; **V5** guard falsifier;
  KV2 production-first amendment needs the founder.
- **R-pf-104 (high):** map intelligibility; V3 idioms chosen, founder timed A2 remains the
  gate (V6).
- **R-pf-098/099/100:** unchanged; V7.
- **KV1 order/confirmation caveat:** winners were B/B/A (tmux B, hyperfine B, cJSON A) and the
  brief was already V2's named candidate; a randomized re-run is the cheapest falsifier before
  V6.
- **Blockers unchanged:** B-pf-001 (contact), B-pf-002 (secrets) for M5.

## Next Actions — NEW SESSION SIGNPOST
1. **New session** with: **plan `map-intelligibility` (V4), specs `map-intelligibility` +
   `maintainer-map`, evidence `docs/evidence/map-idiom-ab.md` + `map-data-coverage.md` §8,
   handoff `state/handoff/latest.md`.** V3 prereg `1f042af`; results commit this session.
2. **V4 first:** implement KV1 brief + KV2 bars + KV3 bars; add `files[].path_class` and
   `stats.functions[].trip_count_source`/`known_trip_value` (schema 1.6→1.7, both optional on
   the wire); wire the output contract + drill lists; fix the V4 list above. Then **V5** (two
   blind reads per key view) → **V6** founder timed holdout (include django/postgres) → V7 →
   V8.
3. **Before V6:** decide the KV2 production-first amendment and consider the randomized pair
   re-run.
4. **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002 (founder-owned).

## Change set (this session)
- **New evidence:** `docs/evidence/map-idiom-ab.md`; `docs/evidence/map-vision/v3/`
  (`prereg.md`, `manifest.json`, `reads.json`, `recall_ranks.json`, 18 PNGs,
  `render/{generate,render,check,recall_ranks}.py`).
- **Modified docs:** `docs/specs/map-intelligibility.md` + `docs/specs/maintainer-map.md`
  (V3 decision section), `CHANGELOG.md`, `.gitignore` (`__pycache__`/`*.pyc`).
- **State:** `state/session-state.json` (D-pf-230, R-pf-105, A-pf-105, phase),
  `docs/activeContext.md`, this handoff.
- **No code/golden change**; `map-build` stays active.
- **Commits:** V3 prereg `1f042af`; V3 results + EOL commits this session.
