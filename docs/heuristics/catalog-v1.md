# Perfpal — C Performance Heuristic Catalog v1

- **Status:** v1 **SIGNED OFF** by founder ("All approved": Tier 1 = MVP set · Tier 2 in scope · §5 exclusions accepted). Dual review passed (@requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES, all fixes applied). Plan `mvp-architecture`, step 2.
- **Scope:** MVP language = C. Static analysis only. Findings are **optimization candidates**, never certainties (spec §Known Risks).
- **Method constraint:** all Tier 1/2 heuristics are implementable on the tree-sitter **syntactic tier** of the parser-agnostic IR (ADR-0001): AST shape, identifier matching, declaration counts, span arithmetic. No macro expansion, no type resolution, no compile database at MVP. Per ADR-0001 guardrail, every heuristic below carries an explicit resolver-tier tag — all current entries are `(syntactic)`; any future semantic-tier heuristic must be tagged `(semantic)` when added.
- **Grounding:** cross-checked against clang-tidy (C-applicable checks), cppcheck performance/style checks, and PVS-Studio published micro-optimization themes (V801–V820). Items **not found in the surveyed tools** are explicitly marked — they are Perfpal's differentiation space.
- **Downstream contract:** every finding emitted under these IDs must carry `confidence` and `severity` fields (metrics.json schema, plan step 3).

---

## 1. Ranking model

| Axis | Values | Meaning |
|---|---|---|
| **Signal (S)** | S1 / S2 / S3 | Expected runtime impact × prevalence in AI-generated C codebases |
| **Cost (C)** | C1 / C2 / C3 | Implementation effort on the syntactic IR + false-positive control effort |

**Tier assignment:** T1 = MVP must-have · T2 = should-have (build if budget allows) · T3 = deferred (semantic tier, runtime data, or post-MVP).

### Confidence model (applies to every finding)
- **HIGH** — pure syntax, low false-positive classes.
- **MEDIUM** — shape heuristic with known FP classes; FP sources documented per entry.
- **LOW** — approximation involved (size models, cross-file name matching).
- **Downgrade rules (from ADR-0001):**
  - Any part of the matched span originates from a macro → confidence capped at MEDIUM and `macro_suspect=true`. Macro origins are detected without expansion via the curated macro-pattern recognizer library referenced in ADR-0001.
  - Cross-file edges derived from name-based call matching → LOW until a semantic resolver tier exists.
  - tree-sitter ERROR nodes within the span → cap at MEDIUM (monitor C11/C17 grammar gaps).

---

## 2. Baseline stats (pillar 1a — always emitted, not heuristics)

Emitted as measurements without severity/confidence; they feed hotspot aggregation (PERF-C-ARCH-002):

- LOC per file (code / comment / blank), translation-unit sizes
- Function count, function length distribution, parameter count distribution
- Cyclomatic complexity per function (branch-count approximation on AST); cognitive complexity (nesting-weighted variant) computed alongside
- Allocation-site count per function/file (`malloc` family)
- Global variable count; `#include` counts per file

Note: **function length** is deliberately a *stat*, not a standalone finding — alone it is a weak perf signal; it feeds PERF-C-ARCH-002 hotspot aggregation instead. All plan-enumerated candidates are accounted for: cyclomatic/cognitive complexity → CALL-002 + stats; function length → stat (above); malloc/free density → ALLOC-001/002; pointer-chasing → LOOP-008; loop nesting depth → LOOP-001; recursion depth → CALL-001 (+ depth note); memcpy/strcpy-family hotspots → STR-001/003 + MEM-002; missed-inline candidates → §5 excluded with reason.

---

## 3. Tier 1 — MVP must-have (15 heuristics)

