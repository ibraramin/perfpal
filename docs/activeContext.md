# Active Context

## Current Task
**Track D2 → S1 → S2 → S3 (2026-10-26): the Atlas Story now carries I/O effects.** Founder review of
D1 accepted presentation/animation but not the information; the requirement is sealed in
`docs/specs/atlas-story.md` as a core-fed **temporal showcase** (entry → stages → loops → I/O → stop)
with the **payload crossing each file boundary** labelled, readable by a vibecoder in 60 seconds and
drillable by a senior dev (`path:line` + types), static inference throughout.
- **S1 committed:** `atlas-layout.json` behind `--story` (exact SVG geometry, state, stages, actions);
  the sim builds from core JSON; HTML extraction deleted.
- **S2 committed and gated (`2108117`…`b8359ae`):** P3 signatures + P1 call payloads emit a
  story-only `flow` block; edges carry `payload`/`label`; unresolved greyed; payoff beats added.
- **S3 committed:** P2 `flow.io[]` (`0ddf4ad`) classifies read/write effects from a fixed v1
  call-name table (file/env/net/stdio) and records a literal only for a plain string argument; the
  story renders `Stores:` on trace beats, a per-file DATA STORES inspector and an I/O trust line with
  literal examples (`9e23aa6`). Gate/vision/panel evidence in `docs/evidence/atlas-sim/`.
- **Not yet:** S4 scenes at 3k files + F1–F8; the companion-vs-report decision.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
- **D-pf-254 (live, S3):** `flow.io` uses a fixed v1 call-name table; a bare C name defined in the
  drawn set, a macro or a fn-pointer is never I/O; a literal is only a plain string literal (never
  formatted/concatenated); non-literal direction and targets stay omitted/`unknown`. `layout_version`
  and `flow_version` remain 1. Limits: wrappers outside the drawn set, fd-based `read`/`write` class,
  method-based Rust file writes.
- **D-pf-253 (live, S2):** `flow` stays story-only and additive; edge labels are computed in the core;
  the resolver is conservative.
- **D-pf-252 (live):** spec sealed; S1 committed (`5faacb7`…`63914f9`); P4/P5 `prototype`.
- **D-pf-251 (live):** D2 founder verdict — Atlas is Perfpal's #1 priority; core additions allowed
  when recorded for merge.

## In Progress
- S3 evidence: gate transcript, `@vision` read of the stores/trust/inspector frames, and the 4-lane
  review panel; the handoff closes S3 after those pass.
- Atlas v2: step 10 approved; step 11 (G-style detail rail) queued; step-13 gate pending.

## Next
- **S4:** scenes/aggregation at 3k files + F1–F8; only then reconsider companion-vs-report (ADR + A12
  if interactive).
- Atlas v2 step 11 remains the parallel queue; no A2/ship until step 13.

## Blockers
- None for the story track. A2/ship still gated by R-pf-109 (Atlas v2 step-13 founder visual gate).
