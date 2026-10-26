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
- **V2 reading path (locked):** `report.html#map` = **key-metrics strip (START actions) → Atlas
  canvas + detail rail → weight/blast/file/coverage charts**; the Level-0/1/2 stems (all 8 current
  stems) are explorer evidence under a labelled `#explorer` section. In the **legacy list-first
  report**, the first content block answers Q1 in plain language; on the Atlas page the first block
  is the key-metrics strip + canvas (step-12 A8 re-baseline).

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

### Locked acceptance view set (V2, 2026-10-21)

Three answer-first slots — one question, one primary encoding each. V3 may choose the idiom
inside a slot; it may not add a fourth slot or stack a second encoding. The feature spec
`map-intelligibility` is authoritative for this table; keep this mirror in sync.

| Slot | Question | Primary encoding | Unit + anchor | Action |
|---|---|---|---|---|
| **KV1 — Start here** | Where do I look first? | severity (ranked list position; the text-first brief is the candidate) | rank #k; cost `static cost units (v2)`, `× median function`, `top 1%` | open/profile `path:line` first |
| **KV2 — Blast radius** | What breaks if I touch it? | coupling (bar = dependent-file count) | `N dependents`, `% of scanned files`, `× median dependent count`, `top 1%` | do not edit directly; expect N dependents to rebuild/re-import |
| **KV3 — Profile queue** | Where might time plausibly go? | cost (bar = `function_cost_v2`) | `static cost units (v2)` / `(v1)` on fallback rows, `× median function`, `top 1%`, unknown-trip badge + known-literal-trip flag | profile the named function first (static candidate, confirm) |

Binding rules:

- **First screen = KV1 (legacy list-first report; superseded for the Atlas page by the V8
  amendment below).** No workspace node-link on the first screen; a plain-language Q1 answer is
  the first content block.
- **All emitted stems under `Explorer`.** The existing `#architecture` section is renamed (heading +
  nav label `Explorer`, `#explorer`), not duplicated. Stems stay available, gated as today
  (fewer than 8 on small scans), and make no answer claim. `layer-findings` is not "density"
  until it has a denominator.
- **Units + anchors mandatory.** No bare measured number (ordinal ranks exempt); every measured
  number states its unit and a scan-relative anchor, with KV3 labelling v1-fallback rows and
  anchoring within the same scale. The 32-char chart-label cap does not apply to key-view text
  rows, which show full `path:line` + function.
- **One encoding per key view.** Severity/confidence are text labels, not visual channels.
- **No silent caps.** KV1 shows 3–5 actions inline and links a complete deterministic ranked
  list; every `hot` function-granularity finding stays reachable. KV3 links the complete
  deterministic function list ranked by cost within the non-demoted set, so the pre-registered
  cost targets stay traceable (A1b).
- **Default-tier honesty.** KV2 claims include blast radius only (0 non-LOW call edges in the
  default tier) and prints `resolved N of M include edges`.
- **Production-first, never silent.** KV1/KV2/KV3 demote test/example/bench/fixture/generated/vendor
  files (`files[].path_class`), print the demoted count; KV2 slots production rows first then up to
  2 greyed demoted rows (never excluded); KV3 also demotes `entry_kind =
  main/script` and no-loop functions below loop-bearing ones (never excluding either; prints both
  counts), labels unknown-trip estimates, and flags known literal trips
  (`known trip ×N — verify it actually runs`).
- **Field decisions:** `files[].path_class` + trip provenance (`trip_count_source`/
  `known_trip_value`) are required; see `docs/evidence/map-data-coverage.md` §8 for the rest and
  their change triggers.

### V3 idiom decision (2026-10-21)

