# Perfpal Report
> schema_version 1\.8 · perfpal 0\.4\.0 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 17 files

## 1. Executive summary
11 findings: 0 hot, 5 warn, 6 info. Parse health: 17 ok, 0 partial, 0 failed.

### Where do I look first?

Ranked: production code first, then severity, then confidence, then static cost.

1. **Linear membership scan inside loop** ([`PERF\-JS\-LOOP\-007:06\_membership\_scan\.ts:8:9`](findings-detail.html#finding-PERF-JS-LOOP-007-06-membership-scan-ts-8-9)) — `06\_membership\_scan\.ts:8` · function `intersect` · warn · MEDIUM · production
   - Cost: `13 static cost units (v2) · 13× median · top 15%`
   - Why: 'seen\.includes' scans the collection linearly on every iteration at loop depth 1 — the nested linear scan makes the loop O\(n²\); building a \`Set\`/\`Map\` once and doing a keyed lookup is a candidate optimization\. Accepted FP class: small \(\< 16\) collections
2. **\`await\` inside a loop** ([`PERF\-JS\-ASYNC\-001:07\_async\_await\.ts:12:19`](findings-detail.html#finding-PERF-JS-ASYNC-001-07-async-await-ts-12-19)) — `07\_async\_await\.ts:12` · function `loadAll` · warn · MEDIUM · production
   - Cost: `11 static cost units (v2) · 11× median · top 17%`
   - Why: an \`await\` runs inside a loop at depth 1 — awaiting each item sequentially serialises one microtask round\-trip per iteration; collecting the promises and awaiting \`Promise\.all\(items\.map\(\.\.\.\)\)\` is a candidate optimization\. Accepted FP class: deliberately sequential work \(rate limiting, ordering\)
3. **Synchronous blocking I/O inside loop** ([`PERF\-JS\-LOOP\-002:05\_sync\_io\_loop\.ts:9:17`](findings-detail.html#finding-PERF-JS-LOOP-002-05-sync-io-loop-ts-9-17)) — `05\_sync\_io\_loop\.ts:9` · function `loadAll` · warn · MEDIUM · production
   - Cost: `10 static cost units (v2) · 10× median · top 20%`
   - Why: synchronous 'readFileSync' runs inside a loop at depth 1 — each iteration performs a blocking syscall/serialization on the single event\-loop thread; batching the work, using the async API \(\`fs/promises\`, async \`exec\`\), or making one call outside the loop is a candidate optimization\. Accepted FP class: a startup\-only loop
4. **String accumulation with \`\+=\` inside loop** ([`PERF\-JS\-STR\-001:03\_string\_build\.ts:8:5`](findings-detail.html#finding-PERF-JS-STR-001-03-string-build-ts-8-5)) — `03\_string\_build\.ts:8` · function `render` · warn · MEDIUM · production
   - Cost: `7 static cost units (v2) · 7.0× median · top 24%`
   - Why: 'out' accumulates a string with '\+=' or 'out = out \+ \.\.\.' inside a loop at depth 1 — this builds a cons\-string chain; V8 ropes mitigate short chains, but flattening the rope at first use is linear and repeated concatenation is quadratic; collecting parts into an array and joining once \(\`parts\.join\(""\)\`\) is a candidate optimization\. Accepted FP class: a tiny bounded loop \(\< 8 iterations\)
5. **Synchronous blocking call inside \`async\` function** ([`PERF\-JS\-ASYNC\-002:08\_async\_blocking\.ts:7:10`](findings-detail.html#finding-PERF-JS-ASYNC-002-08-async-blocking-ts-7-10)) — `08\_async\_blocking\.ts:7` · function `readConfig` · warn · MEDIUM · production
   - Cost: `1 static cost units (v2) · 1.0× median · top 70%`
   - Why: synchronous 'readFileSync' is called inside an \`async\` function — the blocking call stalls the single event\-loop thread for every concurrent task; using the async equivalent \(\`fs/promises\`, async \`exec\`/\`spawn\`\) is a candidate optimization\. Accepted FP class: a CLI single\-shot path

showing top 5 of 11 actions

[Complete ranked list](findings-detail.html#ranked-actions) — all 11 ranked actions (11 findings, production-first).

0 non-production files demoted · 0 component-level findings in Explorer.

### What breaks if I touch it?

- `04\_deep\_nesting\.ts` — 1 dependent · 6% of scanned files · top 6%
- `10\_cycle\_a\.ts` — 1 dependent · 6% of scanned files · top 12%
- `11\_cycle\_b\.ts` — 1 dependent · 6% of scanned files · top 18%
- `13\_controls\.ts` — 1 dependent · 6% of scanned files · top 24%

showing top 4 of 4 files with resolved dependents · 0 non-production files demoted

Blast-radius bars: [report.html#map](report.html#map). resolved 4 of 4 include edges.

### Where might time plausibly go?

Bars scale within each block: blue = ranked functions, grey = demoted functions.

- `06\_membership\_scan\.ts:19` `smallWindow` — 19 static cost units (v2) · 19× median · top 2% · known trip ×4 — verify it actually runs
- `04\_deep\_nesting\.ts:7` `score` — 18 static cost units (v2) · 18× median · top 4% · unknown-trip ×2 — estimate incomplete
- `03\_string\_build\.ts:13` `tinyPrefix` — 15 static cost units (v2) · 15× median · top 6% · known trip ×5 — verify it actually runs
- `01\_alloc\_churn\.ts:11` `buildEnvelopes` — 13 static cost units (v2) · 13× median · top 7% · unknown-trip ×1 — estimate incomplete
- `01\_alloc\_churn\.ts:19` `buildIndexed` — 13 static cost units (v2) · 13× median · top 9% · unknown-trip ×1 — estimate incomplete

showing top 5 of 54 functions

[Complete cost-ranked list (54 functions)](profile-queue.md).

1 startup function demoted · 39 no-loop functions demoted · 0 non-production files demoted.

## 2. Scan provenance & parse health
- Tool: perfpal 0\.4\.0
- Scan root: `<ROOT>`
- Files scanned: 17

### Parse health summary

| status | files |
|---|---:|
| ok | 17 |
| partial | 0 |
| failed | 0 |

No failed or partially parsed files.

Findings overlapping parse-error regions are flagged (`error_node_overlap`) and confidence-capped — never silently dropped.

## 3. Codebase statistics
- Languages: ts, tsx
- Total LOC: 198 code / 82 comment / 46 blank

Top files by code LOC:

| path | loc.code | loc.comment | loc.blank | functions |
|---|---:|---:|---:|---:|
| 14\_types\_only\.ts | 32 | 6 | 9 | 3 |
| 01\_alloc\_churn\.ts | 24 | 7 | 4 | 4 |
| 03\_string\_build\.ts | 21 | 7 | 4 | 5 |
| 06\_membership\_scan\.ts | 18 | 6 | 3 | 4 |
| 02\_array\_accumulation\.ts | 15 | 6 | 3 | 4 |
| 04\_deep\_nesting\.ts | 15 | 5 | 3 | 3 |
| 07\_async\_await\.ts | 12 | 5 | 3 | 4 |
| 13\_controls\.ts | 11 | 7 | 3 | 4 |
| 09\_recursion\.ts | 10 | 5 | 3 | 3 |
| 05\_sync\_io\_loop\.ts | 9 | 5 | 2 | 3 |

Distribution — cyclomatic\_complexity:

| bucket | count |
|---|---:|
| 01\-04 | 54 |
| 05\-09 | 0 |
| 10\-14 | 0 |
| 15\-24 | 0 |
| 25\-49 | 0 |
| 50\+ | 0 |

Distribution — max\_loop\_depth:

| bucket | count |
|---|---:|
| 0 | 39 |
| 1 | 14 |
| 2 | 1 |
| 3 | 0 |
| 4 | 0 |
| 5 | 0 |
| 6\+ | 0 |

## 4. Findings
11 rules: 11 of 11 total findings (0 hot, 5 warn, 6 info).

| rule_id | findings | hot | warn | info | top exemplar |
|---|---:|---:|---:|---:|---|
| `PERF\-JS\-ASYNC\-001` | 1 | 0 | 1 | 0 | \`await\` inside a loop (`PERF\-JS\-ASYNC\-001:07\_async\_await\.ts:12:19`) — `07\_async\_await\.ts:12:19` |
| `PERF\-JS\-ASYNC\-002` | 1 | 0 | 1 | 0 | Synchronous blocking call inside \`async\` function (`PERF\-JS\-ASYNC\-002:08\_async\_blocking\.ts:7:10`) — `08\_async\_blocking\.ts:7:10` |
| `PERF\-JS\-LOOP\-002` | 1 | 0 | 1 | 0 | Synchronous blocking I/O inside loop (`PERF\-JS\-LOOP\-002:05\_sync\_io\_loop\.ts:9:17`) — `05\_sync\_io\_loop\.ts:9:17` |
| `PERF\-JS\-LOOP\-007` | 1 | 0 | 1 | 0 | Linear membership scan inside loop (`PERF\-JS\-LOOP\-007:06\_membership\_scan\.ts:8:9`) — `06\_membership\_scan\.ts:8:9` |
| `PERF\-JS\-STR\-001` | 1 | 0 | 1 | 0 | String accumulation with \`\+=\` inside loop (`PERF\-JS\-STR\-001:03\_string\_build\.ts:8:5`) — `03\_string\_build\.ts:8:5` |
| `PERF\-JS\-ALLOC\-001` | 1 | 0 | 0 | 1 | Loop\-invariant object/array literal inside loop (`PERF\-JS\-ALLOC\-001:01\_alloc\_churn\.ts:14:20`) — `01\_alloc\_churn\.ts:14:20` |
| `PERF\-JS\-ALLOC\-003` | 1 | 0 | 0 | 1 | Superlinear spread accumulation in loop (`PERF\-JS\-ALLOC\-003:02\_array\_accumulation\.ts:8:11`) — `02\_array\_accumulation\.ts:8:11` |
| `PERF\-JS\-ARCH\-001` | 1 | 0 | 0 | 1 | Import cycle (`PERF\-JS\-ARCH\-001:11\_cycle\_b\.ts:3:1`) — `11\_cycle\_b\.ts:3:1` |
| `PERF\-JS\-CALL\-001` | 1 | 0 | 0 | 1 | Direct recursion without a visible base case (`PERF\-JS\-CALL\-001:09\_recursion\.ts:10:10`) — `09\_recursion\.ts:10:10` |
| `PERF\-JS\-LOOP\-001` | 1 | 0 | 0 | 1 | Nested loop depth ≥ 2 (`PERF\-JS\-LOOP\-001:04\_deep\_nesting\.ts:10:5`) — `04\_deep\_nesting\.ts:10:5` |
| `PERF\-JS\-TYPE\-001` | 1 | 0 | 0 | 1 | \`delete\` on an object property (`PERF\-JS\-TYPE\-001:12\_delete\.ts:6:3`) — `12\_delete\.ts:6:3` |

All 11 of 11 findings: [findings-detail.html](findings-detail.html) — complete ranked action list and per-finding detail (11 rules).

## 5. Explorer
[Dependency graph — internal import/require dependencies between scanned files.](charts/include-graph.svg)
[File hot-path map — top files by degree with reachable reserve, heat by candidate function cost (v2 where available; v1 fallback); highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)
[Entry-stratified cost map — files stratified by resolved entry reachability, heat by candidate function cost; LOW-confidence call edges excluded.](charts/layer-entry.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by candidate finding density.](charts/hot-components.svg)
[Workspace map — components as nodes over inter-component call/include edges, heat by candidate finding density.](charts/layer-workspace.svg)
[Component bands — files grouped under capped components, cross-component file edges aggregated as counted bands.](charts/layer-bands.svg)
[Findings map — severity-weighted finding density (hot 3 / warn 2 / info 1 per candidate), dots = severity fill + confidence ring; profile to confirm.](charts/layer-findings.svg)

Component ranking (hottest first — showing top 1 of 1 components):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| \. | \. | 1 | 55.56 |

Component detail pages (one per component above):

[Component \. — per-file hot-path map.](charts/component-01-root.svg)

The dependency graph reflects resolved internal include/module edges between scanned files — JavaScript/TypeScript `import`/`require` bindings; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
Coupling heat blends edge counts with per-function cost (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / alloc_family; each edge weighs 1 + mean endpoint cost normalized by the scan maximum) — charts remain pure functions of metrics.json.
Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic DOT export of the include/call/component graphs (same source data as the SVGs above).
Standing caveat: cross-file call edges are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
| rule_id | title | description |
|---|---|---|
| `PERF\-JS\-ALLOC\-001` | Loop\-invariant object/array literal inside loop | Detects an object/array literal constructed inside a loop at depth ≥ 1 behind a mandatory loop\-invariance/escape gate: silent when a referenced name is the nearest loop's induction variable or is mutated inside that loop, and silent when no enclosing loop contains the span\. Mechanism: V8 young\-generation allocation churn that escape analysis rarely elides; fix: hoist the invariant construction\. Accepted FP class: a literal created once for another reason but mutated per iteration\. |
| `PERF\-JS\-ALLOC\-003` | Superlinear \`concat\`/spread/\`slice\` accumulation in loop | Detects \`concat\`/\`slice\` called inside a loop on a non\-empty receiver that the same loop reassigns \(\`acc = acc\.concat\(x\)\`\), or an array literal with a spread that folds a loop\-reassigned accumulator \(\`acc = \[\.\.\.acc, x\]\`\) — the accumulator pattern\. Each pass copies n elements, making the loop O\(n²\); fix: push into one array or concat/spread once after the loop\. Accepted FP class: a deliberate immutable snapshot per iteration\. Info\. |
| `PERF\-JS\-ARCH\-001` | Import cycle | Detects cycles over RESOLVED relative\-import edges, one finding per broken edge anchored at its earliest directive span\. Mechanism: module\-init ordering becomes observable and bundlers cannot tree\-shake the cycle cleanly; fix: break the lowest\-sorted edge into a shared leaf module\. Accepted FP class: type\-only import cycles \(TypeScript erases them\)\. Info\. |
| `PERF\-JS\-ASYNC\-001` | \`await\` inside a loop | Detects an \`await\` at loop depth ≥ 1 inside an \`async\` function — awaiting each item sequentially serialises one microtask round\-trip per iteration; fix: collect the promises and await \`Promise\.all\(items\.map\(\.\.\.\)\)\`\. Accepted FP class: deliberately sequential work \(rate limiting, ordering\)\. Flagship, Warn\. |
| `PERF\-JS\-ASYNC\-002` | Synchronous blocking call inside \`async\` function | Detects a known synchronous Node API \(\`readFileSync\`, \`execSync\`, sync \`crypto\`, …\) called anywhere in an \`async\` function — the blocking call stalls the single event\-loop thread for every concurrent task; fix: use the async equivalent \(\`fs/promises\`, async \`exec\`/\`spawn\`\)\. Async\-function gate mandatory; accepted FP class: a CLI single\-shot path\. Flagship, Warn\. |
| `PERF\-JS\-CALL\-001` | Direct recursion without a visible base case | Detects a bare direct self\-call \(\`receiver\_root\` absent\) ONLY when no comparison in the same function has a span strictly before the recursive call — the guard\-gated predicate for a missing visible base case\. Mechanism: stack growth plus repeated frame\-setup cost; fix: convert to a loop with an accumulator or add an explicit guard\. Accepted FP class: a recursive tree walk with small depth\. Info\. |
| `PERF\-JS\-LOOP\-001` | Nested loop depth ≥ 2 | Detects a loop nested at depth ≥ 2 \(the JS floor is 2, not the C catalog's 3\), at most one finding per function anchored at the earliest such loop\. Mechanism: every extra level multiplies the iteration count, so the body runs superlinearly; fix: flatten the nesting, hoist invariant work, or add an early exit\. Accepted FP class: matrix work where n² is inherent\. Info\. |
| `PERF\-JS\-LOOP\-002` | Synchronous blocking I/O inside loop | Detects a known synchronous Node API \(\`readFileSync\`, \`execSync\`, \`spawnSync\`, sync \`crypto\`, …\) inside a loop — one blocking syscall/serialization per iteration on the single event\-loop thread; fix: batch, use the async API, or make one call outside the loop\. Accepted FP class: a startup\-only loop\. Warn\. |
| `PERF\-JS\-LOOP\-007` | Linear membership scan inside loop | Detects \`indexOf\`/\`includes\`/\`find\`/\`findIndex\`/\`some\`/\`every\`/\`filter\` called inside a loop with a non\-empty receiver root, excluding canonical loops with a known trip count \< 16 — a linear scan per iteration makes the loop O\(n²\); fix: build a \`Set\`/\`Map\` once and do a keyed lookup\. Flagship, Warn\. |
| `PERF\-JS\-STR\-001` | String accumulation with \`\+=\` or \`x = x \+ …\` inside loop | Detects a string/template literal appended inside a loop in either form: \`s \+= "…"\` or \`s = s \+ "…"\` \(the binary \`\+\` must be the whole right\-hand side\), excluding canonical loops with a known trip count \< 8\. Mechanism: cons\-string flattening at first use \(MEDIUM; V8 ropes mitigate\); fix: collect parts into an array and \`join\(""\)\` once\. Accepted FP class: a tiny bounded loop\. Warn\. |
| `PERF\-JS\-TYPE\-001` | \`delete\` on an object property | Detects \`delete obj\.prop\`; the subscript form \(\`delete obj\["k"\]\`\) is EXCLUDED per the frozen disposition\. Mechanism: a V8 hidden\-class → dictionary\-mode transition slows every later access; fix: set the property to \`undefined\` or rebuild the object\. Accepted FP class: intentional cache eviction \(rare\)\. Info\. |

## Appendix B. How to read this report
**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or architecture impact: review `hot` findings first, schedule `warn`, and treat `info` as context signal feeding aggregation.
**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the false-positive risk of the detection shape — pure-syntax detections are HIGH, shape heuristics with documented FP classes are MEDIUM, and approximations (size models, cross-file name matching) are LOW. Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings whose evidence quality is reduced; their confidence is capped at MEDIUM.
Every finding is an **optimization candidate, never a certainty**: a candidate for human review, not a defect claim. Perfpal v1 runs on a purely syntactic tier (`resolver_tier: syntactic`) — no macro expansion, no type resolution.
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.8.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.
