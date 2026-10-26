# Perfpal Findings Detail

29 of 29 total findings.

## Complete ranked action list (KV1 order)

<a id="ranked-actions"></a>

| rank | finding | title | location | function | severity | confidence | path class | cost anchor |
|---:|---|---|---|---|---|---|---|---|
| 1 | [`PERF\-PY\-LOOP\-002:04\_stdio\_churn\.py:8:14`](#finding-PERF-PY-LOOP-002-04-stdio-churn-py-8-14) | Expensive I/O call inside loop | `04\_stdio\_churn\.py:8` | `read\_each` | warn | MEDIUM | production | 11 static cost units \(v2\) · 2\.8× median · top 10% |
| 2 | [`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:19:18`](#finding-PERF-PY-LOOP-007-07-membership-scan-py-19-18) | Linear index scan of a list inside loop | `07\_membership\_scan\.py:19` | `index\_scan` | warn | MEDIUM | production | 11 static cost units \(v2\) · 2\.8× median · top 12% |
| 3 | [`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:9:12`](#finding-PERF-PY-LOOP-007-07-membership-scan-py-9-12) | Linear list scan inside loop | `07\_membership\_scan\.py:9` | `contains\_any` | warn | MEDIUM | production | 10 static cost units \(v2\) · 2\.5× median · top 15% |
| 4 | [`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:12:9`](#finding-PERF-PY-ALLOC-003-10-comprehension-py-12-9) | Superlinear list accumulation in loop | `10\_comprehension\.py:12` | `transform\_all` | warn | MEDIUM | production | 8 static cost units \(v2\) · 2\.0× median · top 19% |
| 5 | [`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:21:9`](#finding-PERF-PY-ALLOC-003-10-comprehension-py-21-9) | Superlinear list accumulation in loop | `10\_comprehension\.py:21` | `accumulate\_multi` | warn | MEDIUM | production | 8 static cost units \(v2\) · 2\.0× median · top 18% |
| 6 | [`PERF\-PY\-ASYNC\-002:14\_async\_blocking\.py:27:16`](#finding-PERF-PY-ASYNC-002-14-async-blocking-py-27-16) | \`await\` inside a loop | `14\_async\_blocking\.py:27` | `await\_each` | warn | MEDIUM | production | 8 static cost units \(v2\) · 2\.0× median · top 21% |
| 7 | [`PERF\-PY\-ALLOC\-003:01\_alloc\_churn\.py:23:15`](#finding-PERF-PY-ALLOC-003-01-alloc-churn-py-23-15) | Superlinear list accumulation in loop | `01\_alloc\_churn\.py:23` | `superlinear\_concat` | warn | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 26% |
| 8 | [`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:28:18`](#finding-PERF-PY-LOOP-007-07-membership-scan-py-28-18) | Linear list scan inside loop | `07\_membership\_scan\.py:28` | `position\_of` | warn | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 35% |
| 9 | [`PERF\-PY\-N1\-001:12\_io\_n1\.py:8:16`](#finding-PERF-PY-N1-001-12-io-n1-py-8-16) | DB/HTTP call inside loop \(N\+1\) | `12\_io\_n1\.py:8` | `fetch\_each` | warn | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 41% |
| 10 | [`PERF\-PY\-STR\-001:02\_string\_building\.py:8:9`](#finding-PERF-PY-STR-001-02-string-building-py-8-9) | String accumulation in loop | `02\_string\_building\.py:8` | `build\_string` | warn | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 28% |
| 11 | [`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:10:12`](#finding-PERF-PY-ASYNC-001-14-async-blocking-py-10-12) | Blocking call inside \`async def\` | `14\_async\_blocking\.py:10` | `fetch\_blocking` | warn | MEDIUM | production | 4 static cost units \(v2\) · 1\.0× median · top 53% |
| 12 | [`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:15:5`](#finding-PERF-PY-ASYNC-001-14-async-blocking-py-15-5) | Blocking call inside \`async def\` | `14\_async\_blocking\.py:15` | `pause` | warn | MEDIUM | production | 1 static cost units \(v2\) · 0\.3× median · top 91% |
| 13 | [`PERF\-PY\-CALL\-002:05\_recursion\_complexity\.py:16:1`](#finding-PERF-PY-CALL-002-05-recursion-complexity-py-16-1) | High cyclomatic complexity | `05\_recursion\_complexity\.py:16` | `branchy` | info | HIGH | production | 33 static cost units \(v2\) · 8\.3× median · top 1% |
| 14 | [`PERF\-PY\-LOOP\-001:03\_deep\_nesting\.py:9:13`](#finding-PERF-PY-LOOP-001-03-deep-nesting-py-9-13) | Deep loop nesting | `03\_deep\_nesting\.py:9` | `triple\_nested` | info | HIGH | production | 25 static cost units \(v2\) · 6\.3× median · top 3% |
| 15 | [`PERF\-PY\-LOOP\-003:08\_range\_len\.py:8:18`](#finding-PERF-PY-LOOP-003-08-range-len-py-8-18) | \`range\(len\(seq\)\)\` iteration | `08\_range\_len\.py:8` | `positional\_sum` | info | HIGH | production | 15 static cost units \(v2\) · 3\.8× median · top 6% |
| 16 | [`PERF\-PY\-CALL\-001:05\_recursion\_complexity\.py:8:12`](#finding-PERF-PY-CALL-001-05-recursion-complexity-py-8-12) | Direct recursion | `05\_recursion\_complexity\.py:8` | `walk\_down` | info | HIGH | production | 7 static cost units \(v2\) · 1\.8× median · top 32% |
| 17 | [`PERF\-PY\-ALLOC\-001:01\_alloc\_churn\.py:9:15`](#finding-PERF-PY-ALLOC-001-01-alloc-churn-py-9-15) | Loop\-invariant construction inside loop | `01\_alloc\_churn\.py:9` | `loop\_invariant\_construction` | info | MEDIUM | production | 11 static cost units \(v2\) · 2\.8× median · top 9% |
| 18 | [`PERF\-PY\-ALLOC\-004:01\_alloc\_churn\.py:42:22`](#finding-PERF-PY-ALLOC-004-01-alloc-churn-py-42-22) | Eager iterable materialisation inside loop | `01\_alloc\_churn\.py:42` | `eager\_materialise` | info | MEDIUM | production | 11 static cost units \(v2\) · 2\.8× median · top 7% |
| 19 | [`PERF\-PY\-LOOP\-009:09\_try\_in\_loop\.py:8:9`](#finding-PERF-PY-LOOP-009-09-try-in-loop-py-8-9) | \`try\`/\`except\` inside a loop | `09\_try\_in\_loop\.py:8` | `swallow` | info | MEDIUM | production | 11 static cost units \(v2\) · 2\.8× median · top 13% |
| 20 | [`PERF\-PY\-COMPREHENSION\-001:10\_comprehension\.py:12:9`](#finding-PERF-PY-COMPREHENSION-001-10-comprehension-py-12-9) | Manual loop collapsible to a comprehension | `10\_comprehension\.py:12` | `transform\_all` | info | MEDIUM | production | 8 static cost units \(v2\) · 2\.0× median · top 19% |
| 21 | [`PERF\-PY\-EXECUTOR\-001:11\_executors\.py:9:14`](#finding-PERF-PY-EXECUTOR-001-11-executors-py-9-14) | Executor/event\-loop creation per iteration | `11\_executors\.py:9` | `fan\_out` | info | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 38% |
| 22 | [`PERF\-PY\-MEM\-002:06\_memory\_ops\.py:10:16`](#finding-PERF-PY-MEM-002-06-memory-ops-py-10-16) | Container copy inside loop | `06\_memory\_ops\.py:10` | `copy\_each` | info | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 34% |
| 23 | [`PERF\-PY\-PANDAS\-001:13\_pandas\.py:9:19`](#finding-PERF-PY-PANDAS-001-13-pandas-py-9-19) | Row\-wise pandas iteration | `13\_pandas\.py:9` | `row\_wise` | info | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 44% |
| 24 | [`PERF\-PY\-PANDAS\-002:13\_pandas\.py:18:15`](#finding-PERF-PY-PANDAS-002-13-pandas-py-18-15) | \`concat\`/\`append\` in loop | `13\_pandas\.py:18` | `append\_frames` | info | MEDIUM | production | 7 static cost units \(v2\) · 1\.8× median · top 43% |
| 25 | [`PERF\-PY\-ARCH\-001:pkg/cycle\_b\.py:3:1`](#finding-PERF-PY-ARCH-001-pkg-cycle-b-py-3-1) | Import cycle | `pkg/cycle\_b\.py:3` | `\<module\>` | info | MEDIUM | production | 1 static cost units \(v2\) · 0\.3× median · top 100% |
| 26 | [`PERF\-PY\-ARCH\-003:pkg/cycle\_b\.py:9:16`](#finding-PERF-PY-ARCH-003-pkg-cycle-b-py-9-16) | Cross\-file call coupling | `pkg/cycle\_b\.py:9` | `ping\_b` | info | LOW | production | 8 static cost units \(v2\) · 2\.0× median · top 24% |
| 27 | [`PERF\-PY\-ARCH\-003:pkg/cycle\_a\.py:8:12`](#finding-PERF-PY-ARCH-003-pkg-cycle-a-py-8-12) | Cross\-file call coupling | `pkg/cycle\_a\.py:8` | `walk\_cycle` | info | LOW | production | 5 static cost units \(v2\) · 1\.3× median · top 49% |
| 28 | [`PERF\-PY\-ARCH\-003:15\_cross\_file\_and\_arch\.py:8:12`](#finding-PERF-PY-ARCH-003-15-cross-file-and-arch-py-8-12) | Cross\-file call coupling | `15\_cross\_file\_and\_arch\.py:8` | `use\_cross\_file` | info | LOW | production | 2 static cost units \(v2\) · 0\.5× median · top 59% |

<a id="finding-PERF-PY-ALLOC-003-01-alloc-churn-py-23-15"></a>
#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:01\_alloc\_churn\.py:23:15`)
- Location: `01\_alloc\_churn\.py:23` (superlinear\_concat)
- Message: 'acc' is grown with non\-in\-place '\+' inside a loop at depth 1 — CPython rebuilds and copies the whole list each pass \(list\_resize \+ memcpy\), making the loop O\(n²\); using 'append'/'extend' \(or a list comprehension\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=concat, loop\_depth=1, target=acc

<a id="finding-PERF-PY-ALLOC-003-10-comprehension-py-12-9"></a>
#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:12:9`)
- Location: `10\_comprehension\.py:12` (transform\_all)
- Message: list 'out' is grown with 'append' inside the loop at depth 1 without being rebuilt there — the container accumulates across iterations, so list\_resize growth plus any scan over it makes the loop superlinear; accumulating into a local or using a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=append, loop\_depth=1, receiver\_root=out

<a id="finding-PERF-PY-ALLOC-003-10-comprehension-py-21-9"></a>
#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:21:9`)
- Location: `10\_comprehension\.py:21` (accumulate\_multi)
- Message: list 'out' is grown with 'append' inside the loop at depth 1 without being rebuilt there — the container accumulates across iterations, so list\_resize growth plus any scan over it makes the loop superlinear; accumulating into a local or using a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=append, loop\_depth=1, receiver\_root=out

<a id="finding-PERF-PY-ASYNC-001-14-async-blocking-py-10-12"></a>
#### Blocking call inside \`async def\`  (`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:10:12`)
- Location: `14\_async\_blocking\.py:10` (fetch\_blocking)
- Message: 'get' blocks inside an \`async def\` — the synchronous call stalls the single event\-loop thread and every other task; using the async equivalent \(\`asyncio\.sleep\`, an async HTTP/DB client\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=requests\.get, loop\_depth=0, receiver\_root=requests

<a id="finding-PERF-PY-ASYNC-001-14-async-blocking-py-15-5"></a>
#### Blocking call inside \`async def\`  (`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:15:5`)
- Location: `14\_async\_blocking\.py:15` (pause)
- Message: 'sleep' blocks inside an \`async def\` — the synchronous call stalls the single event\-loop thread and every other task; using the async equivalent \(\`asyncio\.sleep\`, an async HTTP/DB client\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=time\.sleep, loop\_depth=0, receiver\_root=time

<a id="finding-PERF-PY-ASYNC-002-14-async-blocking-py-27-16"></a>
#### \`await\` inside a loop  (`PERF\-PY\-ASYNC\-002:14\_async\_blocking\.py:27:16`)
- Location: `14\_async\_blocking\.py:27` (await\_each)
- Message: an \`await\` runs inside a loop at depth 1 — awaiting sequentially serialises each coroutine's latency; collecting the coroutines and awaiting \`asyncio\.gather\(\.\.\.\)\` is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: loop\_depth=1

<a id="finding-PERF-PY-LOOP-002-04-stdio-churn-py-8-14"></a>
#### Expensive I/O call inside loop  (`PERF\-PY\-LOOP\-002:04\_stdio\_churn\.py:8:14`)
- Location: `04\_stdio\_churn\.py:8` (read\_each)
- Message: 'open' acquires a file/network/subprocess handle every iteration at loop depth 1 — setup dominates the loop; hoisting the handle out of the loop is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=open, io\_kind=hoistable, loop\_depth=1

<a id="finding-PERF-PY-LOOP-007-07-membership-scan-py-9-12"></a>
#### Linear list scan inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:9:12`)
- Location: `07\_membership\_scan\.py:9` (contains\_any)
- Message: list 'table' is tested with 'in' inside a loop at depth 1 — CPython's list membership is an O\(n\) linear scan, so re\-testing per outer iteration is O\(n²\); building a set/dict lookup or iterating directly is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, loop\_depth=1, operator=in, root=table

<a id="finding-PERF-PY-LOOP-007-07-membership-scan-py-19-18"></a>
#### Linear index scan of a list inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:19:18`)
- Location: `07\_membership\_scan\.py:19` (index\_scan)
- Message: list 'data' is indexed by loop variable 'i' at depth 1 — CPython's list subscript is O\(1\) per access, but a scan over the whole list re\-walks it per outer iteration \(O\(n²\)\); iterating directly, using \`enumerate\`, or building a set/dict lookup is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, index\_ident=i, loop\_depth=1, root=data

<a id="finding-PERF-PY-LOOP-007-07-membership-scan-py-28-18"></a>
#### Linear list scan inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:28:18`)
- Location: `07\_membership\_scan\.py:28` (position\_of)
- Message: list 'data' is scanned with \`\.index\(\)\` inside a loop at depth 1 — CPython's list \`\.index\(\)\` is an O\(n\) linear scan, so re\-scanning per outer iteration is O\(n²\); building a set/dict lookup or a positional map is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, loop\_depth=1, method=index, root=data

<a id="finding-PERF-PY-N1-001-12-io-n1-py-8-16"></a>
#### DB/HTTP call inside loop \(N\+1\)  (`PERF\-PY\-N1\-001:12\_io\_n1\.py:8:16`)
- Location: `12\_io\_n1\.py:8` (fetch\_each)
- Message: 'conn\.execute' issues a db round\-trip every iteration at loop depth 1 — the N\+1 pattern \(one query/request per element\); batching into a single \`IN \(\.\.\.\)\`, JOIN, or bulk request is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=conn\.execute, family=db, loop\_depth=1, receiver\_root=conn

<a id="finding-PERF-PY-STR-001-02-string-building-py-8-9"></a>
#### String accumulation in loop  (`PERF\-PY\-STR\-001:02\_string\_building\.py:8:9`)
- Location: `02\_string\_building\.py:8` (build\_string)
- Message: string 's' is accumulated with '\+=' inside a loop at depth 1 — CPython strings are immutable, so each pass allocates a new string and copies both operands \(O\(n²\)\); accumulating pieces in a list and calling \`str\.join\` once is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=str, form=in\_place, identifier=s, loop\_depth=1

<a id="finding-PERF-PY-CALL-001-05-recursion-complexity-py-8-12"></a>
#### Direct recursion  (`PERF\-PY\-CALL\-001:05\_recursion\_complexity\.py:8:12`)
- Location: `05\_recursion\_complexity\.py:8` (walk\_down)
- Message: 'walk\_down' calls itself directly at 1 site\(s\) — each recursive frame costs a CPython call and risks \`RecursionError\` at the interpreter recursion limit; converting to iteration or memoizing is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: function=walk\_down, self\_call\_sites=1

<a id="finding-PERF-PY-CALL-002-05-recursion-complexity-py-16-1"></a>
#### High cyclomatic complexity  (`PERF\-PY\-CALL\-002:05\_recursion\_complexity\.py:16:1`)
- Location: `05\_recursion\_complexity\.py:16` (branchy)
- Message: 'branchy' branches 17 ways \(threshold 15\) — complex control flow hides hot paths and resists testing; extracting helpers or table\-driving the dispatch is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: complexity=17, function=branchy, threshold=15

<a id="finding-PERF-PY-LOOP-001-03-deep-nesting-py-9-13"></a>
#### Deep loop nesting  (`PERF\-PY\-LOOP\-001:03\_deep\_nesting\.py:9:13`)
- Location: `03\_deep\_nesting\.py:9` (triple\_nested)
- Message: loops nest 3 deep here \(chain reaches depth 3\) — CPython runs an interpreted loop body per element, so an O\(n^k\) nest is the primary hotspot zone; flattening the nesting or using itertools early\-exit helpers is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: max\_depth=3, nest\_depth=3

<a id="finding-PERF-PY-LOOP-003-08-range-len-py-8-18"></a>
#### \`range\(len\(seq\)\)\` iteration  (`PERF\-PY\-LOOP\-003:08\_range\_len\.py:8:18`)
- Location: `08\_range\_len\.py:8` (positional\_sum)
- Message: \`range\(len\(\.\.\.\)\)\` at loop depth 1 indexes the sequence by position and re\-runs \`len\` each pass — iterating the sequence directly or using \`enumerate\` is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: callee=range, loop\_depth=1

<a id="finding-PERF-PY-ALLOC-001-01-alloc-churn-py-9-15"></a>
#### Loop\-invariant construction inside loop  (`PERF\-PY\-ALLOC\-001:01\_alloc\_churn\.py:9:15`)
- Location: `01\_alloc\_churn\.py:9` (loop\_invariant\_construction)
- Message: 'dict' constructs a fresh value on every iteration at loop depth 1 — CPython allocates a new object header and backing store each pass \(and deallocates it on rebind\); hoisting the loop\-invariant construction is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=dict, loop\_depth=1

<a id="finding-PERF-PY-ALLOC-004-01-alloc-churn-py-42-22"></a>
#### Eager iterable materialisation inside loop  (`PERF\-PY\-ALLOC\-004:01\_alloc\_churn\.py:42:22`)
- Location: `01\_alloc\_churn\.py:42` (eager\_materialise)
- Message: 'list' called with a non\-literal iterable every iteration at loop depth 1 — CPython eagerly allocates and copies the whole sequence, defeating lazy iteration; keeping the lazy generator \(or hoisting\) is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: arg\_kind=identifier, callee=list, loop\_depth=1

<a id="finding-PERF-PY-ARCH-001-pkg-cycle-b-py-3-1"></a>
#### Import cycle  (`PERF\-PY\-ARCH\-001:pkg/cycle\_b\.py:3:1`)
- Location: `pkg/cycle\_b\.py:3` (\<module\>)
- Message: 'pkg/cycle\_b\.py' and 'pkg/cycle\_a\.py' import each other, forming a cycle — CPython executes a partially\-initialised module during the cycle, and the coupling resists testing; breaking the lowest\-sorted edge into a shared leaf module is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: coupled\_with=pkg/cycle\_a\.py, kind=cycle

<a id="finding-PERF-PY-ARCH-002-05-recursion-complexity-py-16-1"></a>
#### Hot component  (`PERF\-PY\-ARCH\-002:05\_recursion\_complexity\.py:16:1`)
- Location: `05\_recursion\_complexity\.py:16` (branchy)
- Message: directory '\.' aggregates complexity 111 across 60 function\(s\) — the hottest component in this scan \(thresholds are conservative and not yet measured on Python\); splitting or simplifying its hottest functions is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: complexity\_sum=111, function\_count=60, max\_loop\_depth=3, path\_prefix=\.

<a id="finding-PERF-PY-COMPREHENSION-001-10-comprehension-py-12-9"></a>
#### Manual loop collapsible to a comprehension  (`PERF\-PY\-COMPREHENSION\-001:10\_comprehension\.py:12:9`)
- Location: `10\_comprehension\.py:12` (transform\_all)
- Message: loop body appends to list 'out' as its only statement at loop depth 1 — a list comprehension compiles to a specialised \`LIST\_APPEND\` with inline iteration, avoiding the loop branch and per\-element method lookup; rewriting it as a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: loop\_depth=1, receiver\_root=out

<a id="finding-PERF-PY-EXECUTOR-001-11-executors-py-9-14"></a>
#### Executor/event\-loop creation per iteration  (`PERF\-PY\-EXECUTOR\-001:11\_executors\.py:9:14`)
- Location: `11\_executors\.py:9` (fan\_out)
- Message: 'concurrent\.futures\.ThreadPoolExecutor' is constructed every iteration at loop depth 1 — CPython spawns worker threads/processes or allocates a new event loop \(selector \+ self\-pipe\) each pass, never amortising the setup; creating the executor once outside the loop is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=concurrent\.futures\.ThreadPoolExecutor, loop\_depth=1

<a id="finding-PERF-PY-LOOP-009-09-try-in-loop-py-8-9"></a>
#### \`try\`/\`except\` inside a loop  (`PERF\-PY\-LOOP\-009:09\_try\_in\_loop\.py:8:9`)
- Location: `09\_try\_in\_loop\.py:8` (swallow)
- Message: a \`try\`/\`except\` sits inside a loop body at depth 1 — CPython sets up exception\-handling state for the block, so per\-iteration \`try\` adds work on the hot path \(the zero\-cost\-\`try\` optimization is version\-dependent: CPython ≥ 3\.11\)\. Hoisting the \`try\` out of the loop or narrowing it is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: loop\_depth=1

<a id="finding-PERF-PY-MEM-002-06-memory-ops-py-10-16"></a>
#### Container copy inside loop  (`PERF\-PY\-MEM\-002:06\_memory\_ops\.py:10:16`)
- Location: `06\_memory\_ops\.py:10` (copy\_each)
- Message: 'deepcopy' copies a whole container every iteration at loop depth 1 — CPython allocates and fills a new object \(\`deepcopy\` also walks the graph with a memo dict\); copying once outside the loop or reusing the buffer is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=copy\.deepcopy, loop\_depth=1, receiver\_root=copy

<a id="finding-PERF-PY-PANDAS-001-13-pandas-py-9-19"></a>
#### Row\-wise pandas iteration  (`PERF\-PY\-PANDAS\-001:13\_pandas\.py:9:19`)
- Location: `13\_pandas\.py:9` (row\_wise)
- Message: 'df\.iterrows' iterates row by row — pandas materialises a Python object per row \(\`iterrows\` builds a \`Series\` with dtype coercion\), so the per\-row overhead dwarfs the vectorised work; column arithmetic or \`groupby\`/\`itertuples\`\-free vectorisation is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=df\.iterrows, loop\_depth=0, receiver\_root=df

<a id="finding-PERF-PY-PANDAS-002-13-pandas-py-18-15"></a>
#### \`concat\`/\`append\` in loop  (`PERF\-PY\-PANDAS\-002:13\_pandas\.py:18:15`)
- Location: `13\_pandas\.py:18` (append\_frames)
- Message: 'pandas\.concat' rebuilds the whole DataFrame every iteration at loop depth 1 — pandas copies the full frame on each \`concat\`/\`append\` \(O\(n²\)\); collecting rows in a list and concatenating once after the loop is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=pandas\.concat, loop\_depth=1, receiver\_root=pd

<a id="finding-PERF-PY-ARCH-003-15-cross-file-and-arch-py-8-12"></a>
<a id="finding-PERF-PY-ARCH-003-pkg-cycle-a-py-8-12"></a>
<a id="finding-PERF-PY-ARCH-003-pkg-cycle-b-py-9-16"></a>
