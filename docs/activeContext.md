# Active Context

## Current Task
**R-pf-112 — branch reconciliation (2026-10-26).** `feat/atlas-sim` is a pre-merge Atlas Story
snapshot; `main` is the completed Atlas v2 line. `ADR-0012` records the recommended direction
**C**: keep the story branch separate from `main` until the **§5 founder session**; no story→main
merge now. The next implementation (after founder approval) is a **main→branch sync** (7 conflicted
paths) followed by the **story packaging slice** (self-contained `story.html` + `perfpal story
<repo>` CLI) and then §5. No push, no merge, no A2/ship claim.
- **Actual divergence:** 63 story commits ahead / 23 main commits behind (the recorded 61/8 was
  stale); merge base `821b984`.
- **Main v2 is DONE:** steps 11–15 (`189df91`, `9a94905`, step 13 `86c0537`/D-pf-253, A2 `D-pf-255`,
  step 15 `57d91a2`/D-pf-256/257/258); `atlas-build` done; `R-pf-109` closed as a record. **Do not
  re-run step 11 on this branch.**
- **Story baseline:** S4 gated at `71d0d04` (897 tests, `sim/verify.py` 304/304, 3k 44/44, page
  463,490 B, first frame 1,706 ms); ADR-0011 decided companion, A12 unchanged; F8 deferred.
- **IDs remapped** to avoid main collisions: D-pf-251..255 → D-pf-260..264; A-pf-124/125 →
  A-pf-126/127; D-pf-259 (ADR-0011) and D-pf-265 (ADR-0012) are live; D-pf-266 is the archive
  pointer. The ledger keeps the old IDs + a `decision_id_remap` entry.
- **Not yet:** the story is not shipped; the companion still `fetch()`es data and has no CLI entry.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
- **D-pf-265 / ADR-0012 (live, 2026-10-26, Proposed):** R-pf-112 reconciled. Direction **C** — the
  story branch stays separate until §5; no story→main merge now. Actual divergence 63/23 (not 61/8);
  7-path sync conflict; IDs remapped; `R-pf-109` closed as a main-line record; `atlas-build` done
  (main D-pf-258). Next: founder approval → main→branch sync → packaging → §5. No push/A2/ship.
- **D-pf-259 / ADR-0011 (live, 2026-10-26):** the Atlas Story stays a **standalone companion**; no
  in-report graduation in v1. **A12 unchanged** (report keeps exactly one fixed `REPORT_SCRIPT`;
  canvas/SVG/animation zero scripts/handlers/network; no CSS/SMIL story film). Graduation conditions:
  §5 founder session on a self-contained story + report gate (main closed 2026-10-26) + deterministic
  player + explicit A12 amendment. No code/schema change; F8 deferred.
- **D-pf-264 (live, 2026-10-26):** S4 done — P6/P7/P8 probes + sim; gate `71d0d04` PASS (897 tests,
  304/304, 3k 44/44, page 463,490 B, first frame 1,706 ms); vision PASS-WITH-NITS; F8 A2 deferred.
- **D-pf-263 (live, S3):** `flow.io` fixed v1 call-name table; a bare C name defined in the drawn
  set, a macro or a fn-pointer is never I/O; a literal is only a plain string literal; non-literal
  direction/targets stay omitted/`unknown`.
- **D-pf-262 (live, S2):** `flow` stays story-only/additive; edge labels are computed in the core.
- **D-pf-261 (live, S1):** core `atlas-layout.json` behind `--story`; sim builds from core JSON.
- **D-pf-260 (live, D2):** D2 founder verdict — presentation/animation accepted, information not
  insightful; Atlas is Perfpal's #1 priority; core additions allowed when recorded for merge.
- **Archived (hot-state budget):** D-pf-238..243 (Atlas model, contracts, structural edges, A8, A12)
  are in `state/archive/ledger.jsonl`; key points unchanged: G is the core canvas, F is supporting;
  structural LOW/MEDIUM edges with disclosure; A8 canvas-first; A12 zero-new-JS. Main's D-pf-251 is
  the **F-treemap decision** — do not conflate it with the renumbered story D-pf-260.

## In Progress
- **ADR-0012 reconciliation record (docs/state only):** direction C recorded; `atlas-build` marked
  done with a pre-merge snapshot banner; branch state/activeContext/track handoff corrected; IDs
  remapped; `R-pf-109` closed as a record. No code, schema, dependency or output change.
- **S4 remains the gated story baseline:** `71d0d04` (tree `51548f3`, 897 tests, 304/304, 3k 44/44),
  evidence `2ebeabd`; F8 deferred.
- **Main v2 is done:** commit `c797004` (records `57d91a2`); `R-pf-109` closed; delegated A2 pass
  (`D-pf-255/257`); `atlas-build` done (`D-pf-258`). This branch snapshot does not contain steps
  11–15 and makes no A2/ship claim.
- **Branch status:** not merged; 23 commits behind main until a sync. No step-11 work here.

## Next
- **Founder approves ADR-0012 direction C** (or chooses A/B). On **C**: merge `main` into
  `feat/atlas-sim` (one merge commit; resolve the 7 paths; restore `strip_entry_line` to
  `pub(crate)`; keep `start_actions`/`build_atlas_view`), run the full story gate, then build the
  self-contained story + `perfpal story` CLI → §5. On **A**: use the merge-based A′ (not a literal
  rebase). On **B**: four curated cherry-picks; keep the branch as the evidence archive.
- **No push/merge without explicit founder go;** no step-11 work on this branch.
- Story follow-up (not active): self-contained companion + CLI entry, then §5; graduation only per
  ADR-0011 Decision 4.

## Blockers
- **R-pf-112:** direction recorded; execution (sync + packaging) pending founder approval; the
  branch stays 23 behind until then.
- **R-pf-105/106/108:** ranking recall / static-cost limits / structural-edge honesty; A1b open.
- **Publish gate:** B-pf-001 (contact) + B-pf-002 (minisign secrets) — founder-owned; beta parked.
- **No A2/ship from this branch;** founder V6 gold standard remains open (delegated A2 only).
