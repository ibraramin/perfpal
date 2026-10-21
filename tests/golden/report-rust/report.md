# Perfpal Report
> schema_version 1\.7 · perfpal 0\.2\.0\-beta\.1 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 14 files

## 1. Executive summary
32 findings: 10 hot, 9 warn, 13 info. Parse health: 14 ok, 0 partial, 0 failed.

### Where do I look first?

Ranked: production code first, then severity, then confidence, then static cost.

1. **Heap allocation inside loop \(Rust\)** ([`PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:34:5`](findings-detail.md#finding-PERF-RS-ALLOC-001-src-01-alloc-churn-rs-34-5)) — `src/01\_alloc\_churn\.rs:34` · function `build\_row\_labels` · hot · HIGH · production
   - Cost: `22 static cost units (v2) · 3.1× median · top 10%`
   - Why: loop performs 3 per\-iteration allocation\(s\) \(format, to\_string, vec\) at loop depth 1 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
2. **Heap allocation inside loop \(Rust\)** ([`PERF\-RS\-ALLOC\-001:src/06\_memory\_ops\.rs:27:9`](findings-detail.md#finding-PERF-RS-ALLOC-001-src-06-memory-ops-rs-27-9)) — `src/06\_memory\_ops\.rs:27` · function `duplicate\_rows` · hot · HIGH · production
   - Cost: `22 static cost units (v2) · 3.1× median · top 10%`
   - Why: loop performs 1 per\-iteration allocation\(s\) \(to\_vec\) at loop depth 2 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
3. **Deallocation inside loop \(Rust\)** ([`PERF\-RS\-ALLOC\-002:src/01\_alloc\_churn\.rs:39:9`](findings-detail.md#finding-PERF-RS-ALLOC-002-src-01-alloc-churn-rs-39-9)) — `src/01\_alloc\_churn\.rs:39` · function `build\_row\_labels` · hot · HIGH · production
   - Cost: `22 static cost units (v2) · 3.1× median · top 10%`
   - Why: deallocation via 'drop' at loop depth 1 — repeated drop\-per\-iteration teardown; hoisting the drop or restructuring value ownership is a candidate optimization
4. **Heap allocation inside loop \(Rust\)** ([`PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:58:9`](findings-detail.md#finding-PERF-RS-ALLOC-001-src-01-alloc-churn-rs-58-9)) — `src/01\_alloc\_churn\.rs:58` · function `grid\_reset` · hot · HIGH · production
   - Cost: `19 static cost units (v2) · 2.7× median · top 25%`
   - Why: loop performs 1 per\-iteration allocation\(s\) \(vec\) at loop depth 2 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization
5. **Heap allocation inside loop \(Rust\)** ([`PERF\-RS\-ALLOC\-001:src/02\_string\_building\.rs:41:5`](findings-detail.md#finding-PERF-RS-ALLOC-001-src-02-string-building-rs-41-5)) — `src/02\_string\_building\.rs:41` · function `len\_with\_reassignment` · hot · HIGH · production
   - Cost: `14 static cost units (v2) · 2.0× median · top 25%`
   - Why: loop performs 1 per\-iteration allocation\(s\) \(format\) at loop depth 1 — allocator churn dominates runtime; hoisting the allocation or reusing a buffer is a candidate optimization

[Complete ranked list](findings-detail.md#ranked-actions) — all 31 findings (production-first).

0 non-production files demoted · 1 component-level finding in Explorer.

### What breaks if I touch it?

- `src/cycle\_a\.rs` — 2 dependents · 14% of scanned files · top 10%
- `src/cycle\_b\.rs` — 2 dependents · 14% of scanned files · top 25%
- `src/common\.rs` — 1 dependent · 7% of scanned files · top 25%

Blast-radius bars: [report.html#map](report.html#map). resolved 5 of 5 include edges.

### Where might time plausibly go?

- `src/03\_deep\_nesting\.rs:15` `grid\_checksum` — 25 static cost units (v2) · 3.6× median · top 5%
- `src/01\_alloc\_churn\.rs:32` `build\_row\_labels` — 22 static cost units (v2) · 3.1× median · top 10%
- `src/06\_memory\_ops\.rs:24` `duplicate\_rows` — 22 static cost units (v2) · 3.1× median · top 10%
- `src/01\_alloc\_churn\.rs:56` `grid\_reset` — 19 static cost units (v2) · 2.7× median · top 25%
- `src/09\_membership\_scan\.rs:13` `contains\_nested` — 19 static cost units (v2) · 2.7× median · top 25%

[Complete cost-ranked list (40 functions)](profile-queue.md).

0 startup functions demoted · 20 no-loop functions demoted.

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
22 rules: 32 of 32 total findings (10 hot, 9 warn, 13 info).

| rule_id | findings | hot | warn | info | top exemplar |
|---|---:|---:|---:|---:|---|
| `PERF\-RS\-ALLOC\-001` | 4 | 4 | 0 | 0 | Heap allocation inside loop \(Rust\) (`PERF\-RS\-ALLOC\-001:src/01\_alloc\_churn\.rs:34:5`) — `src/01\_alloc\_churn\.rs:34:5` |
| `PERF\-RS\-LOOP\-002` | 3 | 3 | 0 | 0 | Expensive setup or I/O call inside loop \(Rust\) (`PERF\-RS\-LOOP\-002:src/01\_alloc\_churn\.rs:40:9`) — `src/01\_alloc\_churn\.rs:40:9` |
| `PERF\-RS\-ALLOC\-002` | 1 | 1 | 0 | 0 | Deallocation inside loop \(Rust\) (`PERF\-RS\-ALLOC\-002:src/01\_alloc\_churn\.rs:39:9`) — `src/01\_alloc\_churn\.rs:39:9` |
| `PERF\-RS\-ALLOC\-003` | 1 | 1 | 0 | 0 | Growable\-buffer additive growth in loop \(Rust\) (`PERF\-RS\-ALLOC\-003:src/01\_alloc\_churn\.rs:49:9`) — `src/01\_alloc\_churn\.rs:49:9` |
| `PERF\-RS\-ARCH\-002` | 1 | 1 | 0 | 0 | Hot component \(Rust\) (`PERF\-RS\-ARCH\-002:src/05\_recursion\_complexity\.rs:20:1`) — `src/05\_recursion\_complexity\.rs:20:1` |
| `PERF\-RS\-STR\-002` | 2 | 0 | 2 | 0 | Length query recomputed in loop \(Rust\) (`PERF\-RS\-STR\-002:src/02\_string\_building\.rs:32:18`) — `src/02\_string\_building\.rs:32:18` |
| `PERF\-RS\-ARCH\-001` | 1 | 0 | 1 | 0 | Module\-graph cycle \(Rust\) (`PERF\-RS\-ARCH\-001:src/cycle\_b\.rs:6:1`) — `src/cycle\_b\.rs:6:1` |
| `PERF\-RS\-CALL\-002` | 1 | 0 | 1 | 0 | High cyclomatic complexity \(Rust\) (`PERF\-RS\-CALL\-002:src/05\_recursion\_complexity\.rs:20:1`) — `src/05\_recursion\_complexity\.rs:20:1` |
| `PERF\-RS\-LOOP\-001` | 1 | 0 | 1 | 0 | Deeply nested loops \(Rust\) (`PERF\-RS\-LOOP\-001:src/03\_deep\_nesting\.rs:19:13`) — `src/03\_deep\_nesting\.rs:19:13` |
| `PERF\-RS\-MEM\-001` | 1 | 0 | 1 | 0 | size\_of passed as an element count \(Rust\) (`PERF\-RS\-MEM\-001:src/06\_memory\_ops\.rs:15:5`) — `src/06\_memory\_ops\.rs:15:5` |
| `PERF\-RS\-MEM\-002` | 1 | 0 | 1 | 0 | Block copy/clone in loop \(Rust\) (`PERF\-RS\-MEM\-002:src/01\_alloc\_churn\.rs:61:13`) — `src/01\_alloc\_churn\.rs:61:13` |
| `PERF\-RS\-STR\-003` | 1 | 0 | 1 | 0 | Unbounded unsafe memory operation \(Rust\) (`PERF\-RS\-STR\-003:src/02\_string\_building\.rs:62:5`) — `src/02\_string\_building\.rs:62:5` |
| `PERF\-RS\-TYPE\-001` | 1 | 0 | 1 | 0 | Large struct passed by value \(Rust\) (`PERF\-RS\-TYPE\-001:src/07\_struct\_by\_value\.rs:40:1`) — `src/07\_struct\_by\_value\.rs:40:1` |
| `PERF\-RS\-ARCH\-003` | 5 | 0 | 0 | 5 | Cross\-module call coupling \(Rust\) (`PERF\-RS\-ARCH\-003:src/10\_cross\_file\_and\_arch\.rs:21:16`) — `src/10\_cross\_file\_and\_arch\.rs:21:16` |
| `PERF\-RS\-CALL\-001` | 1 | 0 | 0 | 1 | Direct recursion \(Rust\) (`PERF\-RS\-CALL\-001:src/05\_recursion\_complexity\.rs:14:9`) — `src/05\_recursion\_complexity\.rs:14:9` |
| `PERF\-RS\-LOOP\-003` | 1 | 0 | 0 | 1 | Character\-wise processing in loop \(Rust\) (`PERF\-RS\-LOOP\-003:src/04\_stdio\_churn\.rs:19:9`) — `src/04\_stdio\_churn\.rs:19:9` |
| `PERF\-RS\-LOOP\-004` | 1 | 0 | 0 | 1 | Variable\-divisor division in innermost loop \(Rust\) (`PERF\-RS\-LOOP\-004:src/03\_deep\_nesting\.rs:22:23`) — `src/03\_deep\_nesting\.rs:22:23` |
| `PERF\-RS\-LOOP\-005` | 1 | 0 | 0 | 1 | Global variable written inside loop \(Rust\) (`PERF\-RS\-LOOP\-005:src/08\_globals\_and\_chains\.rs:41:13`) — `src/08\_globals\_and\_chains\.rs:41:13` |
| `PERF\-RS\-LOOP\-006` | 1 | 0 | 0 | 1 | Indirect call in innermost loop \(Rust\) (`PERF\-RS\-LOOP\-006:src/08\_globals\_and\_chains\.rs:43:9`) — `src/08\_globals\_and\_chains\.rs:43:9` |
| `PERF\-RS\-LOOP\-007` | 1 | 0 | 0 | 1 | Linear membership scan nested in loop \(Rust\) (`PERF\-RS\-LOOP\-007:src/09\_membership\_scan\.rs:16:16`) — `src/09\_membership\_scan\.rs:16:16` |
| `PERF\-RS\-LOOP\-008` | 1 | 0 | 0 | 1 | Member\-chain dereference in innermost loop \(Rust\) (`PERF\-RS\-LOOP\-008:src/08\_globals\_and\_chains\.rs:59:17`) — `src/08\_globals\_and\_chains\.rs:59:17` |
| `PERF\-RS\-TYPE\-002` | 1 | 0 | 0 | 1 | Struct field ordering \(padding candidate, Rust\) (`PERF\-RS\-TYPE\-002:src/07\_struct\_by\_value\.rs:26:1`) — `src/07\_struct\_by\_value\.rs:26:1` |

## 5. Explorer
[Dependency graph — internal mod/use dependencies between scanned files.](charts/include-graph.svg)
[File hot-path map — top files by degree with reachable reserve, heat by candidate function cost (v2 where available; v1 fallback); highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)
[Entry-stratified cost map — files stratified by resolved entry reachability, heat by candidate function cost; LOW-confidence call edges excluded.](charts/layer-entry.svg)
[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)
[Hot components — ranking by candidate finding density.](charts/hot-components.svg)
[Workspace map — components as nodes over inter-component call/include edges, heat by candidate finding density.](charts/layer-workspace.svg)
[Component bands — files grouped under capped components, cross-component file edges aggregated as counted bands.](charts/layer-bands.svg)
[Findings map — severity-weighted finding density (hot 3 / warn 2 / info 1 per candidate), dots = severity fill + confidence ring; profile to confirm.](charts/layer-findings.svg)

Component ranking (hottest first — showing top 1 of 1 components):

| component | path prefix | rank | finding density (per KLOC) |
|---|---|---:|---:|
| src | src | 1 | 197.53 |

Component detail pages (one per component above):

[Component src — per-file hot-path map.](charts/component-01-src.svg)

The dependency graph reflects resolved internal include/module edges between scanned files — Rust `mod`/`use` module paths; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
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
| `PERF\-RS\-LOOP\-002` | Expensive setup or I/O call inside loop \(Rust\) | Detects Rust I/O/setup work nested in loops — \`println\!\`/\`print\!\`\-family macros, reader/writer/socket methods \(\`read\_to\_string\`/\`write\_all\`/\`flush\`/\`recv\`/\`send\`\), and \`File\`/\`fs\`/\`Command\`/\`TcpStream\`/\`env\` setup calls; lowered to warn when conditionally guarded\. The message splits hoistable setup from intrinsic streaming transfer \(recv/send/read\_line — batching is the candidate\); evidence carries io\_kind\. |
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
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.7.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.

## Appendix C. All findings (full detail)
32 of 32 total findings.

Full per-finding detail has moved to [findings-detail.md](findings-detail.md) (22 rules, same 32 of 32 total).
