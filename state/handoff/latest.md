# Handoff — polish-report H1 CLOSED; H2 next (`main` @ close commit; 2026-10-27)

**Objective:** complete H1 of `polish-report` — restore the seven pre-merge Atlas blocks, full-width
hero, and the always-on in-report Atlas story (ADR-0014; A12 amended), with tests/goldens/budget/vision.
**Status:** **H1 CLOSED**. Panel **R1/R2/Q1/Q2 + fresh R3 APPROVED**. Code **2eeca42**; schema **1.8**;
no new deps. **No push, no tag.**
**Plan/phase:** `polish-report` H1 · next: H2 responsive scale (plan line 28; spec §6).

## Delivered

- **ADR-0014** (`901db22`): supersedes ADR-0012 D5; amended A12 = fixed `REPORT_SCRIPT` + fixed
  `STORY_SCRIPT` + inline JSON; no fetch/XHR/eval/HTTP; offline; JS-off canvas + `<noscript>`;
  `--story` layout-only; F6a/b/c; budget; oracle counter-case/fallback.
- **Seven parity blocks + full-width hero** (`c57ce8b`): answer, actions (top 3 KV1), weight+chart,
  blast+chart, cards, noise, links; rail beside 1440 / below ≥1600; four charts kept; drills linked.
- **Always-on story** (`f8be249`, `2eeca42`): Rust payload projection, inline data, scoped CSS,
  JS-off shell hidden + all static nodes visible, S4 whole-repo detail as a closed `<details>`.
- **Counts/stop reason** (`0bfb386`, `2eeca42`): `trust.loopBearingFunctions` from
  `doc.stats.functions` across drawn paths (complete, not top-3; goldens 7/15/15/20/26/2);
  `trust.io.rows`; `stop_state` names the reason and keeps partial/all-test/single-file as notes.
- Docs/contracts/README/CHANGELOG/specs updated to the amended A12; spec §9 rail decision recorded.

## Verification (claim → evidence → result)

| Claim | Evidence | Result |
|---|---|---|
| Full gate | `docs/evidence/polish-report/H1/gate.txt` @db29612 (code 2eeca42) | 6/6 EXIT=0 |
| Budget | `budget.txt` @2eeca42, final4 | hyperfine 333,544 B/132 ms; cJSON 264,807/136; 3k 510,282/164 — PASS |
| Report acceptance | `report-check.txt` @2eeca42 | 1280/1440/3072 no hscroll; 3 scripts; one static SVG; 0 dynamic requests; 19/19 JS-off nodes; final summary |
| Story gate | `docs/evidence/atlas-sim/GATE.txt` @89c7123 | 304/304, 3k 44/44, verdict PASS (final code 2eeca42 is report-only; full gate/report-check cover the delta) |
| Vision | `docs/evidence/polish-report/H1/VISION.md` @8cf425d | 1440/3072 PASS-WITH-NITS, JS-off PASS (final JS-off text change covered by tests/report-check; JS-on frames unchanged) |
| Panel | `docs/evidence/polish-report/H1/PANEL.md` | R1b/R2/Q1/Q2/R3 **APPROVED** |
| Clean/tag | `docs/evidence/polish-report/H1/close.txt` | tree clean; tag empty; no push/tag |

## Panel / close notes

- Q2 approved the story-gate/vision coverage argument: the only code after the story gate (`89c7123`)
  is `2eeca42`, a report-only JS-off narrative change; final full gate @db29612 and report-check
  @2eeca42 cover it. VISION @8cf425d is a JS-on layout gate; final JS-off text is unit/pipeline/
  report-check covered.
- Q2 correction recorded: report-check shows **19/19** static nodes for the hyperfine report (not
  20/20); the assertion is all-nodes and passes.
- `perfpal-report/` is the untracked self-scan output (ignored via `.gitignore`); it is not part of H1.
- `PANEL.md` round 2 records the final approvals; `close.txt` records the clean/tag check.

## Next

1. **H2 — responsive scale & cross-width acceptance** (separate session; plan line 28, spec §6):
   fluid canvas/charts/viewBox, fluid type ramp, optional `--report-width`
   `<auto|1280|1440|1920|2560|3072>`, acceptance at 1280/1440/1920/2560/3072 and DPR 1/2; vision at
   1440/3072; goldens + determinism + story gate. Do not start H3 before H2 closes.
2. **H3** artifact consolidation/formatting; **H4** explorer.html final composition; **H5** finalize.
3. **Post-beta map/ranking:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall.
4. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until cleared.

## Uncommitted files / state

None at the H1 close; code `2eeca42`; close evidence `close.txt`; **no push/tag**.
