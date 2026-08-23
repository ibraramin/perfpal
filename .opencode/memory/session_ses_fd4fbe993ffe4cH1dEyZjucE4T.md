<!-- stm:v1 -->
## Session Memory

### User Instructions
- …

### Long Horizon Context
- …

### Decisions
- …

### Conclusions
- …

### Active References
- …

<existing_memory>
## Session Memory

### User Instructions
- None captured yet.

### Long Horizon Context
- None captured yet.

### Decisions
- None captured yet.

### Conclusions
- None captured yet.

### Active References
- None captured yet.

</existing_memory>

<conversation_update>
USER:
RESEARCH ONLY — no code changes. I'm building a prioritized catalog of STATIC performance heuristics for C code, to be implemented with tree-sitter (syntax-level AST only: no macro expansion, no type resolution, no compile database at MVP). I need grounding against established tools so my catalog doesn't miss well-known checks.

Deliverable — respond in compact bullets, max ~60 lines total:

1. **clang-tidy performance-oriented checks** (performance-* module plus clearly performance-relevant misc-/bugprone- checks that apply to C, not just C++). For each: check name, one-line "what it detects", one-line typical false-positive source. Skip C++-only checks (anything needing templates/RAII/move semantics).

2. **cppcheck performance-related checks** (e.g., redundantCondition, passedByValue, functionStatic style checks that have perf relevance). Same format.