Blind A/B on tmux/hyperfine/cJSON (`docs/evidence/map-idiom-ab.md`): **KV1 = text-first ranked
brief** (2/3), **KV2 = horizontal dependents bars** (3/3), **KV3 = horizontal cost bars** (3/3).
Matrices lose on basename ambiguity; treemaps lose on missing `path:line` and near-equal areas.
The V3 recall check found the locked KV1 severity ranking does not put the pre-registered Q1
answer inline (ranks 210/197/35) and the KV3 no-loop demotion pushes all three Q3 targets down;
V7 re-triages the KV1 rank key and V5 tests the no-loop guard. **KV2 production-first was
ratified by the founder on 2026-10-22 and implemented in V6a** (production rows first, up to 8 +
2 greyed demoted rows, count printed; cJSON's `cJSON.h` now ranks #1 instead of being buried by
22-dependent unity test headers). When the median dependent count is 0, KV2 anchors with
`% of scanned files` + `top P%`, not `× median`.
V4 (2026-10-22) shipped the three key views and the `Explorer` rename; `ARCH-002` component
aggregates are suppressed from KV1 (counted, not ranked; `ARCH-001`/`ARCH-003` stay), KV2
shipped label-only (historical), and A1b is not claimed closed on severity (V7 re-triages the rank key).
V5 (2026-10-22) validated the V4 views with two fresh blind reads per key view: **18/18 valid
reads** state the slot question and one concrete action; the renders are byte-identical to V4
(`docs/evidence/map-vision/v5/`). The **KV3 no-loop falsifier is partially falsified**: the
guard demotes all three Q3 targets, but dropping it alone still leaves them outside the inline
top 8 (shipped→counterfactual ranks 820→325, 58→46, 60→45), so V7 needs call-weighted /
per-iteration ranking, not guard removal. O1 (KV2 `top N%` wording) and O2 (KV2
production-first evidence) feed the founder/V6 decisions. Holdout intact; A1b not claimed
closed. V6a (2026-10-22) implemented the founder-ratified KV2 amendment: production rows first,
up to 8 + 2 greyed demoted rows, demoted count printed; 3 fresh blind renders + **6/6 blind
reads PASS** (cJSON `cJSON.h` #1; hyperfine `BENCH` rows read as demoted-but-never-excluded).
Holdout released only for the V6 timed test (postgres timed, django smoke); `map-build` remains
NOT done; A1b is not claimed closed.

## Non-goals
Full-repo node-link beyond ~1k entities; force-directed full-corpus layout; runtime truth claims.

## Atlas composition (2026-10-23, founder D-pf-238)

**Ratified 2026-10-23** (founder; ADR-0009/0010 Accepted; V8 amendment set).

The core visual is **Perfpal Atlas**: an entry-flow graph page. G (entry-flow) is the core graph;
F (weight map) is not a fallback and not discarded — its weight mass ships as a supporting
below-graph chart, its treemap candidate remains deferred (Explorer-only when built; D-pf-251),
and any other supporting component must pass the six-test inclusion rule (`docs/research/atlas-developer-needs.md` §7). No mode switch. The canvas
is graph-only; the first page pairs it with a small key-metrics strip and a detail rail, and the
designed charts are revealed below. **The v1 seven-block brief is retired (step 12).**

| Level | Composition |
|---|---|
| **Atlas canvas** (first page) | Elected entry (production `main`, then production `script` (Python `__main__`), else highest-fan-in production `lib-root`; else `NO ENTRY` stub), file nodes on BFS depth columns, structural call edges (LOW+MEDIUM, disclosed), component hulls (approximate), unreached tail, numbered representative path trace, legend + caps chip. Graph-only (A9): no cards/tables/bars/selectors |
| **Atlas first page** (strip + rail) | Key-metrics strip above the canvas: entry, HOTTEST, path, caps, three START actions, one caveat, one legend/unit definition. Right detail rail (12 cards): role, cost anchor, top-3 functions, dominant finding, exact function/finding links |
| **Atlas charts** (below) | Four designed charts, state-unconditional: weight (production/non-production share ribbon + production file/component bars), blast radius (dependents, include coverage, callers/callees), file detail (top-6 cost ladder), coverage/links |
| **Drills** | `atlas-q2.html` (Q2 reverse drill), `atlas-weight.html` (complete weight detail), complete action list, complete profile queue, per-component pages |
| **Explorer** | The 8 legacy stems; the F treemap candidate is deferred (Explorer-only when built; D-pf-251), labelled Explorer-tier, no answer claim |

The three questions keep their slots but move on the Atlas page: Q1 = key-metrics strip + canvas
(three START actions in the strip); Q2 = blast-radius chart + `atlas-q2.html`; Q3 = weight/share
charts (file/component share) + `atlas-weight.html` + the complete function-level profile queue in
the drill. One encoding each; severity/confidence stay text. The first page is the strip + canvas;
the strip above the canvas is the first answer-bearing surface and the first below-canvas block is
the weight chart (step-12 A8 re-baseline; `map-intelligibility.md`).

Honesty contract unchanged: caps, stubs, LOW/structural labels and counted demotions survive; the
canvas never prints `resolved` or `reachable_from_entry` for the structural call set; no time
units (`docs/specs/atlas-metrics.md`). Data decision: no schema change in Atlas v1
(`docs/research/atlas-data-gap-audit.md`).