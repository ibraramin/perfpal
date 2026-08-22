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
RESEARCH ONLY — no code changes, findings only in your final message. You are researching for ADR-0001 of "Perfpal", a proprietary static-analysis CLI (C language MVP). HARD CONSTRAINT: zero copyleft (GPL-family) dependencies in the core tool.

Verify with current sources (official repos/docs preferred). Respond in ≤20 structured bullets grouped under: LICENSES / CAPABILITIES / PRECEDENT / SURPRISES. Include source URLs. Mark anything you cannot verify as UNVERIFIED rather than guessing.

LICENSES
1. tree-sitter core runtime license (tree-sitter/tree-sitter) — exact SPDX.
2. tree-sitter-c grammar license (tree-sitter/tree-sitter-c) — exact SPDX; does generated parser code inherit it?
3. libclang licensing — confirm Apache-2.0 WITH LLVM-exception; is it copyleft or not; practical redistribution implications for a proprietary tool linking libclang.
4. Copyleft landmines in either ecosystem (community grammars, submodules, build deps).

CAPABILITIES (C-focused)
5. tree-sitter error-tolerance on syntactically broken code — confirm/deny with evidence.
6. tree-sitter: semantic analysis absent? How are #include/preproc directives modeled (nodes without expansion)?
7. libclang: need for compile_commands.json/flags; behavior on incomplete/non-compiling translation units.
8. libclang APIs relevant to analysis: call-graph extraction support, include-graph (clang_getInclusions), cursor/indexing APIs — brief.
9. Footprint/startup comparison: libclang .so size + per-TU parse cost vs tree-sitter runtime size + per-file parse cost.

PRECEDENT
10. 2–3 real-world tools doing C/C++ analysis or code search with tree-sitter (names + use).
11. 2–3 real-world tools using libclang/Clang for analysis — one line each.

SURPRISES
12. Anything that would change a tree-sitter-vs-libclang decision for a heuristic-only static-analysis MVP targeting possibly-non-compiling AI-generated C codebases.

---

ASSISTANT:
MIT confirmed for both tree-sitter repos. Now verifying LLVM/libclang licensing and tree-sitter error-recovery claims.

---

ASSISTANT:
LLVM license confirmed verbatim. Now checking libclang parse options/inclusion APIs and tree-sitter-c's preprocessor node modeling.

---

ASSISTANT:
Local machine check for real libclang footprint plus precedent searches.

---

ASSISTANT:
# ADR-0001 Research Findings: tree-sitter vs libclang for Perfpal

## LICENSES

1. **tree-sitter core runtime = `MIT`** (exact SPDX). Verified verbatim: "The MIT License (MIT) Copyright (c) 2018 Max Brunsfeld". https://raw.githubusercontent.com/tree-sitter/tree-sitter/master/LICENSE
2. **tree-sitter-c grammar = `MIT`** (exact SPDX): "Copyright (c) 2014 Max Brunsfeld". The generated `src/parser.c` (3.87 MB, Content-Length verified) is checked into that same MIT repo and carries no separate license — generated parsers inherit the grammar's MIT license, so vendoring `parser.c` into a proprietary tool is fine. https://github.com/tree-sitter/tree-sitter-c , https://github.com/tree-sitter/tree-sitter-c/blob/master/LICENSE
3. **libclang = `Apache-2.0 WITH LLVM-exception`** — confirmed verbatim in llvm-project `llvm/LICENSE.TXT` and DeveloperPolicy ("The code is licensed under permissive open source licensing terms, namely the Apache-2.0 with LLVM-exception"). **Not copyleft.** Practical redistribution: Apache-2.0 §4 requires only preserving license/attribution notices when you redistribute LLVM's code itself; linking a proprietary binary does not make your tool Apache/GPL. The exception explicitly lets embedded object-form portions skip §4(a),(b),(d), and includes a built-in GPLv2-conflict waiver clause. Ship `libclang.so` alongside your binary + a notices file; zero obligations on Perfpal's own code. https://github.com/llvm/llvm-project/blob/main/llvm/LICENSE.TXT , https://llvm.org/docs/DeveloperPolicy.html
4. **Copyleft landmines:** LLVM side is clean (>99% relicensed to Apache-2.0 WITH LLVM-exception; the small legacy remainder is UIUC/NCSA — also permissive; third-party code is quarantined in its own dirs with its own licenses). The **tree-sitter grammar ecosystem is the real minefield**: verified copyleft grammars exist — `foxy/tree-sitter-prolog` is **AGPL-3.0**, a nim grammar was **MPL-2.0**, `tree-sitter-forester` is **GPL-3.0**; downstream packs now enforce permissive-only admission precisely because of this. Rule: audit every vendored grammar's LICENSE; core runtime + tree-sitter-c are safe. https://github.com/DataGrout/tree-sitter-grammars/blob/main/README.md , https://github.com/kreuzberg-dev/tree-sitter-language-pack/releases/tag/v1.1.4

## CAPABILITIES

