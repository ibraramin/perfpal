# Map-Intelligibility V1 — data coverage + known-answer recall (H1 vs H2)

> Plan `map-intelligibility` V1 · spec `map-intelligibility` A1b/A3 · date 2026-10-21.
> Inputs: the V0 pre-registration (`3982b03`), the V0 diagnosis (`3a32b01`), the current
> `metrics.json` (schema 1.6) for all six repos, the rendered SVGs, and the chart/report
> source. Machine-readable: `docs/evidence/map-recall-v1.json`.
> **§8 (added 2026-10-21) records the V2 lock**: the accepted view set and the field
> decisions this audit feeds.
>
> **Question:** for every pre-registered action and every I5 GENUINE finding, does a key view
> surface it — at what rank, under what label? Data present but never surfaced = **H2**;
> the field needed to express it is absent = **H1**.

## 1. Method

1. **Field inventory** from `metrics.json` (`schema_version 1.6`): `files[]`,
   `stats.functions[]`, `findings[]`, `graphs.include_graph.{nodes,edges}`,
   `graphs.call_graph.edges`, `components[].metrics`, `parse_health_summary`.
2. **Selection/ranking replicated** from the chart code (then cross-checked against the SVG
   text): `NODE_CAP=50` + `REACHABLE_OVERFLOW=10` (`src/charts/mod.rs:71-76,555-604`),
   `BAND_FILE_CAP=10`, `COMPONENT_CAP=50`; `layer-findings` top-50 by
   `3*hot+2*warn+1*info` **raw count** with no KLOC denominator or minimum-findings guard
   (`src/charts/layers.rs:1414-1431`; weight at `src/charts/mod.rs:148-156`). One model is
   approximate: `layer-entry` membership is modelled as entry-cost top-50 + reachable reserve,
   while the shipped code elects real entries by cost, then non-entries by **non-LOW call
   degree**, then reserves call-distance-reachable files (`src/charts/layers.rs:663-720`); in
   the default tier every call edge is LOW, so the difference is limited to the reserve order.
   Membership for every row below was re-verified against the rendered SVG text.
3. **Recall scoring** per target: file label shown in ≥1 stem; rank in that stem's ranking;
   whether the function/line or the decision is named; whether the finding ID reaches
   `report.html#map` (top-10 components × top-3 findings by severity then ID,
   `src/reporter/mod.rs:1472-1572`).
4. **Labels:** charts draw at most a 32-char path (last 31 chars kept, `src/charts/mod.rs:84`),
   never a function name or finding ID; the `#map` table prints bare `finding_id`s
   (`src/reporter/mod.rs:1556`) and no rule title.

Field facts that matter for the verdicts:

- All six default-tier scans export **only LOW-confidence call edges** — tmux 6,731/6,731,
  hyperfine 297/297, cJSON 587/587, chunkhound 6,169/6,169, ripgrep 944/944, curl
  7,785/7,785. The `layer-entry` caption is literal: `0 resolved call edges; reachability =
  entries only` (`src/charts/layers.rs:858-861`). Resolved call data exists only in the opt-in
  `semantic-clang` tier (I5 §4: cJSON 115 / curl 1,548 edges).
- Include edges are resolved at file level (tmux 214/234, cJSON 96/229, curl 2,502/3,812),
  so include-based blast radius is available in the default tier.
- Entry detection over-marks lib-roots (tmux 1,462 `lib-root` functions; cJSON 628;
  ripgrep 2,672) — `R-pf-098`.
- File selection for `include-graph`/`layer-file` is **degree-desc**, not cost-desc
  (`src/charts/mod.rs:555-604`); the heat is cost, so a high-cost low-degree file is only
  drawn if it wins the 10-slot reachable reserve.

## 2. Recall A — the 9 pre-registered actions

`file` = the file label is drawn in ≥1 of the 8 stems; `#map` = the path:line appears as a
top-3 link in `report.html#map`; `action?` = the view states the pre-registered decision
(profile this function / do not touch this header).

