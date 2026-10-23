# Perfpal Agent Report

> contract 1 · schema 1.7 · perfpal 0.2.0-beta.1 · scan root: `<ROOT>`
> 4 files · 4 functions · 9 findings · languages: c, rust

## How to use this report

- Every row is an **optimization candidate**, not a defect verdict; confirm it with a profiler.
- Cost is **static cost units**, a heuristic; it is never a duration.
- Rank key: production code first, then severity (hot > warn > info), then confidence (HIGH > MEDIUM > LOW), then containing-function static cost descending, then finding_id ascending.
- Machine contract: `agent-actions.json` holds all 9 actions and the top 4 of 4 profile rows.

## Data quality

- parse: 4 ok / 0 partial / 0 failed
- unresolved includes: 0 of 2 · structural call edges: 1 (0 HIGH / 0 MEDIUM / 1 LOW)
- demotions: 0 non-production files · 0 startup functions · 2 no-loop functions (counted, never dropped)
- cost scale: 4 v2 / 0 v1 functions
- resolver tier(s): syntactic
- functions with unknown trip counts: 2 — their cost estimate is incomplete
- component-level findings excluded from actions: 0 (see the human Explorer)
- caps: top 25 actions and 25 profile rows here; all actions + top 100 profile rows in `agent-actions.json`

## Start here — ranked actions

showing top 9 of 9 actions

### 1. PERF-C-ALLOC-001 · `src/main.c:27` · hot/HIGH · production
- what: Heap allocation inside loop
- function: `c_loop` (`src/main.c:23`) · 15 static cost units (v2) · 5.0× median · top 50%
- why: per-iteration heap allocation via 'malloc' at loop depth 1 — allocator churn dominates runtime; hoisting or buffer reuse is a candidate optimization
- evidence: `alloc_site_index=2 · callee=malloc · loop_depth=1`
- fix: Hoist the allocation out of the loop, reuse a buffer across iterations, or reserve once before the loop.
- anchor: `findings-detail.md#finding-PERF-C-ALLOC-001-src-main-c-27-21` · `PERF-C-ALLOC-001:src/main.c:27:21`

### 2. PERF-C-ALLOC-002 · `src/main.c:29` · hot/HIGH · production
- what: Deallocation inside loop
- function: `c_loop` (`src/main.c:23`) · 15 static cost units (v2) · 5.0× median · top 50%
- why: deallocation via 'free' at loop depth 1 — repeated frees per iteration; hoisting the free or restructuring buffer ownership is a candidate optimization
- evidence: `alloc_cooccurrence=true · callee=free · loop_depth=1`
- fix: Move the free outside the loop or reuse a single allocation per iteration instead of freeing each pass.
- anchor: `findings-detail.md#finding-PERF-C-ALLOC-002-src-main-c-29-9` · `PERF-C-ALLOC-002:src/main.c:29:9`

### 3. PERF-C-STR-001 · `src/main.c:28` · hot/HIGH · production
- what: String accumulation in loop
- function: `c_loop` (`src/main.c:23`) · 15 static cost units (v2) · 5.0× median · top 50%
- why: 'strcat' accumulates into 'acc' at loop depth 1 — each call rescans the destination from the start; tracking the running length or sizing the buffer up front is a candidate optimization
- evidence: `callee=strcat · dest=acc · loop_depth=1`
- fix: Track the destination length and append with bounded writes instead of rescanning the string each pass.
- anchor: `findings-detail.md#finding-PERF-C-STR-001-src-main-c-28-9` · `PERF-C-STR-001:src/main.c:28:9`

### 4. PERF-RS-ALLOC-003 · `src/lib.rs:20` · hot/MEDIUM · production
- what: Growable-buffer additive growth in loop (Rust)
- function: `build` (`src/lib.rs:15`) · 20 static cost units (v2) · 6.7× median · top 25%
- why: 'out.reserve' grows a buffer by addition inside a loop at depth 1 — each reserve may reallocate and copy the whole buffer; reserving a larger capacity once up front is a candidate optimization
- evidence: `callee=out.reserve · growth=additive · loop_depth=1`
- fix: Reserve capacity once before the loop, or grow it multiplicatively, instead of reserving per pass.
- anchor: `findings-detail.md#finding-PERF-RS-ALLOC-003-src-lib-rs-20-9` · `PERF-RS-ALLOC-003:src/lib.rs:20:9`

### 5. PERF-RS-STR-002 · `src/lib.rs:22` · warn/HIGH · production
- what: Length query recomputed in loop (Rust)
- function: `build` (`src/lib.rs:15`) · 20 static cost units (v2) · 6.7× median · top 25%
- why: 's.len' recomputes the length of 's' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- evidence: `identifier=s · loop_depth=1`
- fix: Compute len, is_empty, or capacity once before the loop and reuse the value.
- anchor: `findings-detail.md#finding-PERF-RS-STR-002-src-lib-rs-22-17` · `PERF-RS-STR-002:src/lib.rs:22:17`

