# Spec: atlas-metrics

Scope: feature

# Spec: atlas-metrics (feature)

**Status:** planning lock, Atlas v1. **No implementation in this spec.**
**Date:** 2026-10-23. **Owner:** orchestrator. **Supersedes:** the ad-hoc anchor wording in the V7
prototypes; it does **not** supersede the KV slot questions in `map-intelligibility`.
**Inputs:** `docs/research/atlas-developer-needs.md`; `docs/research/atlas-data-gap-audit.md`;
`docs/evidence/map-vision/v7/{f,g}/reads.json`.

## 1. Purpose

One canonical display contract for every number on the Perfpal Atlas page and its drills. The
contract exists because the V7 reads show three live failures: a huge multiple was misread
(`83,354×` → `53,354×`/`3,354×`), readers invented `ms`/`µs` where only static cost units exist, and
the F/G prototypes used different denominators (`× median component` vs `× median function`) that
invite invalid comparison.

## 2. Principles (binding)

1. **One measured encoding per surface.** The canvas encodes **cost** (node label text; no
   colour/size metric). Severity and confidence are text. Coupling is a below-graph block. No
   stacked channels.
2. **Unit + anchor + denominator adjacent.** Every measured number states its unit and its anchor,
   and the anchor names its denominator in the same text run. Ordinal positions (`#1`, `depth 3`,
   `step 2`) are exempt.
3. **No time.** Static cost is never converted to `ms`, `µs`, `s`, throughput or a runtime claim.
   The words `runtime`, `speedup`, `latency` and `hot path` (as a runtime claim) are forbidden on
   Atlas surfaces.
4. **No bare number.** A cost, share, multiple, percentile or count without its unit/anchor (as
   applicable) is a spec violation.
5. **Production-first, never silent.** Demoted files are counted and labelled; they are never
   dropped from the ordering.
6. **Caps and coverage printed.** `N of M`, `N aggregated`, `N drawn of M`, `N unresolved`,
   `N partial/failed` are part of the metric set, not debug text.
7. **Structural is structural.** Edges drawn from the default-tier call graph are
   `structural (LOW name-based + MEDIUM semantic)`; the Atlas never prints `resolved` or `reachable_from_entry` for that
   set. `N MEDIUM / M LOW` is printed.
8. **State-unconditional.** Every metric block renders in every scan state, with a zero-state stub
   when the value is absent. Data changes content, never page structure.
9. **Deterministic formatting.** Same input bytes → same strings; no locale-dependent formatting;
   sorted tie-breaks everywhere.

## 3. Canonical metric set

