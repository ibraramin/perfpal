# Perfpal Findings Detail

29 of 29 total findings.

#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:01\_alloc\_churn\.py:23:15`)
- Location: `01\_alloc\_churn\.py:23` (\<module\>)
- Message: 'acc' is grown with non\-in\-place '\+' inside a loop at depth 1 — CPython rebuilds and copies the whole list each pass \(list\_resize \+ memcpy\), making the loop O\(n²\); using 'append'/'extend' \(or a list comprehension\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=concat, loop\_depth=1, target=acc

#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:12:9`)
- Location: `10\_comprehension\.py:12` (\<module\>)
- Message: list 'out' is grown with 'append' inside the loop at depth 1 without being rebuilt there — the container accumulates across iterations, so list\_resize growth plus any scan over it makes the loop superlinear; accumulating into a local or using a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=append, loop\_depth=1, receiver\_root=out

#### Superlinear list accumulation in loop  (`PERF\-PY\-ALLOC\-003:10\_comprehension\.py:21:9`)
- Location: `10\_comprehension\.py:21` (\<module\>)
- Message: list 'out' is grown with 'append' inside the loop at depth 1 without being rebuilt there — the container accumulates across iterations, so list\_resize growth plus any scan over it makes the loop superlinear; accumulating into a local or using a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, growth=append, loop\_depth=1, receiver\_root=out

#### Blocking call inside \`async def\`  (`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:10:12`)
- Location: `14\_async\_blocking\.py:10` (\<module\>)
- Message: 'get' blocks inside an \`async def\` — the synchronous call stalls the single event\-loop thread and every other task; using the async equivalent \(\`asyncio\.sleep\`, an async HTTP/DB client\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=requests\.get, loop\_depth=0, receiver\_root=requests

#### Blocking call inside \`async def\`  (`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:15:5`)
- Location: `14\_async\_blocking\.py:15` (\<module\>)
- Message: 'sleep' blocks inside an \`async def\` — the synchronous call stalls the single event\-loop thread and every other task; using the async equivalent \(\`asyncio\.sleep\`, an async HTTP/DB client\) is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=time\.sleep, loop\_depth=0, receiver\_root=time

#### \`await\` inside a loop  (`PERF\-PY\-ASYNC\-002:14\_async\_blocking\.py:27:16`)
- Location: `14\_async\_blocking\.py:27` (\<module\>)
- Message: an \`await\` runs inside a loop at depth 1 — awaiting sequentially serialises each coroutine's latency; collecting the coroutines and awaiting \`asyncio\.gather\(\.\.\.\)\` is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: loop\_depth=1

#### Expensive I/O call inside loop  (`PERF\-PY\-LOOP\-002:04\_stdio\_churn\.py:8:14`)
- Location: `04\_stdio\_churn\.py:8` (\<module\>)
- Message: 'open' acquires a file/network/subprocess handle every iteration at loop depth 1 — setup dominates the loop; hoisting the handle out of the loop is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=open, io\_kind=hoistable, loop\_depth=1

#### Linear list scan inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:9:12`)
- Location: `07\_membership\_scan\.py:9` (\<module\>)
- Message: list 'table' is tested with 'in' inside a loop at depth 1 — CPython's list membership is an O\(n\) linear scan, so re\-testing per outer iteration is O\(n²\); building a set/dict lookup or iterating directly is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, loop\_depth=1, operator=in, root=table

#### Linear index scan of a list inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:19:18`)
- Location: `07\_membership\_scan\.py:19` (\<module\>)
- Message: list 'data' is indexed by loop variable 'i' at depth 1 — CPython's list subscript is O\(1\) per access, but a scan over the whole list re\-walks it per outer iteration \(O\(n²\)\); iterating directly, using \`enumerate\`, or building a set/dict lookup is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, index\_ident=i, loop\_depth=1, root=data

