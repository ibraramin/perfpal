# Active Context

## Current Task
**polish-report H3 CLOSED 2026-10-28.** The artifact contract is consolidated to a **9-file set**
plus the opt-in `atlas-layout.json`, the two Atlas drill pages and the chart set: `findings-detail.md`
is gone (no consumer), `findings-detail.html` is the canonical complete human detail, `report.md`
Appendix C is replaced by a §4 complete-list pointer, `agent-actions.json` stays complete,
`profile-queue.*` stays the distinct KV3 list, the schema copy stays, and the 25 caps/`N of M`
disclosures are intact. One warm-paper style contract now spans the Markdown and HTML artifacts
(shared escapers + shell, sans body, mono code, one heading scale, tabular numerals). The dead
click-to-filter UI was removed; `report.html` keeps 3 scripts (pan/zoom + story + inline JSON),
detail/profile/drills are script-free. The JS-off canvas paints 19/19 nodes via a `<noscript>`
final-frame rule (Pause hidden); the drill pages gained h1/h2; empty scans always emit the
`#ranked-actions` stub.
Evidence: `docs/evidence/polish-report/H3/{gate.txt,report-check.txt,budget.txt,VISION.md,ARTIFACTS.md,STYLE.md,DETERMINISM.md,close.txt}`;
code **fbda69b**, evidence **c88d1a6/9da33a9/11676ba**. Full gate **2,252 passed** (1,125 + 1,127),
0 failed, 2 ignored; story gate **304/304 + 3k 44/44**; acceptance + budget **PASS**; vision **PASS**.
Panel **R1/R2/Q1/Q2/R3 APPROVED**. **No push, no tag.**

## Recent Decisions
- **D-pf-280 (live, 2026-10-28):** H3 **CLOSED** — artifact matrix + one warm-paper style contract;
  `findings-detail.md` dropped; Appendix C removed; shared format/shell helpers; dead filter UI
  removed; JS-off final frame; drills/detail headings; goldens 18; gate 2,252; vision PASS.
- **D-pf-279 (live):** H2 closed — responsive 1280–3072 + public layout-only `--report-width`.
- **D-pf-278 (live):** H1 closed — Atlas parity + always-on story.
- **ADR-0015** records the pre-beta artifact consolidation (findings-detail.md, Appendix C, style
  contract); ADR-0014 remains the in-report story contract.
- Archived decisions live in `state/archive/ledger.jsonl`.

## In Progress
- Nothing in flight. H3 is closed on `main` at the evidence/state commits. **No push, no tag** — both
  remain founder-gated.

## Next
1. **polish-report H4 — `report.html` final composition:** move `#explorer` to `explorer.html`,
   final order, A8/page-height re-baseline if composition changes; founder vision gate.
2. **H5 — finalize:** full gate, story gate, docs (README/output-contract/specs/CHANGELOG), state/EOL.
3. **Post-beta map/ranking:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall.
4. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until
   cleared.

## Blockers
- None for H3/H4. The publish gate remains founder-owned; H3 does not claim A2/ship.
- Deferred H3 nits: R-pf-109 (detail trailing column overflow, weight drill H2 echo) and R-pf-110
  (3k report headroom 1.8%) — non-blocking.