| ID | Metric | Definition / source | Unit | Anchor | Format | Placement |
|---|---|---|---|---|---|---|
| **M-01** | Entry | Elected entry function + policy. Source `stats.functions[].entry_kind`, `entry_confidence`. Values: `production main`, `production script (Python __main__)`, `library root (no production main)`, `no entry`. | policy label + confidence label | alternatives count (`N alternatives`) | `ENTRY · production main · confidence HIGH · 0 alternatives` | canvas badge + answer line |
| **M-02** | Estimated reachable files | Source files reachable from the elected entry over the **full structural edge set (pre-cap)**. Not `reachable_from_entry` (resolved-only). | files | `N of M source files` + `% of source files` | `19 of 53 source files reachable (36%)` | canvas footer + answer line |
| **M-03** | Depth | BFS depth from the elected entry (ordinal). | depth | none (ordinal) | `depth 0` … `depth 5` | canvas columns/rings |
| **M-04** | File cost | Σ `function_cost_v2` for the file's functions. Unit v2; v1-fallback rows read v1 and anchor within the v1 set. | `static cost units (v2)` / `(v1)` | `× median function` + `top P% of files` | `file total 611 static cost units (v2) · 87× median function · top 4% of files` | canvas node + file cards |
| **M-05** | Function cost | `function_cost_v2` for one function. | `static cost units (v2)` / `(v1)` | `× median function` + `top P% of functions` | `611 static cost units (v2) · 87× median function · top 4% of functions` | below actions/cards + drill |
| **M-06** | File cost share | File cost ÷ total scanned cost (all functions, all classes). | `% of scanned static cost` | denominator printed | `36% of scanned static cost` | below weight strip |
| **M-07** | Component cost share | Σ file cost under a component/domain prefix ÷ total scanned cost. Membership by prefix, disclosed as approximate. Production-first ordering; non-production components are counted in the labelled band, never dropped. | `% of scanned static cost` | denominator printed | `12% of scanned static cost · 7 files` | below weight strip |
| **M-08** | Include blast radius | Distinct files with a resolved include edge into the target file. | dependent files | `% of source files`; `× median dependent count` only when median > 0; `top P%` | `22 dependent files · 10% of source files · top 2%` | below blast-radius block + drill |
| **M-09** | Call fan-in / fan-out | Distinct caller files / callee files over structural call edges. | files | none | `N caller files · M callee files (structural)` | below blast-radius block |
| **M-10** | Distinct call edges | Distinct caller-function → callee-name → target-file edges, aggregated to a file pair. **Not raw call sites.** | distinct structural edges | none | `56 distinct structural edges` | canvas edge label + coverage line |
| **M-11** | Severity + confidence | `findings[].severity` (`hot`/`warn`/`info`) + `findings[].confidence` (`HIGH`/`MEDIUM`/`LOW`). Text only. | labels | none | `HOT · MEDIUM` | below actions/cards; text only — M-18 is the sole canvas text annotation (no colour/size/area channel) |
| **M-12** | Trip provenance | `trip_count_source`, `known_trip_value`, `unknown_trip_loops`. | label + `×N` | none | `known trip ×10,000 — verify it actually runs` / `estimate incomplete` | below actions/cards |
| **M-13** | Coverage / noise | Parse `ok/partial/failed`; unresolved imports/includes; `MEDIUM/LOW` edge counts; demoted files; unreached production/non-production. | counts | denominators printed | `parse 100 ok / 116 partial / 0 failed · 0/234 unresolved includes · 57 MEDIUM / 88 LOW edges · 42 files demoted · 99 unreached production` | below noise footer + canvas caps chip |
| **M-14** | Caps / aggregation | `N of M files shown`; `N reachable aggregated`; `N edges drawn of M pairs`; `N small scopes hidden`. | counts | denominators printed | `20 of 106 reachable files shown · 86 aggregated · 57 of 145 file pairs drawn` | canvas footer + below |
| **M-15** | Non-production mass | Σ cost of non-production (test/bench/other) files ÷ total scanned cost. Labelled band; **never area**. | `% of scanned static cost` | denominator printed | `non-production (test/bench/other) = 17% of scanned static cost (42 files)` | below weight strip |
| **M-16** | Path trace | Ordered file sequence from the elected entry to a deterministic BFS leaf. Endpoint rule: the BFS-tree leaf (no BFS-tree child) chosen by cost↓, depth↓, path asc; then walk the parent chain. Representative, static. | steps (ordinal) | none | `representative path (static) · 4 steps` + numbered files | canvas |
| **M-17** | Scope counts | Scanned files, functions and findings (page header). | counts | none (scope counts, not measured values) | `216 files · 3,189 functions · 7,545 findings` | page header |
| **M-18** | Node finding annotation | Dominant finding severity + count per file, text only (`HOT ×N` / `WARN ×N` / `INFO ×N`; the full rule list is in the file card below). Keeps findings as annotations on the map (north-star) without a colour/size channel. | severity label + count | none | `HOT ×3` | canvas node (text annotation) |

## 4. Placement contract

| Surface | Allowed metrics | Forbidden |
|---|---|---|
| **Canvas** | M-01, M-02 (canvas footer), M-03, M-04 (text label), M-10, M-13 (caps/coverage subset), M-14, M-16, M-17 (header), M-18 (node annotation); legend with the cost unit, depth key, edge style, production/test style, unreached key | cards, tables, bars, metric panels, metric selectors, a second map, severity colour/size, time |
| **Below graph** | M-01, M-02, M-04, M-05, M-06, M-07, M-08, M-09, M-11, M-12, M-13, M-15 | a second full map; a duplicate of a canvas label with no added denominator/share |
| **Drill** | Complete M-05 ranked list; complete M-08 dependents; M-07 detail; M-04 component multiples (Explorer) | claims the drill cannot support (e.g. resolved-call blast radius in the default tier) |
| **Explorer** | Legacy stems + `weight-treemap`; component multiples with denominator printed | answer claims; key-view status |

## 5. Formatting rules

- **Cost:** integer, thousands separators (`6,030`), no decimals.
- **Multiples:** `<10×` one decimal (`2.6×`); `≥10×` integer with thousands separators
  (`83,354×`); never abbreviate (`53K×` is forbidden); full value repeated in `title`/aria when the
  visible label is shortened by layout.
- **Percent / share:** `<1%` one decimal (`0.2%`); `1–9.9%` one decimal (`6.2%`); `≥10%` integer
  (`36%`). `top P%` is always an integer. `% of source files` uses the same rule.
- **Counts:** integer with thousands separators (`7,543`).
- **Paths:** canvas labels tail-keep 32 characters with the full path in `title`/aria; below-graph
  text rows show the full `path:line`; function names are full (no 32-char cap).
- **Ordinals:** `#1`, `depth 3`, `step 2`.
- **Confidence/severity:** uppercase labels (`HIGH`, `MEDIUM`, `LOW`, `HOT`, `WARN`, `INFO`).
- **Missing values:** print a named stub (`no resolved dependents`, `no entry`) — never a blank,
  never `0×`, never a division by zero.

