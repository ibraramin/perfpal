# Perfpal Findings Detail

32 of 32 total findings.

#### Heap allocation inside loop \(Rust\)  (`PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:34:5`)
- Location: `src/01\_alloc\_churn\.rs:34` (build\_row\_labels)
- Message: loop performs 3 per\-iteration allocation\(s\) \(format, to\_string, vec\) at loop depth 1 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_count=3, callee=format, callees=format, to\_string, vec, loop\_depth=1

#### Heap allocation inside loop \(Rust\)  (`PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:58:9`)
- Location: `src/01\_alloc\_churn\.rs:58` (grid\_reset)
- Message: loop performs 1 per\-iteration allocation\(s\) \(vec\) at loop depth 2 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_count=1, callee=vec, callees=vec, loop\_depth=2

#### Heap allocation inside loop \(Rust\)  (`PERF\-RS\-ALLOC\-001:src/02\_string\_building\.rs:41:5`)
- Location: `src/02\_string\_building\.rs:41` (len\_with\_reassignment)
- Message: loop performs 1 per\-iteration allocation\(s\) \(format\) at loop depth 1 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_count=1, callee=format, callees=format, loop\_depth=1

#### Heap allocation inside loop \(Rust\)  (`PERF\-RS\-ALLOC\-001:src/06\_memory\_ops\.rs:27:9`)
- Location: `src/06\_memory\_ops\.rs:27` (duplicate\_rows)
- Message: loop performs 1 per\-iteration allocation\(s\) \(to\_vec\) at loop depth 2 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_count=1, callee=to\_vec, callees=to\_vec, loop\_depth=2

#### Deallocation inside loop \(Rust\)  (`PERF\-RS\-ALLOC\-002:src/01\_alloc\_churn\.rs:39:9`)
- Location: `src/01\_alloc\_churn\.rs:39` (build\_row\_labels)
- Message: deallocation via 'drop' at loop depth 1 — repeated drop\-per\-iteration teardown; hoisting the drop or restructuring value ownership is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_cooccurrence=true, callee=drop, loop\_depth=1

#### Hot component \(Rust\)  (`PERF\-RS\-ARCH\-002:src/05\_recursion\_complexity\.rs:20:1`)
- Location: `src/05\_recursion\_complexity\.rs:20` (classify)
- Message: directory 'src' aggregates complexity 86 across 40 function\(s\) — the hottest component in this scan; splitting or simplifying its hottest functions is a candidate optimization
- Confidence: HIGH · Severity: hot · Resolver tier: syntactic
- Evidence: alloc\_site\_density=86\.41975308641973, complexity\_sum=86, function\_count=40, max\_loop\_depth=3, path\_prefix=src

#### Growable\-buffer additive growth in loop \(Rust\)  (`PERF\-RS\-ALLOC\-003:src/01\_alloc\_churn\.rs:49:9`)
- Location: `src/01\_alloc\_churn\.rs:49` (grow\_one\_at\_a\_time)
- Message: 'buffer\.reserve' grows a buffer by addition inside a loop at depth 1 — each reserve may reallocate and copy the whole buffer; reserving a larger capacity once up front is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=buffer\.reserve, growth=additive, loop\_depth=1

#### Expensive setup or I/O call inside loop \(Rust\)  (`PERF\-RS\-LOOP\-002:src/01\_alloc\_churn\.rs:40:9`)
- Location: `src/01\_alloc\_churn\.rs:40` (build\_row\_labels)
- Message: 'println' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=println, conditional\_guarded=false, io\_kind=hoistable, loop\_depth=1

#### Expensive setup or I/O call inside loop \(Rust\)  (`PERF\-RS\-LOOP\-002:src/04\_stdio\_churn\.rs:28:9`)
- Location: `src/04\_stdio\_churn\.rs:28` (emit\_rows)
- Message: 'println' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=println, conditional\_guarded=false, io\_kind=hoistable, loop\_depth=1

#### Expensive setup or I/O call inside loop \(Rust\)  (`PERF\-RS\-LOOP\-002:src/04\_stdio\_churn\.rs:29:17`)
- Location: `src/04\_stdio\_churn\.rs:29` (emit\_rows)
- Message: 'sink\.write\_all' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=sink\.write\_all, conditional\_guarded=false, io\_kind=hoistable, loop\_depth=1

#### Module\-graph cycle \(Rust\)  (`PERF\-RS\-ARCH\-001:src/cycle\_b\.rs:6:1`)
- Location: `src/cycle\_b\.rs:6`
- Message: 'src/cycle\_b\.rs' and 'src/cycle\_a\.rs' import each other, forming a module cycle — breaking the lowest\-sorted edge \(here 'src/cycle\_b\.rs' → 'src/cycle\_a\.rs'\) into a shared leaf module is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: coupled\_with=src/cycle\_a\.rs, kind=cycle

