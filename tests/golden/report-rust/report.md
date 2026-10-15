# Perfpal Report
> schema_version 1\.4 · perfpal 0\.2\.0\-beta\.1 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 14 files

## 1. Executive summary
32 findings: 10 hot, 9 warn, 13 info. Parse health: 14 ok, 0 partial, 0 failed.
- Heap allocation inside loop \(Rust\) (PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:34:5)
- Heap allocation inside loop \(Rust\) (PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:58:9)
- Heap allocation inside loop \(Rust\) (PERF\-RS\-ALLOC\-001:src/02\_string\_building\.rs:41:5)

## 2. Scan provenance & parse health
- Tool: perfpal 0\.2\.0\-beta\.1
- Scan root: `<ROOT>`
- Files scanned: 14

### Parse health summary

| status | files |
|---|---:|
| ok | 14 |
| partial | 0 |
| failed | 0 |

No failed or partially parsed files.

Findings overlapping parse-error regions are flagged (`error_node_overlap`) and confidence-capped — never silently dropped.

## 3. Codebase statistics
- Languages: rust
- Total LOC: 162 code / 346 comment / 52 blank

Top files by code LOC:

| path | loc.code | loc.comment | loc.blank | functions |
|---|---:|---:|---:|---:|
| src/05\_recursion\_complexity\.rs | 24 | 13 | 2 | 2 |
| src/08\_globals\_and\_chains\.rs | 24 | 49 | 10 | 6 |
| src/02\_string\_building\.rs | 15 | 47 | 6 | 6 |
| src/01\_alloc\_churn\.rs | 14 | 46 | 5 | 5 |
| src/07\_struct\_by\_value\.rs | 14 | 32 | 6 | 3 |
| src/09\_membership\_scan\.rs | 14 | 18 | 2 | 2 |
| src/10\_cross\_file\_and\_arch\.rs | 12 | 31 | 3 | 3 |
| src/03\_deep\_nesting\.rs | 11 | 19 | 2 | 2 |
| src/04\_stdio\_churn\.rs | 10 | 25 | 4 | 3 |
| src/06\_memory\_ops\.rs | 10 | 20 | 3 | 3 |

Distribution — cyclomatic\_complexity:

| bucket | count |
|---|---:|
| 01\-04 | 39 |
| 05\-09 | 0 |
| 10\-14 | 0 |
| 15\-24 | 1 |
| 25\-49 | 0 |
| 50\+ | 0 |

Distribution — max\_loop\_depth:

| bucket | count |
|---|---:|
| 0 | 20 |
| 1 | 16 |
| 2 | 3 |
| 3 | 1 |
| 4 | 0 |
| 5 | 0 |
| 6\+ | 0 |

## 4. Findings
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
- Evidence: callee=println, conditional\_guarded=false, loop\_depth=1

#### Expensive setup or I/O call inside loop \(Rust\)  (`PERF\-RS\-LOOP\-002:src/04\_stdio\_churn\.rs:28:9`)
- Location: `src/04\_stdio\_churn\.rs:28` (emit\_rows)
- Message: 'println' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=println, conditional\_guarded=false, loop\_depth=1

#### Expensive setup or I/O call inside loop \(Rust\)  (`PERF\-RS\-LOOP\-002:src/04\_stdio\_churn\.rs:29:17`)
- Location: `src/04\_stdio\_churn\.rs:29` (emit\_rows)
- Message: 'sink\.write\_all' runs every iteration at loop depth 1 — hoisting the call out of the loop or batching its work is a candidate optimization
- Confidence: MEDIUM · Severity: hot · Resolver tier: syntactic
- Evidence: callee=sink\.write\_all, conditional\_guarded=false, loop\_depth=1

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

## 5. Architecture
[Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files.](charts/include-graph.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)

Component ranking (hottest first):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| src | src | 1 | 197.53 |

