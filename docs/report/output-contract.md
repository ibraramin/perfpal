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
├── metrics.json              # raw machine-readable data (schema v1.7)
├── metrics.schema.v1.7.json  # byte-copy of the exact schema for this schema_version
├── report.md                 # human-readable report (bounded: groups + links only, zero per-finding rows)
├── report.html               # static self-contained HTML sibling (Atlas map + Explorer; one fixed <script>; groups only)
├── findings-detail.md        # uncapped per-finding Markdown detail + ranked action list (C-split companion to report.md §4/Appendix C)
├── findings-detail.html      # uncapped per-finding HTML tables + ranked action list (companion to report.html #findings)
├── profile-queue.md          # complete cost-ranked function list (KV3 drill companion)
├── profile-queue.html        # complete cost-ranked function list (KV3 drill companion)
├── atlas-weight.html         # weight drill page (Atlas step 12)
├── atlas-q2.html             # blast-radius drill page (Atlas step 12)
├── agent-report.md           # bounded LLM-facing triage report (contract_version 1, seven fixed sections, ≤80 KB on the §12 envelope)
├── agent-actions.json        # complete machine action list (contract_version 1, 100-row profile cap named in-file)
└── charts/                   # static visualizations (set defined by ADR-0002)
    ├── <chart-name>.svg      # e.g. include-graph.svg — naming rules in §3
    ├── layer-workspace.svg   # components-as-nodes overview (Option A slice 2)
    ├── layer-file.svg        # top-files-by-degree overview (Option A slice 2)
    ├── layer-entry.svg       # entry-stratified cost map (map-build I3)
    ├── layer-bands.svg       # files-under-component-bands overview (map-build I3)
    ├── layer-findings.svg    # severity-weighted findings map (map-build I4)
    ├── component-<rank>-<slug>.svg  # per-component detail page (Option A slice 1)
    └── graph.dot             # deterministic DOT export of the graphs (v1.4 step 2)
```

Layout rules:

- **Atomic regeneration.** Perfpal builds the report in a temporary sibling directory, then swaps it into place on success via rename-aside-then-swap (POSIX `rename(2)` cannot overwrite a non-empty directory). A crashed or interrupted run leaves the previous complete report intact — never a half-written one; stale temp directories from crashed runs are removed on the next successful run.
- **Safety guard.** If the target directory exists and does not contain `metrics.json`, Perfpal refuses to replace it unless replacement is explicitly forced — user data is never silently destroyed.
- **Encoding.** All text artifacts are UTF-8, LF line endings, trailing newline.
- **Bundled-schema `$id`.** The `$id` inside the schema copy is the canonical URI, not the local filename — a mismatch between `$id` and filename is expected, not corruption.
- **Paths.** All file paths in any artifact are repo-relative POSIX paths exactly as emitted in `metrics.json`; `scan.root_path` itself is recorded verbatim as given to the CLI (may be absolute) and is rendered as provenance only. `report.md` and chart labels reuse these paths verbatim — no path rewriting. Paths, titles, and messages are escaped when embedded in Markdown so metacharacters in scanned filenames cannot inject links or formatting.
- **Atlas page.** `report.html#map` is the shipped human map (Atlas): a key-metrics strip above a graph-only entry-flow canvas, a right-hand detail rail, then four designed below-graph charts (weight, blast radius, file detail, coverage/links). The page shape is state-unconditional; the canvas draws the default-tier **structural** call graph (`LOW name-based + MEDIUM semantic`), never a runtime claim. The page links `atlas-weight.html`, `atlas-q2.html`, the complete action/profile drills, and the LLM-facing `agent-report.md` / `agent-actions.json`. Contract: `docs/specs/atlas-layout.md`, `docs/specs/atlas-metrics.md`, `docs/specs/atlas-feature.md`.

## 3. File naming & stability

