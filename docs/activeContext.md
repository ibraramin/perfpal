# Active Context

## Current Task
**Track D2 → S1 → S4 → ADR-0011 (2026-10-26): the Atlas Story remains a companion and does not
graduate into the static report.** The sealed requirement (`docs/specs/atlas-story.md`) is a
core-fed **temporal showcase** (entry → stages → loops → I/O → stop) with the payload crossing each
file boundary labelled, readable by a vibecoder in 60 seconds and drillable by a senior dev
(`path:line` + types), static inference throughout. S4 is built and gated (`71d0d04`); the
companion-vs-report decision is made and recorded (`ADR-0011`).
- **S1 committed:** core `atlas-layout.json` behind `--story`; the sim builds from core JSON.
- **S2 committed and gated:** P3 signatures + P1 call payloads; edge labels + payoff beats.
- **S3 committed and gated:** P2 `flow.io[]` + stores/trust/inspector.
- **S4 built and gated (`9263c19`…`71d0d04`):** P6 README `purpose`, P7 authoritative whole-repo KV3
  queue, P8 directory scenes (depth-2, 24/5 caps, omitted totals, node/action scene context); sim
  purpose masthead, enter-beat scene context, whole-repo profile beat, scene-aware start-here,
  below-fold SCENES / PROFILE QUEUE / TRUST ACCOUNTING, unresolved-calls inspector. Deterministic
  3k fixture + `sim/scale.py`/`scale.sh` gate F2–F7 (page **463,490 bytes**, first frame **under the 2,000 ms budget** (final transcript), no h-scroll at 1280, double-scan/load determinism). Gate `dirty 0` PASS; `sim/verify.py`
  **304/304**; 3k **44/44**; 897 Rust tests; `@vision` Read 8 **PASS-WITH-NITS, no blockers**. F8
  (founder A2) is **deferred**, not claimed.
- **Not yet:** the companion is **not shipped** — it still loads data with `fetch()` and lives in
  `sim/`. A self-contained artifact (inline data, no server) + a CLI entry point is the recorded
  precondition for the founder D2/F8 session; it is not scoped or started.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
- **D-pf-256 / ADR-0011 (live, 2026-10-26):** the Atlas Story stays a **standalone companion**; no
  in-report graduation in v1. **A12 unchanged** (report keeps exactly one fixed `REPORT_SCRIPT`;
  canvas/SVG/animation zero scripts/handlers/network; no CSS/SMIL story film). S4 proves the
  mechanism (F1–F7, 3k 44/44, F1 read PASS-WITH-NITS), but the founder D2 interaction gate has not
  run and the report composition is unapproved (`R-pf-109`). Graduation conditions: founder D2 on a
  self-contained story artifact + step-13 report gate + deterministic offline player + explicit A12
  amendment. No code/schema change; F8 deferred.
- **D-pf-255 (live, S4):** P6 purpose (README first prose paragraph, structural fallback), P7 KV3
  queue (authoritative order, top 12 production non-demoted rows + counts) and P8 directory scenes
  (depth-2 partition, 24-scene/5-file caps, production-cost-first, omitted totals, shared
  `atlas::file_cost`, node/action `scene`) are story-only, additive and `layout_version: 1`. Limits:
  README first-only; 12-row queue; 24/5/depth-2 scenes; scene cost is static, not runtime; the 3k
  fixture is synthetic. F8 founder A2 deferred.
- **D-pf-254 (live, S3):** `flow.io` fixed v1 call-name table; a bare C name defined in the drawn
  set, a macro or a fn-pointer is never I/O; a literal is only a plain string literal; non-literal
  direction/targets stay omitted/`unknown`. Limits: wrappers outside the drawn set, fd-based
  `read`/`write` class, method-based Rust writes.
- **D-pf-253 (live, S2):** `flow` stays story-only/additive; edge labels are computed in the core.
- **D-pf-252 (live):** spec sealed; S1 committed; P4/P5 `prototype`.
- **D-pf-251 (live):** D2 founder verdict — Atlas is Perfpal's #1 priority; core additions allowed
  when recorded for merge.

## In Progress
- **ADR-0011 recorded:** companion decision + A12 unchanged + graduation triggers; ADR, spec
  annotations, CHANGELOG and state updated. Docs-only session; no code, schema or dependency change.
- **S4 remains the gated story baseline:** `71d0d04` (tree `51548f3`, dirty 0, 897 tests, 304/304,
  3k 44/44), evidence `2ebeabd`; F8 deferred.
- Atlas v2: step 10 approved; **step 11 (G-style detail rail) is next**; step-13 gate pending.

## Next
- **Atlas v2 step 11** (detail rail) in a fresh session; then step 12 (progressive charts) and the
  step-13 founder/V8 gate. No A2/ship until step 13.
- Story follow-up (not active): self-contained companion artifact + CLI entry, then the founder D2/F8
  session; graduation only per ADR-0011 Decision 4.

## Blockers
- None for the story track. A2/ship still gated by R-pf-109 (Atlas v2 step-13 founder visual gate).
