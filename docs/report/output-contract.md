# Perfpal — `perfpal-report/` Output Contract

- **Status:** v1 draft — plan `mvp-architecture`, step 4. Governs everything Perfpal writes to disk for one scan.
- **Grounding:** [`metrics.schema.v1.json`](../schema/metrics.schema.v1.json) + [`metrics-json-v1.md`](../schema/metrics-json-v1.md) (metrics.json format & versioning policy), [`catalog-v1.md`](../heuristics/catalog-v1.md) (severity/confidence model), spec ["Product Form (decided)"](../specs/perfpal-core-spec.md) + founder confirmation CONFIRMED-5 (`state/session-state.json`: default layout).
- **Audience:** Perfpal implementers (steps 5–6 and beyond) and any consumer of a report directory — humans, tooling, LLMs.

---

## 1. Principles

1. **One scan → one directory.** A completed report directory contains *exactly* the artifacts of that run — nothing stale from previous runs survives.
2. **metrics.json is the source of truth.** `report.md` and `charts/` are derived artifacts: pure functions of `metrics.json` plus the reporter's built-in heuristic catalog (frozen for v1; used only for Appendix A descriptions). If they ever disagree, `metrics.json` wins.
3. **Single version marker.** The `schema_version` inside `metrics.json` governs the entire directory. There is no separate "report format" version to drift.
4. **Self-contained.** The JSON Schema ships inside the report so any consumer (offline tool, LLM session) can validate and interpret `metrics.json` without access to the Perfpal installation.
5. **Deterministic where it matters.** Artifact names, section order, and sort orders are fixed, so two scans of identical input produce structurally identical reports (modulo provenance timestamps).

## 2. Directory layout

Default output directory is `perfpal-report/` in the current working directory. A CLI flag may relocate the directory but never changes its internal layout.

```
perfpal-report/
├── metrics.json              # raw machine-readable data (schema v1.2)
├── metrics.schema.v1.2.json  # byte-copy of the exact schema for this schema_version
├── report.md                 # human-readable report
├── report.html               # static self-contained HTML sibling (Option B slice 1, LINKED variant — zero JavaScript)
└── charts/                   # static visualizations (set defined by ADR-0002)
    ├── <chart-name>.svg      # e.g. include-graph.svg — naming rules in §3
    ├── layer-workspace.svg   # components-as-nodes overview (Option A slice 2)
    ├── layer-file.svg        # top-files-by-degree overview (Option A slice 2)
    ├── component-<rank>-<slug>.svg  # per-component detail page (Option A slice 1)
    └── graph.dot             # deterministic DOT export of the graphs (v1.4 step 2)
```

Layout rules:

- **Atomic regeneration.** Perfpal builds the report in a temporary sibling directory, then swaps it into place on success via rename-aside-then-swap (POSIX `rename(2)` cannot overwrite a non-empty directory). A crashed or interrupted run leaves the previous complete report intact — never a half-written one; stale temp directories from crashed runs are removed on the next successful run.
- **Safety guard.** If the target directory exists and does not contain `metrics.json`, Perfpal refuses to replace it unless replacement is explicitly forced — user data is never silently destroyed.
- **Encoding.** All text artifacts are UTF-8, LF line endings, trailing newline.
- **Bundled-schema `$id`.** The `$id` inside the schema copy is the canonical URI, not the local filename — a mismatch between `$id` and filename is expected, not corruption.
- **Paths.** All file paths in any artifact are repo-relative POSIX paths exactly as emitted in `metrics.json`; `scan.root_path` itself is recorded verbatim as given to the CLI (may be absolute) and is rendered as provenance only. `report.md` and chart labels reuse these paths verbatim — no path rewriting. Paths, titles, and messages are escaped when embedded in Markdown so metacharacters in scanned filenames cannot inject links or formatting.

## 3. File naming & stability

