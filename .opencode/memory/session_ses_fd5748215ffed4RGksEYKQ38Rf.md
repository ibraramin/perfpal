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
STRATEGIC ADVISORY — read-only reasoning, no code. You are the second-opinion advisor for ADR-0001 of "Perfpal".

CONTEXT (all founder-confirmed):
- Perfpal = local CLI, scans a codebase, outputs `perfpal-report/` (metrics.json + report.md + charts/).
- MVP: ONE language = C. Heuristics ONLY from a prioritized catalog (complexity, malloc/free density, pointer-chasing, loop nesting, recursion depth, memcpy/strcpy hotspots, etc.). Strictly static analysis.
- Primary audience: developers of AI-GENERATED codebases — high volume, possibly non-compiling, missing headers/build configs likely.
- Pillar 1b: architecture-level bottleneck visualization — dependency structure (#include graphs), component coupling, hot components. NOT just per-function heatmaps.
- Pillar 2: LLM-agnostic metrics.json with confidence/severity fields.
- HARD CONSTRAINT: no copyleft (GPL-family) dependencies. Source-available proprietary product (acquisition-target strategy).
- Architecture must let runtime-profile ingestion (perf/valgrind) bolt on later WITHOUT rewrite; same philosophy desired for future languages (C++, Python next).

THE FORK: parser strategy for the scanner.
(a) tree-sitter-first — fast, error-tolerant, MIT, but no semantic analysis (no type resolution, no macro expansion).
(b) libclang-first — precise (types, macros, cross-TU), Apache-2.0+LLVM-exception, but heavy, needs compilable code + compile DB, fragile on broken TUs.
(c) Hybrid — tree-sitter fast pass + optional libclang deep pass when compile DB exists.

QUESTIONS (answer in ≤15 bullets):
1. Which option for the MVP, and why — in terms of the audience reality (non-compiling AI-generated code) and the heuristic catalog being mostly syntactic?
2. How should the include-graph / coupling visualization be built if we go tree-sitter-first? Is syntactic #include extraction sufficient for pillar 1b at MVP quality?
3. What abstraction boundary would keep libclang addable later without rewrite (mirror the runtime-bolt-on philosophy)?
4. Top 3 risks of your recommendation, each with a mitigation.
5. What evidence would flip your recommendation?

Give a clear verdict. No file writes.

---

ASSISTANT:
Plan/spec reviewed — consistent with your framing. Advisory follows.

## VERDICT

**Option (a) — tree-sitter-first for MVP — but architected from day one so (c) is an additive resolver, not a redesign.** Don't build the libclang leg yet.

## Reasoning

1. **Audience logic is disqualifying for libclang-first:** libclang presumes compilable TUs and resolvable include paths; non-compiling AI-generated code with missing headers is your *core input*, not an edge case. A parser that degrades to garbage exactly where your market lives loses regardless of its precision advantage.
2. **Catalog logic confirms it:** every heuristic you listed is syntactic — name-matched calls (`malloc`/`memcpy`/`strcpy`), AST nesting depth, expression shape, function-length/complexity counts. Type resolution buys almost nothing for MVP scoring. Bonus alignment: tree-sitter's error recovery + throughput + MIT license all fit; libclang's fragility on broken TUs would poison metrics with parse failures.
3. **Why not hybrid-now (YAGNI):** zero compile DBs will exist in your funnel at MVP; carrying libclang packaging (binary size, version matrix, CI surface) is cost with no user to serve it. Ship the *seam*, not the second engine.
4. **Q2 — yes, syntactic `#include` extraction is MVP-sufficient:** tree-sitter exposes include nodes directly; resolve quoted includes against the filesystem, bucket angle-bracket as "external" nodes excluded from internal coupling.
5. **Build pillar 1b as a two-level graph:** file-level include graph rolled up to component/directory level; hot components = fan-in/centrality joined with aggregated per-file heuristic heat. That join — not per-function heatmaps — is what makes it architecture-level.
6. **Treat unresolvable/cyclic includes as findings, not failures** — broken dependency structure is genuine signal for AI-gen code. Conditional includes: count all branches and note it in methodology. Coupling metrics need *consistent* resolution, not perfect resolution; syntactic gets ~90% of the signal.
7. **Q3 — the boundary is a parser-agnostic IR:** normalized stream of functions/calls/includes keyed by `(path, name, span)`; scanners translate into it; heuristics consume only the IR, never tree-sitter node types directly.
8. **Tag every catalog heuristic with a required tier** (`syntactic` vs `semantic`) in its metadata; the runner picks the deepest available resolver; the confidence field records which tier served each finding — pillar 2's schema doubles as the extension contract.
9. **Never key entities on clang USRs** — they're libclang-specific. `(path, name, span)` identity survives engine swaps; a future semantic pass *upgrades confidence*, never changes schema or identity.
10. **Runtime bolt-on uses the same seam:** perf/valgrind ingestion = a third source annotating the same IR/metrics envelope. One philosophy, three sources (syntactic → semantic → dynamic); if the seam holds for libclang it holds for profiles.
11. **Risk 1 — macro-heavy C distorts call/name matching** (AI code loves reinventing macros): mitigate with macro-suspect flags + confidence downgrades on affected findings, plus a small curated library of common macro patterns as cheap syntactic recognizers.
12. **Risk 2 — name-based call/recursion heuristics produce false positives** (duplicate statics across files, function pointers): mitigate by scoping call matching TU/file-local at MVP; cross-file edges emitted but marked low-confidence.
13. **Risk 3 — the seam leaks** (heuristics quietly depend on tree-sitter shapes, making the libclang pass a rewrite anyway): mitigate with a hard import rule (heuristics may import only the IR module) plus one CI test running the full heuristic suite against fixture IR with *no parser bound* — proves the abstraction before libclang ever exists.
14. **Evidence that flips me to hybrid/libclang:** (i) funnel data shows most target repos ship `compile_commands.json`; (ii) top-demanded heuristics become genuinely semantic (ownership, type-dependent aliasing); (iii) measured tree-sitter recovery failure rate on broken files is high enough that function boundaries aren't recoverable; (iv) precision becomes the competitive moat rather than coverage.
15. **One-line summary for ADR-0001:** choose coverage over precision because your input is broken by design; encode the uncertainty you're accepting into the confidence field (which you already committed to), and make the resolver interface — not the parser — the architectural commitment.
</conversation_update>
