# Perfpal Findings Detail

9 of 9 total findings.

## Complete ranked action list (KV1 order)

<a id="ranked-actions"></a>

| rank | finding | title | location | function | severity | confidence | path class | cost anchor |
|---:|---|---|---|---|---|---|---|---|
| 1 | [`PERF\-C\-ALLOC\-001:src/main\.c:27:21`](#finding-PERF-C-ALLOC-001-src-main-c-27-21) | Heap allocation inside loop | `src/main\.c:27` | `c\_loop` | hot | HIGH | production | 15 static cost units \(v2\) · 5\.0× median · top 25% |
| 2 | [`PERF\-C\-ALLOC\-002:src/main\.c:29:9`](#finding-PERF-C-ALLOC-002-src-main-c-29-9) | Deallocation inside loop | `src/main\.c:29` | `c\_loop` | hot | HIGH | production | 15 static cost units \(v2\) · 5\.0× median · top 25% |
| 3 | [`PERF\-C\-STR\-001:src/main\.c:28:9`](#finding-PERF-C-STR-001-src-main-c-28-9) | String accumulation in loop | `src/main\.c:28` | `c\_loop` | hot | HIGH | production | 15 static cost units \(v2\) · 5\.0× median · top 25% |
| 4 | [`PERF\-RS\-ALLOC\-003:src/lib\.rs:20:9`](#finding-PERF-RS-ALLOC-003-src-lib-rs-20-9) | Growable\-buffer additive growth in loop \(Rust\) | `src/lib\.rs:20` | `build` | hot | MEDIUM | production | 20 static cost units \(v2\) · 6\.7× median · top 25% |
| 5 | [`PERF\-RS\-STR\-002:src/lib\.rs:22:17`](#finding-PERF-RS-STR-002-src-lib-rs-22-17) | Length query recomputed in loop \(Rust\) | `src/lib\.rs:22` | `build` | warn | HIGH | production | 20 static cost units \(v2\) · 6\.7× median · top 25% |
| 6 | [`PERF\-RS\-STR\-002:src/lib\.rs:23:17`](#finding-PERF-RS-STR-002-src-lib-rs-23-17) | Length query recomputed in loop \(Rust\) | `src/lib\.rs:23` | `build` | warn | HIGH | production | 20 static cost units \(v2\) · 6\.7× median · top 25% |
| 7 | [`PERF\-C\-STR\-003:src/main\.c:28:9`](#finding-PERF-C-STR-003-src-main-c-28-9) | Unbounded string operation | `src/main\.c:28` | `c\_loop` | warn | HIGH | production | 15 static cost units \(v2\) · 5\.0× median · top 25% |
| 8 | [`PERF\-RS\-MEM\-001:src/lib\.rs:25:22`](#finding-PERF-RS-MEM-001-src-lib-rs-25-22) | size\_of passed as an element count \(Rust\) | `src/lib\.rs:25` | `build` | warn | MEDIUM | production | 20 static cost units \(v2\) · 6\.7× median · top 25% |

<a id="finding-PERF-C-ALLOC-001-src-main-c-27-21"></a>
#### Heap allocation inside loop  (`PERF\-C\-ALLOC\-001:src/main\.c:27:21`)
- Location: `src/main\.c:27` (c\_loop)
- Message: per\-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_index=2, callee=malloc, loop\_depth=1

<a id="finding-PERF-C-ALLOC-002-src-main-c-29-9"></a>
#### Deallocation inside loop  (`PERF\-C\-ALLOC\-002:src/main\.c:29:9`)
- Location: `src/main\.c:29` (c\_loop)
- Message: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_cooccurrence=true, callee=free, loop\_depth=1

<a id="finding-PERF-C-STR-001-src-main-c-28-9"></a>
#### String accumulation in loop  (`PERF\-C\-STR\-001:src/main\.c:28:9`)
- Location: `src/main\.c:28` (c\_loop)
- Message: 'strcat' accumulates into 'acc' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: callee=strcat, dest=acc, loop\_depth=1

<a id="finding-PERF-RS-ALLOC-003-src-lib-rs-20-9"></a>
#### Growable\-buffer additive growth in loop \(Rust\)  (`PERF\-RS\-ALLOC\-003:src/lib\.rs:20:9`)
- Location: `src/lib\.rs:20` (build)
- Message: 'out\.reserve' grows a buffer by addition inside a loop at depth 1 — each reserve may reallocate and copy the whole buffer; reserving a larger capacity once up front is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=out\.reserve, growth=additive, loop\_depth=1

<a id="finding-PERF-C-STR-003-src-main-c-28-9"></a>
#### Unbounded string operation  (`PERF\-C\-STR\-003:src/main\.c:28:9`)
- Location: `src/main\.c:28` (c\_loop)
- Message: unbounded operation via 'strcat' — a bounded variant or explicit length tracking is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=strcat

<a id="finding-PERF-RS-STR-002-src-lib-rs-22-17"></a>
#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/lib\.rs:22:17`)
- Location: `src/lib\.rs:22` (build)
- Message: 's\.len' recomputes the length of 's' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=s, loop\_depth=1

<a id="finding-PERF-RS-STR-002-src-lib-rs-23-17"></a>
#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/lib\.rs:23:17`)
- Location: `src/lib\.rs:23` (build)
- Message: 'tmp\.len' recomputes the length of 'tmp' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=tmp, loop\_depth=1

<a id="finding-PERF-RS-MEM-001-src-lib-rs-25-22"></a>
#### size\_of passed as an element count \(Rust\)  (`PERF\-RS\-MEM\-001:src/lib\.rs:25:22`)
- Location: `src/lib\.rs:25` (build)
- Message: 'Vec::with\_capacity' receives a \`size\_of\` byte count where an element COUNT is expected — that over\-reserves by a factor of the element size; passing the element count is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=Vec::with\_capacity, misuse=size\_of\_as\_element\_count

<a id="finding-PERF-C-ARCH-003-src-main-c-32-12"></a>
