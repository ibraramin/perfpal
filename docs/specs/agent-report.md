# Spec: agent-report

Scope: feature

**Status:** contract lock (Track C, session C1). **No implementation in this spec.**
**Date:** 2026-10-24. **Owner:** orchestrator. **Plan:** `docs/plans/agent-report.md`. **Track brief:** `docs/tracks/agent-report.md`.
**Inputs:** `docs/research/product-consolidation-options-2026-10-22.md` §Q3 (option A, ratified 2026-10-24);
`docs/specs/atlas-metrics.md` (units/anchors precedent); `src/reporter/views.rs` (shipped rank keys);
`src/reporter/mod.rs` `HEURISTIC_CATALOG_V1` (67 rules); `docs/report/output-contract.md`;
`tests/golden/report-mixed/metrics.json` (sample source).

## 1. Purpose

`agent-report.md` + `agent-actions.json` are the **LLM-facing depth layer** of a Perfpal scan: a
token-efficient, deterministic, action-ranked view of the canonical `metrics.json` — top actions with
`path:line`, rule, why, evidence, confidence, an impact anchor and a curated fix hint; a complete
machine action list; the cost-ranked profile queue; and an explicit data-quality header.

It is **not** a human dashboard, **not** a chart surface, and it makes **no LLM or network calls**. It is
the consumer view the simplified Atlas first page points to, and the artifact a downstream agent reads
without loading the full human report.

## 2. Ratified decisions (C1 gate, founder 2026-10-24)

| ID | Decision | Rationale |
|---|---|---|
| **D1** | `agent-report.md` is a **bounded fixed skeleton** (top 25 actions in full detail, per-rule rollup, top 25 profile rows). `agent-actions.json` carries the **complete** action list. | An inline complete index reaches ~170k tokens on a 7.5k-finding scan and cannot be loaded. Completeness lives in the machine contract, never silently: every cap is printed with `N of M`. |
| **D2** | Fix hints live in a **C-owned table** (`AGENT_FIX_HINTS`) in the new module, locked against the 67-rule catalog. | Zero edits to the A-owned `mod.rs`/`HEURISTIC_CATALOG_V1`; the lock test still makes drift impossible. |
| **D3** | The agent artifacts **omit clock provenance** (`scan.started_at`, `scan.duration_ms`). | Two runs of the **same command** are byte-identical with **no normalization**. `scan.root_path` is the verbatim CLI argument (the shipped `findings-detail.*` omits it entirely), so a different invocation path is a different input by design, not a determinism failure. |
| **D4** | Artifact names are **`agent-report.md`** and **`agent-actions.json`**, always emitted, no flag. | Founder-approved names (research Q3); discoverable without a UX flag. |
| **D5** | The rank keys are the **shipped key-view keys** (KV1 for actions, KV3 for the profile queue), not a new scoring model. | `agent-report.md`, `report.md`, `findings-detail.md` and `profile-queue.md` must never disagree about order. No new weights under this contract version. |
| **D6** | Numbers reuse the **shipped key-view strings** (`views.rs` anchors: no thousands separators, `× median` ratio rule, `top P%`). | Cross-artifact parity beats a formatter consolidation. The Atlas separator rule is an Atlas-surface rule; C follows a future single-formatter contract bump. |
| **D7** | `why`, `title` and `evidence` are **verbatim in `agent-actions.json`**; the Markdown renders them **structure-escaped** (§9.9; a backtick becomes `ˋ`).  The forbidden-time-claim scan applies to **C-authored strings** (headers, data-quality lines, caveats, fix hints), not to quoted analyzer provenance. | Traceability stays in the machine contract; the MD must not let a crafted path or message forge a heading, close a code span or inject HTML. The shipped `message` strings are provenance, not promises the renderer makes. |
| **D8** | An **evidence-only, non-normative reproducer** (`docs/evidence/agent-report/sample/derive_sample.py`) is permitted. | Founder-ratified 2026-10-24 after the Q1 review flagged the brief's "no code" clause: it is never shipped, lives only in C's evidence directory, and exists to prove the committed sample is reproducible. Production code remains C2-only. |

## 3. Artifacts

| Name | Consumer | Completeness | Cap | Size class |
|---|---|---|---|---|
| `agent-report.md` | LLM / human triage | Bounded view | top 25 actions, top 25 profile rows | ≤ 80 KB on the named fixtures and the §12 width envelope |
| `agent-actions.json` | tools / agents | **actions complete**; profile top 100 | `profile_queue_cap = 100` (named in the file) | O(actions) |

- **Always emitted**, alongside the existing artifacts, with no CLI flag (D4).
- **Additive minor bump** (v1.x) to the output contract: consumers already must tolerate unknown files.
  A updates `docs/report/output-contract.md` at merge (A owns that file); C does not edit it.
- **Ownership:** C owns both names and both renderers (`src/reporter/agent_report.rs`, its tests,
  `docs/evidence/agent-report/`). A adds the human-page link slot at step 12. D may read.

## 4. Invariants (binding)

1. **Pure function of the final document.** No I/O, clock, environment, locale, network or randomness.
   Same `MetricsDocument` → same bytes for both artifacts.
2. **No LLM calls, no network, no JavaScript, no new dependencies, no new schema fields, no CLI flag.**
3. **`metrics.json` stays canonical.** Neither artifact is a source of truth; both are regenerated from
   it. No ranked list is added to `metrics.json`.
4. **Deterministic bytes.** Serialize with the shipped serializer conventions (declaration-order struct
   fields, sorted map keys, `preserve_order` OFF). No timestamps (D3). No locale-dependent formatting.
   Every traversal is sorted with a total tie-break.