| ID | Name | Signal | Cost | Default confidence | Severity band |
|---|---|---|---|---|---|
| PERF-C-ALLOC-001 | Heap allocation inside loop | S1 | C1 | HIGH | hot |
| PERF-C-ALLOC-002 | `free()` inside loop | S1 | C1 | HIGH | warn |
| PERF-C-ALLOC-003 | `realloc()` linear growth in loop | S1 | C2 | MEDIUM | hot |
| PERF-C-STR-001 | String accumulation in loop (`strcat` family) | S1 | C1 | HIGH | hot |
| PERF-C-STR-002 | `strlen()` inside loop condition/body | S1 | C1 | HIGH | warn |
| PERF-C-STR-003 | Unbounded copy family (`strcpy`/`sprintf`/`gets`) | S2 | C1 | HIGH | warn |
| PERF-C-LOOP-001 | Deep loop nesting (≥3) | S1 | C1 | HIGH | warn |
| PERF-C-LOOP-002 | Expensive setup / I/O call inside loop | S1 | C2 | MEDIUM | hot |
| PERF-C-LOOP-003 | Char-wise stdio loop (`fgetc`/`fputc` family) | S2 | C1 | MEDIUM | info |
| PERF-C-LOOP-004 | Integer div/mod with variable divisor in innermost loop | S2 | C1 | MEDIUM | info |
| PERF-C-CALL-001 | Direct recursion | S2 | C1 | HIGH | info |
| PERF-C-CALL-002 | High cyclomatic complexity | S2 | C1 | HIGH | warn |
| PERF-C-ARCH-001 | Include-graph fan-out / cycles | S1 | C1 | HIGH | warn |
| PERF-C-ARCH-002 | Hot-component aggregation | S1 | C1 | HIGH | hot |
| PERF-C-ARCH-003 | Cross-file call coupling (name-based) | S2 | C2 | LOW | info |

### Detail

**PERF-C-ALLOC-001 — Heap allocation inside loop** `(syntactic)`
- Detects: `malloc`/`calloc`/`realloc`/`aligned_alloc`/`strdup` call nested anywhere inside a loop body.
- Why: allocator churn dominates runtime; per-iteration allocation is the single most common AI-generated C perf smell. Not found in surveyed tools (closest analogue: PVS-Studio V819) — differentiation item.
- Sketch: call-expression whose callee ∈ alloc-family, ancestor chain contains loop node.
- FPs: intentional per-item allocation where lifetime requires it (still worth surfacing as candidate).
- Macro policy: if alloc call comes from a wrapper macro → `macro_suspect`, confidence MEDIUM.

**PERF-C-ALLOC-002 — `free()` inside loop** `(syntactic)`
- Detects: `free()` (and `fclose` on heap-backed handles) directly inside a loop body.
- Why: paired with ALLOC-001 indicates churn; alone indicates repeated teardown cost.
- FPs: freeing distinct items per iteration is legitimate — pair with ALLOC-001 co-occurrence to raise signal.

**PERF-C-ALLOC-003 — `realloc()` linear growth in loop** `(syntactic)`
- Detects: `realloc(p, n+1)` / `realloc(p, n+k)` shapes nested in loops (additive growth ⇒ quadratic memmove behavior).
- Why: classic O(n²) growth pattern; doubling-growth is the fix.
- FPs: ring-buffer style fixed-size realloc; confidence MEDIUM by default.

**PERF-C-STR-001 — String accumulation in loop** `(syntactic)`
- Detects: `strcat`/`strncat` with destination reused across loop iterations.
- Why: destination rescanned each iteration ⇒ O(n²); not flagged by surveyed tools — differentiation item.
- FPs: bounded small loops; still candidate-grade.

**PERF-C-STR-002 — `strlen()` inside loop condition/body** `(syntactic)`
- Detects: `strlen(s)` appearing in a loop's condition or body where `s` is not reassigned in the loop (syntactic reassignment check only).
- Why: O(n) rescan per iteration.
- FPs: `s` mutated each iteration (then rescan is required) — check assignment statements to same base identifier syntactically.

**PERF-C-STR-003 — Unbounded copy family** `(syntactic)`
- Detects: calls to `strcpy`, `strcat`, `sprintf`, `gets`, `scanf("%s…")`.
- Why: unbounded memory ops; primarily safety but also unbounded-latency hotspots. Grounded in clang-tidy/cppcheck analogues.
- Note: frame output as "unbounded operation" not "bug" (static-analysis framing rule).

