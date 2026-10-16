# Perfpal Report
> schema_version 1\.4 · perfpal 0\.2\.0\-beta\.1 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 3 files

## 1. Executive summary
15 findings: 7 hot, 6 warn, 2 info. Parse health: 3 ok, 0 partial, 0 failed.
- Heap allocation inside loop (PERF\-C\-ALLOC\-001:src/main\.c:34:21)
- Heap allocation inside loop (PERF\-C\-ALLOC\-001:src/main\.c:73:15)
- Deallocation inside loop (PERF\-C\-ALLOC\-002:src/main\.c:37:9)

## 2. Scan provenance & parse health
- Tool: perfpal 0\.2\.0\-beta\.1
- Scan root: `<ROOT>`
- Files scanned: 3

### Parse health summary

| status | files |
|---|---:|
| ok | 3 |
| partial | 0 |
| failed | 0 |

No failed or partially parsed files.

Findings overlapping parse-error regions are flagged (`error_node_overlap`) and confidence-capped — never silently dropped.

## 3. Codebase statistics
- Languages: c
- Total LOC: 102 code / 29 comment / 18 blank

Top files by code LOC:

| path | loc.code | loc.comment | loc.blank | functions |
|---|---:|---:|---:|---:|
| src/main\.c | 93 | 23 | 13 | 11 |
| src/util\.h | 5 | 2 | 3 | 0 |
| src/util\.c | 4 | 4 | 2 | 1 |

Distribution — cyclomatic\_complexity:

| bucket | count |
|---|---:|
| 01\-04 | 11 |
| 05\-09 | 0 |
| 10\-14 | 0 |
| 15\-24 | 1 |
| 25\-49 | 0 |
| 50\+ | 0 |

Distribution — max\_loop\_depth:

| bucket | count |
|---|---:|
| 0 | 5 |
| 1 | 6 |
| 2 | 0 |
| 3 | 1 |
| 4 | 0 |
| 5 | 0 |
| 6\+ | 0 |

## 4. Findings
13 rules: 15 of 15 total findings (7 hot, 6 warn, 2 info).

| rule_id | findings | hot | warn | info | top exemplar |
|---|---:|---:|---:|---:|---|
| `PERF\-C\-ALLOC\-001` | 2 | 2 | 0 | 0 | Heap allocation inside loop (`PERF\-C\-ALLOC\-001:src/main\.c:34:21`) — `src/main\.c:34:21` |
| `PERF\-C\-ALLOC\-002` | 2 | 1 | 1 | 0 | Deallocation inside loop (`PERF\-C\-ALLOC\-002:src/main\.c:37:9`) — `src/main\.c:37:9` |
| `PERF\-C\-ALLOC\-003` | 1 | 1 | 0 | 0 | Realloc linear growth in loop (`PERF\-C\-ALLOC\-003:src/main\.c:73:15`) — `src/main\.c:73:15` |
| `PERF\-C\-ARCH\-002` | 1 | 1 | 0 | 0 | Hot component (`PERF\-C\-ARCH\-002:src/main\.c:84:1`) — `src/main\.c:84:1` |
| `PERF\-C\-LOOP\-002` | 1 | 1 | 0 | 0 | Expensive setup or I/O call inside loop (`PERF\-C\-LOOP\-002:src/main\.c:80:9`) — `src/main\.c:80:9` |
| `PERF\-C\-STR\-001` | 1 | 1 | 0 | 0 | String accumulation in loop (`PERF\-C\-STR\-001:src/main\.c:52:9`) — `src/main\.c:52:9` |
| `PERF\-C\-CALL\-002` | 1 | 0 | 1 | 0 | High cyclomatic complexity (`PERF\-C\-CALL\-002:src/main\.c:84:1`) — `src/main\.c:84:1` |
| `PERF\-C\-LOOP\-001` | 1 | 0 | 1 | 0 | Deeply nested loops (`PERF\-C\-LOOP\-001:src/main\.c:61:13`) — `src/main\.c:61:13` |
| `PERF\-C\-MEM\-001` | 1 | 0 | 1 | 0 | sizeof misuse in allocation argument (`PERF\-C\-MEM\-001:src/main\.c:118:19`) — `src/main\.c:118:19` |
| `PERF\-C\-STR\-003` | 1 | 0 | 1 | 0 | Unbounded string operation (`PERF\-C\-STR\-003:src/main\.c:52:9`) — `src/main\.c:52:9` |
| `PERF\-C\-TYPE\-001` | 1 | 0 | 1 | 0 | Large struct passed by value (`PERF\-C\-TYPE\-001:src/main\.c:113:1`) — `src/main\.c:113:1` |
| `PERF\-C\-ARCH\-003` | 1 | 0 | 0 | 1 | Cross\-file call coupling (`PERF\-C\-ARCH\-003:src/main\.c:127:20`) — `src/main\.c:127:20` |
| `PERF\-C\-LOOP\-004` | 1 | 0 | 0 | 1 | Variable\-divisor division in innermost loop (`PERF\-C\-LOOP\-004:src/main\.c:62:24`) — `src/main\.c:62:24` |