| Artifact | Name | Stability rule |
|---|---|---|
| Raw data | `metrics.json` | Fixed name. |
| Schema copy | `metrics.schema.v<MAJOR>.<MINOR>.json` | Full version always — `schema_version: "1.2"` ⇒ `metrics.schema.v1.2.json` (no elided `.0`; avoids ambiguity between "v1" = 1.0 vs the 1.x series). The file is a byte-copy of the canonical schema artifact for that version, per the per-minor re-emission policy in [metrics-json-v1.md §5](../schema/metrics-json-v1.md). |
| Human report | `report.md` | Fixed name. |
| HTML report | `report.html` | Fixed name (Option B slice 1, LINKED variant). Static, self-contained, ZERO JavaScript (no `<script>`, no event handlers, no `javascript:` URLs); references `charts/*.svg` via relative `<img src>`/`<a href>` links so the HTML stays small on big repos while remaining fully offline within the directory. Header carries schema/file/finding counts only — no timestamps, durations, or absolute paths — so the file is byte-identical across runs UNMASKED. Contents: header, architecture figures mirroring the §5 chart links (overview links allowlisted to the five known stems — unknown stems never become links; every stem/filename escaped at the interpolation site; same exact-registry detail routing), component index with detail-page links, file drill-down (top-10 by degree desc/cost desc/path asc + finding_id lists, with a `…+N more (see findings table)` marker when capped), findings table (full §4 parity: `finding_id`/rule/severity/confidence/resolver/location/symbol/message/evidence/threshold/flags, `finding_id`-ascending; evidence keys and values escaped), Appendix A. Drill-down anchors link component → file → finding via deterministic collision-free `id` anchors (first colliding slug keeps the bare form, later uses get `-2`, `-3`, … in sorted order). All interpolated paths/titles/messages pass through the reporter's shared HTML escaper. Consumers tolerate its absence (pre-slice-1 directories); charts/`metrics.json` remain the data sources, never the HTML. |
| Charts | `charts/<name>.svg` | `<name>` matches `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case ASCII). No timestamps, counters, or input-derived suffixes — the same chart has the same name across runs so directories can be diffed. |
| Component pages | `charts/component-<rank>-<slug>.svg` | One page per capped component (same rank ≤ 50 + top-50 cap as the overview component charts), in `(rank, path_prefix)` order after the overviews. `<rank>` is the zero-padded component rank (`01`–`50`); `<slug>` is the lowercased `path_prefix` with every non-`[a-z0-9]` run collapsed to a single `-` (the `"."` root ⇒ `root`). Each page keeps the top-50 files by INTRA-component degree (edges with both endpoints in the component — cross-component edges never influence the ranking). A file-less component yields a deterministic stub page (`no files recorded`), never an omission. |
| Layered overviews | `charts/layer-workspace.svg`, `charts/layer-file.svg` | Option A slice 2 zoom levels, in emission order after the three v1 overviews and before the component pages. `layer-workspace.svg` shows capped components as nodes over aggregated inter-component call/include edges (heat = finding density per KLOC); emitted whenever `components` is present (deterministic `no components recorded` stub when empty). `layer-file.svg` shows top files by degree over the include DAG (heat = per-file function cost; the hot-path highlight outlines a SINGLE file — highest cost among the drawn files, ties → path-asc first, caption states the post-cap scope); emitted whenever `graphs` is present (deterministic `no files recorded` stub when the node list is empty). Same top-50 caps, same byte-determinism bar as every other chart. Honest coverage note: the per-component pages draw intra-component file edges only and the workspace draws aggregates, so a cross-component FILE edge is currently visible ONLY as an aggregated component-level edge — in neither the workspace nor any detail page. The middle "files-under-component-bands" layer that would draw those edges across bands was deliberately NOT built in this slice; it is recorded as an explicit follow-up third layer, never a silent reshape of these two. |
| Graph export | `charts/graph.dot` | Fixed name (v1.4 step 2). Deterministic DOT rendering of `graphs.*` + `components[]` (same input set as the SVG charts); always emitted, even on empty scans (an empty digraph). |

Chart-set rules (binding; chart set defined by [ADR-0002](../adr/ADR-0002-visualization.md)):

- The **set** of charts is owned by [ADR-0002](../adr/ADR-0002-visualization.md); this contract fixes only naming and stability semantics.
- Chart presence is **best-effort**: the reporter always creates `charts/` (possibly empty on trivial scans); consumers tolerate its absence, and absence is never an error condition. Chart *content* must satisfy Principle 5 too — no embedded timestamps or non-deterministic layout (ADR-0002 owns how).
- Consumers must **tolerate unknown files** under `charts/` (forward compatibility: minor bumps may add charts).

## 4. Versioning & stability rules

The directory inherits the [metrics.json versioning policy](../schema/metrics-json-v1.md#5-versioning--stability-policy) wholesale:

- **Minor bumps (v1.x): additive only.** May add optional sections to `report.md`, add charts, add optional `metrics.json` fields. Old consumers keep working; they may simply miss new information.
- **Major bumps (v2+): breaking.** Renaming/removing files, changing required `report.md` section semantics, changing chart-name meanings, or any closed-enum change in `metrics.json`.
- **Consumer obligations:**
  - Treat unknown files anywhere in the directory as ignorable, not errors.
  - Treat `charts/` as advisory visualization; never parse data out of SVGs — use `metrics.json`.
  - Pin the exact minor schema when strict-validating (see strictness caveat in [metrics-json-v1.md §5](../schema/metrics-json-v1.md)).
  - Discover the bundled schema by mapping `schema_version` to its filename (`"1.2"` ⇒ `metrics.schema.v1.2.json`).
- **Consumption entry points:** humans start at `report.md`; LLMs/tooling start at `metrics.json` (raw), using `report.md` as a guided tour. Both paths must lead to consistent conclusions because of Principle 2.

## 5. `report.md` template outline

The report is generated from `metrics.json` plus the reporter's built-in heuristic catalog (frozen for v1; the catalog supplies only the Appendix A descriptions). Section order is fixed; every number and finding is traceable to a `metrics.json` field — the report never invents or aggregates beyond what is stated here.

```markdown
# Perfpal Report

