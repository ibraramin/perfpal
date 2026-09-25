# ADR-0007: Python Tier Strategy — AST + Binding Awareness (not Bytecode, not C)

- **Status:** Accepted (founder-approved session 38, after an @oracle pressure-test)
- **Date:** 2026-09-25
- **Deciders:** Founder + orchestrator (evidence: @librarian tree-sitter-python / Python-idiom research; @oracle strategy pressure-test, session 38)
- **Gates:** plan `python-backend` (evidence-beta **M3**), `docs/roadmap.md` §M3
- **Related:** [ADR-0001](ADR-0001-parser-strategy.md) (parser strategy — exact-pin G5), [ADR-0003](ADR-0003-language-order.md) (language order), [ADR-0004](ADR-0004-cross-language-deferral.md) (cross-language deferral), plan `semantic-resolver` (the C analogue of T2)

## Context

M3 adds Python (ADR-0003, position 3 — the highest AI-generated-code share). The founder requirement driving this decision: *"a lot of AI-influenced codebases are written in Python, so our python-perfpal has to be GREAT from the start."* A hypothesis was raised that, because CPython (the interpreter) is written in C, a "deep analysis" could analyze the *interpreted C code* and report issues back in Python terms.

Three candidate "deep" tiers were evaluated: (a) Python **bytecode** analysis; (b) **native C-extension** analysis (reusing the existing C backend + libclang resolver); (c) **AST + lightweight binding/type inference**.

## Decision

