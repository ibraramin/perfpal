# Active Context

## Current Task
Session 38 **COMPLETE** — plan `evidence-beta` **M3 (Python backend)** entered through new tracking plan **`python-backend`**. **Planning + design-lock only** (zero product code): discovery + @oracle pressure-test → founder-locked decisions; plan created + linked; **ADR-0007** recorded. **Commit pending founder approval.**

## Recent Decisions
- **Python strategy (ADR-0007):** AST + minimal binding/alias inference — **not bytecode, not native C-extension analysis**. Tiers: T1 static AST+binding (M3) → T2 cross-module resolver (`python-resolver`) → T3 runtime profile (CONFIRMED-1). Bytecode is the profiler axis, not a static tier.
- **Rule set:** retire 10 C/Rust rules with no honest Python analogue (`TYPE-001/002`, `MEM-001`, `ALLOC-002`, `STR-002/003`, `LOOP-004/005/006/008`); keep ~13 reframed mirrors + Python-native additions; every finding cites the CPython mechanism.
- **Inference ships in M3** (additive IR, `resolver_tier=syntactic`, ≤MEDIUM) — dynamic typing is the FP source; flagship rules need receiver/kind awareness.
- **Import graph:** best-effort resolve; unresolved internal imports data-only (no finding, Rust N1 precedent).
- **`examples-python/` = a real package**; grammar pinned `tree-sitter-python = "=0.25.0"` (MIT, ABI 15 ✓); `semantic-clang` stays C-only.
- **Wave plan:** W0 canary → W1 design+pin → W2 backend+inference → W3 catalog+schema 1.3 → W4 proof; one wave per session.

## In Progress
- Nothing — planning/design-lock complete; commit pending founder approval.

## Next
- **NEXT SESSION (`/new`) = plan `python-backend` W0 CANARY:** pinned Aider + chunkhound, throwaway pattern counter per candidate `PERF-PY` rule, frequency × plausibility ranking, Ruff `PERF` baseline, `docs/evidence/python-canary.md`, finalize the rule set. Read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan python-backend`.
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; M2 non-blocking notes (`replace_json_value` uniqueness assert; expose `registered_rule_ids()`).
- **W1 design obligations:** comprehension-as-`LoopNode` (iterable once); module-scope synthetic function record; `.ipynb` decision; ARCH-003 Info/low on Python.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Commit:** pending founder approval (session-38 change set — see `state/handoff/latest.md`).
