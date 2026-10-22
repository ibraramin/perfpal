# Atlas — data-gap audit

**Status:** research input for `map-shipped` and the Atlas build plan. **No implementation.**
**Date:** 2026-10-23 · **Schema audited:** v1.7 (`docs/schema/metrics.schema.v1.7.json`).
**Sources:** `src/`, the V7 G/F prototypes (`docs/evidence/map-vision/v7/{g,f}/render_*.py`),
`docs/schema/metrics-json-v1.md`, `docs/evidence/map-data-coverage.md`.
**Held out:** django/postgres scans stay vaulted; no holdout data is used here.

## 1. Method

For every Atlas element the question is: **does the shipped schema already carry the fact, can it
be computed deterministically chart-side from `metrics.json`, or is it genuinely missing?** A
missing fact is only added if it is additive, optional on the wire and required by a pre-registered
need (N1–N6 in `docs/research/atlas-developer-needs.md`). Everything else is downscoped.

Result in one line: **Atlas v1 needs no schema change.** All blocks are chart-side derivations
from existing fields. One fact is genuinely missing (a general I/O-sink classification) and is
**downscoped**; five additive candidates are recorded as future options with explicit triggers.

## 2. Element inventory

| # | Element | Existing field(s) | Computed at | V7 G derivation | Gap / policy risk | Decision |
|---|---|---|---|---|---|---|
| 1 | **Entry** | `stats.functions[].entry_kind`, `entry_confidence`, `reachable_from_entry` | `src/aggregator/mod.rs:628` (`detect_c_entries`; main 648, handler 656, lib-root 670, stub 677); `:822` Rust; `:995` Python; assigned `:257–264` | `choose_entry` `render_g.py:159–201`: production `main` first (`path_class == production` + `is_program_path`), then production `script` (Python `__main__`), else highest-fan-in production `lib-root`; ties cost↓/fan-in↓/path/name; alternatives within 20% fan-in, max 5 | Schema emits **all** entries (including test mains); `reachable_from_entry` is a union over all entries. G elects **one** entry and does not use the `unknown-entry` stub; it raises `SystemExit` when none is eligible (`:183`). `is_program_path` checks only the first path segment (`:98`), unlike `path_class` (any segment, `src/aggregator/path_class.rs:68`). | **Chart-side election, no schema change.** Define single-entry election (main, script, lib-root) + alternatives; never abort — emit a `NO ENTRY` stub. Disclose `N entries` when >1. |
| 2 | **Call edges** | `graphs.call_graph.edges[]`: `from_function`, `to_name`, `from_path`, `to_path` (nullable), `confidence` (LOW/MEDIUM/HIGH) | `src/analyzer/heuristics/arch.rs:583` (`cross_file_call_edges`); dedupe earliest span `:641–680`; resolved=MEDIUM `:697`; name=LOW `:718`; uniqueness gate `:659`; sort `:721–726`; re-sort `src/aggregator/mod.rs:1209–1219`. **No cap in the schema.** | `load_repo` `render_g.py:123–127` aggregates into `(from_path, to_path)` pairs, filters to scanned files and `src != dst`; edge width `log2(count+1)`; "back-edge" = target column ≤ source column (`:491–497`) | Cross-file only (`schema:222`) — intra-file flow is invisible. G ignores `confidence`, mixing LOW name-based with MEDIUM semantic edges; shipped `render_layer_entry` excludes LOW (`src/charts/layers.rs:610–618`) and captions them. Pair "count" = distinct caller-fn→callee-name→target-file edges, **not raw call sites**. "Back-edge" is layout depth, not a graph cycle. | **Chart-side.** Draw the structural edge set with an explicit confidence mix (`N MEDIUM / M LOW`), never call it "resolved". Compute reachability on the same set and label it **estimated structural reachability**. Do not reuse the schema label `reachable_from_entry` (resolved-only) for the canvas. |
| 3 | **File nodes** | `files[].path`, `function_count`, `path_class`; `stats.functions[].path` | `files` `src/aggregator/mod.rs:84–98`; `path_class` `src/aggregator/path_class.rs:63–87`; resolved reachability `entry_reachable` `:754–773` | `source_files` = `function_count > 0` (`render_g.py:154`); file cost = Σ `function_cost_v2` (`:112–118`); reachability = BFS over file pairs from **all** call edges (`:204–218`); `NODE_CAP=20`, `EDGE_CAP=55` (`:32–33`); selection entry+path then cost/depth (`:281–286`); "0 reachable aggregated" = `depth − selected` empty (`:296`) | G's file reachability is broader than schema `reachable_from_entry` (LOW included; schema excludes LOW). BFS depth uncapped; only node selection capped. Production/test/bench via `path_class` (`:523–525`) — consistent with the shipped classifier. | **Chart-side BFS** with the disclosed structural set. Keep caps chart-side; print `N of M files shown`, `N aggregated`, `N unreached production`, `N unreached non-production`. |
| 4 | **Function cards** | `function_cost_v2`, `call_fanin`, `name`, `id`; `findings.rule_id`, `severity`, `confidence` | `function_cost_v2` `src/aggregator/mod.rs:322–354`; fan-in `src/aggregator/mod.rs:279`; stats `:229–308` | Sort per file cost↓, fan-in↓, name (`render_g.py:119–121`); show 1–3 full names under wrap budget (`:753–761`); `top P%` = `ceil(100·rank/N)` over cost↓/fan-in↓/path/name/id (`:275–277`); dominant rule by count then max severity (`:248–261`) | **"Percentage share" does not exist** in the card — G shows percentile rank, not % of total cost. Median = Python `statistics.median` over **all** functions including zeros (`:269`), repo-wide; for even N it averages the two middle values, whereas shipped Rust `median` is nearest-rank `ceil(0.5·N)`-th smallest (lower-middle for even N) (`src/reporter/views.rs:191–198`) — **anchor drift** between prototype and product. | **Chart-side with the shipped `ScaleOrdering`** (`src/reporter/views.rs:288–331`) so report and Atlas agree. If true share is wanted in the weight strip, compute it chart-side (Σ repo cost exists). Do not call percentile "share". |
| 5 | **Component hulls** | `components[]`: `component_id`, `path_prefix`, `rank`, `metrics`, `finding_count` | `components` `src/aggregator/mod.rs:1291–1356` (one per `parent_dir` `:1294–1309`; rank `:1335–1354`) | **G ignores `components[]`.** `domain_of` (`render_g.py:238–245`): `src/<2 segments>`, else first segment, else filename-stem family; hull groups selected boxes (`:428–467`), requires ≥2 members; no production filter in membership | `components[]` are per-directory and test-dominated/root-only (`g/NOTES.md:1249–1251,1263–1265`); they are **not** the same units as G's hulls. `components[]` has no member-file list, so F also synthesizes membership by prefix (`render_f.py:150–158`). Hull membership excludes unselected files. | **Chart-side grouping, disclosed as approximate.** Do not claim canonical component ids. Additive `components[].member_paths[]` only if the product needs canonical hulls (trigger below). |
| 6 | **Unreached tail** | `files[].path_class`, `function_count`; `stats.functions[].reachable_from_entry` | Function-level reachability `src/aggregator/mod.rs:142–145,754`; per-file set derivable (`src/charts/mod.rs:865–871`) | `prod_sources = function_count > 0 ∧ path_class == production` (`render_g.py:287`); `unreached = prod_sources − BFS depth` (`:288`); sorted cost↓/path; drawn only when `len(depth) ≤ 2` (`:291`), else footer chip (`:625–628`) | G uses its all-confidence file BFS, not the schema boolean. Files with 0 functions are excluded. Production-only tail hides non-production unreached entirely. | **Chart-side**, using estimated reachability; count **both** production and non-production unreached; draw production grey, aggregate the rest. |
| 7 | **Path trace** | No trace field. Inputs: `entry_kind`/`entry_confidence`, `call_graph.edges`, `function_cost_v2` | Entry `choose_entry` `render_g.py:159–201`; BFS adjacency sorted cost↓/path (`:213`); `select_path` `:221–235` | Endpoint = BFS-tree leaf (no BFS-tree child), chosen cost↓ then depth↓ then path; walk `parent` chain (`:226–235`). Deterministic. **Representative entry→BFS-leaf**, not a graph sink, not longest, not costliest path | Leaf ≠ graph sink; endpoint chosen by endpoint cost only (not accumulated path cost). Multi-entry not represented. No persisted trace. | **Chart-side, no schema change.** Document the exact semantics on the view and in the spec: "representative entry-to-leaf path, static". |
| 8 | **Cost + severity** | `function_cost_v2`, `unknown_trip_loops`, `trip_count_source`, `known_trip_value`; `findings.severity`, `confidence` | Cost `src/aggregator/mod.rs:322–354`; provenance `:368–386`; findings `src/analyzer/finding.rs:134–135`; schema `$defs` `:292–299` | Cost unit `static cost units (v2)` (`render_g.py:54–55`); file cost `:112–118`; severity used in dominant-rule (`:248–261`); G **does not** read trip provenance or finding confidence | **No `cost_summary`** in schema or Rust emit; total/median/percentile are chart-side only. No file/function severity aggregate; severity exists only per finding. Cost is heuristic static, never runtime. | **Use as-is; compute aggregates/anchors chart-side.** Add trip-provenance badges (`src/reporter/views.rs:151–169`) so cost is not over-trusted. One anchor convention via `ScaleOrdering`. |
| 9 | **I/O sinks** | **Missing as a first-class field.** Only loop-scoped findings: `PERF-C-LOOP-002` (`src/analyzer/heuristics/loop.rs:27,32–55`), `LOOP-003` (`:21`); evidence `io_kind` = streaming/hoistable (`:237`); Rust `:172`; Python `:195` | Heuristics fire only for I/O **inside loops**; no per-function I/O classification | G has no I/O logic; derives nothing | Cannot identify general I/O sinks. `call_graph` exposes only cross-file `to_name` (not all call names); intra-file and target-less I/O calls vanish. A defensible sink map is unsupported by v1.7. | **Downscope for Atlas v1.** Keep loop-I/O findings in KV1 as heuristics; label them loop-I/O, not "I/O sink". Additive `stats.functions[].io_sink`/`io_kind` only if a pre-registered need appears (trigger below). |
| 10 | **Noise signals** | `files[].path_class`; `parse_health` (status/error_regions/error_node_count/resolver); `parse_health_summary`; `include_graph.edges[].to = null`; `call_graph.edges[].confidence` | `path_class` `src/aggregator/path_class.rs:63–87`; parse health set in scanner, aggregated `src/aggregator/mod.rs:116–125,169–173`; unresolved includes `:1171–1185` | G reads only `path_class` (`:155,523–525`) and `is_program_path` (`:95–98`). **Ignores parse health, resolver status and edge confidence**; the subhead always claims "LOW confidence" (`:848`) even when edges are MEDIUM | Honest-disclosure gap: the prototype cannot show partial/failed parse or the LOW share, despite the README criterion. Caption is factually wrong for semantic builds. Vendor/generated are only `path_class` labels. | **Use as-is.** The shipped renderer must consume parse health + confidence counts + demoted counts and print them in the noise footer. No schema change. |

