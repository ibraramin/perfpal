# Perfpal Report
> schema_version 1\.4 · perfpal 0\.2\.0\-beta\.1 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 20 files

## 1. Executive summary
29 findings: 0 hot, 12 warn, 17 info. Parse health: 20 ok, 0 partial, 0 failed.

## 2. Scan provenance & parse health
- Tool: perfpal 0\.2\.0\-beta\.1
- Scan root: `<ROOT>`
- Files scanned: 20

### Parse health summary

| status | files |
|---|---:|
| ok | 20 |
| partial | 0 |
| failed | 0 |

No failed or partially parsed files.

Findings overlapping parse-error regions are flagged (`error_node_overlap`) and confidence-capped — never silently dropped.

## 3. Codebase statistics
- Languages: python
- Total LOC: 278 code / 48 comment / 109 blank

Top files by code LOC:

| path | loc.code | loc.comment | loc.blank | functions |
|---|---:|---:|---:|---:|
| 05\_recursion\_complexity\.py | 44 | 4 | 8 | 5 |
| 01\_alloc\_churn\.py | 27 | 9 | 12 | 7 |
| 07\_membership\_scan\.py | 26 | 4 | 8 | 5 |
| 14\_async\_blocking\.py | 16 | 5 | 12 | 6 |
| 03\_deep\_nesting\.py | 14 | 2 | 4 | 3 |
| 09\_try\_in\_loop\.py | 14 | 2 | 4 | 3 |
| 10\_comprehension\.py | 14 | 2 | 6 | 4 |
| 13\_pandas\.py | 14 | 3 | 7 | 4 |
| pkg/arch\_helpers\.py | 13 | 0 | 5 | 3 |
| 02\_string\_building\.py | 12 | 2 | 4 | 3 |

Distribution — cyclomatic\_complexity:

| bucket | count |
|---|---:|
| 01\-04 | 67 |
| 05\-09 | 0 |
| 10\-14 | 0 |
| 15\-24 | 1 |
| 25\-49 | 0 |
| 50\+ | 0 |

Distribution — max\_loop\_depth:

| bucket | count |
|---|---:|
| 0 | 42 |
| 1 | 23 |
| 2 | 2 |
| 3 | 1 |
| 4 | 0 |
| 5 | 0 |
| 6\+ | 0 |

## 4. Findings
22 rules: 29 of 29 total findings (0 hot, 12 warn, 17 info).