| # | Repo | Q | Pre-registered action | file | stem rank evidence | #map | action? | Verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | tmux | Q1 | profile `input.c:970 input_parse` | ✅ layer-file (reserve), layer-entry, layer-findings, layer-bands | cost #9/216, weight #5; degree #146 (reserve only) | ❌ "." top-3 = format.c/screen.c/tty.c | ❌ no function/line | **file-only** |
| 2 | tmux | Q2 | do not touch `tmux.h` (160/203 TUs) | ✅ include-graph, layer-file | include degree **#1/216** | ❌ (only component ".") | ❌ hub drawn as a node, no "blast radius" label | **hub visible, decision unlabeled** |
| 3 | tmux | Q3 | profile `screen-write.c:2776 collect_add` | ✅ layer-file, layer-entry, layer-findings, layer-bands | cost #4/216, weight #6 | ❌ | ❌ | **file-only** |
| 4 | hyperfine | Q1 | instrument `src/benchmark/mod.rs:346` | ✅ all 5 file stems | cost #6/53, weight #2, degree #2 | ⚠️ `ALLOC-001:src/benchmark/mod.rs:333:9` (same function `run`, loop head, raw ID) | ❌ no loop/function | **file + same-function finding** |
| 5 | hyperfine | Q2 | do not touch `src/command.rs` (5 importers) | ✅ all 5 file stems | **cost #1/53** (max-cost outline) | ⚠️ `ALLOC-001:src/command.rs:148:9` | ❌ framed as *hot-path/profile*, the opposite decision | **mislabeled** |
| 6 | hyperfine | Q3 | profile `src/timer/mod.rs:82 execute_and_measure` | ✅ all 5 file stems | cost #28/53, weight #20 | ❌ `src/timer` top-3 = linux_counters/macos_counters | ❌ | **file-only** |
| 7 | cJSON | Q1 | profile `cJSON.c:357 parse_number` | ✅ layer-file, layer-entry, layer-findings, layer-bands | cost #3/101, weight #4 | ❌ component "." is rank **13**, outside top-10 | ❌ | **file-only, not on first screen** |
| 8 | cJSON | Q2 | do not touch `cJSON.h` (public ABI) | ✅ include-graph, layer-file, layer-bands | degree #4/101 | ❌ | ❌ no "do not touch" label | **hub visible, decision unlabeled** |
| 9 | cJSON | Q3 | profile `cJSON.c:243 cJSON_New_Item` | ✅ same as #7 | same | ❌ | ❌ | **file-only** |