## 3. Additional findings

### 3.1 Prototype-only synthesis (not schema fields)

G computes and stores: `median_function_cost`, `function_percentiles`, per-file `cost`, aggregated
`pairs`, `depth`/`parent`/`reachable`/`selected`/`omitted`/`unreached`/`shown_unreached`, `hot`,
`drill_target`, `domain` hulls, entry `alternatives`, `finding_summary`, `back_edges`/`drawn_edges`
counts (`render_g.py:269–343,428–467,471–509,248–261,1131–1136`). All of these are deterministic
chart-side derivations and stay so.

### 3.2 Schema fields G ignores but the shipped Atlas needs

- `parse_health` + `parse_health_summary` (noise footer; partial-parse honesty).
- `call_graph.edges[].confidence` (confidence mix; caption correctness).
- `reachable_from_entry` (resolved-only reference; useful as a comparison, **not** as the canvas
  claim).
- Trip provenance `unknown_trip_loops` / `trip_count_source` / `known_trip_value` (KV1 honesty).
- `components[]` (component rows in the weight strip; hull disclosure).
- `findings.confidence` (action rows).
- `include_graph` (Q2 blast radius; the G drill already uses it at `:906–999`).

### 3.3 Policy conflicts to resolve in the spec, not in code

1. **Reachability semantics.** `reachable_from_entry` is resolved-only (schema `:152`); the
   default tier has 0 resolved edges (`docs/evidence/map-data-coverage.md:39–43`). The Atlas canvas
   therefore uses **estimated structural reachability** over the drawn edge set, and must never
   print the schema field's name. The ADR records this as a deliberate, disclosed departure from
   ADR-0008's "resolved call edges only" line.