#### High cyclomatic complexity \(Rust\)  (`PERF\-RS\-CALL\-002:src/05\_recursion\_complexity\.rs:20:1`)
- Location: `src/05\_recursion\_complexity\.rs:20` (classify)
- Message: 'classify' branches 17 ways \(threshold 15\) — complex control flow hides hot paths and resists testing; extracting helpers or table\-driving the dispatch is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: complexity=17, function=classify, threshold=15

#### Deeply nested loops \(Rust\)  (`PERF\-RS\-LOOP\-001:src/03\_deep\_nesting\.rs:19:13`)
- Location: `src/03\_deep\_nesting\.rs:19` (grid\_checksum)
- Message: loops nest 3 deep here \(chain reaches depth 3\) — flattening the nesting or using iterator combinators with early exits is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: max\_depth=3, nest\_depth=3

#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/02\_string\_building\.rs:32:18`)
- Location: `src/02\_string\_building\.rs:32` (scan\_lengths)
- Message: 'joined\.len' recomputes the length of 'joined' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=joined, loop\_depth=1

#### Length query recomputed in loop \(Rust\)  (`PERF\-RS\-STR\-002:src/09\_membership\_scan\.rs:15:21`)
- Location: `src/09\_membership\_scan\.rs:15` (contains\_nested)
- Message: 'table\.len' recomputes the length of 'table' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: identifier=table, loop\_depth=1

#### Unbounded unsafe memory operation \(Rust\)  (`PERF\-RS\-STR\-003:src/02\_string\_building\.rs:62:5`)
- Location: `src/02\_string\_building\.rs:62` (unsafe\_slice)
- Message: 'std::slice::from\_raw\_parts' performs an unchecked \(unbounded\) memory operation — prefer a safe/bounded variant or add an explicit length check; that is a candidate optimization
- Confidence: HIGH · Severity: warn · Resolver tier: syntactic
- Evidence: callee=std::slice::from\_raw\_parts