### 6. PERF-RS-STR-002 · `src/lib.rs:23` · warn/HIGH · production
- what: Length query recomputed in loop (Rust)
- function: `build` (`src/lib.rs:15`) · 20 static cost units (v2) · 6.7× median · top 25%
- why: 'tmp.len' recomputes the length of 'tmp' at loop depth 1 without an assignment to it inside the loop; hoisting the length before the loop is a candidate optimization
- evidence: `identifier=tmp · loop_depth=1`
- fix: Compute len, is_empty, or capacity once before the loop and reuse the value.
- anchor: `findings-detail.md#finding-PERF-RS-STR-002-src-lib-rs-23-17` · `PERF-RS-STR-002:src/lib.rs:23:17`

### 7. PERF-C-STR-003 · `src/main.c:28` · warn/HIGH · production
- what: Unbounded string operation
- function: `c_loop` (`src/main.c:23`) · 15 static cost units (v2) · 5.0× median · top 50%
- why: unbounded operation via 'strcat' — a bounded variant or explicit length tracking is a candidate optimization
- evidence: `callee=strcat`
- fix: Bound every copy with an explicit destination size (snprintf/strlcpy) and pre-size the buffer once.
- anchor: `findings-detail.md#finding-PERF-C-STR-003-src-main-c-28-9` · `PERF-C-STR-003:src/main.c:28:9`

### 8. PERF-RS-MEM-001 · `src/lib.rs:25` · warn/MEDIUM · production
- what: size_of passed as an element count (Rust)
- function: `build` (`src/lib.rs:15`) · 20 static cost units (v2) · 6.7× median · top 25%
- why: 'Vec::with_capacity' receives a ˋsize_ofˋ byte count where an element COUNT is expected — that over-reserves by a factor of the element size; passing the element count is a candidate optimization
- evidence: `callee=Vec::with_capacity · misuse=size_of_as_element_count`
- fix: Divide the byte count by size_of::<T>() when passing it as an element count.
- anchor: `findings-detail.md#finding-PERF-RS-MEM-001-src-lib-rs-25-22` · `PERF-RS-MEM-001:src/lib.rs:25:22`

### 9. PERF-C-ARCH-003 · `src/main.c:32` · info/LOW · production
- what: Cross-file call coupling
- function: `c_loop` (`src/main.c:23`) · 15 static cost units (v2) · 5.0× median · top 50%
- why: call to 'util_init' matches a definition in 'src/util.c' by name only — LOW-confidence hypothesis pending semantic resolution; confirming the match (or co-locating hot callees) is a candidate optimization
- evidence: `callee_name=util_init · matched_path=src/util.c`
- fix: Confirm the coupled edge with a semantic check, then narrow the interface or pass the dependency in.
- anchor: `findings-detail.md#finding-PERF-C-ARCH-003-src-main-c-32-12` · `PERF-C-ARCH-003:src/main.c:32:12`

## Action index by rule

| rule | title | actions | hot | warn | info | top rank |
|---|---|---:|---:|---:|---:|---:|
| PERF-C-ALLOC-001 | Heap allocation inside loop | 1 | 1 | 0 | 0 | 1 |
| PERF-C-ALLOC-002 | Deallocation inside loop | 1 | 1 | 0 | 0 | 2 |
| PERF-C-ARCH-003 | Cross-file call coupling | 1 | 0 | 0 | 1 | 9 |
| PERF-C-STR-001 | String accumulation in loop | 1 | 1 | 0 | 0 | 3 |
| PERF-C-STR-003 | Unbounded string operation | 1 | 0 | 1 | 0 | 7 |
| PERF-RS-ALLOC-003 | Growable-buffer additive growth in loop (Rust) | 1 | 1 | 0 | 0 | 4 |
| PERF-RS-MEM-001 | size_of passed as an element count (Rust) | 1 | 0 | 1 | 0 | 8 |
| PERF-RS-STR-002 | Length query recomputed in loop (Rust) | 2 | 0 | 2 | 0 | 5 |

## Profile queue

showing top 4 of 4 functions

1. `build` — `src/lib.rs` — 20 static cost units (v2) · 6.7× median · top 25% · production · unknown-trip ×1 — estimate incomplete
2. `c_loop` — `src/main.c` — 15 static cost units (v2) · 5.0× median · top 50% · production · unknown-trip ×1 — estimate incomplete
3. `util_init` — `src/util.c` — 3 static cost units (v2) · 1.0× median · top 75% · production
4. `rust_surface` — `src/lib.rs` — 1 static cost units (v2) · 0.3× median · top 100% · production

## What this scan cannot see

- **Syntactic tier only** (`resolver_tier: syntactic`): no macro expansion, no type resolution.
- **Structural call edges** are name-based/semantic and may over/under-approximate; not resolved.
- **Static cost is a heuristic**, not a duration; a high rank is a hypothesis to profile.
- **Partial or failed parses** and unresolved includes reduce coverage; the counts above say by how much.
- **Findings are candidates** for human or agent review, never defect claims.
