# Perfpal — Project Rules

## Phase 0 Protocol (Mandatory)
1. Delegate to `@librarian` for web research on problem domain / key libraries / architecture patterns.
2. Ask the founder 3–5 penetrating questions about requirements, constraints, non-goals, and acceptance criteria. Never guess — ask.
3. Expand `.opencode/spec.md` from their answers (goal, scope, non-goals, tech stack, acceptance criteria, open questions).
4. WAIT for the user's explicit "Specs approved" before writing any application code.

## Session Contract
- ONE SESSION = ONE TASK. Commit per task. Fresh session per new task.
- Start of session: read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` first.
- End of session: update `state/session-state.json` (append-only arrays), regenerate `state/handoff/latest.md`, update `docs/activeContext.md` + `CHANGELOG.md`, commit (with user approval only).
- After every major task: delegate to `@reviewer` and `@requirements-reviewer` before declaring done.

## Delegation Rules
- `@librarian`: external docs, APIs, real-world patterns.
- `@explorer`: read-only codebase search — summaries only.
- `@fixer`: fast implementation after plan approval.
- `@designer`: UI/UX work.
- `@oracle`: hard architecture decisions, second opinions.
- `@vision`: images — ALWAYS a filesystem path.
- `@reviewer`: after EVERY major task — read-only, 9 categories, severity-ranked.
- `@requirements-reviewer`: after every major task — verbatim vs original request.
- `@docs-writer`: per milestone — read-code-first, never invents.
- `@bootstrap`: new projects: Phase 0 discovery + state skeleton.

## Context Discipline
- Compact BETWEEN tasks with a focus directive, never mid-debug.
- Working window sweet spot ~200–300k tokens.
- Never change config, tool set, or model mid-session.