5. **tree-sitter error-tolerance: CONFIRMED.** Official README: "Robust enough to provide useful results even in the presence of syntax errors." Independent confirmation from Semgrep engineering: "Tree-sitter has some powerful error-recovery mechanism where a mistake in a file … does not cause the whole file to fail; Instead tree-sitter tries its best to parse what it can and to return a valid AST." https://github.com/tree-sitter/tree-sitter#features , https://github.com/returntocorp/semgrep/issues/2186
6. **No semantic analysis — syntax only.** Preprocessor directives are plain syntax nodes, never expanded: `preproc_include` (with `path` child: `string_literal` or `system_lib_string`), `preproc_def`, `preproc_function_def`, `preproc_if/ifdef/elif/else`, `preproc_call`, `preproc_defined`, `preproc_params`. Verified against `src/node-types.json`. Consequence: an include-graph from tree-sitter is text-level only. https://github.com/tree-sitter/tree-sitter-c/blob/master/src/node-types.json
7. **libclang needs compile flags**: `clang_parseTranslationUnit(tu, src, command_line_args, ...)` takes compiler args; `compile_commands.json` is consumed via the `CXCompilationDatabase` API (standard libclang API — not re-fetched this session, UNVERIFIED-by-fetch). **Broken-TU behavior is configurable** — verified flags in the doxygen enum: `SingleFileParse` (skip includes), `KeepGoing` (continue past fatal errors), `Incomplete`, `SkipFunctionBodies`, `IgnoreNonErrorsFromIncludedFiles`, `RetainExcludedConditionalBlocks`, plus `DetailedPreprocessingRecord`; diagnostics remain retrievable from a partially parsed TU. https://clang.llvm.org/doxygen/group__CINDEX__TRANSLATION__UNIT.html
8. **Analysis APIs:** `clang_getInclusions` + `CXInclusionVisitor` give the true post-preprocessing include graph with inclusion stacks ("Visit the set of preprocessor inclusions in a translation unit") — requires parsing with `DetailedPreprocessingRecord` (confirmed by Doug Gregor on LLVM forums). Cursor traversal (`clang_visitChildren`) + indexing API (`clang_indexTranslationUnit` + IndexerCallbacks) power cross-reference indexes. **No first-class call-graph API in the libclang C API** — tools like ccls build caller/callee hierarchies themselves from cursor references (`CXCursor_CallExpr` → `clang_getCursorReferenced`). https://clang.llvm.org/doxygen/group__CINDEX__MISC.html , https://discourse.llvm.org/t/lib-clang-help-request-usage-of-clang-getincludedfile/21584
9. **Footprint/startup (measured data points):** Debian `libclang1-19` ≈ **37 MB installed** (amd64) *plus* hard dependency `libllvm19` ≈ **127 MB installed** → ~164 MB distro route; bundled alternative: PyPI `libclang` 18.1.1 manylinux x86_64 wheel = **24.5 MB compressed** (unpacked size UNVERIFIED, likely ~100 MB). tree-sitter route: runtime is ~20 dependency-free C files + `parser.c` at 3.87 MB source (compiled total ~1 MB order-of-magnitude, approximate). Speed: official claim "Fast enough to parse on every keystroke"; third-party docs cite ~10 ms / 1 MB full parse (~100 MB/s) — approximate, unofficial mirror. Real-world contrast: ast-grep scans a corpus at ~79 MiB peak RSS (C-backed build) vs ccls reporting **180 MiB–1.8 GB RSS** for libclang-based indexing. Precise libclang per-TU ms figures: UNVERIFIED (commonly tens–hundreds ms small files, seconds for large TUs). https://packages.debian.org/trixie/libclang1-19 , https://packages.debian.org/trixie/libllvm19 , https://pypi.org/project/libclang/#files , https://astgrep.com/blog/tree-sitter-end-to-end , https://github.com/MaskRay/ccls/blob/master/README.md

## PRECEDENT

10. **GitHub code navigation + new code search** — "Code navigation is implemented using the Tree-sitter parser ecosystem"; tag queries extract definitions/references (incl. `@definition.macro`, `@reference.call`) across billions of documents. https://github.com/github/code-navigation
11. **Semgrep** — parses most languages including C via tree-sitter; error recovery is central to its design (partial-parse reporting, skipped-token handling). https://semgrep.dev/docs/contributing/adding-a-language
12. **ast-grep** — structural search/lint/rewrite CLI on tree-sitter; C (`.c`, `.h`) is a built-in language; its blog documents production tree-sitter runtime tuning. https://astgrep.com/reference/languages
13. **ccls** — C/C++/ObjC language server built directly on libclang: cross-references, call/callee hierarchies, whole-project parallel indexing. https://github.com/MaskRay/ccls
14. **rust-bindgen** — generates Rust FFI bindings by parsing C headers through libclang (its packaging issues document libclang's size/linking pain). https://github.com/rust-lang/rust-bindgen/issues/918
15. **clangd** — "based on the `clang` compiler, and at its core runs the clang parser in a loop" (uses Clang's internal C++ libraries rather than `libclang.so` proper — same license family, heavier integration). https://clangd.llvm.org/design

## SURPRISES

16. **Licensing does NOT differentiate these options** — MIT vs Apache-2.0-WITH-LLVM-exception both satisfy the zero-copyleft constraint cleanly; the ADR should be decided on capability/footprint, with grammar-license audits as the only hygiene item.
