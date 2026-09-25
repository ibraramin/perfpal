# Handoff — Latest

## Objective
Session 38 **COMPLETE** — plan `evidence-beta` **M3 (Python backend)** entered through a new tracking plan **`python-backend`**. This session was **planning + design-lock ONLY** (zero product code): discovery (@librarian + 2×@explorer) + an @oracle strategy pressure-test produced founder-locked decisions, the plan was created and linked, and **ADR-0007** records the Python tier strategy. **Commit pending founder approval.**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (B0–B6; publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 design-lock ✅ → W0 canary)** · **`python-backend` NEW (active)**.

## M3 locked decisions (this session)
| # | Decision |
|---|---|
| Split | W0 canary → W1 design+pin → W2 backend+inference → W3 catalog+schema 1.3 → W4 proof; one wave per session |
| Strategy | AST + lightweight binding inference. **Not bytecode, not native C-extension** (ADR-0007). Tiers: T1 (M3) → T2 `python-resolver` → T3 runtime profile |
| Rule set | **Retire 10** (`TYPE-001/002`, `MEM-001`, `ALLOC-002`, `STR-002/003`, `LOOP-004/005/006/008`); keep ~13 reframed mirrors + Python-native adds; every finding cites the CPython mechanism |
| Inference | Ships **in** M3: import-alias map, local kind lattice, param annotations, receiver root, async flag (additive IR, `syntactic`, ≤MEDIUM) |
| Import graph | Best-effort resolve; unresolved internal → data-only, **no finding** |
| Corpus | `examples-python/` as a real package |
| Grammar | `tree-sitter-python = "=0.25.0"` (official org, MIT, ABI 15 ✓ window 13–15) |
| Resolver | `semantic-clang` stays **C-only** |

## Verification
- **Planning-only: no product code, no gates run** (nothing to build). Artifacts: plan `python-backend` (`docs/plans/python-backend.md`, linked `perfpal-core-spec` + `evidence-beta-spec`; **no feature spec**), `docs/adr/ADR-0007-python-tier-strategy.md`, CHANGELOG + state files.
- Discovery evidence: @librarian (tree-sitter-python 0.25.0 pin/ABI/license + full node-types map + Python idiom reframings); @explorer ×2 (backend/IR/scanner edit surface; schema/registry/tests/doc enumeration points); @oracle (strategy pressure-test).

## Risks / Open (append-only)
- **W1 design obligations:** comprehensions must be modelled as `LoopNode` (iterable once — the Rust `for` fix); module-level code needs a synthetic function record or script-style AI files yield zero findings; `.ipynb` decided explicitly; ARCH-003 noisier on Python (expect Info/low).
- **Precision-feedback risk:** the M2 benchmark/precision harness has no Python coverage until W4 → the W0 canary is the mitigation (else M4 is the first Python precision test = the M1 failure mode).
- **Tooling gap:** `ruff` is NOT installed on this host; cargo is off PATH by default (`export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"`). The canary needs these resolved (or skip the Ruff baseline).
- **Precedent conflict (recorded):** evidence-beta's M3 line says "mirror the FULL catalog"; the founder + @oracle approved retiring 10. The W1 disposition table supersedes the plan wording.

## Next Actions
1. **Founder approves this ritual commit** → commit the working tree (**EXCLUDE `.opencode/**`**):
   `docs(python-backend): plan python-backend (evidence-beta M3) + ADR-0007 Python tier strategy (AST+binding, retire-10, bytecode is not a tier); planning + design-lock only, no product code`
2. **NEXT SESSION (`/new`) = plan `python-backend` W0 CANARY:** fetch pinned **Aider** (Apache-2.0) + **chunkhound** (MIT), run a throwaway pattern counter (Python stdlib `ast` or tree-sitter-python) per candidate `PERF-PY` rule + silent control, rank frequency × plausibility, baseline a Ruff `PERF` run, write `docs/evidence/python-canary.md`, finalize the rule set. Read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST, then `readPlan python-backend`.
3. Then **W1** (pin + `tests/python_grammar_link.rs` + `docs/architecture/python-backend.md` with the node-kind map + disposition table) → **W2** backend + inference + import resolution → **W3** catalog + registry + reporter + schema 1.3 → **W4** examples-python / precision / benchmark / goldens / docs + dual review.

## Change set (this session, uncommitted; EXCLUDE `.opencode/**`)
- **New:** `docs/plans/python-backend.md` (planning-toolkit plan, linked to both repo specs), `docs/adr/ADR-0007-python-tier-strategy.md`.
- **Modified:** `CHANGELOG.md` ([Unreleased] Added bullet), `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Note:** Cargo/`.gitignore` untouched; **no `src/` change**.