The dependency graph reflects resolved internal include/module edges between scanned files — C `#include` directives and Rust `mod`/`use` module paths; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
Coupling heat blends edge counts with per-function cost (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / alloc_family; each edge weighs 1 + mean endpoint cost normalized by the scan maximum) — charts remain pure functions of metrics.json.
Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic DOT export of the include/call/component graphs (same source data as the SVGs above).
Standing caveat: cross-file call edges are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
| rule_id | title | description |
|---|---|---|
| `PERF\-RS\-ALLOC\-001` | Heap allocation inside loop \(Rust\) | Detects Rust allocation sites — qualified constructors \(\`Box::new\`/\`Rc::new\`/\`Arc::new\`/\`Vec::new\`/\`String::new\`/collection \`new\`/\`with\_capacity\`\), methods \(\`to\_vec\`/\`to\_owned\`/\`to\_string\`/\`collect\`/\`with\_capacity\`\), and the \`vec\!\`/\`format\!\` macros — at \`in\_loop\_depth ≥ 1\` AND \`is\_innermost\`; surviving sites are aggregated into ONE finding per innermost enclosing loop \(\`alloc\_count\` plus sorted\-unique short callee tokens capped at 8 with a \`\+N more\` suffix\)\. Literal\-receiver \`"…"\.to\_string\(\)\`/\`"…"\.to\_owned\(\)\` \(including raw/byte\-string prefixes\) are suppressed\. |
| `PERF\-RS\-ALLOC\-002` | Deallocation inside loop \(Rust\) | Detects a direct \`drop\` call inside a loop body \(allocator churn pairing; escalated to hot when the same function also allocates in a loop\)\. |
| `PERF\-RS\-ALLOC\-003` | Growable\-buffer additive growth in loop \(Rust\) | REFRAMED for Rust \(no \`realloc\`\): detects additive \`reserve\`/\`reserve\_exact\` capacity growth inside a loop — the \`realloc\(p, n\+k\)\` analogue; multiplicative capacity growth is the fix, not the finding\. |
| `PERF\-RS\-ARCH\-001` | Module\-graph fan\-out / cycles \(Rust\) | Detects module fan\-out/fan\-in, deep module chains, and cyclic module edges from resolved Rust module edges; unresolved internal \`mod\`/\`use\` edges are retained as graph data but intentionally emit no finding \(their resolver cannot see crate\-root or inline\-module items\)\. |
| `PERF\-RS\-ARCH\-002` | Hot\-component aggregation \(Rust\) | Ranks Rust component directories by aggregated metrics \(complexity sum, alloc\-site density, max loop depth\) — the hottest component only\. |
| `PERF\-RS\-ARCH\-003` | Cross\-module call coupling \(Rust\) | Detects cross\-file/module call edges via name matching — LOW confidence, upgraded to MEDIUM when a semantic resolver fact resolves the definition\. |
| `PERF\-RS\-CALL\-001` | Direct recursion \(Rust\) | Detects a function body containing a direct call to its own name \(presence\-only; recursion depth is post\-MVP\)\. |
| `PERF\-RS\-CALL\-002` | High cyclomatic complexity \(Rust\) | Detects branch\-count complexity above threshold \(default ≥ 15\), reported per function\. |
| `PERF\-RS\-LOOP\-001` | Deeply nested loops \(Rust\) | Detects loop nesting depth ≥ 3 — O\(n^k\) candidate zones and primary input to hotspot maps\. |
| `PERF\-RS\-LOOP\-002` | Expensive setup or I/O call inside loop \(Rust\) | Detects Rust I/O/setup work nested in loops — \`println\!\`/\`print\!\`\-family macros, reader/writer/socket methods \(\`read\_to\_string\`/\`write\_all\`/\`flush\`/\`recv\`/\`send\`\), and \`File\`/\`fs\`/\`Command\`/\`TcpStream\`/\`env\` setup calls; lowered to warn when conditionally guarded\. |
| `PERF\-RS\-LOOP\-003` | Character\-wise processing in loop \(Rust\) | REFRAMED for Rust \(no char stdio\): detects char/byte\-at\-a\-time iterator or reader methods \(\`chars\`/\`bytes\`/\`char\_indices\`/\`byte\_indices\`/\`read\_byte\`\) as the primary statement of a loop body\. |
| `PERF\-RS\-LOOP\-004` | Variable\-divisor division in innermost loop \(Rust\) | Detects / or % with a non\-literal divisor inside the innermost loop body \(literal divisors skipped by design\)\. |
| `PERF\-RS\-LOOP\-005` | Global variable written inside loop \(Rust\) | Detects assignment to a Rust \`static\`/\`const\`\-backed identifier inside a loop body \(name\-based scope classification, MEDIUM confidence\)\. |
| `PERF\-RS\-LOOP\-006` | Indirect call in innermost loop \(Rust\) | Detects indirect \(function\-pointer/trait\-object\) calls inside innermost loop bodies \(LOW confidence; field\-method/closure dispatch is classified direct and not seen\)\. |
| `PERF\-RS\-LOOP\-007` | Linear membership scan nested in loop \(Rust\) | Detects a comparison against a subscript indexed by an enclosing loop's induction variable — hand\-rolled O\(n²\) membership scan\. |
| `PERF\-RS\-LOOP\-008` | Member\-chain dereference in innermost loop \(Rust\) | Detects field/index access chains of depth ≥ 3 inside innermost loop bodies — pointer/lifetime\-chasing cache\-miss candidates\. |
| `PERF\-RS\-MEM\-001` | size\_of passed as an element count \(Rust\) | REFRAMED for Rust: detects \`Vec::with\_capacity\`/\`String::with\_capacity\` receiving a \`size\_of\`/\`size\_of\_val\` byte count where an element count is expected \(over\-reserves by the element size\)\. |
| `PERF\-RS\-MEM\-002` | Block copy/clone in loop \(Rust\) | REFRAMED for Rust \(no explicit size argument\): detects block\-copy methods \(\`copy\_from\_slice\`/\`copy\_nonoverlapping\`/\`extend\_from\_slice\`\) inside loops — warn in the innermost loop, info otherwise\. \(\`clone\`/\`to\_vec\` were removed by the M1 Precision Gate: borrow\-checker clones and \`Copy\` bit\-copies are not block copies\.\) |
| `PERF\-RS\-STR\-002` | Length query recomputed in loop \(Rust\) | Detects \`len\`/\`is\_empty\`/\`capacity\` recomputed inside a loop where the receiver is not reassigned in the innermost enclosing loop\. |
| `PERF\-RS\-STR\-003` | Unbounded unsafe memory operation \(Rust\) | REFRAMED for Rust \(memory\-safe by default\): detects the unsafe unbounded\-op family \(\`copy\_nonoverlapping\`/\`copy\`/\`from\_raw\_parts\`/\`from\_utf8\_unchecked\`/\`set\_len\`\) anywhere — unchecked\-length surfaces\. |
| `PERF\-RS\-TYPE\-001` | Large struct passed by value \(Rust\) | Detects a bare\-named struct parameter whose LP64 estimate reaches the approximate\-size floor \(LOW confidence — no real type layout\); one finding per function aggregating qualifying parameters\. |
| `PERF\-RS\-TYPE\-002` | Struct field ordering \(padding candidate, Rust\) | Detects struct definitions whose declared\-order aligned layout wastes padding versus the packed member sum beyond the threshold — but default \`repr\(Rust\)\` may reorder fields automatically, so this is a lower\-confidence candidate \(check \`\#\[repr\(C\)\]\`/FFI\)\. |

## Appendix B. How to read this report
**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or architecture impact: review `hot` findings first, schedule `warn`, and treat `info` as context signal feeding aggregation.
**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the false-positive risk of the detection shape — pure-syntax detections are HIGH, shape heuristics with documented FP classes are MEDIUM, and approximations (size models, cross-file name matching) are LOW. Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings whose evidence quality is reduced; their confidence is capped at MEDIUM.
Every finding is an **optimization candidate, never a certainty**: a candidate for human review, not a defect claim. Perfpal v1 runs on a purely syntactic tier (`resolver_tier: syntactic`) — no macro expansion, no type resolution.
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.4.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.