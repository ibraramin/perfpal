# Active Context

## Current Task
**polish-report H1 DONE 2026-10-27.** `report.html` now restores all seven pre-merge branch blocks
(answer, actions, weight, blast, cards, noise, links) plus the full-width hero (rail beside at 1440,
below at ≥1600), and carries the **always-on Atlas story** (ADR-0014): one fixed report script, one
fixed story script and one inline JSON payload (three script elements; A12 as amended), no
fetch/XHR/eval, offline `file://`, JS-off final frame = the static canvas + `<noscript>` narrative,
reduced-motion/print final frame, S4 whole-repo detail as a closed `<details>`, and `--story`
layout-only. Schema **1.8**, no new dependency, byte-determinism preserved.
Evidence: `docs/evidence/polish-report/H1/{gate.txt,report-check.txt,budget.txt,VISION.md}`;
`docs/evidence/atlas-sim/GATE.txt` (304/304, 3k 44/44, verdict PASS). Full gate: fmt/clippy/test/
deny/semantic all exit 0. Budget: hyperfine 333,544 B/144 ms, cJSON 264,746 B/160 ms, 3k
510,282 B/156 ms — inside ADR-0014 ceilings. Vision: 1440/3072 PASS-WITH-NITS, JS-off PASS.
Panel: round 1 returned R2 APPROVED and R1/Q1/Q2/R3 CHANGES_REQUESTED. The JS-off counts now use the
complete drawn-slice loop count and the stop reason is explicit; round-2 verdicts are appended in
`PANEL.md` at close.
**No push, no tag.**

## Recent Decisions
- **D-pf-277 (live, 2026-10-27):** H1 implementation complete — seven blocks + full-width hero +
  always-on in-report story (ADR-0014; A12 amended; F6a/b/c); schema 1.8/no deps; gates + vision
  green; panel round 2 changes requested on the JS-off counts/stop reason; fixes applied; final
  panel/close pending; H2 next.
- **D-pf-276 (live):** M3 done — the merge train closed at v0.4.0/schema 1.8.
- **D-pf-251 (live):** F treemap remains undecided; keep W1.
- **ADR-0014** supersedes ADR-0012 D5 for the in-report decision; ADR-0012 remains the companion
  record. Canonical specs/README/CHANGELOG now state the amended A12 (three script elements).
- Archived decisions live in `state/archive/ledger.jsonl` (D-pf-249 pointer).

## In Progress
- Nothing in flight. H1 is closed on `main`; the working tree is clean at the close commit.
  **No push, no tag** — both remain founder-gated.

## Next
1. **polish-report H2 — responsive scale & cross-width acceptance:** fluid canvas/charts/viewBox,
   fluid type ramp, optional `--report-width`, acceptance at 1280/1440/1920/2560/3072 and DPR 1/2;
   vision at 1440/3072; goldens + determinism + story gate.
2. **H3 — artifact consolidation + formatting** (role matrix, report.md Appendix C, style contract).
3. **H4 — report.html final composition** (#explorer → explorer.html; order; A8 re-baseline).
4. **H5 — finalize** (full gate, story gate, docs, state, EOL).
5. **Post-beta map/ranking:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall
   (R-pf-105/106/108).
6. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until
   cleared.

## Blockers
- None for H1. The publish gate remains founder-owned; A2/ship is not claimed by H1.