5. **No silent caps.** Every cap prints `N of M` and is named in `data_quality.caps`; JSON actions are
   uncapped.
6. **Candidates, never verdicts.** Every artifact states that findings are optimization candidates and
   cost is static, not time.
7. **Fixed skeleton, named stubs.** Both files have the same shape for application, library, no-entry,
   empty, all-test, capped, partial-parse and single-file scans; absent data renders a named stub.
8. **Units + anchors.** Every emitted cost carries `static cost units (v2|v1)`, `× median` (only when
   the median is > 0) and `top P%`; every share names its denominator. Ordinal ranks are exempt.
9. **One rank key per list**, reused verbatim from `views.rs` (§5). No new weights.
10. **Complete action list in JSON**; the MD never pretends to be complete.

## 5. Rank keys (normative)

### 5.1 Actions — KV1 order, verbatim

`src/reporter/views.rs` `build_kv1_rows` (KV1), unchanged:

1. **Production-first** — `files[].path_class == "production"` before every other class (missing class
   reads `unknown` and sorts with the non-production tier).
2. **Severity** — `hot` < `warn` < `info`.
3. **Confidence** — `HIGH` < `MEDIUM` < `LOW`.
4. **Containing-function cost** — `function_cost_v2` **descending**; a finding with no containing
   function, or a v1-only function, sorts as **0** (shipped `cost_of`, `views.rs:459-468`).
5. **`finding_id` ascending** — total tie-break.

`-ARCH-002` (hot-component aggregate) findings are **excluded** from the action list and counted as
`data_quality.component_findings_excluded`. `complete_action_count == actions.len()`.

### 5.2 Profile queue — KV3 order, verbatim

`src/reporter/views.rs` `build_kv3_rows` (complete profile queue), unchanged:

1. production before non-production;
2. `max_loop_depth > 0` before `max_loop_depth == 0` (no-loop demoted);
3. `entry_kind == "main" | "script"` last (startup demoted);
4. cost **descending** — `function_cost_v2` where present, else the v1 chart-side cost
   (`crate::charts::function_cost`);
5. `path`, then function `name`, then `id` ascending.

Ranks are contiguous over **all** functions (demoted rows included, never dropped); `rank` is 1-based
in this ordering. v2 and v1 rows keep their own scale ordering for the `× median` / `top P%` anchor.

### 5.3 Changing a rank key

A rank change is a contract change: bump `contract_version`, update this spec, and state it in the
track handoff. There is no per-scan rank switch and no weight tuning in the renderer.

## 6. `agent-actions.json` (normative)

Top-level envelope (all keys always present):

