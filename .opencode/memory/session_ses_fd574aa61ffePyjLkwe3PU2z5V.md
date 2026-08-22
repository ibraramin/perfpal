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
RESEARCH ONLY — no code changes. You are researching for ADR-0001 of "Perfpal", a proprietary static-analysis CLI (C language MVP). HARD CONSTRAINT: zero copyleft (GPL-family) dependencies in the core tool.

Verify the following facts with current sources (official repos/docs preferred). Respond in ≤20 structured bullets grouped under: LICENSES / CAPABILITIES / PRECEDENT / SURPRISES. Include source URLs.

LICENSES
1. tree-sitter core runtime license (tree-sitter/tree-sitter) — confirm exact SPDX.
2. tree-sitter-c grammar license (tree-sitter/tree-sitter-c) — confirm exact SPDX; note whether generated parser code inherits it.
3. libclang licensing — confirm Apache 2.0 WITH LLVM-exception; state clearly whether this counts as copyleft or not, and any practical redistribution implications for a proprietary tool linking against libclang binaries.
4. Any known copyleft landmines in either ecosystem (e.g., specific grammars, submodules, build deps).

CAPABILITIES (C-focused)
5. tree-sitter: error-tolerance on syntactically broken code — confirm/deny with evidence.
6. tree-sitter: does it resolve types, macros, includes? (Expected: NO semantic analysis; #include/preproc appear as nodes without expansion.) Confirm.
7. libclang: does it require compile_commands.json / include paths to produce useful results for real-world C projects? What happens on incomplete/non-compiling translation units?
8. libclang: available APIs relevant to us — call graph extraction, include-graph/reference info, cursor-based metrics. Briefly.
9. Rough footprint/startup cost comparison for scanning many files (libclang binary size, per-TU parse cost vs tree-sitter).

PRECEDENT
10. 2–3 real-world tools doing C/C++ static analysis or code search with tree-sitter (names + what they use it for).
11. 2–3 real-world tools using libclang for analysis (e.g., clang-tidy itself) — one line each.

SURPRISES
12. Anything that would change a tree-sitter-vs-libclang decision for a heuristic-only, static-analysis MVP targeting possibly-non-compiling AI-generated C codebases.

Return findings only; do not write files.

---

ASSISTANT:
# ADR-0001 Research Findings — tree-sitter vs libclang (Perfpal MVP)

## LICENSES

- **tree-sitter core runtime** (`tree-sitter/tree-sitter`): **MIT**, SPDX `MIT` — "Copyright (c) 2018 Max Brunsfeld", standard permission-to-sell text. Verified from the repo's LICENSE file. Source: https://github.com/tree-sitter/tree-sitter/blob/master/LICENSE
- **tree-sitter-c grammar** (`tree-sitter/tree-sitter-c`): **MIT**, SPDX `MIT` — "Copyright (c) 2014 Max Brunsfeld", single LICENSE file, no exceptions. The generated `parser.c`/`parser.h` are produced from this repo's MIT-licensed `grammar.js`, so the generated code is a derivative of an MIT work and **inherits MIT** — vendoring the generated parser into a proprietary product is fully permitted (keep the copyright notice). Sources: https://github.com/tree-sitter/tree-sitter-c/blob/master/LICENSE , https://github.com/tree-sitter/tree-sitter-c/blob/master/src/node-types.json
- **libclang**: **Apache-2.0 WITH LLVM-exception** (valid SPDX expression; this is the official LLVM license since the 2019 relicensing). **Not copyleft** — Apache 2.0 is permissive. The LLVM exception explicitly waives Apache §4(a)/(b)/(d) for LLVM portions embedded in Object form, i.e., you may redistribute binaries that embed/link LLVM **without source-disclosure-style conditions**; your proprietary code stays proprietary. Remaining practical obligations when you *ship libclang itself* (or a build embedding it): pass through the Apache-2.0 license text + NOTICE, retain attribution. A patent grant (§3) and explicit GPLv2-conflict waiver are included. Source: https://github.com/llvm/llvm-project/blob/main/llvm/LICENSE.TXT
- **Copyleft landmines**: (a) Official tree-sitter grammars (incl. `-c`, `-cpp`) are MIT, but **third-party community grammars can be GPL-3.0** — confirmed examples: `nverno/tree-sitter-jq`, `jbearak/tree-sitter-stata`, `corth-lang/tree-sitter-corth` all GPLv3. Audit every non-official grammar before vendoring its generated parser. (b) **Semgrep OSS is LGPL-2.1** (weak copyleft) — do not copy its OCaml CST→AST mapping code into Perfpal core. (c) LLVM side is clean: entire clang/LLVM core is Apache-2.0 WITH LLVM-exception (pre-2019 releases were UIUC/BSD-style, also permissive). Sources: https://github.com/nverno/tree-sitter-jq , https://github.com/jbearak/tree-sitter-stata , https://github.com/semgrep/semgrep (LICENSE: LGPL-2.1)

## CAPABILITIES (C-focused)

- **Error tolerance: CONFIRMED.** Official docs: tree-sitter aims to be "**Robust** enough to provide useful results even in the presence of syntax errors." It emits `ERROR`/`MISSING` nodes and keeps producing a walkable CST instead of aborting; downstream tools track recoverable partial parses (Semgrep/OpenGrep record `skipped_tokens`/`inserted_tokens` from tree-sitter ERROR/MISSING nodes). Caveat: recovery quality varies per grammar/heuristic (known imperfect-recovery issues exist). Sources: https://tree-sitter.github.io/tree-sitter/ , https://news.ycombinator.com/item?id=46217506 , https://github.com/opengrep/opengrep/blob/f458d7f0/src/parsing/Parsing_result2.mli
- **No semantic analysis: CONFIRMED.** tree-sitter yields a concrete syntax tree only. The C grammar models preprocessing *syntactically* as node kinds: `preproc_include`, `preproc_def`, `preproc_function_def`, `preproc_call`, `preproc_if(def)`, etc. — no expansion, no type resolution, no symbol binding. Types appear as syntactic `type_identifier` nodes. Sources: https://tree-sitter.github.io/tree-sitter/ , https://github.com/tree-sitter/tree-sitter-c/blob/master/src/node-types.json
- **libclang flag requirements: CONFIRMED.** Useful results require per-file compile flags; the canonical mechanism is `compile_commands.json` (via `CMAKE_EXPORT_COMPILE_COMMANDS`, Bear, Bazel extractor) or the simpler `compile_flags.txt`. On missing includes/incomplete TUs: preprocessor errors (`file not found`) cascade — declarations living in unfound headers simply don't exist in the AST, so dependent functions/types/calls vanish from analysis. Options like `CXTranslationUnit_SingleFileParse | KeepGoing | Incomplete` mitigate but don't fix: "too little is reported when parsing incomplete files… not really possible to even parse correctly without the complete translation unit" (LLVM discourse). C is somewhat more forgiving than C++ but the dependency on include paths is fundamental. Sources: https://clang.llvm.org/docs/JSONCompilationDatabase.html , https://discourse.llvm.org/t/using-libclang-for-simple-per-file-static-analysis/28194 , https://stackoverflow.com/questions/72452070/
- **libclang APIs relevant to Perfpal**: cursor traversal via `clang_visitChildren` over typed `CXCursor`s (function defs, calls, decls — this is the basis for DIY call-graph extraction using `CXCursor_CallExpr` + `clang_getCursorReferenced`/`clang_getCursorDefinition`; there is **no turnkey call-graph API** in the C API); include graph via `clang_getInclusions(tu, CXInclusionVisitor, …)` which reports each included file with its inclusion stack (verified in `Index.h`); reference/indexing info via `clang_indexTranslationUnit`, `clang_findReferencesInFile`; plus diagnostics, tokenization, and a `CXCompilationDatabase` loader for compile_commands.json. Source: https://github.com/llvm/llvm-project/blob/main/clang/include/clang-c/Index.h
- **Footprint/startup**: `libclang.so` is heavy — ~124 MB self-built (release, unstripped), ~70 MB+ typical distro builds, often plus a large `libLLVM*.so` unless dylib-linked; per-TU parse cost scales with the whole include closure (tens of ms to seconds/TU, plus significant memory). tree-sitter runtime is a tiny dependency-free C library (compiled artifact on the order of hundreds of KB; generated C parser adds similar), designed to "parse on every keystroke" — sub-ms to low-ms per file, cost linear in file size only since nothing is expanded. Sources: https://discourse.llvm.org/t/libclang-so-3-9-quite-large-124-mb-in-centos-6-5-after-compiling-from-source-in-release/43535 , https://bbs.archlinux.org/viewtopic.php?id=238427 , https://github.com/android/ndk/issues/2010 , https://tree-sitter.github.io/tree-sitter/

## PRECEDENT

- **tree-sitter used for C-family analysis/search at scale**: (1) **GitHub code navigation** — search-based tagging and stack-graphs "precise code navigation" are "implemented using the Tree-sitter parser ecosystem" (https://github.com/github/code-navigation , https://github.blog/open-source/introducing-stack-graphs/); (2) **Semgrep** — parses C/C++ via tree-sitter C/C++ grammars and maps CST→generic AST, explicitly handling partial parses (https://github.com/semgrep/semgrep/blob/develop/languages/cpp/tree-sitter/Parse_cpp_tree_sitter.ml); (3) **ast-grep** — structural search/lint/rewrite CLI built directly on tree-sitter parsers (https://github.com/ast-grep/ast-grep). (Editors Neovim/Helix also use it for highlighting.)
- **libclang/Clang frontend used for analysis**: (1) **clang-tidy / clangd** — linting and IDE semantics on the Clang frontend (https://clang.llvm.org/extra/clang-tidy/); (2) **OCLint** — C/C++/ObjC quality rules over the Clang AST, "based on Clang Tooling", BSD-licensed (https://oclint.org/); (3) **include-what-you-use** — #include analysis built against Clang libraries, tightly version-coupled and flag-dependent (https://include-what-you-use.org/).

## SURPRISES

- **For possibly-non-compiling AI-generated C, the capability asymmetry is decisive**: libclang without correct flags/includes doesn't degrade gracefully — whole declaration sets silently disappear from the AST — while tree-sitter always returns a complete CST with localized `ERROR` nodes. For a heuristic-only MVP, tree-sitter matches the input reality; libclang effectively assumes a build system that AI-generated repos often lack.
