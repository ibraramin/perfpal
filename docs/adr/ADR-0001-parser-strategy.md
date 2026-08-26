# ADR-0001: Parser Strategy — tree-sitter-first behind a parser-agnostic IR

- **Status:** Accepted (founder-approved 2026-08-23)
- **Date:** 2026-08-23
- **Deciders:** Founder + orchestrator (evidence: @librarian license audit, @oracle advisory)
- **Gates:** All scanner/analyzer implementation in `mvp-architecture` and beyond.

## Context

Perfpal's MVP is a static-analysis CLI for **C** whose primary input is **AI-generated codebases**: high volume, frequently non-compiling, often missing headers and build configuration (`compile_commands.json`). The scanner must feed three consumers: syntactic performance heuristics (complexity, malloc/free density, pointer-chasing, loop nesting, recursion, memcpy-family hotspots), architecture-level dependency/coupling visualization (pillar 1b), and an LLM-agnostic metrics.json with confidence/severity fields (pillar 2).

Hard constraint: **zero copyleft (GPL-family) dependencies** in the core tool. Product is source-available proprietary (acquisition-target strategy).

The fork: **(a)** tree-sitter-first, **(b)** libclang-first, **(c)** hybrid now.

## Decision

**Adopt tree-sitter (core runtime MIT + official `tree-sitter-c` grammar MIT) as the MVP C parser.**

The architectural commitment is **not the parser** — it is a **parser-agnostic IR**:

- Normalized stream of functions / calls / includes / preprocessor-macro events keyed by `(path, name, span)` identity.
- Heuristics consume **only the IR**, never tree-sitter node types.
- Resolver tiers: `syntactic` (MVP, tree-sitter) → `semantic` (future, optional libclang) → `dynamic` (future, runtime profiles). One seam, three sources.
- Every catalog heuristic is tagged with its required tier; the confidence field records which tier served each finding.
- Entity identity never uses engine-specific keys (e.g., clang USRs). A future semantic pass upgrades confidence; it never changes schema or identity.

libclang is **deferred as an additive resolver**, mirroring the runtime-profile bolt-on philosophy. Hybrid is not built now.

## License Audit (no-copyleft constraint)

| Component | SPDX | Copyleft? | Notes |
|---|---|---|---|
| tree-sitter core runtime | `MIT` | No | Dependency-free C library; vendoring permitted, retain notice. |
| tree-sitter-c grammar + generated `parser.c` | `MIT` | No | Generated parser inherits grammar's MIT; vendoring into proprietary tool permitted with notice. |
| libclang / LLVM (future option) | `Apache-2.0 WITH LLVM-exception` | No | Permissive; linking does not taint proprietary code. Ship notices file if bundled. |
| Semgrep OSS (reference only) | `LGPL-2.1` | Weak copyleft | **Never copy** its CST→AST mapping code into Perfpal core. |

**Verified landmines & policy:**
1. Community tree-sitter grammars can be GPL-3.0 / AGPL-3.0 / MPL-2.0 (verified examples exist). **Policy: vendor only official `tree-sitter-*` repos unless each grammar's LICENSE is audited first.**
2. LLVM legacy remainder is UIUC/NCSA (permissive); third-party LLVM deps are quarantined — clean.
3. Conclusion: **licensing does not differentiate the options** — both satisfy the constraint. The decision rests on capability and footprint.

## Rationale

1. **Audience reality decides it.** libclang presumes compilable translation units and resolvable include paths; on missing headers, declarations silently vanish from its AST. Non-compiling input is our *core case*, not an edge case. tree-sitter always returns a best-effort CST with localized `ERROR` nodes.
2. **The heuristic catalog is syntactic.** Name-matched calls, nesting depth, complexity counts need no type resolution or macro expansion; precision buys ~nothing for MVP scoring.
3. **Footprint asymmetry (~20×).** tree-sitter route: <5 MB source, compiles anywhere, no runtime deps → small self-contained CLI. libclang route: ~25 MB compressed / 100+ MB unpacked library plus version churn and platform-linking friction.
4. **Precedent.** GitHub code navigation, Semgrep, and ast-grep all do C/C++ analysis/search at scale on tree-sitter; ccls/rust-bindgen document libclang's build-config dependence and packaging pain.

## Alternatives Considered

- **libclang-first — rejected.** Disqualifying fragility on non-compiling TUs (silent AST truncation); heavy footprint; needs compile DB that AI-generated repos rarely ship. Remains license-clean and viable as a future semantic resolver.
- **Hybrid-now — rejected (YAGNI).** Zero compile databases will exist in the MVP funnel; carrying libclang packaging costs with no user to serve. Ship the *seam*, not the second engine.

## Consequences

**Positive**
- Coverage over precision, matched to broken-by-design input; accepted uncertainty is encoded in the confidence field rather than hidden.
- Small, dependency-free, fast binary; parses files linearly regardless of project buildability.
- The IR seam serves all three future sources (semantic resolver, runtime profiles, additional languages).

**Negative / mitigated**
- No macro expansion → macro-suspect flags + confidence downgrades on affected findings; small curated library of common macro patterns as cheap syntactic recognizers.
- Name-based call/recursion matching risks false positives → scope call matching TU/file-local at MVP; cross-file edges emitted but marked low-confidence.
- Include graph is text-level → resolve quoted includes against the filesystem; bucket angle-bracket includes as "external" (excluded from internal coupling). Unresolvable/cyclic includes become **findings, not failures** — broken dependency structure is genuine signal for AI-generated code.
- tree-sitter fails silently, never loudly → heuristics MUST check `ERROR`/partial-parse status before scoring; unparseable regions are reported, not analyzed as fiction.

## Guardrails (enforced from day one)

1. Hard import rule: heuristics may import only the IR module — never parser internals.
2. CI test: full heuristic suite runs against fixture IR with **no parser bound** — proves the abstraction before any second resolver exists.
3. Every heuristic tagged `syntactic` or `semantic` in catalog metadata.
4. Grammar-vendoring policy per License Audit above.
5. Pin exact versions of tree-sitter core and `tree-sitter-c`; upgrades are deliberate events with node-types diff review (both ship breaking changes across releases).

## Reversal Triggers (what flips us to hybrid/libclang)

1. Funnel data shows most target repos ship `compile_commands.json`.
2. Top-demanded heuristics become genuinely semantic (ownership, type-dependent aliasing).
3. Measured tree-sitter recovery failure rate leaves function boundaries unrecoverable on real inputs.
4. Precision becomes the competitive moat rather than coverage.

## References

- tree-sitter: https://tree-sitter.github.io/tree-sitter/ · LICENSE (MIT): https://github.com/tree-sitter/tree-sitter/blob/master/LICENSE
- tree-sitter-c (MIT): https://github.com/tree-sitter/tree-sitter-c · node-types: https://github.com/tree-sitter/tree-sitter-c/blob/master/src/node-types.json
- LLVM licensing: https://github.com/llvm/llvm-project/blob/main/llvm/LICENSE.TXT · https://llvm.org/docs/DeveloperPolicy.html
- libclang TU parse options: https://clang.llvm.org/doxygen/group__CINDEX__TRANSLATION__UNIT.html · include graph: https://clang.llvm.org/doxygen/group__CINDEX__MISC.html
- Precedent: https://github.com/github/code-navigation · https://github.com/semgrep/semgrep · https://github.com/ast-grep/ast-grep · https://github.com/MaskRay/ccls
