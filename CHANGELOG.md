# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Heuristic catalog v1 for C (`docs/heuristics/catalog-v1.md`): 15 Tier-1 MVP heuristics + 8 Tier-2 should-haves, ranked by signal-value vs implementation cost; grounded against clang-tidy/cppcheck/PVS-Studio; differentiation items marked; dual review passed (@requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES, all fixes applied). Awaiting founder sign-off.
- ADR-0001: Parser strategy ACCEPTED — tree-sitter-first behind a parser-agnostic IR; includes dependency-license audit vs the no-copyleft constraint, guardrails, and reversal triggers.
- Project bootstrap: state skeleton, rules, docs directories, CHANGELOG, and todos.
- Phase 0 landscape research summary (static analyzers, visualization tools, LLM metric gaps).
- Founder clarifying question list drafted.

### Changed
- Founder SIGNED OFF heuristic catalog v1 ("All approved"): Tier 1 (15 heuristics) confirmed as MVP set, Tier 2 (8) included in MVP scope, deferred/excluded list accepted as out-of-scope. Catalog IDs (`PERF-C-*`) stable for schema work.
- Updated session-state.json, handoff/latest.md, and activeContext.md to reflect retry status and research findings.
- Expanded .opencode/spec.md with full founder answers: CLI-only, architecture-level visualization, dual LLM output, source-available license, no copyleft deps.
- Finalized Phase 0: all 5 blocking questions answered, assumptions confirmed, formal planning objects created (spec: perfpal-core-spec, plan: mvp-architecture).
- Founder formally APPROVED specs (perfpal-core-spec + mvp-architecture) — design gate open. Initial skeleton committed as 6bbfafd.
