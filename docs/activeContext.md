# Active Context

## Current Task
**Track D2 → S1 → S2 (2026-10-24): the Atlas Story now carries data movement.** Founder review of D1
accepted presentation/animation but not the information; the requirement is sealed in
`docs/specs/atlas-story.md` as a core-fed **temporal showcase** (entry → stages → loops → I/O → stop)
with the **payload crossing each file boundary** labelled, readable by a vibecoder in 60 seconds and
drillable by a senior dev (`path:line` + types), static inference throughout.
- **S1 committed:** `atlas-layout.json` behind `--story` (exact SVG geometry, state, stages, actions);
  the sim builds from core JSON; HTML extraction deleted.
- **S2 committed and gated (`2108117`…`b8359ae`, not pushed):** P3 signatures + P1 call payloads emit a
  story-only `flow` block; every drawn edge carries `payload`/`label` (`type-only` /
  `literal-observed` / `unresolved`), unresolved greyed; payoff beats (`hottest` function, profile,
  start-here, trust) added; cJSON badge fixed. 853 tests; `sim/verify.py` 162/162; gate PASS;
  `@vision` Read 6 PASS.
- **Not yet:** S3 I/O effects (P2), S4 scenes at 3k files + F1–F8; the companion-vs-report decision.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
- **D-pf-253 (live, S2):** `flow` stays story-only and additive (`flow_version: 1`, `layout_version` 1);
  edge labels are computed in the core; the resolver is conservative (unique direct names; ambiguous /
  method / fn-pointer / macro ⇒ `unresolved`); the profile beat is the drawn-slice top-3-per-file
  queue, not a whole-repo KV3 ranking.
- **D-pf-252 (live):** spec sealed; S1 committed (`5faacb7`…`63914f9`); P4/P5 `prototype`.
- **D-pf-251 (live):** D2 founder verdict — Atlas is Perfpal's #1 priority; core additions allowed
  when recorded for merge.

## In Progress
- S2 committed: `2108117`…`b8359ae`, not pushed; evidence tip is the S2 transcript; P3/P1 are
  `prototype` in `docs/atlas/core-probes.md`.
- Atlas v2: step 10 approved; step 11 (G-style detail rail) queued; step-13 gate pending.

## Next
- **S3:** P2 I/O effects (read/write, target file/env/net/stdio, literal when written) → data stores on
  scenes + trust line; one core commit + registry row.
- **S4:** scenes/aggregation at 3k files + F1–F8; only then reconsider companion-vs-report (ADR + A12
  if interactive).
- Atlas v2 step 11 remains the parallel queue; no A2/ship until step 13.

## Blockers
- None for the story track. A2/ship still gated by R-pf-109 (Atlas v2 step-13 founder visual gate).
