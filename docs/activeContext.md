# Active Context

## Current Task
**Track D2 → S1 → S2 → S3 → S4 (2026-10-26): Atlas Story S4 is built and gated.** The sealed
requirement (`docs/specs/atlas-story.md`) is a core-fed **temporal showcase** (entry → stages → loops
→ I/O → stop) with the payload crossing each file boundary labelled, readable by a vibecoder in 60
seconds and drillable by a senior dev (`path:line` + types), static inference throughout.
- **S1 committed:** core `atlas-layout.json` behind `--story`; the sim builds from core JSON.
- **S2 committed and gated:** P3 signatures + P1 call payloads; edge labels + payoff beats.
- **S3 committed and gated:** P2 `flow.io[]` + stores/trust/inspector.
- **S4 built and gated (`9263c19`…final close commit):** P6 README `purpose`, P7 authoritative whole-repo KV3
  queue, P8 directory scenes (depth-2, 24/5 caps, omitted totals, node/action scene context); sim
  purpose masthead, enter-beat scene context, whole-repo profile beat, scene-aware start-here,
  below-fold SCENES / PROFILE QUEUE / TRUST ACCOUNTING, unresolved-calls inspector. Deterministic
  3k fixture + `sim/scale.py`/`scale.sh` gate F2–F7 (page **463,490 bytes**, first frame **under the 2,000 ms budget** (final transcript), no h-scroll at 1280, double-scan/load determinism). Gate `dirty 0` PASS; `sim/verify.py`
  **304/304**; 3k **44/44**; 897 Rust tests; `@vision` Read 8 **PASS-WITH-NITS, no blockers**. F8
  (founder A2) is **deferred**, not claimed.
- **Not yet:** the final 4-lane panel/close and the companion-vs-report ADR.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
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
- **S4 close:** final gate on the docs close commit, 4-lane review panel (R1/R2/Q1/Q2), evidence/state
  commit. S4 code/gate/vision are green.
- Atlas v2: step 10 approved; step 11 (G-style detail rail) queued; step-13 gate pending.

## Next
- Finish the S4 close; then the companion-vs-report ADR (+A12 if interactive; S4 evidence is now
  available).
- Atlas v2 step 11 remains the parallel queue; no A2/ship until step 13.

## Blockers
- None for the story track. A2/ship still gated by R-pf-109 (Atlas v2 step-13 founder visual gate).
