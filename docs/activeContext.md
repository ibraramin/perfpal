# Active Context

## Current Task
**Track D2 → S1 (2026-10-24): the Atlas Story is now core-fed.** Founder review of D1: presentation and
animation accepted, but "the info isn't insightful" — the story replayed file-level facts with motion.
The requirement was re-derived from the ratified docs and sealed as `docs/specs/atlas-story.md`:
a **temporal showcase** (entry → stages → loops → I/O → stop) with the **payload crossing each file
boundary** labelled, readable by a vibecoder in 60 seconds and drillable by a senior dev
(`path:line` + types), static inference throughout, shipped as `perfpal <repo> --story`.
- **S1 committed (not pushed):** Perfpal core emits `atlas-layout.json` behind `--story` — the exact geometry the SVG
  draws, presentation strings, state flags, derived `stages`, KV1 actions and drawn/scan findings
  counts. `sim/` builds its data from that artifact + `metrics.json`; HTML extraction
  (`sim/extract.py`) is **deleted**. `sim/verify.py` 101/101.
- **Not yet:** data movement itself (P1–P3 → S2/S3), scenes at scale (S4), payoff beats.

## Core-integration rule (live, founder 2026-10-24)
Atlas is a view of Perfpal, not a separate pipeline. Any data it needs that the core does not emit
becomes a **core probe**: additive, story-only where possible, one commit per probe, and recorded in
`docs/atlas/core-probes.md` (files, fidelity, limits, real-implementation note) so it can be
implemented properly and merged later. Existing scan outputs must stay byte-identical.

## Recent Decisions
- **D-pf-252 (live):** Atlas Story spec `docs/specs/atlas-story.md` sealed; S1 committed
  (`5faacb7`…`63914f9`, not pushed, gate PASS); P4/P5 status `prototype`; next is S2 (P3 signatures + P1 call
  payloads).
- **D-pf-251 (live, 2026-10-24):** D2 founder verdict — D1 presentation/animation accepted, information
  not insightful; Atlas is Perfpal's #1 priority; Atlas consumes Perfpal core data; core additions are
  allowed for this track when recorded for later merge.
- **D-pf-249 (live):** D1 prototype (committed `7093c77`); its HTML-extraction data path is superseded
  by D-pf-252.

## In Progress
- S1 committed: `5faacb7`…`63914f9` (core, sim, panel fixes, provenance/gate), not pushed; evidence tip
  `5bd4987`. `atlas-layout.json` carries `layout_version: 1`.
- Atlas v2: step 10 approved; step 11 (G-style detail rail) queued; step-13 gate pending.

## Next
- **S2:** P3 signature summary + P1 call-site payloads → data labels on the film's edges, unresolved
  indirection greyed; then the payoff beats (`hottest` function, `profile`, `start-here`, `trust`).
- **S3:** P2 I/O effects (file/env/net read/write with literal targets).
- **S4:** scenes at 3k files + F1–F8 gate; only then reconsider the companion-vs-report decision.
- Atlas v2 step 11 remains the parallel queue; no A2/ship until step 13.

## Blockers
- None for the story track. A2/ship still gated by R-pf-109 (Atlas v2 step-13 founder visual gate).
