# Perfpal Findings Detail

15 of 15 total findings.

## Complete ranked action list (KV1 order)

<a id="ranked-actions"></a>

| rank | finding | title | location | function | severity | confidence | path class | cost anchor |
|---:|---|---|---|---|---|---|---|---|
| 1 | [`PERF\-C\-ALLOC\-001:src/main\.c:34:21`](#finding-PERF-C-ALLOC-001-src-main-c-34-21) | Heap allocation inside loop | `src/main\.c:34` | `process` | hot | HIGH | production | 14 static cost units \(v2\) · 2\.0× median · top 25% |
| 2 | [`PERF\-C\-ALLOC\-002:src/main\.c:37:9`](#finding-PERF-C-ALLOC-002-src-main-c-37-9) | Deallocation inside loop | `src/main\.c:37` | `process` | hot | HIGH | production | 14 static cost units \(v2\) · 2\.0× median · top 25% |
| 3 | [`PERF\-C\-ALLOC\-001:src/main\.c:73:15`](#finding-PERF-C-ALLOC-001-src-main-c-73-15) | Heap allocation inside loop | `src/main\.c:73` | `grow` | hot | HIGH | production | 13 static cost units \(v2\) · 1\.9× median · top 25% |
| 4 | [`PERF\-C\-STR\-001:src/main\.c:52:9`](#finding-PERF-C-STR-001-src-main-c-52-9) | String accumulation in loop | `src/main\.c:52` | `join` | hot | HIGH | production | 7 static cost units \(v2\) · 1\.0× median · top 25% |
| 5 | [`PERF\-C\-ALLOC\-003:src/main\.c:73:15`](#finding-PERF-C-ALLOC-003-src-main-c-73-15) | Realloc linear growth in loop | `src/main\.c:73` | `grow` | hot | MEDIUM | production | 13 static cost units \(v2\) · 1\.9× median · top 25% |
| 6 | [`PERF\-C\-LOOP\-002:src/main\.c:80:9`](#finding-PERF-C-LOOP-002-src-main-c-80-9) | Expensive setup or I/O call inside loop | `src/main\.c:80` | `report` | hot | MEDIUM | production | 9 static cost units \(v2\) · 1\.3× median · top 25% |
| 7 | [`PERF\-C\-CALL\-002:src/main\.c:84:1`](#finding-PERF-C-CALL-002-src-main-c-84-1) | High cyclomatic complexity | `src/main\.c:84` | `dispatch` | warn | HIGH | production | 57 static cost units \(v2\) · 8\.1× median · top 10% |
| 8 | [`PERF\-C\-LOOP\-001:src/main\.c:61:13`](#finding-PERF-C-LOOP-001-src-main-c-61-13) | Deeply nested loops | `src/main\.c:61` | `deep\_sum` | warn | HIGH | production | 27 static cost units \(v2\) · 3\.9× median · top 25% |
| 9 | [`PERF\-C\-ALLOC\-002:src/main\.c:44:9`](#finding-PERF-C-ALLOC-002-src-main-c-44-9) | Deallocation inside loop | `src/main\.c:44` | `drain` | warn | HIGH | production | 7 static cost units \(v2\) · 1\.0× median · top 25% |
| 10 | [`PERF\-C\-STR\-003:src/main\.c:52:9`](#finding-PERF-C-STR-003-src-main-c-52-9) | Unbounded string operation | `src/main\.c:52` | `join` | warn | HIGH | production | 7 static cost units \(v2\) · 1\.0× median · top 25% |
| 11 | [`PERF\-C\-MEM\-001:src/main\.c:118:19`](#finding-PERF-C-MEM-001-src-main-c-118-19) | sizeof misuse in allocation argument | `src/main\.c:118` | `clone\_row` | warn | MEDIUM | production | 5 static cost units \(v2\) · 0\.7× median · top 25% |
| 12 | [`PERF\-C\-TYPE\-001:src/main\.c:113:1`](#finding-PERF-C-TYPE-001-src-main-c-113-1) | Large struct passed by value | `src/main\.c:113` | `weigh` | warn | LOW | production | 1 static cost units \(v2\) · 0\.1× median · top 25% |
| 13 | [`PERF\-C\-LOOP\-004:src/main\.c:62:24`](#finding-PERF-C-LOOP-004-src-main-c-62-24) | Variable\-divisor division in innermost loop | `src/main\.c:62` | `deep\_sum` | info | MEDIUM | production | 27 static cost units \(v2\) · 3\.9× median · top 25% |
| 14 | [`PERF\-C\-ARCH\-003:src/main\.c:127:20`](#finding-PERF-C-ARCH-003-src-main-c-127-20) | Cross\-file call coupling | `src/main\.c:127` | `main` | info | LOW | production | 11 static cost units \(v2\) · 1\.6× median · top 25% |

<a id="finding-PERF-C-ALLOC-001-src-main-c-34-21"></a>
#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:34:21`)
- Location: `src/main\.c:34` (process)
- Message: per\-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=malloc, loop\_depth=1

<a id="finding-PERF-C-ALLOC-001-src-main-c-73-15"></a>
#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:73:15`)
- Location: `src/main\.c:73` (grow)
- Message: per\-iteration heap allocation via 'realloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=realloc, loop\_depth=1

<a id="finding-PERF-C-ALLOC-002-src-main-c-37-9"></a>
#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:37:9`)
- Location: `src/main\.c:37` (process)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_cooccurrence=true, callee=free, loop\_depth=1

<a id="finding-PERF-C-ARCH-002-src-main-c-84-1"></a>
#### Hot component  (`PERF\-C\-ARCH\-002:src/main\.c:84:1`)
- Location: `src/main\.c:84` (dispatch)
- Message: directory 'src' aggregates complexity 42 across 12 function\(s\) — the hottest component in this scan; splitting or simplifying its hottest functions is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_density=39\.2156862745098, complexity\_sum=42, function\_count=12, max\_loop\_depth=3, path\_prefix=src

<a id="finding-PERF-C-STR-001-src-main-c-52-9"></a>
#### String accumulation in loop  (`PERF\-C\-STR\-001:src/main\.c:52:9`)
- Location: `src/main\.c:52` (join)
- Message: 'strcat' accumulates into 'out' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: callee=strcat, dest=out, loop\_depth=1

<a id="finding-PERF-C-ALLOC-003-src-main-c-73-15"></a>
#### Realloc linear growth in loop  (`PERF\-C\-ALLOC\-003:src/main\.c:73:15`)
- Location: `src/main\.c:73` (grow)
- Message: 'realloc' grows by addition inside a loop at depth 1 — each growth copies the whole buffer; doubling the capacity instead of adding a fixed amount is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=1, callee=realloc, growth=additive, loop\_depth=1

<a id="finding-PERF-C-LOOP-002-src-main-c-80-9"></a>
#### Expensive setup or I/O call inside loop  (`PERF\-C\-LOOP\-002:src/main\.c:80:9`)
- Location: `src/main\.c:80` (report)
- Message: 'printf' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=printf, conditional\_guarded=false, io\_kind=hoistable, loop\_depth=1

<a id="finding-PERF-C-ALLOC-002-src-main-c-44-9"></a>
#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:44:9`)
- Location: `src/main\.c:44` (drain)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=free, loop\_depth=1

<a id="finding-PERF-C-CALL-002-src-main-c-84-1"></a>
#### High cyclomatic complexity  (`PERF\-C\-CALL\-002:src/main\.c:84:1`)
- Location: `src/main\.c:84` (dispatch)
- Message: 'dispatch' branches 23 ways \(threshold 15\) — complex control flow hides hot paths and resists testing; extracting helpers or table\-driving the dispatch is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: complexity=23, function=dispatch, threshold=15

<a id="finding-PERF-C-LOOP-001-src-main-c-61-13"></a>
#### Deeply nested loops  (`PERF\-C\-LOOP\-001:src/main\.c:61:13`)
- Location: `src/main\.c:61` (deep\_sum)
- Message: loops nest 3 deep here \(chain reaches depth 3\) — flattening the nesting or adding early exits is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: max\_depth=3, nest\_depth=3

<a id="finding-PERF-C-STR-003-src-main-c-52-9"></a>
#### Unbounded string operation  (`PERF\-C\-STR\-003:src/main\.c:52:9`)
- Location: `src/main\.c:52` (join)
- Message: unbounded operation via 'strcat' — a bounded variant or explicit length tracking is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=strcat

<a id="finding-PERF-C-MEM-001-src-main-c-118-19"></a>
#### sizeof misuse in allocation argument  (`PERF\-C\-MEM\-001:src/main\.c:118:19`)
- Location: `src/main\.c:118` (clone\_row)
- Message: 'malloc' size argument carries a sizeof misuse shape \(pointer\_param\_sizeof\) — the allocated byte count likely does not match the element type; sizing by the pointee type is a candidate fix
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=malloc, misuse=pointer\_param\_sizeof

<a id="finding-PERF-C-TYPE-001-src-main-c-113-1"></a>
#### Large struct passed by value  (`PERF\-C\-TYPE\-001:src/main\.c:113:1`)
- Location: `src/main\.c:113` (weigh)
- Message: parameters 'struct Big b' \(struct 'Big', ~48 bytes\) are copied by value on every call under the LP64 estimate — passing pointers or taking the structs by reference is a candidate optimization
- Confidence: LOW · Severity: warn · Resolver tier: syntactic
- Evidence: estimated\_sizes=48, params=struct Big b, structs=Big, threshold=32

<a id="finding-PERF-C-LOOP-004-src-main-c-62-24"></a>
#### Variable\-divisor division in innermost loop  (`PERF\-C\-LOOP\-004:src/main\.c:62:24`)
- Location: `src/main\.c:62` (deep\_sum)
- Message: '/' by a non\-constant divisor executes in the innermost loop — real integer division runs per iteration; hoisting loop\-invariant divisors or precomputing reciprocals is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: divisor=d, divisor\_kind=identifier, op=/

<a id="finding-PERF-C-ARCH-003-src-main-c-127-20"></a>