| rule_id | findings | hot | warn | info | top exemplar |
|---|---:|---:|---:|---:|---|
| `PERF\-PY\-ALLOC\-003` | 3 | 0 | 3 | 0 | Superlinear list accumulation in loop (`PERF\-PY\-ALLOC\-003:01\_alloc\_churn\.py:23:15`) — `01\_alloc\_churn\.py:23:15` |
| `PERF\-PY\-LOOP\-007` | 3 | 0 | 3 | 0 | Linear index scan of a list inside loop (`PERF\-PY\-LOOP\-007:07\_membership\_scan\.py:19:18`) — `07\_membership\_scan\.py:19:18` |
| `PERF\-PY\-ASYNC\-001` | 2 | 0 | 2 | 0 | Blocking call inside \`async def\` (`PERF\-PY\-ASYNC\-001:14\_async\_blocking\.py:10:12`) — `14\_async\_blocking\.py:10:12` |
| `PERF\-PY\-ASYNC\-002` | 1 | 0 | 1 | 0 | \`await\` inside a loop (`PERF\-PY\-ASYNC\-002:14\_async\_blocking\.py:27:16`) — `14\_async\_blocking\.py:27:16` |
| `PERF\-PY\-LOOP\-002` | 1 | 0 | 1 | 0 | Expensive I/O call inside loop (`PERF\-PY\-LOOP\-002:04\_stdio\_churn\.py:8:14`) — `04\_stdio\_churn\.py:8:14` |
| `PERF\-PY\-N1\-001` | 1 | 0 | 1 | 0 | DB/HTTP call inside loop \(N\+1\) (`PERF\-PY\-N1\-001:12\_io\_n1\.py:8:16`) — `12\_io\_n1\.py:8:16` |
| `PERF\-PY\-STR\-001` | 1 | 0 | 1 | 0 | String accumulation in loop (`PERF\-PY\-STR\-001:02\_string\_building\.py:8:9`) — `02\_string\_building\.py:8:9` |
| `PERF\-PY\-ARCH\-003` | 3 | 0 | 0 | 3 | Cross\-file call coupling (`PERF\-PY\-ARCH\-003:15\_cross\_file\_and\_arch\.py:8:12`) — `15\_cross\_file\_and\_arch\.py:8:12` |
| `PERF\-PY\-ALLOC\-001` | 1 | 0 | 0 | 1 | Loop\-invariant construction inside loop (`PERF\-PY\-ALLOC\-001:01\_alloc\_churn\.py:9:15`) — `01\_alloc\_churn\.py:9:15` |
| `PERF\-PY\-ALLOC\-004` | 1 | 0 | 0 | 1 | Eager iterable materialisation inside loop (`PERF\-PY\-ALLOC\-004:01\_alloc\_churn\.py:42:22`) — `01\_alloc\_churn\.py:42:22` |
| `PERF\-PY\-ARCH\-001` | 1 | 0 | 0 | 1 | Import cycle (`PERF\-PY\-ARCH\-001:pkg/cycle\_b\.py:3:1`) — `pkg/cycle\_b\.py:3:1` |
| `PERF\-PY\-ARCH\-002` | 1 | 0 | 0 | 1 | Hot component (`PERF\-PY\-ARCH\-002:05\_recursion\_complexity\.py:16:1`) — `05\_recursion\_complexity\.py:16:1` |
| `PERF\-PY\-CALL\-001` | 1 | 0 | 0 | 1 | Direct recursion (`PERF\-PY\-CALL\-001:05\_recursion\_complexity\.py:8:12`) — `05\_recursion\_complexity\.py:8:12` |
| `PERF\-PY\-CALL\-002` | 1 | 0 | 0 | 1 | High cyclomatic complexity (`PERF\-PY\-CALL\-002:05\_recursion\_complexity\.py:16:1`) — `05\_recursion\_complexity\.py:16:1` |
| `PERF\-PY\-COMPREHENSION\-001` | 1 | 0 | 0 | 1 | Manual loop collapsible to a comprehension (`PERF\-PY\-COMPREHENSION\-001:10\_comprehension\.py:12:9`) — `10\_comprehension\.py:12:9` |
| `PERF\-PY\-EXECUTOR\-001` | 1 | 0 | 0 | 1 | Executor/event\-loop creation per iteration (`PERF\-PY\-EXECUTOR\-001:11\_executors\.py:9:14`) — `11\_executors\.py:9:14` |
| `PERF\-PY\-LOOP\-001` | 1 | 0 | 0 | 1 | Deep loop nesting (`PERF\-PY\-LOOP\-001:03\_deep\_nesting\.py:9:13`) — `03\_deep\_nesting\.py:9:13` |
| `PERF\-PY\-LOOP\-003` | 1 | 0 | 0 | 1 | \`range\(len\(seq\)\)\` iteration (`PERF\-PY\-LOOP\-003:08\_range\_len\.py:8:18`) — `08\_range\_len\.py:8:18` |
| `PERF\-PY\-LOOP\-009` | 1 | 0 | 0 | 1 | \`try\`/\`except\` inside a loop (`PERF\-PY\-LOOP\-009:09\_try\_in\_loop\.py:8:9`) — `09\_try\_in\_loop\.py:8:9` |
| `PERF\-PY\-MEM\-002` | 1 | 0 | 0 | 1 | Container copy inside loop (`PERF\-PY\-MEM\-002:06\_memory\_ops\.py:10:16`) — `06\_memory\_ops\.py:10:16` |
| `PERF\-PY\-PANDAS\-001` | 1 | 0 | 0 | 1 | Row\-wise pandas iteration (`PERF\-PY\-PANDAS\-001:13\_pandas\.py:9:19`) — `13\_pandas\.py:9:19` |
| `PERF\-PY\-PANDAS\-002` | 1 | 0 | 0 | 1 | \`concat\`/\`append\` in loop (`PERF\-PY\-PANDAS\-002:13\_pandas\.py:18:15`) — `13\_pandas\.py:18:15` |

## 5. Architecture
[Dependency graph — internal import dependencies between scanned files.](charts/include-graph.svg)
[File hot-path map — top files by degree, heat by function cost; highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)
[Workspace map — components as nodes over inter-component call/include edges, heat by finding density.](charts/layer-workspace.svg)

Component ranking (hottest first):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| \. | \. | 1 | 105.26 |
| pkg | pkg | 2 | 96.77 |

Component detail pages (one per component above):

[Component \. — per-file hot-path map.](charts/component-01-root.svg)
[Component pkg — per-file hot-path map.](charts/component-02-pkg.svg)

The dependency graph reflects resolved internal include/module edges between scanned files — Python `import` bindings; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
Coupling heat blends edge counts with per-function cost (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / alloc_family; each edge weighs 1 + mean endpoint cost normalized by the scan maximum) — charts remain pure functions of metrics.json.
Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic DOT export of the include/call/component graphs (same source data as the SVGs above).
Standing caveat: cross-file call edges are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
| rule_id | title | description |
|---|---|---|
| `PERF\-PY\-ALLOC\-001` | Loop\-invariant construction inside loop | Detects a container/object constructor \(list/dict/set/tuple/str/bytes/open/DataFrame/Series/array/ndarray\) called inside a loop with only literal \(or no\) arguments — a loop\-invariant value rebuilt each pass; f\-string\-in\-loop is NOT a finding \(98–99\.6% of raw C hits were f\-strings\)\. |
| `PERF\-PY\-ALLOC\-003` | Superlinear list accumulation in loop | Detects non\-in\-place \`acc = acc \+ \[\.\.\.\]\` concatenation of a list\-bound name reassigned inside the loop, and append/extend onto a list\-bound receiver NOT rebuilt inside any enclosing loop \(the accumulator carried across iterations\) — CPython rebuilds and copies the list each pass \(O\(n²\)\)\. Silent when the receiver is reset inside the loop \(the benign CTL\-APPEND\-LOCAL control\)\. Flagship\. |
| `PERF\-PY\-ALLOC\-004` | Eager iterable materialisation inside loop | Detects \`list\(\)\`/\`dict\(\)\`/\`set\(\)\`/\`tuple\(\)\` with a non\-literal iterable argument inside a loop — forces an eager O\(n\) allocation and copy, defeating lazy iteration\. |
| `PERF\-PY\-ARCH\-001` | Import cycle | Detects cycles among RESOLVED internal Python import edges \(via break\_cycles\); unresolved internal imports stay data\-only with no finding\. Info, conservative — thresholds are not decision\-grade on Python\. |
| `PERF\-PY\-ARCH\-002` | Hot component | Ranks directories by aggregated cyclomatic complexity and reports only the hottest component above a conservative floor\. Info — thresholds are not measured on Python in W0 \(measured in W4\)\. |
| `PERF\-PY\-ARCH\-003` | Cross\-file call coupling | Detects name\-based cross\-file call edges \(a callee name with exactly one other\-file definition\); LOW confidence because Python duck typing makes name coupling noisy\. |
| `PERF\-PY\-ASYNC\-001` | Blocking call inside \`async def\` | Detects \`time\.sleep\`, blocking HTTP clients \(requests/httpx/urllib\), synchronous DB driver methods and \`open\` inside an \`async def\` — the blocking call stalls the single event\-loop thread\. The \`async def\` gate is required \(166 production blocking calls in sync functions vs 0 real hits\)\. |
| `PERF\-PY\-ASYNC\-002` | \`await\` inside a loop | Detects an \`await\` whose event metadata reports \`in\_loop\_depth ≥ 1\` \(additive AwaitEvent stream\) — sequential awaits serialise latency that \`asyncio\.gather\` could overlap\. |
| `PERF\-PY\-CALL\-001` | Direct recursion | Detects a function body containing a direct call to its own name; framed as CPython call cost and \`RecursionError\` depth, not C stack cost\. |
| `PERF\-PY\-CALL\-002` | High cyclomatic complexity | Detects branch\-count complexity above threshold \(default ≥ 15\), reported per function\. |
| `PERF\-PY\-COMPREHENSION\-001` | Manual loop collapsible to a comprehension | Detects a loop whose body is a single \`list\.append\` statement \(single\-statement gate\) appending to a list\-bound receiver — a list comprehension compiles to a specialised \`LIST\_APPEND\` with inline iteration\. |
| `PERF\-PY\-EXECUTOR\-001` | Executor/event\-loop creation per iteration | Detects \`ThreadPoolExecutor\(\.\.\.\)\`, \`ProcessPoolExecutor\(\.\.\.\)\` and \`asyncio\.new\_event\_loop\(\)\` constructed inside a loop — worker/selector setup is re\-paid every iteration and never amortises\. |
| `PERF\-PY\-LOOP\-001` | Deep loop nesting | Detects loop nesting depth exactly 3 \(one finding per maximal nest entry\) — an O\(n^k\) candidate zone under interpreted iteration\. |
| `PERF\-PY\-LOOP\-002` | Expensive I/O call inside loop | Detects file/network/subprocess/socket I/O inside a loop — direct I/O names, network\-rooted methods, and filehandle read/write methods\. \`print\`/\`input\` are deliberately excluded \(72–83% of the C rule's raw hits were logging\)\. The message splits hoistable handle setup \(open/urlopen/Popen/\.\.\.\) from element\-wise streaming transfer \(batching is the candidate\); evidence carries io\_kind\. |
| `PERF\-PY\-LOOP\-003` | \`range\(len\(seq\)\)\` iteration | Detects \`range\(len\(seq\)\)\` inside a loop \(the \`len\(\)\` gate keeps it false\-positive\-free\) — prefer direct iteration or \`enumerate\`\. |
| `PERF\-PY\-LOOP\-007` | Linear index scan of a list inside loop | Detects \(a\) membership \`x in list\` / \`x not in list\` where the container is a list\-bound name \(additive \`ComparisonEvent\.operator\`\), and \(b\) a list\-bound name subscripted by an enclosing loop's induction variable — including a lone \`seq\[i\]\` and deeper \`seq\[i\]\.field\` chains\. The kind gate excludes set/dict \(kills the CTL\-IN\-SET false\-positive class\)\. Flagship\. |
| `PERF\-PY\-LOOP\-009` | \`try\`/\`except\` inside a loop | Detects a \`try\_statement\` whose event metadata reports \`in\_loop\_depth ≥ 1\` \(additive TryEvent stream\) — CPython sets up per\-iteration exception handling\. Honest that the zero\-cost\-\`try\` claim is version\-dependent \(CPython ≥ 3\.11\)\. Info\. |
| `PERF\-PY\-MEM\-002` | Container copy inside loop | Detects \`copy\`/\`deepcopy\` \(module function or \`\.copy\(\)\` method\) inside a loop — a whole new container is materialised per iteration \(\`deepcopy\` also walks the graph with a memo dict\)\. |
| `PERF\-PY\-N1\-001` | DB/HTTP call inside loop \(N\+1\) | Detects a DB/HTTP round\-trip per iteration \(receiver\-root and call\-name family gated\) — the N\+1 pattern; batch into \`IN \(\.\.\.\)\`, a JOIN, or a bulk request\. Flagship\. |
| `PERF\-PY\-PANDAS\-001` | Row\-wise pandas iteration | Detects \`iterrows\`/\`itertuples\` or \`apply\(axis=1\)\` \(receiver\-root gated\) — pandas materialises a Python object per row, dwarfing the vectorised work\. Info; frequency caveat \(4 hits on the W0 corpora\)\. |
| `PERF\-PY\-PANDAS\-002` | \`concat\`/\`append\` in loop | Detects \`pd\.concat\`/\`DataFrame\.append\` inside a loop — the whole frame is copied per iteration \(O\(n²\)\)\. \`\.append\` is syntactically indistinguishable from \`list\.append\`, so the receiver must be a proven DataFrame\. Info; 0 hits on the W0 corpora\. |
| `PERF\-PY\-STR\-001` | String accumulation in loop | Detects string accumulation across iterations on a \`str\`\-bound identifier inside a loop: both the explicit \`s = s \+ piece\` form and the in\-place \`s \+= piece\` form \(additive augmented\-assignment BinaryOpEvent\)\. Gated on the name being reassigned within the loop \(accumulation, not a loop\-invariant read\) — CPython strings are immutable, so each pass allocates and copies \(O\(n²\)\); \`str\.join\` is the fix\. |

## Appendix B. How to read this report
**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or architecture impact: review `hot` findings first, schedule `warn`, and treat `info` as context signal feeding aggregation.
**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the false-positive risk of the detection shape — pure-syntax detections are HIGH, shape heuristics with documented FP classes are MEDIUM, and approximations (size models, cross-file name matching) are LOW. Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings whose evidence quality is reduced; their confidence is capped at MEDIUM.
Every finding is an **optimization candidate, never a certainty**: a candidate for human review, not a defect claim. Perfpal v1 runs on a purely syntactic tier (`resolver_tier: syntactic`) — no macro expansion, no type resolution.
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.4.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.

## Appendix C. All findings (full detail)
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