#### size\_of passed as an element count \(Rust\)  (`PERF\-RS\-MEM\-001:src/06\_memory\_ops\.rs:15:5`)
- Location: `src/06\_memory\_ops\.rs:15` (over\_reserve)
- Message: 'Vec::with\_capacity' receives a \`size\_of\` byte count where an element COUNT is expected — that over\-reserves by a factor of the element size; passing the element count is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=Vec::with\_capacity, misuse=size\_of\_as\_element\_count

#### Block copy/clone in loop \(Rust\)  (`PERF\-RS\-MEM\-002:src/01\_alloc\_churn\.rs:61:13`)
- Location: `src/01\_alloc\_churn\.rs:61` (grid\_reset)
- Message: 'cell\.copy\_from\_slice' copies a block every iteration at loop depth 2 — hoisting the copy, batching the copies, or reusing a buffer is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=cell\.copy\_from\_slice, innermost=true, loop\_depth=2

#### Large struct passed by value \(Rust\)  (`PERF\-RS\-TYPE\-001:src/07\_struct\_by\_value\.rs:40:1`)
- Location: `src/07\_struct\_by\_value\.rs:40` (consume\_blob)
- Message: parameters 'BigBlob' \(struct 'BigBlob', ~32 bytes\) are copied by value on every call under the LP64 estimate — taking the structs by reference \(\`\&T\`\) or \`Box\`\-ing them is a candidate optimization
- Confidence: LOW · Severity: warn · Resolver tier: syntactic
- Evidence: estimated\_sizes=32, params=BigBlob, structs=BigBlob, threshold=32

#### Direct recursion \(Rust\)  (`PERF\-RS\-CALL\-001:src/05\_recursion\_complexity\.rs:14:9`)
- Location: `src/05\_recursion\_complexity\.rs:14` (fib)
- Message: 'fib' calls itself directly at 2 site\(s\) — recursion keeps stack frames alive; converting to iteration or memoizing is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: function=fib, self\_call\_sites=2

#### Character\-wise processing in loop \(Rust\)  (`PERF\-RS\-LOOP\-003:src/04\_stdio\_churn\.rs:19:9`)
- Location: `src/04\_stdio\_churn\.rs:19` (count\_chars\_per\_iteration)
- Message: character/byte\-wise processing via 'text\.chars' as the loop body's primary statement — processing the input in buffered blocks or chunks is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=text\.chars, loop\_depth=1

#### Variable\-divisor division in innermost loop \(Rust\)  (`PERF\-RS\-LOOP\-004:src/03\_deep\_nesting\.rs:22:23`)
- Location: `src/03\_deep\_nesting\.rs:22` (grid\_checksum)
- Message: '%' by a non\-constant divisor executes in the innermost loop — real integer division runs per iteration; hoisting loop\-invariant divisors or precomputing reciprocals is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: divisor=divisor, divisor\_kind=identifier, op=%

#### Global variable written inside loop \(Rust\)  (`PERF\-RS\-LOOP\-005:src/08\_globals\_and\_chains\.rs:41:13`)
- Location: `src/08\_globals\_and\_chains\.rs:41` (run\_handlers)
- Message: static/const\-backed 'GLOBAL\_ACC' is stored every iteration at loop depth 1 — hoisting the write out of the loop or caching it in a local is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: loop\_depth=1, target=GLOBAL\_ACC

#### Member\-chain dereference in innermost loop \(Rust\)  (`PERF\-RS\-LOOP\-008:src/08\_globals\_and\_chains\.rs:59:17`)
- Location: `src/08\_globals\_and\_chains\.rs:59` (read\_depth)
- Message: 'configs' field/index chain of depth 4 executes in the innermost loop — caching intermediate references or flattening the structure is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: chain\_depth=4, loop\_depth=1, root=configs

#### Cross\-module call coupling \(Rust\)  (`PERF\-RS\-ARCH\-003:src/10\_cross\_file\_and\_arch\.rs:21:16`)
- Location: `src/10\_cross\_file\_and\_arch\.rs:21` (orchestrate)
- Message: call to 'build\_row\_labels' matches a definition in 'src/01\_alloc\_churn\.rs' by name only — LOW\-confidence hypothesis; confirming the match \(or co\-locating hot callees\) is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee\_name=build\_row\_labels, matched\_path=src/01\_alloc\_churn\.rs

#### Cross\-module call coupling \(Rust\)  (`PERF\-RS\-ARCH\-003:src/10\_cross\_file\_and\_arch\.rs:22:20`)
- Location: `src/10\_cross\_file\_and\_arch\.rs:22` (orchestrate)
- Message: call to 'scale\_checksum' matches a definition in 'src/03\_deep\_nesting\.rs' by name only — LOW\-confidence hypothesis; confirming the match \(or co\-locating hot callees\) is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee\_name=scale\_checksum, matched\_path=src/03\_deep\_nesting\.rs

#### Cross\-module call coupling \(Rust\)  (`PERF\-RS\-ARCH\-003:src/10\_cross\_file\_and\_arch\.rs:23:18`)
- Location: `src/10\_cross\_file\_and\_arch\.rs:23` (orchestrate)
- Message: call to 'shared\_checksum' matches a definition in 'src/common\.rs' by name only — LOW\-confidence hypothesis; confirming the match \(or co\-locating hot callees\) is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee\_name=shared\_checksum, matched\_path=src/common\.rs

#### Cross\-module call coupling \(Rust\)  (`PERF\-RS\-ARCH\-003:src/cycle\_a\.rs:18:5`)
- Location: `src/cycle\_a\.rs:18` (cycle\_a\_step)
- Message: call to 'cycle\_b\_step' matches a definition in 'src/cycle\_b\.rs' by name only — LOW\-confidence hypothesis; confirming the match \(or co\-locating hot callees\) is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee\_name=cycle\_b\_step, matched\_path=src/cycle\_b\.rs

#### Cross\-module call coupling \(Rust\)  (`PERF\-RS\-ARCH\-003:src/cycle\_b\.rs:15:5`)
- Location: `src/cycle\_b\.rs:15` (cycle\_b\_root)
- Message: call to 'cycle\_a\_root' matches a definition in 'src/cycle\_a\.rs' by name only — LOW\-confidence hypothesis; confirming the match \(or co\-locating hot callees\) is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee\_name=cycle\_a\_root, matched\_path=src/cycle\_a\.rs

#### Indirect call in innermost loop \(Rust\)  (`PERF\-RS\-LOOP\-006:src/08\_globals\_and\_chains\.rs:43:9`)
- Location: `src/08\_globals\_and\_chains\.rs:43` (run\_handlers)
- Message: indirect call through 'handlers\[i\]' runs in the innermost loop at depth 1 — resolving the target once outside the loop is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: callee=handlers\[i\], loop\_depth=1

#### Linear membership scan nested in loop \(Rust\)  (`PERF\-RS\-LOOP\-007:src/09\_membership\_scan\.rs:16:16`)
- Location: `src/09\_membership\_scan\.rs:16` (contains\_nested)
- Message: element lookup compared against a non\-literal at loop depth 2 re\-scans linearly on every outer iteration — using a HashSet/HashMap lookup table or sorting plus binary search is a candidate optimization
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: index\_ident=j, loop\_depth=2

#### Struct field ordering \(padding candidate, Rust\)  (`PERF\-RS\-TYPE\-002:src/07\_struct\_by\_value\.rs:26:1`)
- Location: `src/07\_struct\_by\_value\.rs:26`
- Message: struct 'PaddedLayout' shows ~14 bytes of declared\-order padding \(aligned estimate 32 vs packed sum 18\) — if the type is \`\#\[repr\(C\)\]\`/FFI\-sensitive, ordering fields largest\-alignment\-first is a candidate optimization; default \`repr\(Rust\)\` may already reorder fields automatically, so treat this as lower\-confidence
- Confidence: LOW · Severity: info · Resolver tier: syntactic
- Evidence: aligned\_estimate=32, packed\_sum=18, padding\_bytes=14, struct=PaddedLayout, threshold=8