## 5. Architecture
[Dependency graph — internal #include dependencies between scanned files.](charts/include-graph.svg)
[File hot-path map — top files by degree, heat by function cost; highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)
[Workspace map — components as nodes over inter-component call/include edges, heat by finding density.](charts/layer-workspace.svg)

Component ranking (hottest first — showing top 1 of 1 components):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| src | src | 1 | 147.06 |

Component detail pages (one per component above):

[Component src — per-file hot-path map.](charts/component-01-src.svg)

The dependency graph reflects resolved internal include/module edges between scanned files — C `#include` directives; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
Coupling heat blends edge counts with per-function cost (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / alloc_family; each edge weighs 1 + mean endpoint cost normalized by the scan maximum) — charts remain pure functions of metrics.json.
Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic DOT export of the include/call/component graphs (same source data as the SVGs above).
Standing caveat: cross-file call edges are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
| rule_id | title | description |
|---|---|---|
| `PERF\-C\-ALLOC\-001` | Heap allocation inside loop | Detects malloc/calloc/realloc/aligned\_alloc/strdup calls nested anywhere inside a loop body\. |
| `PERF\-C\-ALLOC\-002` | \`free\(\)\` inside loop | Detects free\(\) directly inside a loop body \(paired with ALLOC\-001 it indicates allocator churn\)\. |
| `PERF\-C\-ALLOC\-003` | \`realloc\(\)\` linear growth in loop | Detects realloc\(p, n\+k\) additive\-growth shapes nested in loops — quadratic memmove behavior\. |
| `PERF\-C\-ARCH\-002` | Hot\-component aggregation | Ranks components by aggregated metrics \(complexity sum, alloc\-site density, max loop depth, finding density\)\. |
| `PERF\-C\-ARCH\-003` | Cross\-file call coupling \(name\-based\) | Detects cross\-file call edges via name matching — LOW confidence until a semantic resolver tier exists\. |
| `PERF\-C\-CALL\-002` | High cyclomatic complexity | Detects branch\-count complexity above threshold \(default ≥ 15\), reported per function\. |
| `PERF\-C\-LOOP\-001` | Deep loop nesting \(≥3\) | Detects loop nesting depth ≥ 3 — O\(n^k\) candidate zones and primary input to hotspot maps\. |
| `PERF\-C\-LOOP\-002` | Expensive setup / I/O call inside loop | Detects printf\-family, file\-I/O, getenv, system, and regex\-compilation calls nested in loops\. The message splits hoistable setup \(hoisting or batching is the candidate\) from intrinsic streaming I/O \(read/write — batching into block transfers is the candidate\); evidence carries io\_kind\. Calls in do\-while\(0\) wrapper bodies stay silent\. |
| `PERF\-C\-LOOP\-004` | Integer div/mod with variable divisor in innermost loop | Detects / or % with a non\-literal divisor inside the innermost loop body \(literal divisors skipped by design\)\. |
| `PERF\-C\-MEM\-001` | sizeof\-misuse shapes in allocation args | Detects malloc\(x \* sizeof\(ptr\)\) and sizeof misplacement shapes in allocation arguments — wrong\-size allocations\. |
| `PERF\-C\-STR\-001` | String accumulation in loop \(\`strcat\` family\) | Detects strcat/strncat with a destination reused across loop iterations \(O\(n²\) rescan\)\. |
| `PERF\-C\-STR\-003` | Unbounded copy family \(\`strcpy\`/\`sprintf\`/\`gets\`\) | Detects unbounded copy calls \(strcpy, strcat, sprintf, gets, scanf %s\) — unbounded\-latency hotspots\. |
| `PERF\-C\-TYPE\-001` | Large struct passed by value | Detects struct\-typed by\-value parameters whose TU\-defined struct reaches the approximate\-size floor \(LOW confidence — no real type layout at MVP\)\. |

## Appendix B. How to read this report
**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or architecture impact: review `hot` findings first, schedule `warn`, and treat `info` as context signal feeding aggregation.
**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the false-positive risk of the detection shape — pure-syntax detections are HIGH, shape heuristics with documented FP classes are MEDIUM, and approximations (size models, cross-file name matching) are LOW. Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings whose evidence quality is reduced; their confidence is capped at MEDIUM.
Every finding is an **optimization candidate, never a certainty**: a candidate for human review, not a defect claim. Perfpal v1 runs on a purely syntactic tier (`resolver_tier: syntactic`) — no macro expansion, no type resolution.
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.4.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.

## Appendix C. All findings (full detail)
15 of 15 total findings.

Full per-finding detail has moved to [findings-detail.md](findings-detail.md) (13 rules, same 15 of 15 total).