2. **Component identity.** `components[]` (directory parents) ≠ G hulls (domain prefixes). The
   Atlas must choose one unit per surface: hulls are approximate structure; component rows in the
   weight strip use `components[].path_prefix` with membership by prefix, disclosed. Do not mix
   the two in one table.
3. **Median convention.** Rust nearest-rank `ceil(0.5·N)`-th smallest (lower-middle for even N) vs Python even-N average. The build must
   reuse `ScaleOrdering` (`src/reporter/views.rs:288–331`); no second median implementation.
4. **Entry abort.** The prototype raises `SystemExit` when no eligible entry exists. The shipped
   view must render a `NO ENTRY` state with the same below-graph brief (and the weight strip), not
   fail the report.

## 4. Additive schema candidates (none required for v1)

| Candidate | Why it would be added | Trigger (what would change the decision) | v1 decision |
|---|---|---|---|
| `components[].member_paths[]` | Canonical hull membership; removes the prefix approximation and the F/G unit mismatch | The founder requires canonical component ids in the shipped view, or a key view makes a component-level claim that prefix matching cannot support | **Not added.** Chart-side `domain_of`, disclosed. |
| `stats.functions[].entry_id` / `reached_by_entry[]` | Multi-entry attribution; today `reachable_from_entry` is a boolean union | A holdout shows multiple meaningful production entries (handlers, daemons, multi-bin) and one elected entry misleads | **Not added.** Elect one, disclose `N entries` and alternatives. |
| `stats.functions[].io_sink` / `io_kind` | General I/O-sink map; v1.7 only has loop-scoped I/O findings | A pre-registered need or holdout read asks "where does I/O happen?" and the loop-I/O findings cannot answer it | **Downscoped.** Loop-I/O findings only. |
| `graphs.call_graph.edges[].call_site_count` | Raw call-site weight; edges are deduped per caller/callee/target | A key view needs call *volume* rather than distinct structural pairs | **Not added.** Use distinct pairs; label them as such. |
| `stats.cost_summary { unit, total, median_function, function_count, p95, max }` | Removes anchor drift for JSON consumers and report/Atlas parity | Report and Atlas anchors diverge, or an LLM consumer needs the anchor without re-deriving | **Not added.** One shared chart-side helper (`ScaleOrdering`) is enough. |