**PERF-C-LOOP-001 — Deep loop nesting (≥3)** `(syntactic)`
- Detects: loop nesting depth ≥ 3.
- Why: O(n^k) candidate zones; primary input to hotspot maps.
- Configurable threshold (default 3).

**PERF-C-LOOP-002 — Expensive setup / I/O call inside loop** `(syntactic)`
- Detects: `printf`-family, `fopen`/`fclose`/`open`/`close`/`read`/`write`, `getenv`, `system`, regex-compilation calls nested in loops.
- Why: syscall/format-parse churn per iteration; hoisting is the standard fix. No equivalent C check found in surveyed tools — differentiation item.
- FPs: intentionally interleaved logging; severity lowered when call is in conditional branch inside loop.

**PERF-C-LOOP-003 — Char-wise stdio loop** `(syntactic)`
- Detects: `fgetc`/`getc`/`getchar`/`fputc`/`putc` as the primary statement of a loop body.
- Why: per-byte libc locking overhead vs block reads (`fread`/`fwrite`).
- FPs: state machines requiring byte-at-a-time parsing.

**PERF-C-LOOP-004 — Integer div/mod with variable divisor in innermost loop** `(syntactic)`
- Detects: `/` or `%` whose divisor is a non-literal expression, inside the innermost loop body. Literal divisors are skipped by design.
- Why: integer division latency (~20+ cycles); compilers strength-reduce *constant* divisors via magic-number multiply-shift (powers of two via shift), but *variable* divisors defeat those transforms and pay real division per iteration.
- FPs: divisor and dividend both loop-invariant (whole expression hoistable) — rare in hot inner loops; info-level only.

**PERF-C-CALL-001 — Direct recursion** `(syntactic)`
- Detects: function body contains a call to its own name.
- Why: stack-depth risk + potential exponential blowup; cheap and precise syntactically.
- Depth note: recursion *depth* is not statically bounded at MVP (needs interprocedural call-graph + value analysis); MVP emits presence-only, depth analysis deferred to the semantic tier.
- Mutual recursion: deferred to ARCH-003 graph once cross-file edges exist (post-MVP upgrade path).

**PERF-C-CALL-002 — High cyclomatic complexity** `(syntactic)`
- Detects: branch-count complexity above threshold (default ≥ 15), reported per function.
- Why: pillar-1a stat promoted to finding when extreme; correlates with unpredictable branches and untestable hot paths.

**PERF-C-ARCH-001 — Include-graph fan-out / cycles (pillar 1b)** `(syntactic)`
- Detects: per-file include fan-out/fan-in, deep include chains, cyclic includes — from `#include` directive scan.
- Why: architecture-level coupling signal; hub headers force monolithic rebuilds and indicate god-components. Core input for the dependency-structure visualization.
- Note: angle-bracket vs quoted includes separated; system headers excluded from coupling edges.

**PERF-C-ARCH-002 — Hot-component aggregation (pillar 1b)** `(syntactic)`
- Detects: files/components ranked by aggregated metrics (complexity sum, alloc-site density, max loop depth, finding density).
- Why: turns per-function noise into architecture-level bottleneck ranking — the "hot components" view required by the spec.
- Pure aggregation over other outputs; no independent detection logic.

**PERF-C-ARCH-003 — Cross-file call coupling (name-based)** `(syntactic)`
- Detects: call-graph edges between functions in different files via name matching (file-local matches first, per ADR-0001 resolver tiers).
- Why: enables component coupling heatmap; explicit LOW confidence until semantic resolver exists.
- FPs: same-name static functions in different TUs; duplicate symbol names across vendored deps.

---

## 4. Tier 2 — Should-have (8 heuristics)

All entries `(syntactic)` unless noted.