**Score: 0/9 actions stated at function/line granularity; 2/9 "do not touch" hubs visible but
unlabeled; 1/9 (#5) framed as the opposite decision; 9/9 files drawn somewhere; 2/9 paths
receive a raw finding link on `#map` (both hyperfine — and #4's link is the same function
`run` at the loop head, the strongest single positive in the set).**

The two "hub" rows are the only place the map *could* answer Q2, and both are read by the
blind vision pass as **Q1 "inspect first"** — `tmux.h` and `cJSON.h` were named as the place to
start, not as the thing not to touch. The data is right; the question label is wrong.

## 3. Recall B — the 9 I5 GENUINE findings

All 9 are present in the current `metrics.json` (schema 1.6) and are `hot`; confidence is
HIGH except ripgrep (MEDIUM) and the two curl LOOP-002s (MEDIUM). The question is whether a
view surfaces them.

| # | Repo | Finding (path:line) | File drawn? | Stems / rank | #map table | Verdict |
|---|---|---|---|---|---|---|
| 1 | chunkhound | `write.rs:133:13` | ✅ | layer-entry, layer-findings (weight #16/887), layer-bands | ❌ component rank **12** | file-only |
| 2 | chunkhound | `write.rs:239:17` | ✅ | same | ❌ | file-only |
| 3 | chunkhound | `pipeline.rs:922:29` | ✅ | layer-entry, layer-findings (weight #14/887) | ❌ component rank **53** | file-only |
| 4 | chunkhound | `pipeline.rs:1431:25` | ✅ | same | ❌ | file-only |
| 5 | chunkhound | `pipeline.rs:1493:9` | ✅ | same; band omitted (component >50) | ❌ | file-only |
| 6 | chunkhound | `compaction.rs:32:5` | ✅ | layer-findings (weight #48), layer-bands | ❌ component rank 12 | file-only |
| 7 | ripgrep | `globset/src/lib.rs:424:9` | ✅ | all 5 file stems; weight #18/98 | ❌ component rank **19** | file-only |
| 8 | curl | `lib/file.c:387:14` | ❌ | **no overview stem** (degree #51, not entry, weight #80, band cut); file appears only on the linked per-component page `component-09-lib.svg` | ❌ `lib` is rank 9 but its top-3 are three ALLOC-001 findings elsewhere | **invisible in every overview stem** |
| 9 | curl | `lib/file.c:582:15` | ❌ | same | ❌ | **invisible in every overview stem** |

**Score: 0/9 finding IDs appear in any overview chart; 0/9 reach the `#map` table; 0/9 are
surfaced at finding granularity (7/9 surface only as a file label); 2/9 (both curl LOOP-002
GENUINE) are invisible in every overview stem.**

Why the curl pair disappears is structural, not accidental: `lib/file.c` is degree #51 of
923 (just outside the 50-node file cap, and not reachable-reserve), not an entry file, and its
raw severity weight (37) ranks #80 of 923, outside the 50-node findings cap. Its component
`lib` is rank 9 and does reach `#map`, but the table shows only the top-3 findings by
severity-then-ID and all three are `ALLOC-001` findings in other files. The data exists; every
overview view's cap/ranking hides the finding (the file label survives only on the linked
per-component page, with no finding ID). **This is the cleanest H2 proof in the corpus.**

## 4. H1 vs H2 — decision per question

| Question | Verdict | Evidence | If H1, what is missing |
|---|---|---|---|
| **Q1 where to look first** | **H2 (visualisation + ranking)** | Files are extracted and drawn; `cJSON.c` (pre-reg Q1) is absent from `#map` because component "." is rank 13; tmux's first screen ranks `fuzz` (171.17, test/fuzzer) above `input.c`; the chart selects file nodes by include degree, not cost; labels are paths only; 0/9 GENUINE IDs surface. | No missing field blocks a file-level answer. Additive `files[].path_class` would make the production filter principled (R-pf-099). |
| **Q2 what breaks if touched** | **H2 for includes; H1 for calls (default tier)** | Include edges resolve (214/234 tmux, 96/229 cJSON, 2,502/3,812 curl) and the include graph shows the hubs (`tmux.h` degree #1, `cJSON.h` #4), but the map never labels a node as blast radius and the coupling matrix has no scale. All six default scans export **0 non-LOW call edges**, so a call-based blast radius cannot exist at function granularity in the shipped default. | Include blast radius: no field missing (compute reverse in-degree from `graphs.include_graph.edges`). Call blast radius: resolver-tier capability, not a schema field — semantic tier has 115/1,548 resolved edges (I5 §4) but is opt-in; default C must either surface LOW edges with an honest label or downscope Q2 to includes. |
| **Q3 where time plausibly goes** | **H2 (visualisation + ranking)**, with H1 only for *measured* time | `function_cost_v2` exists; the views surface cost but (a) the top node is a test/FP/startup path in all three samples (`format.c`, `src/command.rs`, `testunity.c`), (b) no units or comparison anchors, (c) no function-level cost. Measured runtime is absent by design; "plausibly" is satisfied by a static hint. | `stats.cost_summary` (unit/median/p95/max) would give stable anchors; `path_class` + trip provenance would fix the ranking. |

**Overall: H2 dominates.** The missing-field list is short; the dominant failures are
selection keys, caps, labels, and the absence of a plain-language answer view. That is exactly
the V2/V3 redesign surface, not a data-extraction project.

## 5. Missing fields / policy gaps (additive only)

| Gap | Needed for | Exists today? | Additive note |
|---|---|---|---|
| `files[].path_class` (`production/test/example/bench/fixture/generated/vendor/unknown`) | Q1/Q3 production-path ranking (R-pf-099) | ❌ no | Optional enum; deterministic path classifier; never removes data, only labels it. |
| `stats.functions[].trip_count_source` + `known_trip_value` | Q3 known-trip guard (literal `1e6` inflation) | ⚠️ only `unknown_trip_loops` (count) | Optional on the wire; **V2 raised this to REQUIRED** (see §8) — additive, no change to `function_cost_v2`. |
| `stats.cost_summary {unit, median, p95, max}` | Q3 units/anchors (`× median`, `top 1%`) | ❌ no | Additive object; chart-side anchors can also be computed per scan. |
| `components[].metrics.coupling_in/out` (or a documented reverse-include count) | Q2 blast-radius key view | ❌ no (edges exist) | Compute chart-side from `graphs.include_graph.edges`; add only if a stable, contract-level number is needed. |
| `stats.functions[].graph_root` (or corrected `entry_kind`) | Q2/Q3 strata (R-pf-098) | ⚠️ `entry_kind` over-marks `lib-root` | Additive boolean `graph_root` (no incoming resolved call edge) or fix detection; founder decision A-pf-098. |
| `components[].metrics.finding_density_guarded` / min-findings policy | Q1 tiny-scope density (R-pf-100) | ⚠️ `finding_count` exists; no guard | Prefer a chart-side minimum (e.g. ≥5 findings or ≥200 LOC) with an honest "N small scopes hidden" caption; schema add only if the guard must be contractual. |
| `graphs.call_graph.edges[].resolved/tier` uniform field | Q2 call-blast radius across tiers | ⚠️ default only LOW `confidence`; semantic tier is separate | Additive `resolved: bool`/`tier` for a uniform view predicate; no change to existing edges. |
| Rule titles / symbols / lines in key views | Q1 plain language | ✅ `findings[].title`, `stats.functions[].name`, `span` | **Not a schema gap** — a display gap (`#map` prints raw IDs, `src/reporter/mod.rs:1556`; appendix decodes later). |

## 6. What this means for V2/V3 (design constraints, not the design)

- **Q1 key view must be answer-first at function granularity:** 3–5 ranked actions with
  `path:line`, function, rule title, severity/confidence, production class, and a cost anchor —
  the text-first brief is a legitimate primary view (spec §Visual methodology). The current
  8 stems cannot be "fixed" into this by relabeling; they select by degree and cap by raw
  counts.
- **Q2 key view may only claim includes in the default tier.** A reverse-include blast-radius
  bar/table ("editing this rebuilds N TUs / is included by N files") is supported today. Any
  call-based claim must carry the LOW-confidence/semantic-tier label or be deferred.
- **Q3 key view must show units + anchor + production filter**, and must keep the
  "candidate — profile to confirm" honesty. The top cost node must not be a test/startup path.
- **The 8 stems survive only as a labelled Explorer tier** (no orphan views; one encoding per
  key view). The current `layer-findings` title says "density" but uses raw severity-weighted
  counts with no denominator — either rename it or give it a denominator before it is promoted.
- **`#map` must carry the answer, not the chart index.** Today the first screen is a
  component-density node-link with a top-10 table of raw finding IDs; it cannot answer Q1 for
  cJSON (component "." rank 13) or Q2 at all.

## 7. Caveats / reproduce

- Selection/ranking is replicated from `src/charts/*`; membership was cross-checked against
  the SVG text for the key cases (tmux `input.c`/`screen-write.c` present; curl `lib/file.c`
  absent from every stem; chunkhound `write.rs`/`pipeline.rs` absent from include-graph/layer-file;
  cJSON.c present). The reserve is cost-sorted (`src/charts/mod.rs:590-604`), which is why
  low-degree high-cost files can still appear in `layer-file`.
- The 9 GENUINE findings were rescanned at the current tree with `perfpal 0.2.0-beta.1`
  (schema 1.6): chunkhound 887 files / 8,124 findings, ripgrep 98 / 1,310, curl 923 / 10,985;
  all 9 IDs are present verbatim in `metrics.json`.
- The semantic tier's resolved edges are cited from I5 §4 (cJSON 115 / curl 1,548); the
  semantic output directories were not regenerated for V1. Default-tier conclusions are
  unaffected.
- Recall is a data/view test, not a human test; V0's 28 blind vision reads supply the
  legibility half. A2 (founder timed test) remains open and is V6.

## 8. V2 lock — accepted view set + field decisions (2026-10-21)

The locked slots are in `docs/specs/map-intelligibility.md` → "Locked acceptance view set"
(mirrored in `maintainer-map.md`): **KV1 Start here (Q1, severity)**, **KV2 Blast radius (Q2,
coupling)**, **KV3 Profile queue (Q3, cost)**, with the 8 legacy stems under a labelled
`Explorer` divider. This section records only what those slots need from the data, so V4 edits
exactly these fields; every row carries its **what would change it** trigger.
**No implementation in V2.**

| Slot / need | Field | Decision | What would change it |
|---|---|---|---|
| KV1 production-first ranking; KV3 filter | `files[].path_class` | **REQUIRED (additive, V4; schema 1.6→1.7)** — optional enum `production/test/example/bench/fixture/generated/vendor/unknown`; deterministic path classifier; absent ⇒ `unknown`; demoted files are counted, never dropped | classifier precision <95% on the 6-repo sample, or founder prefers label-only over demotion ⇒ keep chart-side label, no schema field |
| KV1/KV3 recall (A1b) | none — `findings[].finding_id/title/severity/confidence/span` exist | display fix only: KV1 shows rule **title** + `path:line` + function with a complete deterministic ranked drill list (3–5 inline); KV3 links the complete deterministic function list ranked by cost within the non-demoted set | V3/V5 shows the 3–5 inline actions are not the ones a reader acts on ⇒ change the ranking key, not the schema |
| KV2 blast radius | none — reverse include in-degree from `graphs.include_graph.edges` | compute chart-side per **file**; print `resolved N of M include edges`; no call claim in the default tier | Q2 moves to component units, or an LLM consumer needs a contract number ⇒ `components[].metrics.coupling_in/out` |
| KV3 unit + anchor | none — median/p95 from `stats.functions[].function_cost_v2` (v1 fallback) | one deterministic chart-side helper shared by `report.md` + charts; unit text `static cost units (v2)` (v1-fallback rows read `(v1)` and anchor within the v1 set); `format_fixed_point` remains the only non-integer path | report/chart anchors diverge, or JSON consumers need the anchor without re-deriving ⇒ `stats.cost_summary {unit, median, p95, max}` |
| KV3 estimate honesty | `stats.functions[].trip_count_source` + `known_trip_value` | **REQUIRED (additive, V4; optional on the wire)** — `trip_count_source` is `literal` (every loop has a canonical literal trip), `partial` (some), or `unknown` (loops but none literal); the field is omitted when the function has no loops. `known_trip_value` is the largest **clamped** literal trip multiplier used by `function_cost_v2` (`min(t, MAX_TRIP)`), omitted when no literal trip. KV3 prints `known trip ×N — verify it actually runs` on known-literal trips, demotes no-loop functions (`max_loop_depth == 0`) below loop-bearing ones, and keeps `unknown_trip_loops > 0` as the "estimate incomplete" badge. Three-branch guard for the documented FPs: django `contrib/sessions/backends/cache.py` (literal-10000 retry guard → 10,000× model artifact) and tmux `format.c` (statement/branch-count dispatcher inflation; demoted if `max_loop_depth == 0`, otherwise labelled) — both production, non-`main`, `unknown_trip_loops = 0`. | the guard does not stop those FPs from being named in V5 ⇒ rank by per-iteration cost or adopt the two-list variant (known-trip vs unknown-trip) |
| KV3 entry/startup demotion | `entry_kind`, `reachable_from_entry` exist | **do not use entry strata** in key views: `R-pf-098` over-marks lib-roots and the default tier resolves 0 call edges; demote `main/script` by `entry_kind` only | V7 fixes entry detection and the founder wants a reachable-only Q3 ⇒ `graph_root`/corrected `entry_kind` becomes needed |
| density guard (R-pf-100) | `components[].metrics.finding_density` exists | **chart-side, Explorer only** — minimum (≥5 findings or ≥200 LOC) + `N small scopes hidden (components/files)` on component/file density views; `layer-findings` stays Explorer and may not claim "density" until it has a denominator. No key view is scope-density ranked. | the guard must be contractual for JSON consumers ⇒ `finding_density_guarded` |
| uniform call-edge tier | `confidence` exists; semantic tier separate | **deferred** — KV2 makes no call-based claim, so no uniform `resolved`/`tier` field is needed for the accepted set | a key view starts making call-based blast-radius claims |

**V5 falsifier update (2026-10-22):** the KV3 no-loop guard demotes all three pre-registered
Q3 targets (`max_loop_depth == 0`), but removing only the no-loop term still leaves them out
of the inline top 8 (shipped→counterfactual ranks 820→325, 58→46, 60→45). The trigger in the
KV3 row fired as a **guard contribution, not a sole cause**: the static-cost key cannot
surface per-byte/per-call leaves. V7 owns the switch to call-weighted/per-iteration cost or a
separate call-frequency view; a two-list variant of the same static cost is insufficient.
Evidence: `docs/evidence/map-vision/v5/no_loop_falsifier_v5.json`.

**Not a schema gap:** rule titles/symbols/lines already exist (`findings[].title`,
`stats.functions[].name`, `span`); `#map` printing bare IDs is a display defect
(`src/reporter/mod.rs:1556`).

**V3 degrees of freedom:** idiom only. The slot, question, encoding, unit/anchor rule,
production-first rule, default-tier claim limit and A1b recall guarantee are locked; changing
any of those reopens V2.

**V4 scope:** two additive changes only — `files[].path_class` and
`stats.functions[].trip_count_source` + `known_trip_value` (both optional on the wire; absence
means not computed; schema 1.6→1.7). The V4 batch must
ship `docs/schema/metrics.schema.v1.7.json` + `src/schema/metrics.schema.v1.7.json` (byte-copy),
bump `SCHEMA_VERSION` / `SCHEMA_ARTIFACT_NAME` (`src/schema/mod.rs:89-95`), satisfy the
embedded-schema drift test (`src/schema/mod.rs:640-646`), and note the additive fields in
`docs/schema/metrics-json-v1.md` + the `SCHEMA_VERSION` doc comment. Every other number is
computed chart-side from existing fields; `format_fixed_point` remains the only non-integer
output path.