## 6. Anchor rules

1. **`median function`** = the shipped `median()` value over **all functions in the scan, zeros
   included**: nearest-rank, `ceil(0.5·N)`-th smallest (1-based; lower-middle for even N),
   `src/reporter/views.rs:191–198`. The Atlas reuses the shipped `ScaleOrdering` ordering
   (`src/reporter/views.rs:288–331`) for the value and ranks; no second median implementation.
   The shipped `ratio_label` emits no thousands separators and says `× median`; `atlas_metrics`
   owns the rendered label (`× median function`, separators) so the contract rules below hold.
2. **`median dependent count`** = median reverse-include count over all source files, zeros
   included. If the median is 0, the multiple is omitted and `top P%` is used.
3. **`top P%`** = `max(1, round(100 × rank / N))`, capped at 100, rank 1-based, stable tie-break
   (`cost desc → fan-in desc → path asc → name asc → id asc`), `N` = the population named in the
   label (`functions`, `files`, `source files`). This matches the shipped `top_label()`
   (`src/reporter/views.rs:231–236`). Rank 1 = `top 1%`.
4. **Denominator adjacency.** A multiple and its denominator are one text run; a share and its
   denominator are one text run. A reader must not need another block to know the base.
5. **No cross-scale comparison.** `× median function` and `× median file` / `× median component`
   never appear in the same table or sentence as if comparable. The Atlas page uses only
   `× median function`; the weight strip uses `% of scanned cost`; the Explorer treemap may use
   `× median component` with its denominator.
6. **v1 fallback.** Rows computed from v1 cost read `static cost units (v1)` and anchor within the
   v1 set. A v1 row and a v2 row never share a multiple.

## 7. Cost, severity, confidence

- Cost is a **static heuristic in cost units**, never a duration. A tooltip/aria may explain the
  unit; it may not translate it to time.
- Severity and confidence are **text labels only**. They may use a text chip; they may not use
  colour, size, area, border weight or animation as a channel.
- The `HOTTEST` badge on the canvas is a **cost rank** (M-04 rank 1), not a severity claim.
- Trip provenance is printed next to any action whose cost includes a known literal trip; unknown
  trip loops print `estimate incomplete`.

## 8. Coverage and honesty

- The noise footer is mandatory in every state. It prints, at minimum: parse `ok/partial/failed`;
  unresolved imports/includes; `MEDIUM/LOW` edge counts; demoted file count; caps; unreached
  production and non-production counts.
- `structural` edges are captioned `structural (LOW name-based + MEDIUM semantic), may over/under-approximate`. They are
  never called resolved, exact or runtime.
- Production-first ordering applies to KV1 actions and the weight strip; demoted files are counted,
  never excluded from the ordering.
- An empty state is a named stub (`no findings`, `no resolved dependents`, `no entry`, `no cost`).

## 9. Forbidden strings/claims (checked by test)

- `ms`, `µs`, `runtime`, `speedup`, `latency`, `throughput` as **claims** on an Atlas surface.
  Emitted labels use `static` / `BFS-derived`; a doc-level sentence that says "not runtime" is not
  an emitted surface. The test scans emitted HTML/SVG strings, not markdown docs.
- A bare cost, a bare `×`, a `%` without a denominator, a multiple without its denominator.
- `resolved` or `reachable_from_entry` for the structural call set.
- A colour/size/area second metric on the canvas.
- A data-keyed mode switch (a block that appears only for some scans).

## 10. Acceptance

- **A7 (units/anchors):** a blind read of each Atlas surface finds no bare measured number and can
  state each visible unit; ordinal ranks exempt.
- **A11 (denominator):** a blind read can state what each multiple/share is relative to; any read
  that mistakes `× median function` for time fails the gate.
- **A9 (canvas purity):** DOM assertion: the canvas contains no `<table>`, no bar/card list, no
  metric selector; only graph nodes/edges/labels/legend/caps.
- **A10 (state-unconditional):** the same seven blocks render for application, library, no-entry,
  empty, all-test, capped, partial-parse and single-file fixtures; only content/labels change.
- **A4 (determinism):** double scan → byte-identical HTML/SVG; formatting has no locale input.
- **Unit tests:** formatting (`<10×`, `≥10×`, huge multiple, thousands separators), zero median,
  zero total cost, v1 fallback, percentile ties, stub text.

## 11. Open items (owner: build plan)

- Whether the weight strip's component rows use `components[].path_prefix` or the chart-side
  `domain_of` grouping; the spec allows either but not both in one table.
- The exact `title`/aria wording for a shortened huge multiple.
- Whether `% of source files` counts files with `function_count > 0` (default) or all scanned files
  (only if the holdout shows the distinction matters).