3. **Any other notable published C performance static-analysis checklists** (PVS-Studio public docs category names are fine as a list of check themes; don't copy text).

4. For each item in 1–3, tag whether it is implementable with PURE SYNTAX analysis on a C AST (identifier matching, call-expression shape, loop nesting, declaration counts) vs requires TYPE/SEMANTIC info (type sizes, aliasing, value ranges, macro expansion). Use tags [SYNTAX] / [SEMANTIC].

5. A short list (≤8 items) of well-known C performance anti-patterns that established tools do NOT check but are commonly cited (e.g., allocation inside loops, strcat-in-loop quadratic behavior) — name + why it matters in one line.

Do NOT include general advice, benchmarks, or prose essays. Bullets only. If a tool's check list is long, prioritize the top ~15 most relevant to performance.

---

ASSISTANT:
Research complete — grounded against clang-tidy's official check list, cppcheck docs/wiki, and PVS-Studio public docs. Findings:

## 1. clang-tidy (C-applicable only)
- `performance-no-int-to-ptr` — int→pointer casts (tagged-ptr/alignment hazards) — FP: legitimate intptr round-trips — **[SYNTAX]** (cast shape)
- `performance-type-promotion-in-math-fn` — float args promoted to double in math fns (`powf`→`pow`) — FP: intentional precision — **[SEMANTIC]** (arg types)
- CSA `optin.performance.Padding` — struct padding waste from field order — FP: ABI-required layout — **[SEMANTIC]** (type layout)
- `bugprone-sizeof-expression` — sizeof on pointers/array params/wrong operand — FP: intentional sizeof(ptr) — **[SYNTAX]** partial (full accuracy needs types)
- `bugprone-suspicious-memset-usage` — memset size transposed / sizeof(ptr) / char-vs-int swapped — FP: none common — **[SYNTAX]** partial
- `bugprone-misplaced-operator-in-strlen-in-alloc` — `malloc(strlen(s) * sizeof(...))` operator misplacement — FP: rare — **[SYNTAX]** (call shape)
- `bugprone-misplaced-pointer-arithmetic-in-alloc` — arithmetic outside alloc size calc — FP: rare — **[SYNTAX]**
- `misc-redundant-expression` — identical subexpressions (`a && a`) — FP: macros expanding same var — **[SYNTAX]**
- `bugprone-redundant-branch-condition` — condition re-tested after already guaranteed — FP: defensive style — **[SEMANTIC]** (needs flow)
- `bugprone-branch-clone` — if/else branches with identical bodies — FP: deliberate symmetry — **[SYNTAX]** (AST equality)
- Note: rest of performance-* module (for-range-copy, unnecessary-copy/value-param, inefficient-algorithm/vector-op/string-concat, move-*/noexcept-*, endl, enum-size…) is C++-only — excluded per your constraint.

## 2. cppcheck (severity "performance" exists; most C hits land as style)
- `passedByValue` — large struct passed by value — FP: small PODs, callee stores copy — **[SEMANTIC]** (needs struct size)
- `redundantCondition` — condition always true/false in context — FP: readability-first style — **[SEMANTIC]** (value flow)
- `redundantAssignment` — value overwritten before read — FP: volatile/debug writes — **[SEMANTIC]** (def-use)
- `duplicateExpression` — `x == x`, `x && x` — FP: macro expansion artifacts — **[SYNTAX]**
- `redundantPointerOp` — `*&p`, `&*p` — FP: macro-generated code — **[SYNTAX]**
- `oppositeInnerCondition` — inner loop condition contradicts outer → wasted iterations — FP: complex guards — **[SEMANTIC]**
- `multiplySizeof` / `divideSizeof` — suspicious `sizeof` arithmetic in allocs — FP: stride math done right — **[SYNTAX]**
- `nullPointerRedundantCheck` — null-check after deref (dead guard work) — FP: none major — **[SEMANTIC]** (flow)
- `postfixOperator`, `uselessCalls*`, STL checks, `functionStatic`/`functionConst` — all C++-only; no C analogue (pure-function detection isn't exposed as a C warning).

## 3. PVS-Studio published perf themes ("Micro-optimizations", V801–V820 range)
- Heavy args passed by value (V801/V813) — **[SEMANTIC]**
- Struct padding / field reorder to shrink size (V802) — **[SEMANTIC]**
- Excessive temporaries/copies where reference suffices (V808/V809/V814/V820) — **[SEMANTIC]**
- Memory allocated+released repeatedly inside loop body (V819) — **[SEMANTIC]** (needs call/dataflow)
- Loop-invariant / repeated subexpression evaluation (V807/V815) — **[SEMANTIC]**
- Inefficient string handling patterns (V803–V806) — **[SEMANTIC]**
- Costly casts / type-width mismatches in arithmetic (V811) — **[SEMANTIC]**

## 4. Anti-patterns mainstream tools DON'T flag (your whitespace to fill)
- `malloc`/`free` per loop iteration — allocator churn dominates runtime (only PVS V819 catches it; clang-tidy/cppcheck don't) — **[SYNTAX]** detectable (alloc call nested in loop)
- `strcat(dst, …)` accumulating in a loop — O(n²): destination rescanned each iteration — **[SYNTAX]** (call-in-loop shape)
- `strlen()` in loop condition/guard — O(n) rescan per iteration — **[SYNTAX]** (identifier + loop nesting)
- Per-record `fopen`/`fprintf`/`fclose` (or open/write/close) in loop — syscall + format-parse churn — **[SYNTAX]**
- Char-wise `fgetc`/`getchar` loops instead of block reads — libc locking + per-byte overhead — **[SYNTAX]**
- `calloc`/`memset` zeroing immediately followed by full overwrite — wasted pass over memory — **[SEMANTIC]** (needs def-use)
- Invariant expensive setup recomputed per iteration (`regcomp`, `getenv`, table init) — hoisting missed — **[SEMANTIC]** (args must be provably invariant)
- Hand-rolled linear membership scan inside inner loop — O(n²) lookup vs hash/sort — **[SYNTAX]** heuristic (nested-loop + comparison shape)

Sources: clang.llvm.org extra-checks list (24.0 git), cppcheck manual + SourceForge ListOfChecks wiki, PVS-Studio docs (V801 page, micro-optimization/Tizen article confirming V801–V820 theme set).
</conversation_update>
