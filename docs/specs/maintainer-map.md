# Spec: maintainer-map

Scope: repo

# Maintainer Map Spec (repo scope)

The birdseye map is the core feature. Maintainers get one visual answering three questions;
findings ride as annotations on the map, never as a list to cross-reference by hand.

## The three questions (priority order)
1. Where do I look first? Ranked hot-spots per KLOC, not raw counts.
2. What breaks if I touch it? Coupling / blast-radius via includes + calls.
3. Where might time plausibly go? Static cost hints only — candidates to profile, never certainties.

## Composition
- Level 0 workspace: components as nodes, aggregated inter-component edges, heat = finding density.
- Level 1 file: top files by degree, heat = per-file function cost, single outlined hottest file.
- Level 2 component pages: intra-component files + edges (existing pages).
- Drill path: report header → workspace → file/component → findings-detail rows (finding_id + path:line:col).

## Honesty contract
- Caps pre-layout (top-50, ties path-asc), top-10 report rows, labels tail-kept 32 chars.
- Omit: cross-component file edges (aggregated + captioned), LOW-confidence cross-file call edges,
  flame graphs, history/diff, JS layout.
- Mark in-chart: `showing top S of M`, `…+N more`, `no files/components recorded` stubs (never blanks).
- Every heat legend reads candidate-density/cost — profile to confirm; severity+confidence glued to each annotation.

## Constraints
Static-only, no-build, musl-static, byte-identical double-scan, offline SVG/HTML, additive schema only.
JS+TS backends and the paper queue behind the map.

## Visual intelligibility (founder directive, 2026-10-21)

The I5 human A5 test **failed** ("I can't understand anything"). The successor spec
`map-intelligibility` governs the fix. Durable rules from this point:

- **Two tiers.** *Answer-first key views* must let a maintainer unfamiliar with the repo name
  a concrete action in plain language; *explorer / info-dump stems* may be dense but must be
  labelled as such and must never be the only path to an answer.
- **Vision gate.** Every chart/visual change is reviewed by `@vision` against this
  methodology before it is claimed done: what it communicates, what cannot be answered, which
  maintainer question it serves. Transcripts are evidence.
- **No orphan views.** Every key view names its question and the action it enables; a view
  that answers nothing is removed or demoted.
- **Plain language first.** Jargon stays in the explorer tier; key views use maintainer words
  ("what breaks if I change this", "where is time likely going", "what is safe to ignore").
- The honesty contract above is unchanged: candidates, caps, stubs and LOW labels survive
  any redesign.

## Non-goals
Full-repo node-link beyond ~1k entities; force-directed full-corpus layout; runtime truth claims.