#### Linear list scan inside loop  (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:28:18`)
- Location: `07\_membership\_scan\.py:28` (\<module\>)
- Message: list 'data' is scanned with \`\.index\(\)\` inside a loop at depth 1 — CPython's list \`\.index\(\)\` is an O\(n\) linear scan, so re\-scanning per outer iteration is O\(n²\); building a set/dict lookup or a positional map is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=list, loop\_depth=1, method=index, root=data

#### DB/HTTP call inside loop \(N\+1\)  (`PERF\-PY\-N1\-001:12\_io\_n1\.py:8:16`)
- Location: `12\_io\_n1\.py:8` (\<module\>)
- Message: 'conn\.execute' issues a db round\-trip every iteration at loop depth 1 — the N\+1 pattern \(one query/request per element\); batching into a single \`IN \(\.\.\.\)\`, JOIN, or bulk request is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: callee=conn\.execute, family=db, loop\_depth=1, receiver\_root=conn

#### String accumulation in loop  (`PERF\-PY\-STR\-001:02\_string\_building\.py:8:9`)
- Location: `02\_string\_building\.py:8` (\<module\>)
- Message: string 's' is accumulated with '\+=' inside a loop at depth 1 — CPython strings are immutable, so each pass allocates a new string and copies both operands \(O\(n²\)\); accumulating pieces in a list and calling \`str\.join\` once is a candidate optimization
- Confidence: MEDIUM · Severity: warn · Resolver tier: syntactic
- Evidence: binding\_kind=str, form=in\_place, identifier=s, loop\_depth=1

#### Direct recursion  (`PERF\-PY\-CALL\-001:05\_recursion\_complexity\.py:8:12`)
- Location: `05\_recursion\_complexity\.py:8` (\<module\>)
- Message: 'walk\_down' calls itself directly at 1 site\(s\) — each recursive frame costs a CPython call and risks \`RecursionError\` at the interpreter recursion limit; converting to iteration or memoizing is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: function=walk\_down, self\_call\_sites=1

#### High cyclomatic complexity  (`PERF\-PY\-CALL\-002:05\_recursion\_complexity\.py:16:1`)
- Location: `05\_recursion\_complexity\.py:16` (\<module\>)
- Message: 'branchy' branches 17 ways \(threshold 15\) — complex control flow hides hot paths and resists testing; extracting helpers or table\-driving the dispatch is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: complexity=17, function=branchy, threshold=15

#### Deep loop nesting  (`PERF\-PY\-LOOP\-001:03\_deep\_nesting\.py:9:13`)
- Location: `03\_deep\_nesting\.py:9` (\<module\>)
- Message: loops nest 3 deep here \(chain reaches depth 3\) — CPython runs an interpreted loop body per element, so an O\(n^k\) nest is the primary hotspot zone; flattening the nesting or using itertools early\-exit helpers is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: max\_depth=3, nest\_depth=3

#### \`range\(len\(seq\)\)\` iteration  (`PERF\-PY\-LOOP\-003:08\_range\_len\.py:8:18`)
- Location: `08\_range\_len\.py:8` (\<module\>)
- Message: \`range\(len\(\.\.\.\)\)\` at loop depth 1 indexes the sequence by position and re\-runs \`len\` each pass — iterating the sequence directly or using \`enumerate\` is a candidate optimization
- Confidence: HIGH · Severity: info · Resolver tier: syntactic
- Evidence: callee=range, loop\_depth=1

#### Loop\-invariant construction inside loop  (`PERF\-PY\-ALLOC\-001:01\_alloc\_churn\.py:9:15`)
- Location: `01\_alloc\_churn\.py:9` (\<module\>)
- Message: 'dict' constructs a fresh value on every iteration at loop depth 1 — CPython allocates a new object header and backing store each pass \(and deallocates it on rebind\); hoisting the loop\-invariant construction is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=dict, loop\_depth=1

#### Eager iterable materialisation inside loop  (`PERF\-PY\-ALLOC\-004:01\_alloc\_churn\.py:42:22`)
- Location: `01\_alloc\_churn\.py:42` (\<module\>)
- Message: 'list' called with a non\-literal iterable every iteration at loop depth 1 — CPython eagerly allocates and copies the whole sequence, defeating lazy iteration; keeping the lazy generator \(or hoisting\) is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: arg\_kind=identifier, callee=list, loop\_depth=1

#### Import cycle  (`PERF\-PY\-ARCH\-001:pkg/cycle\_b\.py:3:1`)
- Location: `pkg/cycle\_b\.py:3` (\<module\>)
- Message: 'pkg/cycle\_b\.py' and 'pkg/cycle\_a\.py' import each other, forming a cycle — CPython executes a partially\-initialised module during the cycle, and the coupling resists testing; breaking the lowest\-sorted edge into a shared leaf module is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: coupled\_with=pkg/cycle\_a\.py, kind=cycle

#### Hot component  (`PERF\-PY\-ARCH\-002:05\_recursion\_complexity\.py:16:1`)
- Location: `05\_recursion\_complexity\.py:16` (\<module\>)
- Message: directory '\.' aggregates complexity 111 across 60 function\(s\) — the hottest component in this scan \(thresholds are conservative and not yet measured on Python\); splitting or simplifying its hottest functions is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: complexity\_sum=111, function\_count=60, max\_loop\_depth=3, path\_prefix=\.

#### Manual loop collapsible to a comprehension  (`PERF\-PY\-COMPREHENSION\-001:10\_comprehension\.py:12:9`)
- Location: `10\_comprehension\.py:12` (\<module\>)
- Message: loop body appends to list 'out' as its only statement at loop depth 1 — a list comprehension compiles to a specialised \`LIST\_APPEND\` with inline iteration, avoiding the loop branch and per\-element method lookup; rewriting it as a comprehension is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: loop\_depth=1, receiver\_root=out

#### Executor/event\-loop creation per iteration  (`PERF\-PY\-EXECUTOR\-001:11\_executors\.py:9:14`)
- Location: `11\_executors\.py:9` (\<module\>)
- Message: 'concurrent\.futures\.ThreadPoolExecutor' is constructed every iteration at loop depth 1 — CPython spawns worker threads/processes or allocates a new event loop \(selector \+ self\-pipe\) each pass, never amortising the setup; creating the executor once outside the loop is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=concurrent\.futures\.ThreadPoolExecutor, loop\_depth=1

#### \`try\`/\`except\` inside a loop  (`PERF\-PY\-LOOP\-009:09\_try\_in\_loop\.py:8:9`)
- Location: `09\_try\_in\_loop\.py:8` (\<module\>)
- Message: a \`try\`/\`except\` sits inside a loop body at depth 1 — CPython sets up exception\-handling state for the block, so per\-iteration \`try\` adds work on the hot path \(the zero\-cost\-\`try\` optimization is version\-dependent: CPython ≥ 3\.11\)\. Hoisting the \`try\` out of the loop or narrowing it is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: loop\_depth=1

#### Container copy inside loop  (`PERF\-PY\-MEM\-002:06\_memory\_ops\.py:10:16`)
- Location: `06\_memory\_ops\.py:10` (\<module\>)
- Message: 'deepcopy' copies a whole container every iteration at loop depth 1 — CPython allocates and fills a new object \(\`deepcopy\` also walks the graph with a memo dict\); copying once outside the loop or reusing the buffer is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=copy\.deepcopy, loop\_depth=1, receiver\_root=copy

#### Row\-wise pandas iteration  (`PERF\-PY\-PANDAS\-001:13\_pandas\.py:9:19`)
- Location: `13\_pandas\.py:9` (\<module\>)
- Message: 'df\.iterrows' iterates row by row — pandas materialises a Python object per row \(\`iterrows\` builds a \`Series\` with dtype coercion\), so the per\-row overhead dwarfs the vectorised work; column arithmetic or \`groupby\`/\`itertuples\`\-free vectorisation is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=df\.iterrows, loop\_depth=0, receiver\_root=df

#### \`concat\`/\`append\` in loop  (`PERF\-PY\-PANDAS\-002:13\_pandas\.py:18:15`)
- Location: `13\_pandas\.py:18` (\<module\>)
- Message: 'pandas\.concat' rebuilds the whole DataFrame every iteration at loop depth 1 — pandas copies the full frame on each \`concat\`/\`append\` \(O\(n²\)\); collecting rows in a list and concatenating once after the loop is a candidate optimization
- Confidence: MEDIUM · Severity: info · Resolver tier: syntactic
- Evidence: callee=pandas\.concat, loop\_depth=1, receiver\_root=pd
