# Perfpal Findings Detail

11 of 11 total findings.

## Complete ranked action list (KV1 order)

<a id="ranked-actions"></a>

| rank | finding | title | location | function | severity | confidence | path class | cost anchor |
|---:|---|---|---|---|---|---|---|---|
| 1 | [`PERF\-JS\-LOOP\-007:06\_membership\_scan\.ts:8:9`](#finding-PERF-JS-LOOP-007-06-membership-scan-ts-8-9) | Linear membership scan inside loop | `06\_membership\_scan\.ts:8` | `intersect` | warn | MEDIUM | production | 13 static cost units \(v2\) · 13× median · top 15% |
| 2 | [`PERF\-JS\-ASYNC\-001:07\_async\_await\.ts:12:19`](#finding-PERF-JS-ASYNC-001-07-async-await-ts-12-19) | \`await\` inside a loop | `07\_async\_await\.ts:12` | `loadAll` | warn | MEDIUM | production | 11 static cost units \(v2\) · 11× median · top 17% |
| 3 | [`PERF\-JS\-LOOP\-002:05\_sync\_io\_loop\.ts:9:17`](#finding-PERF-JS-LOOP-002-05-sync-io-loop-ts-9-17) | Synchronous blocking I/O inside loop | `05\_sync\_io\_loop\.ts:9` | `loadAll` | warn | MEDIUM | production | 10 static cost units \(v2\) · 10× median · top 20% |
| 4 | [`PERF\-JS\-STR\-001:03\_string\_build\.ts:8:5`](#finding-PERF-JS-STR-001-03-string-build-ts-8-5) | String accumulation with \`\+=\` inside loop | `03\_string\_build\.ts:8` | `render` | warn | MEDIUM | production | 7 static cost units \(v2\) · 7\.0× median · top 24% |
| 5 | [`PERF\-JS\-ASYNC\-002:08\_async\_blocking\.ts:7:10`](#finding-PERF-JS-ASYNC-002-08-async-blocking-ts-7-10) | Synchronous blocking call inside \`async\` function | `08\_async\_blocking\.ts:7` | `readConfig` | warn | MEDIUM | production | 1 static cost units \(v2\) · 1\.0× median · top 70% |
| 6 | [`PERF\-JS\-LOOP\-001:04\_deep\_nesting\.ts:10:5`](#finding-PERF-JS-LOOP-001-04-deep-nesting-ts-10-5) | Nested loop depth ≥ 2 | `04\_deep\_nesting\.ts:10` | `score` | info | MEDIUM | production | 18 static cost units \(v2\) · 18× median · top 4% |
| 7 | [`PERF\-JS\-ALLOC\-001:01\_alloc\_churn\.ts:14:20`](#finding-PERF-JS-ALLOC-001-01-alloc-churn-ts-14-20) | Loop\-invariant object/array literal inside loop | `01\_alloc\_churn\.ts:14` | `buildEnvelopes` | info | MEDIUM | production | 13 static cost units \(v2\) · 13× median · top 7% |
| 8 | [`PERF\-JS\-ALLOC\-003:02\_array\_accumulation\.ts:8:11`](#finding-PERF-JS-ALLOC-003-02-array-accumulation-ts-8-11) | Superlinear spread accumulation in loop | `02\_array\_accumulation\.ts:8` | `flatten` | info | MEDIUM | production | 13 static cost units \(v2\) · 13× median · top 13% |
| 9 | [`PERF\-JS\-CALL\-001:09\_recursion\.ts:10:10`](#finding-PERF-JS-CALL-001-09-recursion-ts-10-10) | Direct recursion without a visible base case | `09\_recursion\.ts:10` | `descend` | info | MEDIUM | production | 5 static cost units \(v2\) · 5\.0× median · top 33% |
| 10 | [`PERF\-JS\-ARCH\-001:11\_cycle\_b\.ts:3:1`](#finding-PERF-JS-ARCH-001-11-cycle-b-ts-3-1) | Import cycle | `11\_cycle\_b\.ts:3` | `\<module\>` | info | MEDIUM | production | 1 static cost units \(v2\) · 1\.0× median · top 78% |
| 11 | [`PERF\-JS\-TYPE\-001:12\_delete\.ts:6:3`](#finding-PERF-JS-TYPE-001-12-delete-ts-6-3) | \`delete\` on an object property | `12\_delete\.ts:6` | `dropField` | info | MEDIUM | production | 1 static cost units \(v2\) · 1\.0× median · top 81% |

<a id="finding-PERF-JS-ASYNC-001-07-async-await-ts-12-19"></a>
#### \`await\` inside a loop  (`PERF\-JS\-ASYNC\-001:07\_async\_await\.ts:12:19`)
- Location: `07\_async\_await\.ts:12` (loadAll)
- Message: an \`await\` runs inside a loop at depth 1 — awaiting each item sequentially serialises one microtask round\-trip per iteration; collecting the promises and awaiting \`Promise\.all\(items\.map\(\.\.\.\)\)\` is a candidate optimization\. Accepted FP class: deliberately sequential work \(rate limiting, ordering\)
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: loop\_depth=1

<a id="finding-PERF-JS-ASYNC-002-08-async-blocking-ts-7-10"></a>
#### Synchronous blocking call inside \`async\` function  (`PERF\-JS\-ASYNC\-002:08\_async\_blocking\.ts:7:10`)
- Location: `08\_async\_blocking\.ts:7` (readConfig)
- Message: synchronous 'readFileSync' is called inside an \`async\` function — the blocking call stalls the single event\-loop thread for every concurrent task; using the async equivalent \(\`fs/promises\`, async \`exec\`/\`spawn\`\) is a candidate optimization\. Accepted FP class: a CLI single\-shot path
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=readFileSync, loop\_depth=0

<a id="finding-PERF-JS-LOOP-002-05-sync-io-loop-ts-9-17"></a>
#### Synchronous blocking I/O inside loop  (`PERF\-JS\-LOOP\-002:05\_sync\_io\_loop\.ts:9:17`)
- Location: `05\_sync\_io\_loop\.ts:9` (loadAll)
- Message: synchronous 'readFileSync' runs inside a loop at depth 1 — each iteration performs a blocking syscall/serialization on the single event\-loop thread; batching the work, using the async API \(\`fs/promises\`, async \`exec\`\), or making one call outside the loop is a candidate optimization\. Accepted FP class: a startup\-only loop
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=readFileSync, loop\_depth=1

<a id="finding-PERF-JS-LOOP-007-06-membership-scan-ts-8-9"></a>
#### Linear membership scan inside loop  (`PERF\-JS\-LOOP\-007:06\_membership\_scan\.ts:8:9`)
- Location: `06\_membership\_scan\.ts:8` (intersect)
- Message: 'seen\.includes' scans the collection linearly on every iteration at loop depth 1 — the nested linear scan makes the loop O\(n²\); building a \`Set\`/\`Map\` once and doing a keyed lookup is a candidate optimization\. Accepted FP class: small \(\< 16\) collections
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=includes, loop\_depth=1, receiver\_root=seen

<a id="finding-PERF-JS-STR-001-03-string-build-ts-8-5"></a>
#### String accumulation with \`\+=\` inside loop  (`PERF\-JS\-STR\-001:03\_string\_build\.ts:8:5`)
- Location: `03\_string\_build\.ts:8` (render)
- Message: 'out' accumulates a string with '\+=' or 'out = out \+ \.\.\.' inside a loop at depth 1 — this builds a cons\-string chain; V8 ropes mitigate short chains, but flattening the rope at first use is linear and repeated concatenation is quadratic; collecting parts into an array and joining once \(\`parts\.join\(""\)\`\) is a candidate optimization\. Accepted FP class: a tiny bounded loop \(\< 8 iterations\)
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: loop\_depth=1, target=out

<a id="finding-PERF-JS-ALLOC-001-01-alloc-churn-ts-14-20"></a>
#### Loop\-invariant object/array literal inside loop  (`PERF\-JS\-ALLOC\-001:01\_alloc\_churn\.ts:14:20`)
- Location: `01\_alloc\_churn\.ts:14` (buildEnvelopes)
- Message: an object literal is constructed on every iteration at loop depth 1 — V8 allocates it in the young generation and escape analysis rarely elides a per\-iteration literal; hoisting the loop\-invariant construction \(or reusing one object\) is a candidate optimization\. Accepted FP class: a literal created once for a different reason but mutated per iteration
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: kind=object, loop\_depth=1, referenced\_names=

<a id="finding-PERF-JS-ALLOC-003-02-array-accumulation-ts-8-11"></a>
#### Superlinear spread accumulation in loop  (`PERF\-JS\-ALLOC\-003:02\_array\_accumulation\.ts:8:11`)
- Location: `02\_array\_accumulation\.ts:8` (flatten)
- Message: 'acc = \[\.\.\.acc, \.\.\.\]' copies n elements per iteration at loop depth 1 — the spread materialises a fresh array each pass, making the loop O\(n²\); pushing into one array \(\`acc\.push\(x\)\`\) or spreading once after the loop is a candidate optimization\. Accepted FP class: a deliberate immutable snapshot per iteration
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: kind=spread\-accumulator, loop\_depth=1, receiver\_root=acc

<a id="finding-PERF-JS-ARCH-001-11-cycle-b-ts-3-1"></a>
#### Import cycle  (`PERF\-JS\-ARCH\-001:11\_cycle\_b\.ts:3:1`)
- Location: `11\_cycle\_b\.ts:3` (\<module\>)
- Message: '11\_cycle\_b\.ts' and '10\_cycle\_a\.ts' import each other, forming a cycle — module initialisation order becomes observable and bundlers cannot tree\-shake the cycle cleanly; breaking the lowest\-sorted edge into a shared leaf module is a candidate optimization\. Accepted FP class: type\-only import cycles \(TypeScript erases them\)
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: coupled\_with=10\_cycle\_a\.ts, kind=cycle

<a id="finding-PERF-JS-CALL-001-09-recursion-ts-10-10"></a>
#### Direct recursion without a visible base case  (`PERF\-JS\-CALL\-001:09\_recursion\.ts:10:10`)
- Location: `09\_recursion\.ts:10` (descend)
- Message: 'descend' calls itself directly with no visible base\-case guard — each recursive frame grows the stack and repeats frame\-setup cost; converting to a loop with an accumulator, or adding an explicit base\-case guard, is a candidate optimization\. Accepted FP class: a recursive tree walk with small depth, or a same\-name wrapper assignment delegating to another function
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: function=descend, loop\_depth=0

<a id="finding-PERF-JS-LOOP-001-04-deep-nesting-ts-10-5"></a>
#### Nested loop depth ≥ 2  (`PERF\-JS\-LOOP\-001:04\_deep\_nesting\.ts:10:5`)
- Location: `04\_deep\_nesting\.ts:10` (score)
- Message: loops nest 2 deep here \(chain reaches depth 2\) — every extra level multiplies the iteration count, so the body runs superlinearly; flattening the nesting, hoisting invariant work, or adding an early exit is a candidate optimization\. Accepted FP class: matrix work where n² is inherent
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: max\_depth=2, nest\_depth=2

<a id="finding-PERF-JS-TYPE-001-12-delete-ts-6-3"></a>
#### \`delete\` on an object property  (`PERF\-JS\-TYPE\-001:12\_delete\.ts:6:3`)
- Location: `12\_delete\.ts:6` (dropField)
- Message: 'delete' removes a property from a long\-lived object \('prop'\), forcing a V8 hidden\-class → dictionary\-mode transition that slows every later access; setting the property to \`undefined\` on a fresh per\-call copy or rebuilding the object is a candidate optimization\. Accepted FP class: intentional cache eviction \(rare\) and \`process\.env\.\*\` deletes \(a host\-owned global\)
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: property=prop