1. **Static, AST-based analysis only.** Python uses the official `tree-sitter-python = "=0.25.0"` grammar (MIT, `github.com/tree-sitter/tree-sitter-python`; generated parser declares ABI **15**, inside the pinned runtime's accepted window `13..=15`; exact pin per ADR-0001 guardrail G5). The parser-agnostic IR contract is unchanged — a new language is the sanctioned `modules.md` §5.2 extension point, so **no IR-contract ADR is required** for the language addition itself.
2. **Bytecode is NOT a static-analyzer tier.** It requires CPython (or a reimplementation of `compile`) at analysis time, which destroys the single-binary / zero-runtime-dependency model; opcode shapes are version-adaptive (PEP 659 inline caches; 3.12/3.13 churn), so opcode-pinned rules rot every minor release; and the real Python bottlenecks (library calls, superlinear accumulation, I/O) are invisible in bytecode. Bytecode belongs to the **runtime-profile-ingestion** axis (CONFIRMED-1), not the static analyzer. Corollary: AST restatements of dead bytecode-era micro-opt advice (attribute/global hoisting, `len()` caching, locals-binding, `%` vs `*`) are **out of scope**.
3. **Native C-extension analysis is rejected for M3.** AI Python repos are ~pure Python; the C that actually runs under them is usually pip-installed third-party, not in-repo (in-repo C/Cython-transpiled `.c` is already scanned by the C backend once dispatch sees it). Coverage is too low to justify a dedicated tier.
4. **The Python tier model is three levels:**
   - **T1 — static AST + binding awareness (THIS plan / M3).** Syntactic rules plus a *minimal* binding/alias inference pass (§5).
   - **T2 — cross-module resolver (post-M3 plan `python-resolver`).** The Python analogue of the C `semantic-resolver` tier.
   - **T3 — runtime-profile ingestion (CONFIRMED-1).** Where bytecode / `cProfile` / `perf`-style inputs belong.
5. **Minimal binding/alias inference ships IN M3** (not deferred), because dynamic typing is the Python false-positive source and the flagship rules are impossible or dishonest without it: per-file import-alias map (`import pandas as pd`, `from pandas import DataFrame as DF`), per-function single-assignment local kind lattice (`list|dict|set|str|bytes|DataFrame|Series|ndarray|filehandle|unknown`), parameter annotations, call-receiver root, and an `async` flag on `FunctionRecord`. Additive `#[serde(default)]` IR fields only; `resolver_tier` stays `syntactic` with confidence ≤ MEDIUM where inference is used; rules must never claim `semantic` (the central `apply_upgrade_hooks` gate reverts unbacked claims).
6. **Rule set: retire 10, keep ~13 mirrors, add Python-native rules.** Retire `TYPE-001`, `TYPE-002`, `MEM-001`, `ALLOC-002` (no Python analogue) and `STR-002`, `STR-003`, `LOOP-004`, `LOOP-005`, `LOOP-006`, `LOOP-008` (premise does not transfer / bytecode-era folklore). Keep with reframing `ALLOC-001`, `ALLOC-003`, `STR-001`, `LOOP-001`, `LOOP-002`, `LOOP-003`, `LOOP-007`, `CALL-001`, `CALL-002`, `ARCH-001`, `ARCH-002`, `ARCH-003`, `MEM-002`. Add Python-native rules (row-wise pandas / `concat`-in-loop, N+1 DB/HTTP in loop, iterator materialisation, `range(len())`, **blocking calls inside `async def`**, executor/event-loop per iteration, manual comprehension loops). **Every finding must cite the CPython mechanism** that causes the cost (mechanism + fix + accepted FP class per entry). The durable disposition table lands in `docs/architecture/python-backend.md` (W1).
7. **Python import graph = best-effort resolution.** Relative imports + scan-root packages resolve to real files; **unresolved internal imports stay data-only with NO finding** (mirrors the Rust N1 decision that removed an entire FP finding stream). `__init__.py` re-export hub handling is decided explicitly in W1.
8. **`semantic-clang` (libclang) stays C-only.** Python is syntactic-tier throughout (`resolver_tier = syntactic`); no Rust/Python semantic resolver exists.
9. **`examples-python/` is a real package** (`__init__.py` + numbered modules, one per heuristic family) so import resolution and ARCH-001/003 are exercised end-to-end (mirrors the `examples-rust/` mini-crate, whose `use`-cycle only resolves with a package layout).

## Consequences

- M3 delivers a **narrower but honest** catalog (≈13 mirrors + Python-native additions) rather than a mechanical 23-rule mirror — **precision over breadth**, consistent with the evidence-first pitch (ADR-0005) and the M1 lesson (5 rules caused ~89% of the self-scan noise).
- The additive IR fields (binding kinds, receiver root, `async`) are **IR-internal** (serde-default, not serialized into `metrics.json`) → **no schema bump from inference**; the schema MINOR `1.2 → 1.3` comes *only* from admitting the `"python"` language token.
- A **W0 canary** (pinned Aider + chunkhound + a Ruff `PERF` baseline, `docs/evidence/python-canary.md`) precedes catalog freeze, so M4 is not the first Python precision test.
- Bytecode/profiler work is explicitly routed to T3; a future Python *type* resolver is a separable T2 milestone (`python-resolver`), not M3 scope.
- Design obligations carried into W1: **comprehensions must be modelled as `LoopNode`** with the iterable evaluated once (the Rust `for`-iterable fix, `rust-backend.md` §3.8); **module-level code needs a synthetic function record** or script-style AI files (top-level loops) yield zero findings; **`.ipynb` notebooks** must be decided explicitly (parse cells with line-offset mapping, or document the exclusion on the accepted-limitations page); **ARCH-003 is noisier on Python** (duck typing + ubiquitous names `get`/`save`/`run`) — expect Info/low and measure.

## Alternatives rejected

- **Bytecode analysis as a Tier-1 static rule source** — runtime dependency, version-fragile opcodes, invisibility of the dominant bottlenecks.
- **Native C-extension analysis** — low coverage on AI Python repos; in-repo C already handled by the C backend.
- **Full type-checker integration (mypy / pyright)** — an external runtime (Python/Node) breaks the single-binary model, and the marginal precision for *performance* rules over lightweight binding inference does not justify it.
- **Pure syntactic M3 (no inference at all)** — cannot distinguish `list` vs `set` membership, DataFrame vs custom class, or `pd.concat` vs a local method, so the flagship rules would be impossible or dishonest.
- **`re.compile`-outside-loop rule** — CPython's 512-entry `re` cache makes the "compile outside the loop" advice folklore; not shipped.
