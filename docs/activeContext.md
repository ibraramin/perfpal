# Active Context

## Current Task
**V7 prototype burst complete (F/G, 12/12 blind reads). Founder corrected the model: G is the CORE
graph; F is NOT a fallback and not discarded — F-derived components are supporting elements,
included only if developer-needs research justifies them. Next = research/planning only
(`map-shipped`).**
- **Founder constraints:** graph page = 100% graph; metrics/brief below or elsewhere; standardize
  metrics; G core; F components as needed; paper parked until after agent report + map + JS/TS.
- **Evidence:** `docs/evidence/map-vision/v7/` — F/G renderers, 18 PNGs (9 each), 12 blind reads
  (`f/reads.json`, `g/reads.json`), `COMPARISON.md`, `README.md`; plans `map-f-treemap`,
  `map-g-flow`; animation spec `map-animation`.
- **What the reads found:** G strong on hyperfine/tmux (entry + HOTTEST + action); cJSON is a
  dead-end (library, no production main). F's brief is reliable everywhere; its cJSON treemap is
  hijacked by test cost (93.5%), production cells 0.2%.
- **No code; holdout vaulted; `map-build` NOT done; A1b NOT closed.**

## Recent Decisions
- **D-pf-238 (live):** founder correction — G core, F components supporting (not fallback);
  full-page graph; metrics below; research/planning next.
- **D-pf-237 (live):** entry-flow direction + prototypes + animation mandatory.
- **D-pf-236 (live):** consolidation; one core + three consumers.

## In Progress
- **Research/planning (`map-shipped`)** — start a fresh session. First: developer-needs research
  (what to see alongside the main graph), then metrics standardization, data-gap audit, full-page
  combined layout, component-inclusion rule, naming, spec/ADR, final build plan. No implementation.

## Next
- **Fresh session signposts:** plan `map-shipped`; specs `map-animation`/`map-intelligibility`/
  `maintainer-map`; evidence `docs/evidence/map-vision/v7/`; decision D-pf-238.
- **Feature name:** **Perfpal Atlas** (founder, 2026-10-22). G core = flow map; F components =
  weight-map support.
- **Agent report track** remains queued (A-pf-111); map research first per founder.
- **Do not mark `map-build` done.** A1b stays open. M5 needs B-pf-001 + B-pf-002.

## Blockers
- **Map legibility:** R-pf-104/R-pf-105/R-pf-106 — views validated (A1), but ranking recall
  (A1b) is open; founder A2 is the V6 gate.
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prereg + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Map research:** R1–R4 → D-pf-215.
- **I1–I4:** entries C/Rust/Python + cost-v2 (`e0df47a`); entry map + bands (`d78bb85`);
  annotations + findings stem (`f56d072`).
- **I5:** proof + demo + **A5 FAIL** (`dbb9bbe`→`6f31a4f`), successor plan created.
- **V0/V1:** prereg `3982b03` → diagnosis `3a32b01` → recall `5ba3740` → review fixes
  `bb5b2f9`/`54a4586`/`2be3523`.
- **V2:** methodology + spec lock `45fa514`; EOL `423de20`.
- **V3:** prereg `1f042af` → 18 prototypes + 27 blind reads → decisions `81b685e`; EOL `e0ded77`.
- **V4:** schema 1.7 `bc887ef` → key views `de93292` → ARCH scope `0bd6d08` → rank key/nav
  `ae50e61` → dual bar scale `f896631` → review round 2 `882ddce` → vision/recall evidence
  `2ec547c`; EOL `758f492`.
- **V5:** fresh scans + 9 byte-identical renders + 18 blind reads + no-loop falsifier;
  evidence `docs/evidence/map-vision/v5/`; EOL this commit.
