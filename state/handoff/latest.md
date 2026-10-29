# Handoff — polish-report H3 CLOSED; H4 next (`main` @ close commit; 2026-10-28)

**Objective:** complete H3 of `polish-report` — one artifact role matrix, remove the duplicate
full-findings rendering, and enforce one warm-paper formatting contract across the human artifacts,
without a schema bump or an H2 layout regression.
**Status:** **H3 CLOSED**. Panel **R1/R2/Q1/Q2/R3 APPROVED** (final round). Code **fbda69b**;
evidence **c88d1a6**, **9da33a9**, **11676ba**; schema **1.8**; no new deps.
**No push, no tag.**
**Plan/phase:** `polish-report` H3 · next: H4 `report.html` final composition (plan line 30).

## Delivered

- **Role matrix (9-file set).** `findings-detail.md` dropped (no consumer); `findings-detail.html`
  is the canonical complete human detail; `agent-actions.json` stays complete; `profile-queue.*`
  stays the distinct KV3 list; the schema copy stays (offline validation + discovery); the 25 caps
  and every `N of M` disclosure are intact. `atlas-layout.json` is the documented opt-in story
  artifact. `tests/artifact_matrix.rs` is the executable mirror (6/6), including a `--story`
  emission test and an end-to-end invalid `--report-width` exit-1 test.
- **Appendix C removed.** `report.md` §4 ends with
  `All <N> of <M> findings: [findings-detail.html](findings-detail.html) — complete ranked action
  list and per-finding detail (<R> rules).`; §1 says “all N ranked actions (M findings)”.
- **One style contract.** `src/reporter/format.rs` owns the escapers + Markdown helpers; one
  warm-paper shell (`html_page_head` + shared tokens/CSS) serves report/detail/profile; the Atlas
  drills share the palette/heading scale. Sans body, mono code, one heading scale, tabular numerals,
  unified tables/links/empty states. `profile-queue.md` gains `## Complete cost-ranked list` +
  `No functions recorded.`
- **Dead filter removed.** The C-split made click-to-filter impossible; the filter bar/controls/
  script are gone. `report.html` keeps 3 scripts (pan/zoom + story + inline JSON); detail/profile/
  drills are script-free.
- **JS-off final frame.** A `<noscript><style>` block forces `animation:none; opacity:1;
  transform:none; stroke-dashoffset:0` on `#map #atlas-canvas *` and hides the inert Pause, so the
  static map paints all 19 nodes immediately.
- **Drills/detail.** Drill pages gain `<h1>Perfpal Atlas</h1>` + `<h2>Weight detail|Blast drill</h2>`
  on the shared heading scale (10px density exception); `findings-detail.html` shows its own H1;
  empty scans always emit the `#ranked-actions` stub so report links resolve.
- **Goldens:** 18 artifacts across six trees regenerated; the detail-md artifact is gone.

## Verification (claim → evidence → result)

| Claim | Evidence | Result |
|---|---|---|
| Full gate | `docs/evidence/polish-report/H3/gate.txt` @`fbda69b` | fmt/clippy/test/deny/semantic clippy/semantic test **all EXIT=0**; **2,252 passed** (1,125 default + 1,127 semantic), 0 failed, 2 ignored |
| Story gate | `…/story-gate.txt` (HEAD `9da33a9`, code `fbda69b`) | **304/304 + 3k 44/44**, `worktree_dirty_zero`, verdict **PASS** |
| Acceptance | `…/report-check.txt` @`fbda69b` + tree hash | 5 widths × DPR 1/2 interactive + JS-off, auto + pinned 1440; no hscroll/canvas fill/no clipped labels/story final; **PASS** |
| Budget | `…/budget.txt` @`fbda69b` + tree hash | hyperfine 338,091 B; cJSON 269,354; 3k 514,829 (9,459 B / 1.8% headroom) — **PASS** |
| Artifact matrix | `…/artifact-matrix.txt` | **6/6** |
| Style contract | `…/style-contract.txt` | **8/8** |
| Determinism | `…/determinism.txt` | **24/24** |
| Vision | `…/VISION.md` | **PASS** (JS-off 19/19 nodes; drills; detail H1) |
| Clean/tag | `…/close.txt` + final check | tree clean; no tag; no push |

## Panel / deferred nits

- Round 1 found the dead filter UI, a missing `atlas-layout.json` matrix row, and missing vision
  evidence; round 2 found empty-scan dead `#ranked-actions` links and drills outside the heading
  scale. All fixed and re-verified.
- Deferred (non-blocking, in state as **R-pf-109**/**R-pf-110**): the findings-detail trailing column
  after `confidence` clips at the table edge (pre-existing horizontal scroll); the weight drill H2
  echoes its first section title; 3k report headroom is 1.8%.

## Next

1. **H4 — `report.html` final composition** (separate session; plan line 30): move `#explorer` to
   `explorer.html`, final order, A8 re-baseline if composition changes; founder vision gate.
2. **H5 — finalize:** full gate, story gate, docs (README/output-contract/specs/CHANGELOG), state/EOL.
3. **Post-beta:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall.
4. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until
   cleared.

## Uncommitted files / state

State/handoff/activeContext only at this close commit; **no push/tag**.
