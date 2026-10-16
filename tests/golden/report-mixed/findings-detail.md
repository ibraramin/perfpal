# Perfpal Findings Detail

9 of 9 total findings.

#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:27:21`)
- Location: `src/main\.c:27` (c\_loop)
- Message: per\-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=2, callee=malloc, loop\_depth=1

#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:29:9`)
- Location: `src/main\.c:29` (c\_loop)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_cooccurrence=true, callee=free, loop\_depth=1

#### String accumulation in loop  (`PERF\-C\-STR\-001:src/main\.c:28:9`)
- Location: `src/main\.c:28` (c\_loop)
- Message: 'strcat' accumulates into 'acc' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: callee=strcat, dest=acc, loop\_depth=1

#### Growable\-buffer additive growth in loop \(Rust\)  (`PERF\-RS\-ALLOC\-003:src/lib\.rs:20:9`)
- Location: `src/lib\.rs:20` (build)
- Message: 'out\.reserve' grows a buffer by addition inside a loop at depth 1 — each reserve may reallocate and copy the whole buffer; reserving a larger capacity once up front is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=out\.reserve, growth=additive, loop\_depth=1

#### Unbounded string operation  (`PERF\-C\-STR\-003:src/main\.c:28:9`)
- Location: `src/main\.c:28` (c\_loop)
- Message: unbounded operation via 'strcat' — a bounded variant or explicit length tracking is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=strcat

#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/lib\.rs:22:17`)
- Location: `src/lib\.rs:22` (build)
- Message: 's\.len' recomputes the length of 's' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=s, loop\_depth=1

#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/lib\.rs:23:17`)
- Location: `src/lib\.rs:23` (build)
- Message: 'tmp\.len' recomputes the length of 'tmp' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=tmp, loop\_depth=1

#### size\_of passed as an element count \(Rust\)  (`PERF\-RS\-MEM\-001:src/lib\.rs:25:22`)
- Location: `src/lib\.rs:25` (build)
- Message: 'Vec::with\_capacity' receives a \`size\_of\` byte count where an element COUNT is expected — that over\-reserves by a factor of the element size; passing the element count is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=Vec::with\_capacity, misuse=size\_of\_as\_element\_count