> schema_version <schema_version> · perfpal <tool.version> · scanned <scan.root_path>
> started <scan.started_at> · duration <scan.duration_ms> ms · <scan.file_count> files

## 1. Executive summary
One paragraph: total findings by severity (hot/warn/info), parse health
one-liner (ok/partial/failed counts from parse_health_summary), the first 3
hot findings in the §4 sort order (all if fewer than 3), by title + finding_id.

## 2. Scan provenance & parse health
Tool name/version, scan parameters as recorded in `scan`. Parse-health
summary table (ok / partial / failed counts) plus a detail table of
failed/partial files with error_node_count. Framing note: findings
overlapping error regions are flagged (error_node_overlap), not silently
dropped.

## 3. Codebase statistics
Per-language totals (languages, loc_total). Top 10 files by code LOC,
descending, ties broken by path (table: path, loc.code/comment/blank,
function_count). Function-metric
distributions from stats.distributions (complexity buckets, loop depth,
function length) rendered as compact tables.

## 4. Findings
Grouped by severity: hot → warn → info. Within a group sorted by
confidence (HIGH → MEDIUM → LOW), then rule_id, then path, then
span.start_line, then span.start_col — a total order. When `findings`
is empty, this section renders a single line: "No findings." Each
finding renders as:

#### <title>  (`finding_id`)
- Location: `path:start_line` (symbol, if present)
- Message: <message verbatim>
- Confidence / severity / resolver_tier
- Flags: macro_suspect, error_node_overlap (only when true)
- Evidence: key/value list from evidence (only when present)

## 5. Architecture
Embedded charts via relative links (charts/<name>.svg), each with a
one-line caption. Overview links render in chart-emission order
(include-graph, layer-file, component-coupling, hot-components,
layer-workspace). Component ranking table from components[]:
component_id, path_prefix, rank, finding_density. Component detail index:
one link per capped component whose `charts/component-<rank>-<slug>.svg`
page was emitted (`[Component <prefix> — per-file hot-path
map.](charts/component-<rank>-<slug>.svg)`), in rank order directly under
the table. Include-graph and
call-graph notes, with the standing caveat that cross-file call edges
are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
Table: rule_id → title → one-line description, covering every rule_id
present in this report's findings.

## Appendix B. How to read this report
Confidence/severity semantics (catalog §1, §6); optimization-candidate
framing — findings are candidates for review, never certainties;
pointer to metrics.json + bundled schema for programmatic consumption.
```

Determinism rules:

- Ordering above is total and depends only on `metrics.json` content — same input ⇒ same section content byte-for-byte, except provenance values (`started_at`, `duration_ms`) which legitimately vary between runs. This presupposes deterministic `files[]` ordering in `metrics.json` (scanner responsibility: emit sorted by path).
- Fields that are optional in the schema (`duration_ms`, `loc_total`, `parse_health_summary`, `stats.distributions`, `graphs`, `components`) render as `n/a` inline or cause their clause/table to be omitted; the report remains well-formed on a trivial scan.
- Findings are cited by their exact `finding_id` strings so consumers can cross-reference `report.md` ↔ `metrics.json` mechanically ([identity format](../schema/metrics-json-v1.md#4-finding-identity)).

## 6. Deliberate non-goals in v1

- No HTML/PDF/terminal rendering (post-MVP; terminal-native charts already deprioritized in the spec).
- No diff/history mode comparing two report directories (post-MVP).
- No compression/archiving (`.tar.gz` bundles) — plain directory only.
- No CLI surface definition here (stdout text, exit codes, flag names) — owned by the module architecture doc (step 6).
- No i18n — all human-readable strings are English in v1.
