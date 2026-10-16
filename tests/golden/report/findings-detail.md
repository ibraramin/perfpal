# Perfpal Findings Detail

15 of 15 total findings.

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
- Evidence: callee=printf, conditional\_guarded=false, io\_kind=hoistable, loop\_depth=1

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
