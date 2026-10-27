# Active Context

## Current Task
**polish-report H2 DONE 2026-10-28.** `report.html` is fluid from 1280 to 3072 CSS px: fluid
body/page, full-bleed `#map` anchored to the page body (classic-scrollbar safe), preserved canvas
viewBox, fluid Atlas charts/figures, a `clamp()` type ramp anchored at 1440 (1.5× at 3072), and a
full-width content-driven Atlas story. New public `--report-width
<auto|1280|1440|1920|2560|3072>` is layout-only: only `report.html` CSS plus the
`data-report-width` marker change; metrics, story payload, scripts (A12: three) and every other
artifact stay byte-identical. Six report.html goldens re-generated; schema **1.8**, no new
dependency, byte-determinism preserved.
Evidence: `docs/evidence/polish-report/H2/{gate.txt,report-check.txt,budget.txt,PAGE-HEIGHT.md,VISION.md,PANEL.md}`;
`docs/evidence/atlas-sim/GATE.txt` @4d401db (304/304, 3k 44/44). Full gate: fmt/clippy/test
1,111/deny/semantic 1,113 all exit 0. Budgets: hyperfine 338,100 B/120–128 ms, cJSON
269,363/128–132, 3k 514,838/124–132 — inside ceilings. A8: doc 16,927 @1440 / 21,241 @3072;
story 1,029/1,875; answer bottom ≤436. Vision: 1440/3072/JS-off APPROVED-WITH-NITS.
Panel: R1/R2/Q1/Q2/R3 **APPROVED**. **No push, no tag.**

## Recent Decisions
- **D-pf-279 (live, 2026-10-28):** H2 **CLOSED** — code `4d401db`, evidence `1df5286`; responsive
  scale + public layout-only `--report-width`; scrollbar-safe full-bleed band; full gate +
  story gate green; budgets/A8/vision evidence committed; H3 next; no push/tag.
- **D-pf-278 (live, 2026-10-27):** H1 **CLOSED** — panel approved; full gate 6/6; story gate
  304/304+3k; vision 1440/3072 PASS-WITH-NITS; H2 next.
- **D-pf-276 (live):** M3 done — merge train closed at v0.4.0/schema 1.8.
- **D-pf-251 (live):** F treemap remains undecided; keep W1.
- **ADR-0014** is the in-report story contract; ADR-0012 remains the companion record.
- Archived decisions live in `state/archive/ledger.jsonl` (D-pf-249 pointer).

## In Progress
- Nothing in flight. H2 is closed on `main` at the evidence commit. **No push, no tag** — both
  remain founder-gated.

## Next
1. **polish-report H3 — artifact consolidation + formatting:** role matrix, `report.md` Appendix C
   duplication, one canonical human detail rendering, style contract; update output-contract §2
   and goldens; no schema change.
2. **H4** — `report.html` final composition (`#explorer` → `explorer.html`; order; A8 re-baseline);
   **H5** — finalize (full gate, story gate, docs, state, EOL).
3. **Post-beta map/ranking:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall
   (R-pf-105/106/108).
4. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until
   cleared.

## Blockers
- None for H2. The publish gate remains founder-owned; H2 does not claim A2/ship.