| ID | Name | Signal | Cost | Default confidence | Severity band |
|---|---|---|---|---|---|
| PERF-C-TYPE-001 | Large struct passed by value | S2 | C2 | LOW | warn |
| PERF-C-TYPE-002 | Struct member ordering (padding candidate) | S2 | C2 | LOW | info |
| PERF-C-MEM-001 | sizeof-misuse shapes in allocation args | S2 | C2 | MEDIUM | warn |
| PERF-C-MEM-002 | Block copy/set family in loop (`memcpy`/`memmove`/`memset`) | S2 | C1 | MEDIUM | info |
| PERF-C-LOOP-005 | Global variable written inside loop | S2 | C2 | MEDIUM | info |
| PERF-C-LOOP-006 | Indirect call via function pointer in innermost loop | S2 | C2 | LOW | info |
| PERF-C-LOOP-007 | Linear membership scan nested in loop | S2 | C2 | LOW | info |
| PERF-C-LOOP-008 | Pointer-chasing chain in innermost loop | S2 | C2 | MEDIUM | info |

- **TYPE-001/002** use an approximate size model (member counting + literal array dims within the TU). Grounded in cppcheck `passedByValue` and PVS V801/V802 themes. LOW confidence is mandatory — no real type layout at MVP.
- **MEM-001** covers `malloc(x * sizeof(ptr))` / `sizeof` misplacement shapes (clang-tidy `bugprone-sizeof-expression`, cppcheck `multiplySizeof` analogues). Perf relevance: wrong-size allocations.
- **MEM-002** detects `memcpy`/`memmove`/`memset` calls with non-literal size arguments inside loop bodies — repeated large block-copy/set churn (plan-enumerated memcpy-family hotspot). Known FP class: per-element small-struct copies are idiomatic and cheap; severity stays info unless the call sits in an innermost loop, then warn.
- **LOOP-005** detects assignment to a file-scope/global identifier inside a loop body. Why: potential loop-invariant store or shared-state contention signal. FPs: accumulator updates that legitimately belong in the loop; confidence MEDIUM because identifier scope classification is name-based at MVP.
- **LOOP-006** detects calls through function-pointer variables inside innermost loop bodies. Why: indirect calls inhibit inlining and hurt branch prediction. FPs: dispatch-table patterns are idiomatic and often intentional; LOW confidence without type info confirming the callee is a function pointer.
- **LOOP-007** detects comparison-against-array-element-indexed-by-outer-counter shapes (hand-rolled O(n²) lookup). Noisy without types — candidate-grade only.
- **LOOP-008** detects member-access chains of depth ≥ 3 (`a->b->c->d`) inside innermost loop bodies — pointer-chasing cache-miss candidates (plan-enumerated). Known FP class: dispatch through ops/vtable-style tables (`ctx->ops->fn`) is idiomatic C design, not a defect; severity stays info, confidence MEDIUM with this FP class documented.

---

## 5. Deferred / excluded (with reasons)

| Candidate | Reason |
|---|---|
| Loop-invariant computation hoisting (general) | Requires provable invariance = dataflow (semantic tier) |
| calloc/memset zeroing followed by full overwrite | Requires def-use flow (semantic tier) |
| Dead stores / redundant assignments | Def-use flow; compiler-warning territory |
| Cache-line layout / aliasing analysis | Needs type layout + target memory model |
| Vectorization opportunities | Compiler territory; needs target model |
| Inline-expansion candidates ("missed-inline") | Compiler territory; static hint too weak without sizes |
| Duplicate-code clone detection | Expensive; post-MVP consideration |
| Correctness-only checks (branch-clone, redundant expression, null-after-deref) | Out of perf scope for MVP catalog (scope discipline) |
| int→pointer cast hazards | Niche perf impact; revisit post-MVP |
| Mutual recursion detection | Upgrade path of ARCH-003 once cross-file edges mature |

---

## 6. Severity guidance

Bands used above; exact enum values finalized in the metrics.json schema (plan step 3):

- **hot** — likely dominant runtime/architecture impact; review first.
- **warn** — measurable impact or structural risk; schedule.
- **info** — context signal; feed aggregation and LLM raw data.

Severity is a *prioritization hint*, never a certainty claim; every finding remains an optimization candidate.

---

## 7. Founder sign-off

**SIGNED OFF** — founder approval received ("All approved"): (1) Tier 1 approved as the MVP set; (2) Tier 2 included in MVP scope; (3) §5 deferred/excluded accepted as out-of-scope. Recorded in `state/session-state.json`.
