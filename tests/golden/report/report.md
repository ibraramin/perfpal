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
#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:34:21`)
- Location: `src/main\.c:34` (process)
- Message: per\-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=malloc, loop\_depth=1

#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:73:15`)
- Location: `src/main\.c:73` (grow)
- Message: per\-iteration heap allocation via 'realloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=realloc, loop\_depth=1

#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:37:9`)
- Location: `src/main\.c:37` (process)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_cooccurrence=true, callee=free, loop\_depth=1

#### Hot component  (`PERF\-C\-ARCH\-002:src/main\.c:84:1`)
- Location: `src/main\.c:84` (dispatch)
- Message: directory 'src' aggregates complexity 42 across 12 function\(s\) — the hottest component in this scan; splitting or simplifying its hottest functions is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_density=39\.2156862745098, complexity\_sum=42, function\_count=12, max\_loop\_depth=3, path\_prefix=src

#### String accumulation in loop  (`PERF\-C\-STR\-001:src/main\.c:52:9`)
- Location: `src/main\.c:52` (join)
- Message: 'strcat' accumulates into 'out' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: callee=strcat, dest=out, loop\_depth=1

#### Realloc linear growth in loop  (`PERF\-C\-ALLOC\-003:src/main\.c:73:15`)
- Location: `src/main\.c:73` (grow)
- Message: 'realloc' grows by addition inside a loop at depth 1 — each growth copies the whole buffer; doubling the capacity instead of adding a fixed amount is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=realloc, growth=additive, loop\_depth=1

#### Expensive setup or I/O call inside loop  (`PERF\-C\-LOOP\-002:src/main\.c:80:9`)
- Location: `src/main\.c:80` (report)
- Message: 'printf' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=printf, conditional\_guarded=false, loop\_depth=1

#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:44:9`)
- Location: `src/main\.c:44` (drain)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=free, loop\_depth=1

#### High cyclomatic complexity  (`PERF\-C\-CALL\-002:src/main\.c:84:1`)
- Location: `src/main\.c:84` (dispatch)
- Message: 'dispatch' branches 23 ways \(threshold 15\) — complex control flow hides hot paths and resists testing; extracting helpers or table\-driving the dispatch is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: complexity=23, function=dispatch, threshold=15

#### Deeply nested loops  (`PERF\-C\-LOOP\-001:src/main\.c:61:13`)
- Location: `src/main\.c:61` (deep\_sum)
- Message: loops nest 3 deep here \(chain reaches depth 3\) — flattening the nesting or adding early exits is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: max\_depth=3, nest\_depth=3

#### Unbounded string operation  (`PERF\-C\-STR\-003:src/main\.c:52:9`)
- Location: `src/main\.c:52` (join)
- Message: unbounded operation via 'strcat' — a bounded variant or explicit length tracking is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=strcat

#### sizeof misuse in allocation argument  (`PERF\-C\-MEM\-001:src/main\.c:118:19`)
- Location: `src/main\.c:118` (clone\_row)
- Message: 'malloc' size argument carries a sizeof misuse shape \(pointer\_param\_sizeof\) — the allocated byte count likely does not match the element type; sizing by the pointee type is a candidate fix
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=malloc, misuse=pointer\_param\_sizeof

#### Large struct passed by value  (`PERF\-C\-TYPE\-001:src/main\.c:113:1`)
- Location: `src/main\.c:113` (weigh)
- Message: parameters 'struct Big b' \(struct 'Big', ~48 bytes\) are copied by value on every call under the LP64 estimate — passing pointers or taking the structs by reference is a candidate optimization
- Confidence: LOW · Severity: warn · Resolver tier: syntactic
- Evidence: estimated\_sizes=48, params=struct Big b, structs=Big, threshold=32

#### Variable\-divisor division in innermost loop  (`PERF\-C\-LOOP\-004:src/main\.c:62:24`)
- Location: `src/main\.c:62` (deep\_sum)
- Message: '/' by a non\-constant divisor executes in the innermost loop — real integer division runs per iteration; hoisting loop\-invariant divisors or precomputing reciprocals is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: divisor=d, divisor\_kind=identifier, op=/

## 5. Architecture
[Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files.](charts/include-graph.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)

Component ranking (hottest first):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| src | src | 1 | 147.06 |

The dependency graph reflects resolved internal include/module edges between scanned files — C `#include` directives and Rust `mod`/`use` module paths; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
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
| `PERF\-C\-LOOP\-002` | Expensive setup / I/O call inside loop | Detects printf\-family, file\-I/O, getenv, system, and regex\-compilation calls nested in loops\. |
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