## 5. What is computed chart-side (build contract)

Entry election + alternatives; structural adjacency and confidence mix; BFS depth/parent/reachable;
node selection and caps; file cost; component grouping/hulls; dominant finding; top functions;
percentiles; median; weight-strip shares; blast-radius counts; coverage counts; path trace; noise
footer counts; all number formatting and anchor text. None of this is persisted in `metrics.json`.

## 6. Acceptance for the data decision

- **A3 (data decision):** every Atlas element in §2 states "existing field X", "chart-side
  derivation Y", or "missing Z, downscoped/additive". No element is implicitly unsupported.
- **No schema drift:** `docs/schema/metrics.schema.v1.7.json` and `src/schema/` are unchanged by
  the Atlas v1 build unless a trigger in §4 fires; a schema change is a separate additive plan.
- **Determinism:** all chart-side derivations sort every traversal; no set iteration leaks into
  output; double-scan byte-identical HTML/SVG.
- **Honesty:** the canvas never prints `reachable_from_entry`, "resolved", "runtime" or a time unit
  for a structural edge; the noise footer prints the confidence mix and caps.

## 7. References

- `docs/schema/metrics.schema.v1.7.json` (call graph `:222–244`, entry/reachability `:142–156`,
  components `:252–277`, files `:67–115`, findings `$defs` `:292–352`).
- `src/aggregator/mod.rs` (functions `:84–98,229–308,322–354,368–386,429,583,628,754–773,
  1209–1219,1291–1356`); `src/aggregator/path_class.rs:63–87`;
  `src/analyzer/heuristics/arch.rs:583–726`; `src/analyzer/heuristics/loop.rs`;
  `src/reporter/views.rs:151–169,191–198,288–331`; `src/charts/layers.rs:610–618`.
- `docs/evidence/map-vision/v7/g/render_g.py` (entry `:159–201`, reachability `:204–218`, path
  `:221–235`, cards `:248–277`, hulls `:428–467`, edges `:471–509`);
  `docs/evidence/map-vision/v7/f/render_f.py:150–158`.
- `docs/evidence/map-data-coverage.md` §4–§8.