| Key | Type | Rule |
|---|---|---|
| `artifact` | string | constant `"agent-actions"` (self-identifying) |
| `contract_version` | integer | `1` for this spec; bumped only by a contract change |
| `schema_version` | string | mirrored verbatim from `metrics.json` (e.g. `"1.7"`) |
| `tool` | object | `{name, version}` verbatim from `metrics.json` |
| `scan` | object | `{root_path, languages, file_count, loc_total}` — **no** `started_at`, **no** `duration_ms` (D3). `loc_total` is `object \| null` (the producer omits it on a zero-file scan). |
| `rank_key` | object | `{actions: string[], profile_queue: string[]}` — the ordered clause lists from §5, machine-readable (the profile list expands §5.2's final tie-break into its three atomic keys: `path`, `function_name`, `function_id`) |
| `data_quality` | object | §10 |
| `actions` | array | every action, KV1 order, uncapped |
| `complete_action_count` | integer | `actions.len()` (invariant, tested) |
| `profile_queue` | array | first `profile_queue_cap` rows of the KV3 ordering (§6.2) |
| `profile_queue_cap` | integer | `100` (the authoritative JSON profile cap; the MD caps are `data_quality.caps`) |
| `complete_profile_count` | integer | all functions in the KV3 ordering (uncapped count) |

Action object:

| Key | Type | Rule |
|---|---|---|
| `action_id` | string | = `findings[].finding_id`; unique per scan and **stable only until the code changes** (the schema's within-report guarantee; no new id scheme) |
| `rank` | integer | 1-based, contiguous, KV1 order |
| `rule_id`, `title`, `path` | string | verbatim from the finding |
| `symbol` | string \| null | verbatim; `null` when the finding has no symbol |
| `span` | object | `{start_line, start_col, end_line, end_col}` verbatim |
| `severity`, `confidence`, `resolver_tier` | string | verbatim tokens (`hot|warn|info`, `HIGH|MEDIUM|LOW`) |
| `macro_suspect`, `error_node_overlap` | boolean | verbatim |
| `threshold` | object \| null | verbatim when the schema field is present, else `null` |
| `path_class` | string | `files[].path_class`, `"unknown"` when absent |
| `function` | object \| null | narrowest containing function (§6.1); `null` when none contains the location |
| `why` | string | `findings[].message` **verbatim** (D7) |
| `evidence` | object | `findings[].evidence` verbatim; `{}` when absent |
| `fix_hint` | string \| null | curated hint from §8; `null` only for a `rule_id` absent from the table |
| `detail_anchor` | string | the shipped `findings-detail.html` anchor fragment (§9.6) |

### 6.1 `function` object

`{id, name, path, span, cost, cost_scale, cost_anchor}` where `cost_scale` is `"v2"`/`"v1"` and
`cost_anchor` is the shipped `ScaleOrdering::anchor` string (D6), e.g.
`213 static cost units (v2) · 30× median · top 1%`.

`cost` is the row's ranked value: `function_cost_v2` when present, otherwise the v1 chart-side
`crate::charts::function_cost` (the shipped `function_value`, `views.rs:280-285`) — never the v2-only
`cost_of` value used for the *action* tie-break. It is a **JSON number**; an integral value is emitted
**without a fractional part** (`15`, not `15.0`) so the committed sample and the renderer agree
byte-for-byte. `function` is `null` only when no function span contains the finding, so a non-null
`function` always has a numeric `cost`.

### 6.2 `profile_queue` row object

| Key | Type | Rule |
|---|---|---|
| `rank` | integer | 1-based, contiguous, KV3 order over **all** functions (demoted included) |
| `function_id` | string | `stats.functions[].id` verbatim |
| `path` | string | function path |
| `function_name` | string | function `name` (distinct from the action's `function` **object**; the key name is part of the contract) |
| `path_class` | string | `files[].path_class`, `"unknown"` when absent |
| `cost` | number | the KV3 ranked value (`function_value`), integral emitted without a fractional part |
| `cost_scale` | string | `"v2"`/`"v1"` |
| `cost_anchor` | string | the shipped anchor string |
| `unknown_trip_loops` | integer | `stats.functions[].unknown_trip_loops` verbatim |
| `known_trip_value` | integer \| null | `stats.functions[].known_trip_value` verbatim; `null` when absent |

### 6.3 Example (real datum, mixed golden fixture)

```json
{
  "action_id": "PERF-C-ALLOC-001:src/main.c:27:21",
  "rank": 1,
  "rule_id": "PERF-C-ALLOC-001",
  "title": "Heap allocation inside loop",
  "path": "src/main.c",
  "symbol": "c_loop",
  "span": {"start_line": 27, "start_col": 21, "end_line": 27, "end_col": 30},
  "severity": "hot",
  "confidence": "HIGH",
  "resolver_tier": "syntactic",
  "macro_suspect": false,
  "error_node_overlap": false,
  "threshold": null,
  "path_class": "production",
  "function": {
    "id": "src/main.c#c_loop#23:1",
    "name": "c_loop",
    "path": "src/main.c",
    "span": {"start_line": 23, "start_col": 1, "end_line": 33, "end_col": 1},
    "cost": 15,
    "cost_scale": "v2",
    "cost_anchor": "15 static cost units (v2) · 5.0× median · top 50%"
  },
  "why": "per-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization",
  "evidence": {"alloc_site_index": "2", "callee": "malloc", "loop_depth": "1"},
  "fix_hint": "Hoist the allocation out of the loop, reuse a buffer across iterations, or reserve once before the loop.",
  "detail_anchor": "finding-PERF-C-ALLOC-001-src-main-c-27-21"
}
```

### 6.4 Serialization

Pretty-printed JSON (2-space indent), `\n`-terminated, UTF-8, keys in the declared order, map-valued
fields (`evidence`) with sorted keys. No floating-point formatting drift: `cost` is a JSON number and
an integral value is emitted without a fractional part (`15`, never `15.0`). A consumer may reorder keys; byte-determinism is a property of the renderer, not of JSON.

## 7. `agent-report.md` (normative skeleton)

Exactly these sections, in this order, in every scan state:

````markdown
# Perfpal Agent Report

> contract 1 · schema {schema_version} · {tool.name} {tool.version} · scan root: `{root_path}`
> {file_count} files · {function_count} functions · {finding_count} findings · languages: {languages}

## How to use this report
- Every row is an **optimization candidate**, not a defect verdict; confirm it with a profiler.
- Cost is **static cost units**, a heuristic; it is never a duration.
- Rank key: production code first, then severity (hot > warn > info), then confidence (HIGH > MEDIUM > LOW), then containing-function static cost descending, then finding_id ascending.
- Machine contract: `agent-actions.json` holds all {complete_action_count} actions and the top {min(profile_queue_cap, complete_profile_count)} of {complete_profile_count} profile rows.

## Data quality
- parse: {ok} ok / {partial} partial / {failed} failed   (or `parse: not recorded` when the summary is absent)
- unresolved includes: {unresolved} of {total} · structural call edges: {total} ({high} HIGH / {medium} MEDIUM / {low} LOW)
- demotions: {N} non-production files · {N} startup functions · {N} no-loop functions (counted, never dropped)
- cost scale: {v2} v2 / {v1} v1 functions
- resolver tier(s): {sorted tiers}
- functions with unknown trip counts: {N} — their cost estimate is incomplete
- component-level findings excluded from actions: {N} (see the human Explorer)
- caps: top 25 actions and 25 profile rows here; all actions + top 100 profile rows in `agent-actions.json`

## Start here — ranked actions

showing top {min(25, complete_action_count)} of {complete_action_count} actions

### {rank}. {rule_id} · `{path}:{line}` · {severity}/{confidence} · {path_class}
- what: {title}
- function: `{name}` (`{path}:{line}`) · {cost_anchor}      (or the named stub)
- why: {message}
- evidence: `{k}={v} · …`                                    (sorted keys; omit line when empty)
- fix: {fix_hint}                                            (or "no curated hint for this rule yet")
- anchor: `findings-detail.md#{detail_anchor}` · `{action_id}`

{only when complete_action_count > 25} showing the top 25 of {complete_action_count} actions; the complete list is `agent-actions.json`.

## Action index by rule

| rule | title | actions | hot | warn | info | top rank |
|---|---|---:|---:|---:|---:|---:|
| {rule_id} | {title} | {count} | {hot} | {warn} | {info} | {best rank} |

## Profile queue

showing top {min(25, complete_profile_count)} of {complete_profile_count} functions

{rank}. `{function_name}` — `{path}` — {cost_anchor} · {path_class}[ · unknown-trip ×N — estimate incomplete]

{only when complete_profile_count > 25} showing the top 25 of {complete_profile_count} functions; complete human list in `profile-queue.md`, machine rows in `agent-actions.json`.

## What this scan cannot see
- **Syntactic tier only** (`resolver_tier: {sorted tiers}`): no macro expansion, no type resolution.
- **Structural call edges** are name-based/semantic and may over/under-approximate; not resolved.
- **Static cost is a heuristic**, not a duration; a high rank is a hypothesis to profile.
- **Partial or failed parses** and unresolved includes reduce coverage; the counts above say by how much.
- **Findings are candidates** for human or agent review, never defect claims.
````

Rules:

- **Content varies, structure does not.** Heading text, section order, bullet labels and the caveat
  bullet **set** are fixed (the tier bullet is conditional; see below). Counts, rows and stubs change
  with data.
- **Stubs** (exact strings): `no findings — nothing to rank on this scan.`,
  `no functions recorded — nothing to queue.`, `no rules fired — nothing to index.`,
  `no enclosing function recorded for this location`, `no curated hint for this rule yet`,
  `parse: not recorded`. (Entry/`no resolved dependents`/`no cost` stubs
  belong to the Atlas surfaces and are **not** part of this artifact.)
- **Tier caveat is conditional, shape is fixed.** When `resolver_tiers` is exactly `["syntactic"]`, the
  caveat reads as shown. When any finding carries the `semantic` tier (a `--features semantic-clang`
  build), the same bullet reads: `- **Resolver tiers** (`resolver_tier: {sorted tiers}`): semantic
  resolution applies where libclang was available; unresolved sites still fall back to syntactic
  analysis (no macro expansion, no type inference).` The bullet count and section set do not change,
  and the MD never claims "syntactic only" while listing a semantic tier.
- **Caps** print `showing top N of M` on every capped block (including `N == M`), so the shape never
  changes with data. The two conditional tail paragraphs (`complete list is …` / `complete human list …`)
  appear only when the population exceeds the inline cap — they are part of the fixed skeleton, not
  ad-hoc prose.
- **The only table is the per-rule rollup** (≤ 67 rows, sorted by `rule_id`, counts over **all**
  actions); the MD uses no other tables (tables cost tokens and break on long paths).
- **Byte budget + fixture scope:** `agent-report.md` ≤ 80 KB on the **named fixtures** (§12) and on any
  scan whose paths are ≤ 200 B, function names ≤ 64 B, titles ≤ 120 B, messages ≤ 500 B and total
  evidence ≤ 300 B per
  action (C2 asserts this
  with a synthetic at-width fixture). No truncation is ever applied: inputs wider than that are still
  rendered correctly but may exceed the target, and the JSON remains authoritative. Measured as
  `md.len()` in bytes in C2's test.
- **No HTML, no SVG, no charts, no embedded data blobs.**
- **Anchor links are relative** (`findings-detail.md#…`) so the directory stays portable.
- **Escaping:** every interpolated value passes §9.9 before it enters the MD; JSON keeps the raw value.

## 8. Fix hints

**Policy**

- One curated sentence per catalog rule, joined by `rule_id`; **the renderer never invents a fix**.
- Imperative, ≤ 20 words, names the concrete code change, no API that the rule does not report.
- **No time claim and no time unit** (`ms`, `µs`, seconds as a cost unit) and none of the claim words
  `runtime`, `speedup`, `latency`, `throughput`.
- **No certainty**: no "will", "guarantees", "fixes", "must"; the header owns the candidate framing.
- Unknown `rule_id` → `fix_hint: null` in JSON and the stub line in MD (never a guessed hint).
- **Completeness lock:** a test asserts the table covers exactly the 67 ids in
  `HEURISTIC_CATALOG_V1`; a new catalog rule without a hint fails the test. Hints are data, not logic:
  changing wording is not a contract change, but the lock keeps the set aligned.

**Table**

| rule_id | fix_hint |
|---|---|
| PERF-C-ALLOC-001 | Hoist the allocation out of the loop, reuse a buffer across iterations, or reserve once before the loop. |
| PERF-C-ALLOC-002 | Move the free outside the loop or reuse a single allocation per iteration instead of freeing each pass. |
| PERF-C-ALLOC-003 | Replace additive realloc growth with geometric growth (double capacity) or one pre-sized allocation. |
| PERF-C-STR-001 | Track the destination length and append with bounded writes instead of rescanning the string each pass. |
| PERF-C-STR-002 | Compute the length once before the loop and reuse it while the receiver is unchanged. |
| PERF-C-STR-003 | Bound every copy with an explicit destination size (snprintf/strlcpy) and pre-size the buffer once. |
| PERF-C-LOOP-001 | Flatten the nesting, move loop-invariant work outward, or replace the innermost scan with a lookup table. |
| PERF-C-LOOP-002 | Hoist handle or regex setup out of the loop, or batch element-wise transfers into block reads and writes. |
| PERF-C-LOOP-003 | Read or write in blocks with fread/fwrite instead of one character per call. |
| PERF-C-LOOP-004 | Hoist the divisor when it is loop-invariant, or precompute its reciprocal once. |
| PERF-C-CALL-001 | Add an iterative form or memoise results so the same inputs are not recomputed down the call chain. |
| PERF-C-CALL-002 | Extract branches into helpers, replace long dispatch chains with a table, and return early. |
| PERF-C-ARCH-001 | Break the cycle by moving shared declarations into a leaf header and reduce fan-out through a facade. |
| PERF-C-ARCH-002 | Treat the hottest component as the first refactor target: split its costliest files and narrow its interface. |
| PERF-C-ARCH-003 | Confirm the coupled edge with a semantic check, then narrow the interface or pass the dependency in. |
| PERF-C-TYPE-001 | Pass large structs by pointer (or const reference in C++) instead of by value. |
| PERF-C-TYPE-002 | Reorder struct members large-to-small to reduce padding, and keep the declared layout when it is an ABI contract. |
| PERF-C-MEM-001 | Size allocations as element count times element size, and use the pointee type inside sizeof. |
| PERF-C-MEM-002 | Hoist constant-size block copies out of the loop, or copy whole blocks instead of per-iteration slices. |
| PERF-C-LOOP-005 | Accumulate into a local and write the global once after the loop. |
| PERF-C-LOOP-006 | Hoist the function-pointer lookup out of the loop and prefer a direct call in the hot body. |
| PERF-C-LOOP-007 | Replace the linear scan with a hash set, a sorted lookup, or a prebuilt index. |
| PERF-C-LOOP-008 | Cache the dereferenced pointer in a local and keep the hot chain contiguous in memory. |
| PERF-PY-ALLOC-001 | Build the loop-invariant container, string, or object once before the loop and reuse it. |
| PERF-PY-ALLOC-003 | Append to one accumulator, or extend from a generator, instead of rebuilding and copying the list each pass. |
| PERF-PY-ALLOC-004 | Keep the iterable lazy (generator or iterator) and materialise it once outside the loop. |
| PERF-PY-ARCH-001 | Break the import cycle by moving shared symbols into a lower module both sides can import. |
| PERF-PY-ARCH-002 | Start with the hottest package: split its largest modules and narrow their interfaces. |
| PERF-PY-ARCH-003 | Add type annotations or a semantic check to confirm the coupled name, then inject the dependency. |
| PERF-PY-ASYNC-001 | Offload the blocking call with asyncio.to_thread or switch to the client's async API. |
| PERF-PY-ASYNC-002 | Collect the awaitables and await them together with asyncio.gather when order is not required. |
| PERF-PY-CALL-001 | Rewrite the recursion iteratively or memoise results to bound recursion depth and call cost. |
| PERF-PY-CALL-002 | Split branches into helpers, use dispatch tables, and return early on the common path. |
| PERF-PY-COMPREHENSION-001 | Replace the append loop with a list comprehension. |
| PERF-PY-EXECUTOR-001 | Create the executor or event loop once outside the loop and submit work to it. |
| PERF-PY-LOOP-001 | Flatten the nesting, hoist invariant work, or replace the innermost scan with a set or dict. |
| PERF-PY-LOOP-002 | Hoist handle setup out of the loop and batch element-wise reads and writes; keep files open across iterations. |
| PERF-PY-LOOP-003 | Iterate the sequence directly or use enumerate instead of range(len(seq)). |
| PERF-PY-LOOP-007 | Use a set or dict for membership, or build an index once instead of scanning the list per iteration. |
| PERF-PY-LOOP-009 | Move the try/except outside the loop when the failure is not per-element, or isolate the risky step. |
| PERF-PY-MEM-002 | Copy once before the loop, or mutate in place through a view instead of copying per iteration. |
| PERF-PY-N1-001 | Batch the per-iteration round trip into one query with IN (...) or a JOIN, or one bulk HTTP request. |
| PERF-PY-PANDAS-001 | Vectorise the row operation with column arithmetic or numpy.where instead of iterrows or apply. |
| PERF-PY-PANDAS-002 | Collect the frames in a list and call pandas.concat once after the loop. |
| PERF-PY-STR-001 | Collect the pieces in a list and join once after the loop. |
| PERF-RS-ALLOC-001 | Hoist construction out of the loop, reuse one buffer, or reserve capacity once before the loop. |
| PERF-RS-ALLOC-002 | Drop once outside the loop, or reuse the value instead of dropping it each iteration. |
| PERF-RS-ALLOC-003 | Reserve capacity once before the loop, or grow it multiplicatively, instead of reserving per pass. |
| PERF-RS-ARCH-001 | Split the module chain, move shared items into a leaf module, and break the cycle. |
| PERF-RS-ARCH-002 | Refactor the hottest component first: split its costliest modules and reduce cross-module calls. |
| PERF-RS-ARCH-003 | Resolve the callee explicitly (direct import or trait bound) and narrow the cross-module call. |
| PERF-RS-CALL-001 | Use an explicit worklist or memoise to remove the recursive call from the hot path. |
| PERF-RS-CALL-002 | Split the function, replace match and if chains with dispatch, and return early. |
| PERF-RS-LOOP-001 | Flatten the nesting, hoist invariant work, or replace the innermost scan with a set. |
| PERF-RS-LOOP-002 | Hoist setup (File, Command, env) out of the loop and batch streaming calls into block reads or writes. |
| PERF-RS-LOOP-003 | Read into a buffer and process chunks instead of one character or byte per iteration. |
| PERF-RS-LOOP-004 | Hoist the divisor when it is loop-invariant, or precompute its reciprocal once. |
| PERF-RS-LOOP-005 | Accumulate in a local and write the static once after the loop. |
| PERF-RS-LOOP-006 | Hoist the function-pointer or trait-object dispatch out of the loop, or monomorphise the hot call. |
| PERF-RS-LOOP-007 | Use a HashSet or BTreeSet for membership, or build an index once before the loop. |
| PERF-RS-LOOP-008 | Cache the dereferenced field in a local and keep the hot data contiguous (Vec over linked nodes). |
| PERF-RS-MEM-001 | Divide the byte count by size_of::<T>() when passing it as an element count. |
| PERF-RS-MEM-002 | Copy the block once before the loop, or extend_from_slice into a pre-sized target. |
| PERF-RS-STR-002 | Compute len, is_empty, or capacity once before the loop and reuse the value. |
| PERF-RS-STR-003 | Replace the unsafe unbounded operation with a checked slice API, or assert the length at the boundary. |
| PERF-RS-TYPE-001 | Pass large structs by reference (&T) instead of by value. |
| PERF-RS-TYPE-002 | Order fields large-to-small, or use #[repr(C)] when the layout is part of an FFI contract. |

`PERF-RS-STR-001` is **retired** (reserved id, no emitter): it has no hint and is not in the lock set.

## 9. Units, anchors, provenance

1. **Cost anchor** — verbatim `views::ScaleOrdering::anchor`: `{cost} static cost units (v2|v1)` +
   ` · {ratio}× median` (only when the median is > 0) + ` · top P%`. Reuse the shipped function; do not
   re-derive a second median or a second percentile implementation (D5/D6).
2. **Multiple** — `ratio_label`: `<10×` one decimal (`5.0×`, `8.7×`); `≥10×` integer (`24×`); no
   thousands separators, no abbreviation. Inherited wording (e.g. `1 static cost units (v2)`) is kept
   for parity; a wording fix is a shared-formatter change outside this contract.
3. **Top percentile** — `max(1, round(100·rank/N))` inside the row's own scale population, rank 1-based.
4. **Median** — nearest-rank over all members of the same scale, zeros included, lower-middle for even
   N (the shipped `median()`, not the interquartile mean).
5. **Location** — `path:line` in MD (1-based `start_line`); `path:start_line:start_col` for
   `finding_id`. Paths are repo-relative POSIX, printed in full (no truncation, no ellipsis).
6. **`detail_anchor`** — the shipped `finding_anchor` slug (`finding-` + every non-ASCII-alphanumeric
   replaced by `-`, no collapsing), plus the shipped collision suffixing (`-2`, `-3`, … assigned over
   the `finding_id`-ascending occurrence order). C2 must reuse the shipped assignment (make it
   `pub(crate)` during the post-step-12 wiring) or duplicate it with an equality test — a second,
   subtly different slug is a defect.
7. **No time.** No time unit (`ms`, `µs`, seconds as a cost unit) and none of the claim words
   `runtime`, `speedup`, `latency`, `throughput` in C-authored strings. Cost is static cost units; the
   caveat block states this once per artifact. **Scan definition (testable):** a case-insensitive
   word-boundary regex `\b(ms|µs|seconds|speedup|latency|throughput|runtime)\b` over every MD line **except**
   the analyser-provenance lines (`- why:`, `- what:`, `- function:`, `- evidence:`, `- anchor:`,
   the numbered profile rows, the `### ` action headings, rollup `| PERF-` rows) and the rule
   definitions in this spec. The evidence reproducer implements
   exactly this predicate.
8. **No bare number.** A cost, multiple, share or count carries its unit/anchor (ordinal ranks exempt).
   The shipped `top P%` keeps its shipped wording (no population noun); that inherited label is the
   format of record, and the population is named in the section heading/cap line.
9. **Text safety (MD only).** Every scanner-controlled value (`path`, `root_path`, `symbol`,
   `function.name`, `title`, `why`, evidence keys/values, `action_id`, `rule_id`, `path_class`,
   `schema_version`, `tool.name`, `tool.version`) is rendered single-line and
   structure-safe before interpolation.
   - **Always:** `\r`/`\n`/`\t` → the two-character escapes `\r`/`\n`/`\t`; every backtick → `ˋ`
     (U+02CB MODIFIER LETTER GRAVE ACCENT). **Backslash does not escape a backtick inside a CommonMark
     code span** — a `` \` `` still terminates the span — so the character itself is substituted. The
     substitution is lossy by design; the JSON keeps the original.
   - **Inside a code span** (path, id, name, evidence): `<`/`>` stay literal — the span is inert.
   - **Prose fields** (`title`, `why`): `<` → `&lt;`, `>` → `&gt;` (no raw HTML can be injected).
    - **Table cells only:** `\` → `\\`, then `|` → `\|` (a cell ending in an unescaped backslash would
      otherwise swallow the delimiter).
   - **Not escaped:** C-authored fixed strings (the 67 `fix_hint` sentences, headers, caveat bullets) —
     they are trusted content, not scanner data.
   This prevents a crafted path or message from forging a heading, closing a code span, breaking the
   rollup table or injecting HTML. `agent-actions.json` keeps the raw value (D7) — escaping is a
   rendering rule, not a data change.

## 10. `data_quality` (normative)

| Field | Source | Definition | Null/stub rule |
|---|---|---|---|
| `parse_health` | `parse_health_summary` | `{ok, partial, failed}` file counts | **`null`** when the summary is absent (the MD prints `parse: not recorded`); `{0,0,0}` only when the summary is present and zero |
| `unresolved_includes` | `graphs.include_graph.edges[].resolved` | `unresolved` = edges with `resolved == false`; `total` = all include edges | `{0,0}` when no graph |
| `structural_call_edges` | `graphs.call_graph.edges[].confidence` | total + HIGH/MEDIUM/LOW counts | `{0,0,0,0}` when no graph |
| `non_production_files` | `files[].path_class` | files whose class is not `production` | `0` |
| `startup_functions` | `stats.functions[].entry_kind` | functions with `main` or `script` | `0` |
| `no_loop_functions` | `stats.functions[].max_loop_depth` | functions with `max_loop_depth == 0` | `0` |
| `cost_scale` | `stats.functions[].function_cost_v2` | `{v2_functions, v1_functions}` | `{0,0}` |
| `unknown_trip_functions` | `stats.functions[].unknown_trip_loops` | functions with at least one unknown trip count | `0` |
| `component_findings_excluded` | `findings[].rule_id` | count of `-ARCH-002` findings excluded from actions | `0` |
| `resolver_tiers` | `findings[].resolver_tier` | sorted unique tiers present | `["syntactic"]` when no findings (the scan tier is syntactic-only in v1) |
| `caps` | contract | `{actions_detail: 25, profile_inline: 25}` — the **MD** caps; the JSON action list is uncapped and `profile_queue_cap` (top level) is the JSON profile cap | constant |

`scan.loc_total` is `object | null`; a zero-file scan omits it in `metrics.json`, so the envelope carries
`null` (the MD header does not print LOC, so no stub is needed there). `scan.languages` is **always
verbatim**: the producer emits the supported set on a zero-file scan (`src/aggregator/mod.rs:205-215`),
and the MD prints the array as-is — the file counts, not a special-cased language stub, tell the reader
the scan is empty.

The MD prints every field in §7's order; the JSON prints them as objects/numbers/`null`. A field is never
omitted: a zero is printed as zero, an absent source as its stub or `null`.

## 11. State matrix (fixed shape)

| State | Behavior |
|---|---|
| application scan | full blocks |
| library-root (no production main) | full blocks; entry policy is not an agent-report field |
| no-entry scan | full blocks; no entry claim anywhere |
| empty scan (0 files) | all seven sections; `no findings`, `no functions`, `no rules fired`; `parse: not recorded` when the summary is absent; `languages` prints the producer's supported set verbatim; `scan.loc_total: null` in JSON |
| all-test/bench scan | actions and profile rows carry `path_class` in both surfaces; the demotions line prints the non-production/startup/no-loop counts |
| capped/large scan (thousands of findings) | top 25 detail + rollup over all actions + `showing top 25 of N`; JSON complete |
| partial-parse scan | counts in data quality (or `parse: not recorded`); no silent drop |
| single-file scan | full blocks; rank/anchors over the one-file population |
| v1-only cost scan | anchors read `static cost units (v1)`; no v2 multiple shares a row; `function.cost` carries the v1 chart-side value |
| semantic-clang scan | full blocks; `resolver_tiers` lists `semantic`; the tier caveat switches to the resolver-tier wording (§7) — no "syntactic only" claim |
| all-component scan (every finding is `-ARCH-002`) | full blocks; the action list and rollup are empty (fixed stubs); `component_findings_excluded = N` carries the disclosure |

## 12. Determinism and size

- **No clock provenance** (D3). `scan.root_path` is the verbatim CLI argument; byte-identity is claimed
  for **repeated runs of the same command** (the C3 test scans the same fixture path twice). Changing the
  argument (`.`, `./`, an absolute path, a symlink) changes `root_path` by design — it is provenance,
  not a tree-derived hash.
- Sorted traversals everywhere (`BTreeMap`/sorted keys); no `HashMap` iteration; no locale formatting;
  no floating-point rounding differences (reuse the shipped formatter).
- **Byte-identical double run** is a hard test: two renders of the same document produce identical bytes
  for both artifacts, with **no normalization**.
- **Size:** the hard assertions are:
  - `agent-report.md` ≤ 80 KB (81,920 B) on the **committed golden fixtures** (`tests/golden/report`,
    `report-mixed`, `report-python`, `report-rust`), always run in `cargo test`.
  - the **synthetic at-envelope fixture**: exactly **25 detail actions + 67 rollup rules + 25 profile
    rows**, with paths = 200 B repeated at every rendering site (heading, function line, `action_id`),
    titles = 120 B, function names = 64 B, messages = 500 B, evidence = 300 B/action, symbols = 64 B.
    This is the worst-case
    width × cap combination, and it must stay ≤ 80 KB (81,920 B).
  - **Measured at C2 (2026-10-24):** `report` 13,469 B · `report-mixed` 8,889 B · `report-python`
    26,889 B · `report-rust` 25,848 B · synthetic 76,416 B (budget 81,920 B). The unit test asserts these
    exact sizes so a wording change that moves them is a deliberate contract edit.
  - the three external scans (`~/Opencode/perfpal-test/out-{cJSON,hyperfine,bat}`) are an **optional
    manual check** (`PERFPAL_EXTERNAL_SCANS=1 cargo test --locked --test agent_report -- --nocapture`),
    not part of the required gate. A fresh checkout or CI therefore gates on the committed fixtures,
    never on a user-local path.
  **This is a fixture-and-envelope guarantee, not a theorem for arbitrary input.** Real scans are
  expected to land well under the budget (hyperfine's 348 actions ≈ 30 KB). Inputs beyond the envelope
  are rendered correctly with **no truncation**, may exceed 80 KB, and `agent-actions.json` is then
  authoritative. JSON grows O(actions) with the 100-row profile cap.
- **Amended at C2 kickoff (founder, 2026-10-24): the budget is 80 KB (81,920 B).** The original 64 KB
  figure and the 200 B path envelope were chosen independently and cannot both hold: the §12 synthetic
  at the literal width measures **76,416 B**, so the budget was raised to keep the worst-case width
  honest (5,504 B headroom, 6.7% of the budget) rather than shrink the fixture. The C1 evidence
  (`derive_sample.py`, `c1-gate.sh`) keeps its local 64 KB constant as historical evidence; the
  committed sample is 8,889 B, so those checks are unaffected. This is a bound change, not a shape
  change: `contract_version` stays 1.

## 13. Downstream eval (protocol; results gate C3, not CI)

**Question:** does an agent given the repo + `agent-report.md` fix the intended issue?

- **Fixtures:** `cJSON`, `hyperfine`, `bat` (`~/Opencode/perfpal-test/`, already scanned). The vaulted
  Track-A holdouts are **not** used.
- **Tasks:** two per fixture (N = 6): (T1) "apply the top-ranked action"; (T2) "apply one action from the
  top 3, your choice". The agent sees only the repo + `agent-report.md` (no `metrics.json`, no
  `report.md`, no `findings-detail.*`).
- **Metrics (definitions):** *top-1 hit* = T1's edited file matches the rank-1 action's `path`;
  *top-3 hit* = T2's edited file matches one of ranks 1–3; *hallucination rate* = unsupported cited
  claims ÷ all cited claims in the final message (a cited claim is unsupported when its `path:line`,
  `rule_id` or evidence value is absent from the report or invented); *time-to-fix* = wall clock from
  the task prompt to the agent's final message.
- **Proposed thresholds (founder ratifies at C3, N-aware):** top-1 ≥ 4/6, top-3 ≥ 6/6, hallucination
  ≤ 10% of cited claims, and each task ≤ 10 min (report the median of the six; no per-task median).
- **Recording:** `docs/evidence/agent-report/eval/README.md` + one transcript per task, with the model
  id/version, the exact prompt, the seed (if any) and the `agent-report.md` SHA-256. The eval is
  **manual/external** (this product makes no LLM calls) and is **not** a CI gate.
- **Weakest assumption (stated in the evidence):** analyzer precision is sufficient for an agent to act
  without triage. If it is not, the artifact is a **ranked triage queue**, and the eval reports it that
  way — it does not silently raise the ranking's authority.

## 14. Acceptance

**C2 (unit)**

1. Rank parity: `actions` order == `views::complete_actions` order; profile order ==
   `views::complete_profile_queue` order (element-wise, same fixtures).
2. `actions.len() == complete_action_count`; `profile_queue.len() ==
   min(profile_queue_cap, complete_profile_count)`; ranks are `1..=N` with no gap or duplicate.
3. Hint lock: the `AGENT_FIX_HINTS` key set **equals** the 67 `HEURISTIC_CATALOG_V1` ids (set equality,
   not a count); an unknown id yields `null` + the MD stub.
4. Anchor parity: every `cost_anchor` byte-equals the shipped `views` anchor for the same function.
5. Determinism: double render byte-equal; `scan` has no `started_at`/`duration_ms`; integral costs
   serialize without a fractional part.
6. Size bound: MD ≤ 80 KB on each committed golden fixture and on the §12 synthetic fixture
   (25 actions + 67 rollup rules + 25 profile rows at the stated widths); the external scans are the
   **C3** optional manual check in §12 (gated by `PERFPAL_EXTERNAL_SCANS=1`), not part of the C2 unit
   gate, matching plan DoD #15.
7. Forbidden strings: the §9.7 predicate over C-authored lines (including `seconds`) finds no time
   unit/claim; quoted `why`/`what`/rollup provenance lines are exempt.
8. Stubs: each stub in the §7 list is reachable from a fixture state (or from a direct renderer call
   for the `null` parse-health case).
9. Profile contract: every row carries the §6.2 keys with the declared types; `function_name` is a
   string (never an object).
10. Text safety: a fixture with a path containing `\n`, a backtick and `<script>` renders one line, no
    heading injection, and no executable/raw HTML (values inside inline code spans stay literal and inert
    per §9.9; prose and table cells are entity-escaped); the fixture also carries a hostile `rule_id` and
    `path_class`; the backtick renders as `ˋ` (U+02CB) so no code span can be closed; the JSON keeps the
    original bytes.

**C3 (integration)**

11. Double run of the same fixture tree → byte-identical `agent-report.md` + `agent-actions.json`.
12. Sample regen: the committed `docs/evidence/agent-report/sample/*` **byte-equals** the renderer
    output (empty diff, asserted by a test).
13. State matrix: every state in §11 (eleven rows) renders the fixed skeleton (section/heading assertions).
14. Artifact set: a scan emits both files; no schema file, no `Cargo.toml`/`Cargo.lock`, no `deny.toml`
    change; no `<script>`/network anywhere in the artifacts.
15. A's step-12 link slot resolves to an existing `agent-report.md` (A owns the link line; C verifies it
    with a rendered-output assertion).
16. Downstream eval recorded (§13) or explicitly deferred with the founder's words in the handoff.

**DoD commands** live in `docs/plans/agent-report.md` and must exit 0.

## 15. Non-goals

- No new heuristics, no severity/confidence re-scoring, no new weights.
- No runtime claims, no time conversion, no profiler data.
- No charts, HTML, JS, network, LLM calls, telemetry.
- No `metrics.json` schema change and no second ranked list inside it.
- No CLI flag/mode switch; no rename of existing artifacts.
- No edits to A surfaces (`atlas*.rs`, Atlas tests/goldens, `docs/specs/atlas-*`,
  `docs/plans/atlas-build.md`) or D surfaces; `mod.rs`/`cli` wiring only after A step 12.
- No merge to `main` and no push in C1.

## 16. Open items (founder-ratify at C2 kickoff)

1. `profile_queue_cap = 100` for the JSON (vs 50/200) — token/utility trade-off.
2. Fix-hint wording review (§8): 67 sentences; the founder may edit any wording before C2 transcribes it.
3. Eval thresholds (§13) at N = 6: top-1 ≥ 4/6, top-3 ≥ 6/6, hallucination ≤ 10%, per-task ≤ 10 min.
4. Whether A links `agent-report.md` from `report.html` only or also from `report.md` (A owns).
5. Shared-formatter consolidation with `atlas_metrics` (separators, singular unit) — deferred to a
   contract bump so C never diverges from the shipped key views today.
6. Whether the synthetic at-envelope fixture (§12) becomes a committed golden or stays a generated
   test input, and whether the three external `perfpal-test` scans are vendored for CI.