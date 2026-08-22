# Handoff — Latest

## Objective
Phase 0 Discovery complete for Perfpal. All founder answers confirmed. Formal planning objects created. Ready to execute `mvp-architecture` plan.

## Phase
phase0-complete / design-plan-ready (mvp-architecture)

## Key Decisions
- Product form: **CLI only**, output to `perfpal-report/` directory.
- Audience: **local developers**.
- Visual: **static charts** exposing **architecture-level issues** (dependency structure, component coupling, hot components).
- MVP language: **C** (confirmed).
- MVP scope: **static analysis only**, heuristics only, one language. Runtime profiling architected as future bolt-on.
- Heuristics: **prioritized catalog** for sign-off, not exhaustive.
- LLM output: **LLM-agnostic JSON schema** + human-readable `report.md`. No vendor-specific tuning.
- License: **source-available proprietary**. **No copyleft dependencies**.
- Target LLM: **undecided / generic**.
- Output layout: `perfpal-report/` with `metrics.json`, `report.md`, `charts/`.

## Risks
- Parser choice (tree-sitter vs libclang vs hybrid) is the highest-priority architectural fork.
- No copyleft constraint limits dependency pool; every transitive dep must be audited.
- Architecture-level visualization for C is hard (headers, preprocessor, implicit dependencies).
- Static heuristics are approximate; must be framed as "optimization candidates," not ground truth.
- No formal success criteria; "done" defined by acceptance criteria in spec.

## Next Actions
1. Start **fresh session** (`/new`).
2. Read plan `mvp-architecture` via `readPlan`.
3. Execute Step 1: **ADR-0001 parser choice**.
4. Continue through remaining 6 steps.

## Uncommitted Files
All skeleton files (rules, state, docs, CHANGELOG, todos) remain untracked. No commits yet.