| Artifact | Name | Stability rule |
|---|---|---|
| Raw data | `metrics.json` | Fixed name. |
| Schema copy | `metrics.schema.v<MAJOR>.<MINOR>.json` | Full version always — `schema_version: "1.7"` ⇒ `metrics.schema.v1.7.json` (no elided `.0`; avoids ambiguity between "v1" = 1.0 vs the 1.x series). The file is a byte-copy of the canonical schema artifact for that version, per the per-minor re-emission policy in [metrics-json-v1.md §5](../schema/metrics-json-v1.md). |
| Human report | `report.md` | Fixed name. §1 Executive summary is the key-view brief: at most 5 KV1 rows, 5 KV2 rows and 5 KV3 rows (the explicit bounded-human exception), each row carrying the same fields as the HTML key views and linking the complete drill artifacts. |
| HTML report | `report.html` | Fixed name (Option B slice 2). Static, self-contained, LINKED variant plus exactly ONE inline `<script>` block with a FIXED hand-rolled source (no libraries, no vendoring, under 60 lines — byte-identical every render, so the file stays byte-identical across runs UNMASKED): pan/zoom over the `layer-workspace`/`layer-file`/`layer-entry`/`layer-bands`/`layer-findings` figures (drag + wheel + buttons via CSS transforms on the existing linked `<img>` — SVGs never inlined, no data duplicated for JS) and click-to-filter on the findings table (component/file row clicks hide non-matching rows via `hidden` over already-rendered DOM `data-*` keys, with a visible `filter: X — clear` reset control; no re-fetch, no network). Safety: no `Date`/`random`/`fetch`/XHR/`eval`/`Function(`/string-arg timers/`innerHTML`/`outerHTML`/`insertAdjacentHTML`/`document.write` (interpolated strings pass through the shared escaper at RENDER time; the script only reads `data-*` and writes `textContent`/`hidden`/style), no inline event-handler attributes (all binding via `addEventListener`), row clicks originating inside an inner `a[href]` never filter (bubbling guard — following a link never also filters), fully usable with JS disabled (all rows/figures present statically — the script only enhances), and fully offline (no external refs). Header carries schema/file/finding counts only — no timestamps, durations, or absolute paths. Contents: header, breadcrumb (`#map` → `#components` → `#files` → findings detail), a `#map` Atlas section (key-metrics strip + graph-only entry-flow canvas + detail rail, then the weight, blast-radius, file-detail and coverage/links charts; the Atlas canvas is inline SVG with no `<script>`/event handlers/network; drill links to `atlas-weight.html` / `atlas-q2.html` and complete-list links into `findings-detail.html#ranked-actions` and `profile-queue.html`), `Explorer` figures mirroring the §5 chart links (overview links allowlisted to the eight known stems — unknown stems never become links; every stem/filename escaped at the interpolation site; same exact-registry detail routing; layer figures carry the `figure.pz` pan/zoom hook), component index (top-10 with an honest `showing top <S> of <M> components` note) with detail-page links (rows carry `data-prefix` click keys), file drill-down (top-10 files by degree desc/cost desc/path asc + top-10 finding_id links per file by severity-rank → `finding_id` into `findings-detail.html`, rows carry `data-path` click keys, with a `…+N more (see findings table)` marker when files are capped and a `…+N more` marker when a file's links are capped), findings section (C-split group-and-fold: per-rule `<details><summary>` groups ONLY — zero per-finding rows by construction; each summary carries rule + count + hot/warn/info split + top-exemplar link into `findings-detail.html` plus a per-group full-detail link; honest `<R> rules: <N> of <M> total findings` header plus a `findings-detail.html` pointer; drill-down `finding_id` lists link cross-page (`findings-detail.html#…`); filter bar with `all findings` label + `clear` reset; fully usable with JS disabled — every group present statically), Appendix A. Drill-down anchors link component → file → finding via deterministic collision-free `id` anchors (first colliding slug keeps the bare form, later uses get `-2`, `-3`, … in sorted order). All interpolated paths/titles/messages pass through the reporter's shared HTML escaper. Consumers tolerate its absence (pre-slice-1 directories); charts/`metrics.json` remain the data sources, never the HTML. |
| Findings detail (Markdown) | `findings-detail.md` | Fixed name (C-split). The uncapped per-finding detail promoted out of `report.md` §4/Appendix C so the human file stays bounded at any scale: `# Perfpal Findings Detail` title, honest `<N> of <M> total findings.` header, a new top `## Complete ranked action list (KV1 order)` section anchored `ranked-actions` (one row per function-granularity finding in KV1 order: rank, finding_id link, title, path:line, function, severity, confidence, path class, cost anchor), then every finding in full in the legacy total order (stable severity partition hot → warn → info; document order within a group) via the preserved full-detail renderer — same rows the pre-split Appendix C carried, byte-identical content. No provenance surfaces (`started_at`/`duration_ms`/`root_path` never render), so the file is byte-identical across runs UNMASKED. `report.md` Appendix C links here; when `findings` is empty this file renders `No findings.` |
| Findings detail (HTML) | `findings-detail.html` | Fixed name (C-split). The uncapped per-finding tables promoted out of `report.html` #findings: a standalone page reusing the report head/style/table markup plus the same single fixed `<script>` block, with a top `id="ranked-actions"` section carrying the complete ranked action list (same KV1 rows as the Markdown artifact) followed by one `rule-<rule_id>` section per firing rule (pinned group order) each carrying that rule's rows in the shared full-table markup (`finding_id`/rule/severity/confidence/resolver/location/symbol/message/evidence/threshold/flags, members `finding_id`-ascending; rows carry `data-path` filter keys; filter bar + honest header reused so click-to-filter keeps working). Finding-row `id` anchors come from the same assignment as `report.html`, so every `findings-detail.html#…` link there resolves here. No provenance surfaces — byte-identical across runs UNMASKED. Consumers tolerate its absence (pre-split directories). |
| Profile queue (Markdown) | `profile-queue.md` | Fixed name (V4, map-intelligibility). Standalone drill companion to KV3: `# Perfpal Profile Queue` title, header `N functions ranked by static cost — candidates, confirm with a profiler`, then the complete deterministic cost-ranked function list (production first → loop-bearing before no-loop → non-startup before startup → cost desc → path → name → id). Every function is listed — startup (`entry_kind` main/script), no-loop (`max_loop_depth == 0`) and non-production rows are labelled, never excluded. Each row carries `path:line`, path class, a cost unit (`static cost units (v2)` or `static cost units (v1)`, so v1 rows are marked `(v1)`), anchors, and the unknown/known trip badges. No provenance surfaces — byte-identical across runs UNMASKED. |
| Profile queue (HTML) | `profile-queue.html` | Fixed name (V4). Standalone page mirroring `findings-detail.html`'s head/style/nav/fixed-script structure, titled `Perfpal Profile Queue`, with one complete cost-ranked table (same rows/units/anchors/badges as the Markdown artifact). No provenance surfaces — byte-identical across runs UNMASKED. |
| Atlas weight drill | `atlas-weight.html` | Fixed name (Atlas step 12). Standalone warm-paper page (breadcrumb back to `report.html#map`) titled `Perfpal Atlas — weight detail`: the complete weight detail — every source file and component by `% of scanned static cost`, production-first ordering with a labelled per-row class column, and the per-file anchors the rail/weight chart link to. No script, no network; no provenance surfaces — byte-identical across runs. |
| Atlas Q2 drill | `atlas-q2.html` | Fixed name (Atlas step 12). Standalone warm-paper page (breadcrumb back to `report.html#map`) titled `Perfpal Atlas — Q2 reverse drill`: the focus-file blast radius (resolved include dependents and include coverage), component DSM, structural caller/callee counts and include coverage, each with its denominator and the mandatory structural label. No script, no network; no provenance surfaces — byte-identical across runs. |
| Agent report (Markdown) | `agent-report.md` | Fixed name (Track C, `contract_version 1`; spec [`agent-report.md`](../specs/agent-report.md)). Bounded LLM-facing triage layer: a fixed seven-section skeleton (header, how-to-use/rank key, data quality, ranked actions top 25, per-rule rollup, profile queue top 25, limits), every cap printed as `showing top N of M`, named stubs in every empty state, ≤ 80 KB on the named fixtures (spec §12 envelope; wider inputs render in full and may exceed it). Ranked by the shipped KV1 key (no second ranking); advisory framing — optimization candidates, not confirmed bottlenecks. Clock provenance (`started_at`/`duration_ms`) is never rendered, so repeated runs are byte-identical; `root_path` is verbatim CLI provenance. |
| Agent actions (JSON) | `agent-actions.json` | Fixed name (Track C, `contract_version 1`). The **complete** machine action list (`actions.len() == complete_action_count`, ranks 1..N contiguous) plus the profile rows (spec §6.2) capped at 100 with the cap named in-file, the data-quality block, and per-action `fix_hint`/`detail_anchor`. Pure function of `metrics.json`; same KV1/KV3 order as the human artifacts; no network, no JS, no schema change. |
| Charts | `charts/<name>.svg` | `<name>` matches `[a-z0-9]+(-[a-z0-9]+)*` (kebab-case ASCII). No timestamps, counters, or input-derived suffixes — the same chart has the same name across runs so directories can be diffed. |
| Component pages | `charts/component-<rank>-<slug>.svg` | One page per capped component (same rank ≤ 50 + top-50 cap as the overview component charts), in `(rank, path_prefix)` order after the overviews. `<rank>` is the zero-padded component rank (`01`–`50`); `<slug>` is the lowercased `path_prefix` with every non-`[a-z0-9]` run collapsed to a single `-` (the `"."` root ⇒ `root`). Each page keeps the top-50 files by INTRA-component degree (edges with both endpoints in the component — cross-component edges never influence the ranking). A file-less component yields a deterministic stub page (`no files recorded`), never an omission. |
| Layered overviews | `charts/layer-workspace.svg`, `charts/layer-file.svg` | Option A slice 2 zoom levels. REAL emission order is include-graph, layer-file, layer-entry, component-coupling, hot-components, layer-workspace, layer-bands, layer-findings, then the per-component detail pages. `layer-workspace.svg` shows capped components as nodes over aggregated inter-component call/include edges (heat = candidate finding density per KLOC); emitted whenever `components` is present (deterministic `no components recorded` stub when empty). `layer-file.svg` shows top files by degree over the include DAG WITH a reachable reserve (heat = candidate per-file function cost, v2 where available with v1 fallback; the hot-path highlight outlines a SINGLE file — highest cost among the drawn files, ties → path-asc first, caption states the post-cap scope); emitted whenever `graphs` is present (deterministic `no files recorded` stub when the node list is empty). Same top-50 caps, same byte-determinism bar as every other chart. |
| Entry layer | `charts/layer-entry.svg` | Entry-stratified cost map (map-build I3): include-graph nodes capped entry-first (REAL entry files — `unknown-entry` stubs excluded — by cost desc/path asc, non-entry files by non-LOW call degree desc/path asc, then up to 10 reserve slots for files reachable in the BFS distance map), laid out in BFS-distance columns from real resolved entry points (`entry`, `depth 1`…, present distances only). Depth is capped at 8: all deeper files fold into one `depth ≥ 9` column captioned `N files at depth ≥ 9 folded into one column`. Shown files no entry can reach land in a final `not reached` region sub-layered by `topo_layers`. Heat = candidate per-file function cost (v2 where available; v1 fallback), normalized over shown files; a SINGLE hottest shown entry-reachable file is outlined. LOW-confidence call edges are excluded from the strata and counted; `unknown-entry` stub files are captioned (`N unknown-entry stub files (no entry detected; shown as not reached)`). The resolved-edge caption reports the full graph count and the shown subset (`N resolved call edges (M among shown) · E entry file(s)`), so a strata path through a capped-out file is never misreported as zero. A scan with no REAL entry points falls back to a plain topo call-DAG (captioned) instead of a blank. Emitted whenever `graphs` is present (deterministic `no files recorded` stub when the node list is empty). |
| Component bands | `charts/layer-bands.svg` | Middle bands (map-build I3, A-pf-077): capped components (`rank ≤ 50`, top 50 by `(rank, path_prefix)`) as vertical bands, each holding its member files (union of include-graph nodes and `files[].path` mapped by `parent_dir`) in one horizontal row ordered cost desc/path asc, capped at 10 per band with `…+N more files in shown bands`. Intra-band include/resolved-call file edges are drawn under the nodes (deduped and cycle-broken via the shared policy; `N cyclic intra-band edge(s) broken` captioned); cross-band file edges are AGGREGATED into one counted ribbon per ordered band pair in a right-hand gutter (stroke width `1 + min(count,5)`). LOW-confidence call edges are excluded and counted; cross-component file edges whose endpoints fall outside the shown file/component scope are counted (`…+N more cross-component edges outside shown scope`). Heat = candidate per-file function cost (v2 where available; v1 fallback), normalized over SHOWN files. A file-less component renders a `no files recorded` band, never an omission. Emitted whenever `components` is present (deterministic `no components recorded` stub when empty). |
| Findings map | `charts/layer-findings.svg` | Findings-as-annotations (map-build I4): include-graph nodes ∪ file paths carrying findings, capped at 50 by `(weighted desc, path asc)` with `weighted = 3*hot + 2*warn + 1*info` (no finding is excluded from the counts; the cap only selects SHOWN files, captioned `showing top <S> of <M> files by weighted findings (…+N more)`). Same DAG pipeline as `layer-file.svg` (break cycles → topo layers → 3-pass barycenter); heat = node weight / max shown weight (0..10) with the documented `hot 3 / warn 2 / info 1 per candidate` subtitle; every node carries its finding dot (severity fill + confidence ring, counts in the `<title>`), and the single hottest shown file is outlined. Zero findings yields the deterministic `no findings recorded — nothing to annotate (heuristic candidates only)` stub. Emitted whenever `graphs` is present (after `layer-bands`, before the per-component pages). |
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
  - Discover the bundled schema by mapping `schema_version` to its filename (`"1.7"` ⇒ `metrics.schema.v1.7.json`).
- **Consumption entry points:** humans start at `report.html#map` (the Atlas page: key-metrics strip + graph + rail, then the four designed charts) or `report.md` (bounded groups + links); depth is one click away in `atlas-weight.html`, `atlas-q2.html`, `findings-detail.md` / `findings-detail.html` and `profile-queue.md` / `profile-queue.html`. LLMs/tooling start at `agent-report.md` (bounded, ranked, fix hints) or `agent-actions.json` (complete machine list), with `metrics.json` (raw) as the source of truth and `report.md` as a guided tour. Both paths must lead to consistent conclusions because of Principle 2.
- **Bounded-human rule (C-split):** `report.md` and `report.html` carry ZERO per-finding rows at any scale — one group row per firing rule plus links into the detail artifacts. **Exception (report.md V4 key views):** a fixed ≤5 key-view rows per question (KV1 brief, KV2 bars, KV3 bars) is explicitly allowed — a bounded answer-first block, not a per-finding dump; uncapped per-finding rows remain forbidden. `-ARCH-002` hot-component aggregates are suppressed from KV1's action ranking and counted as "component-level finding(s) in Explorer" (`ARCH-001`/`ARCH-003` are file-level actions and stay ranked). The two remaining unbounded human-report sections are capped the same way (C-full-split follow-up, same `take(10)` precedent as the §3 top-files table and the `#files` drill-down): the §5 component table / Component index shows the top 10 in rank order with an honest `showing top <S> of <M> components` header/note, and each file drill-down row links at most the top 10 findings by severity-rank → `finding_id` with an honest `…+N more` marker. Full component and finding data stays in `metrics.json` (machine source of truth) and the uncapped `findings-detail.*` artifacts. The artifact SET per scan is therefore the 10 fixed files (`metrics.json`, bundled schema, `report.md`, `report.html`, `findings-detail.md`, `findings-detail.html`, `profile-queue.md`, `profile-queue.html`, `agent-report.md`, `agent-actions.json`) plus the two Atlas drill pages (`atlas-weight.html`, `atlas-q2.html`), `charts/graph.dot` and the chart set (3 v1 overviews + 5 layered/map overviews + one `component-<rank>-<slug>.svg` page per capped component).

## 5. `report.md` template outline

The report is generated from `metrics.json` plus the reporter's built-in heuristic catalog (frozen for v1; the catalog supplies only the Appendix A descriptions). Section order is fixed; every number and finding is traceable to a `metrics.json` field — the report never invents or aggregates beyond what is stated here.

```markdown
# Perfpal Report

> schema_version <schema_version> · perfpal <tool.version> · scanned <scan.root_path>
> started <scan.started_at> · duration <scan.duration_ms> ms · <scan.file_count> files

## 1. Executive summary
One paragraph: total findings by severity (hot/warn/info) and the parse-health
one-liner (ok/partial/failed counts from parse_health_summary). Then the
answer-first key-view brief: KV1 "Where do I look first?" (≤5 ranked actions
with path:line, function, rule title, severity/confidence, path class, full
rationale, and cost unit+anchor when a containing function exists), a link to
the complete ranked list in `findings-detail.md#ranked-actions`, then compact
KV2 "What breaks if I touch it?" (≤5 bars) and KV3 "Where might time plausibly
go?" (≤5 cost rows with a link to `profile-queue.md`). Component-level
`-ARCH-002` hot-component aggregates are excluded from the KV1 action ranking and reported as
a count in Explorer.

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
Group-and-fold summary: ONE row per firing rule plus an honest header
line `<R> rules: <N> of <M> total findings (<h> hot, <w> warn, <i> info).`
`N` always equals `M` today — every finding is represented via its rule
group — so a future row cap cannot silently drop findings without breaking
the header. Row columns: rule_id, finding count, hot/warn/info split, and
the top exemplar (the first finding in `finding_id` order, cited by exact
`finding_id` + `path:start_line:start_col` — the unambiguous three-part
location, never a range — so it cross-references `findings-detail.md`
mechanically). When `findings` is empty, this section renders a single
line: "No findings." The uncapped per-finding detail lives in the
`findings-detail.md` artifact (C-split) — §4 carries zero `####` rows at any
scale.

Group order (binding, deterministic): severity-rank ascending — the hottest
severity present in the group first (`hot` 0 → `warn` 1 → `info` 2) — then
member count descending, then `rule_id` ascending. Members within a group
are in `finding_id`-ascending order regardless of analyzer emission order,
so exemplars are deterministic.

## 5. Explorer
Embedded charts via relative links (charts/<name>.svg), each with a
one-line caption. The eight overview stems render under the `Explorer`
heading (`#explorer`, formerly `Architecture`); the answer-first key views
live in `report.html#map`. Overview links render in chart-emission order
(include-graph, layer-file, layer-entry, component-coupling,
hot-components, layer-workspace, layer-bands, layer-findings). Component ranking table from
components[] (top 10 in
document/rank order with an honest `showing top <S> of <M> components`
header — full component data stays in `metrics.json`):
component_id, path_prefix, rank, finding_density. Component detail index:
one link per SHOWN (top-10) component whose `charts/component-<rank>-<slug>.svg`
page was emitted (`[Component <prefix> — per-file hot-path
map.](charts/component-<rank>-<slug>.svg)`), in rank order directly under
the table. Include-graph and
call-graph notes, with the standing caveat that cross-file call edges
are LOW confidence (name-based) unless the run used the semantic resolver
tier, in which case resolved edges are MEDIUM and only the unresolved
name-based edges stay LOW (ADR-0001).

## Appendix A. Heuristic reference
Table: rule_id → title → one-line description, covering every rule_id
present in this report's findings.

## Appendix B. How to read this report
Confidence/severity semantics (catalog §1, §6); optimization-candidate
framing — findings are candidates for review, never certainties;
pointer to metrics.json + bundled schema for programmatic consumption.

## Appendix C. All findings (full detail)
Link stub for the C-split detail artifact: an honest `<N> of <M> total
findings.` header (always agreeing with §4 today) plus a pointer —
`Full per-finding detail has moved to
[findings-detail.md](findings-detail.md) (<R> rules, same <N> of <M>
total).` Zero `####` rows render here at any scale; the uncapped
per-finding detail the §4 summary folds lives in `findings-detail.md`:

#### <title>  (`finding_id`)
- Location: `path:start_line` (symbol, if present)
- Message: <message verbatim>
- Confidence / severity / resolver_tier
- Flags: macro_suspect, error_node_overlap (only when true)
- Evidence: key/value list from evidence (only when present)

in the legacy total order (stable severity partition hot → warn → info;
document order within a group), rendered by the preserved full-detail
renderer behind its own entry point. When `findings` is empty, this section
(and the detail artifact) renders a single line: "No findings."
```

Determinism rules:

- Ordering above is total and depends only on `metrics.json` content — same input ⇒ same section content byte-for-byte, except provenance values (`started_at`, `duration_ms`) which legitimately vary between runs. This presupposes deterministic `files[]` ordering in `metrics.json` (scanner responsibility: emit sorted by path). Finding-group order (§4 rows, `#findings` groups, detail-artifact groups) never depends on analyzer emission order: groups sort by severity-rank → count-desc → `rule_id`-asc and members by `finding_id`-asc. The detail artifacts carry no provenance surfaces at all, so they are byte-identical across runs UNMASKED (same bar as `report.html` and the charts).
- Fields that are optional in the schema (`duration_ms`, `loc_total`, `parse_health_summary`, `stats.distributions`, `graphs`, `components`) render as `n/a` inline or cause their clause/table to be omitted; the report remains well-formed on a trivial scan.
- Findings are cited by their exact `finding_id` strings so consumers can cross-reference `report.md` ↔ `metrics.json` mechanically ([identity format](../schema/metrics-json-v1.md#4-finding-identity)).

## 6. Deliberate non-goals in v1

- No HTML/PDF/terminal rendering (post-MVP; terminal-native charts already deprioritized in the spec).
- No diff/history mode comparing two report directories (post-MVP).
- No compression/archiving (`.tar.gz` bundles) — plain directory only.
- No CLI surface definition here (stdout text, exit codes, flag names) — owned by the module architecture doc (step 6).
- No i18n — all human-readable strings are English in v1.
