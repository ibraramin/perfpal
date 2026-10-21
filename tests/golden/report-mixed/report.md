# Perfpal Report
> schema_version 1\.7 · perfpal 0\.2\.0\-beta\.1 · scanned <ROOT>
> started <MASKED> · duration <MASKED> ms · 4 files

## 1. Executive summary
9 findings: 4 hot, 4 warn, 1 info. Parse health: 4 ok, 0 partial, 0 failed.

### Where do I look first?

1. **Heap allocation inside loop** ([`PERF\-C\-ALLOC\-001:src/main\.c:27:21`](findings-detail.md#finding-PERF-C-ALLOC-001-src-main-c-27-21)) — `src/main\.c:27` · function `c\_loop` · hot · HIGH · production
   - Cost: `15 static cost units (v2) · 5.0× median · top 25%`
   - Why: per\-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
2. **Deallocation inside loop** ([`PERF\-C\-ALLOC\-002:src/main\.c:29:9`](findings-detail.md#finding-PERF-C-ALLOC-002-src-main-c-29-9)) — `src/main\.c:29` · function `c\_loop` · hot · HIGH · production
   - Cost: `15 static cost units (v2) · 5.0× median · top 25%`
   - Why: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
3. **String accumulation in loop** ([`PERF\-C\-STR\-001:src/main\.c:28:9`](findings-detail.md#finding-PERF-C-STR-001-src-main-c-28-9)) — `src/main\.c:28` · function `c\_loop` · hot · HIGH · production
   - Cost: `15 static cost units (v2) · 5.0× median · top 25%`
   - Why: 'strcat' accumulates into 'acc' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
4. **Growable\-buffer additive growth in loop \(Rust\)** ([`PERF\-RS\-ALLOC\-003:src/lib\.rs:20:9`](findings-detail.md#finding-PERF-RS-ALLOC-003-src-lib-rs-20-9)) — `src/lib\.rs:20` · function `build` · hot · MEDIUM · production
   - Cost: `20 static cost units (v2) · 6.7× median · top 25%`
   - Why: 'out\.reserve' grows a buffer by addition inside a loop at depth 1 — each reserve may reallocate and copy the whole buffer; reserving a larger capacity once up front is a candidate optimization
5. **Length query recomputed in loop \(Rust\)** ([`PERF\-RS\-STR\-002:src/lib\.rs:22:17`](findings-detail.md#finding-PERF-RS-STR-002-src-lib-rs-22-17)) — `src/lib\.rs:22` · function `build` · warn · HIGH · production
   - Cost: `20 static cost units (v2) · 6.7× median · top 25%`
   - Why: 's\.len' recomputes the length of 's' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization

[Complete ranked list](findings-detail.md#ranked-actions) — all 9 findings (production-first).

0 non-production files demoted · 0 component-level findings in Explorer.

### What breaks if I touch it?

- `src/util\.h` — 2 dependents · 50% of scanned files · top 25%

Blast-radius bars: [report.html#map](report.html#map). resolved 2 of 2 include edges.

### Where might time plausibly go?

- `src/lib\.rs:15` `build` — 20 static cost units (v2) · 6.7× median · top 25%
- `src/main\.c:23` `c\_loop` — 15 static cost units (v2) · 5.0× median · top 25%
- `src/util\.c:6` `util\_init` — 3 static cost units (v2) · 1.0× median · top 25%
- `src/lib\.rs:33` `rust\_surface` — 1 static cost units (v2) · 0.3× median · top 25%

[Complete cost-ranked list (4 functions)](profile-queue.md).

0 startup functions demoted · 2 no-loop functions demoted.

## 2. Scan provenance & parse health
- Tool: perfpal 0\.2\.0\-beta\.1
- Scan root: `<ROOT>`
- Files scanned: 4

### Parse health summary

| status | files |
|---|---:|
| ok | 4 |
| partial | 0 |
| failed | 0 |

No failed or partially parsed files.

Findings overlapping parse-error regions are flagged (`error_node_overlap`) and confidence-capped — never silently dropped.

## 3. Codebase statistics
- Languages: c, rust
- Total LOC: 36 code / 47 comment / 6 blank

Top files by code LOC:

| path | loc.code | loc.comment | loc.blank | functions |
|---|---:|---:|---:|---:|
| src/lib\.rs | 14 | 20 | 1 | 2 |
| src/main\.c | 14 | 18 | 1 | 1 |
| src/util\.c | 4 | 3 | 1 | 1 |
| src/util\.h | 4 | 6 | 3 | 0 |

Distribution — cyclomatic\_complexity:

| bucket | count |
|---|---:|
| 01\-04 | 4 |
| 05\-09 | 0 |
| 10\-14 | 0 |
| 15\-24 | 0 |
| 25\-49 | 0 |
| 50\+ | 0 |

Distribution — max\_loop\_depth:

| bucket | count |
|---|---:|
| 0 | 2 |
| 1 | 2 |
| 2 | 0 |
| 3 | 0 |
| 4 | 0 |
| 5 | 0 |
| 6\+ | 0 |

## 4. Findings
8 rules: 9 of 9 total findings (4 hot, 4 warn, 1 info).

| rule_id | findings | hot | warn | info | top exemplar |
|---|---:|---:|---:|---:|---|
| `PERF\-C\-ALLOC\-001` | 1 | 1 | 0 | 0 | Heap allocation inside loop (`PERF\-C\-ALLOC\-001:src/main\.c:27:21`) — `src/main\.c:27:21` |
| `PERF\-C\-ALLOC\-002` | 1 | 1 | 0 | 0 | Deallocation inside loop (`PERF\-C\-ALLOC\-002:src/main\.c:29:9`) — `src/main\.c:29:9` |
| `PERF\-C\-STR\-001` | 1 | 1 | 0 | 0 | String accumulation in loop (`PERF\-C\-STR\-001:src/main\.c:28:9`) — `src/main\.c:28:9` |
| `PERF\-RS\-ALLOC\-003` | 1 | 1 | 0 | 0 | Growable\-buffer additive growth in loop \(Rust\) (`PERF\-RS\-ALLOC\-003:src/lib\.rs:20:9`) — `src/lib\.rs:20:9` |
| `PERF\-RS\-STR\-002` | 2 | 0 | 2 | 0 | Length query recomputed in loop \(Rust\) (`PERF\-RS\-STR\-002:src/lib\.rs:22:17`) — `src/lib\.rs:22:17` |
| `PERF\-C\-STR\-003` | 1 | 0 | 1 | 0 | Unbounded string operation (`PERF\-C\-STR\-003:src/main\.c:28:9`) — `src/main\.c:28:9` |
| `PERF\-RS\-MEM\-001` | 1 | 0 | 1 | 0 | size\_of passed as an element count \(Rust\) (`PERF\-RS\-MEM\-001:src/lib\.rs:25:22`) — `src/lib\.rs:25:22` |
| `PERF\-C\-ARCH\-003` | 1 | 0 | 0 | 1 | Cross\-file call coupling (`PERF\-C\-ARCH\-003:src/main\.c:32:12`) — `src/main\.c:32:12` |

## 5. Explorer
[Dependency graph — internal #include, mod/use dependencies between scanned files.](charts/include-graph.svg)
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
| src | src | 1 | 250.00 |

Component detail pages (one per component above):

[Component src — per-file hot-path map.](charts/component-01-src.svg)

The dependency graph reflects resolved internal include/module edges between scanned files — C `#include` directives and Rust `mod`/`use` module paths; unresolved internal edges are retained as data, and external (system headers/crates) dependencies are excluded from coupling edges.
Coupling heat blends edge counts with per-function cost (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / alloc_family; each edge weighs 1 + mean endpoint cost normalized by the scan maximum) — charts remain pure functions of metrics.json.
Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic DOT export of the include/call/component graphs (same source data as the SVGs above).
Standing caveat: cross-file call edges are LOW confidence until a semantic resolver tier exists (ADR-0001).

## Appendix A. Heuristic reference
| rule_id | title | description |
|---|---|---|
| `PERF\-C\-ALLOC\-001` | Heap allocation inside loop | Detects malloc/calloc/realloc/aligned\_alloc/strdup calls nested anywhere inside a loop body\. |
| `PERF\-C\-ALLOC\-002` | \`free\(\)\` inside loop | Detects free\(\) directly inside a loop body \(paired with ALLOC\-001 it indicates allocator churn\)\. |
| `PERF\-C\-ARCH\-003` | Cross\-file call coupling \(name\-based\) | Detects cross\-file call edges via name matching — LOW confidence until a semantic resolver tier exists\. |
| `PERF\-C\-STR\-001` | String accumulation in loop \(\`strcat\` family\) | Detects strcat/strncat with a destination reused across loop iterations \(O\(n²\) rescan\)\. |
| `PERF\-C\-STR\-003` | Unbounded copy family \(\`strcpy\`/\`sprintf\`/\`gets\`\) | Detects unbounded copy calls \(strcpy, strcat, sprintf, gets, scanf %s\) — unbounded\-latency hotspots\. |
| `PERF\-RS\-ALLOC\-003` | Growable\-buffer additive growth in loop \(Rust\) | REFRAMED for Rust \(no \`realloc\`\): detects additive \`reserve\`/\`reserve\_exact\` capacity growth inside a loop — the \`realloc\(p, n\+k\)\` analogue; multiplicative capacity growth is the fix, not the finding\. |
| `PERF\-RS\-MEM\-001` | size\_of passed as an element count \(Rust\) | REFRAMED for Rust: detects \`Vec::with\_capacity\`/\`String::with\_capacity\` receiving a \`size\_of\`/\`size\_of\_val\` byte count where an element count is expected \(over\-reserves by the element size\)\. |
| `PERF\-RS\-STR\-002` | Length query recomputed in loop \(Rust\) | Detects \`len\`/\`is\_empty\`/\`capacity\` recomputed inside a loop where the receiver is not reassigned in the innermost enclosing loop\. |

## Appendix B. How to read this report
**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or architecture impact: review `hot` findings first, schedule `warn`, and treat `info` as context signal feeding aggregation.
**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the false-positive risk of the detection shape — pure-syntax detections are HIGH, shape heuristics with documented FP classes are MEDIUM, and approximations (size models, cross-file name matching) are LOW. Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings whose evidence quality is reduced; their confidence is capped at MEDIUM.
Every finding is an **optimization candidate, never a certainty**: a candidate for human review, not a defect claim. Perfpal v1 runs on a purely syntactic tier (`resolver_tier: syntactic`) — no macro expansion, no type resolution.
For programmatic consumption, read `metrics.json` — the source of truth — and validate it against the bundled schema `metrics.schema.v1.7.json` shipped alongside it. Where this report and `metrics.json` ever disagree, `metrics.json` wins.

## Appendix C. All findings (full detail)
9 of 9 total findings.

Full per-finding detail has moved to [findings-detail.md](findings-detail.md) (8 rules, same 9 of 9 total).
