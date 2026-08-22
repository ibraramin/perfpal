# Spec: perfpal-core-spec

Scope: repo

# Perfpal — Repository Scope Spec

## Vision
A CLI tool that scans a codebase and produces quantifiable, performance-oriented insight: simple language stats, architecture-level bottleneck visualization, and LLM-consumable metrics — so humans can see bottlenecks and LLMs can optimize code with explicit, quantified signals instead of guessing.

Primary audience: local developers working with **AI-generated codebases** (sheer volume makes manual validation impossible); equally usable on human-written code.

## Capability Pillars
1. **Rudimentary stats (1a)** — per-language structural/performance statistics (complexity, sizes, counts).
2. **Architecture-level bottleneck visualization (1b)** — NOT just per-function metric heatmaps: expose *architecture-type issues* (dependency structure, component coupling, hot components/processes) so bottlenecks are visible visually. Static charts; rendering form = whatever is easiest to develop well (terminal ideal but explicitly deprioritized).
3. **LLM-facing output (2)** — BOTH raw machine-readable data AND a human-readable report. Generic, LLM-agnostic JSON schema (target LLM undecided by design). AI-codebase comprehension rides on pillar 2's structural view.

## Product Form (decided)
- **CLI only** (`perfpal`); all artifacts written to an output directory. Default layout: `perfpal-report/` containing `metrics.json`, `report.md`, `charts/`.
- Runs locally; local developers are the priority (no SaaS/CI-first concerns).

## MVP Definition (decided)
- **One language: C** (confirmed). Rationale: shared foundations with C++/CPython; richest documentation/ecosystem.
- **Heuristics only**, from a **prioritized catalog signed off by the founder** (not literally "all known heuristics").
- **Strictly static analysis**, architected so runtime-profile ingestion (perf/valgrind/cProfile) bolts on later without rewrite (confirmed).
- No formal success criteria required yet.

## Non-Goals (MVP)
Web UI, SaaS/multi-user, CI integration focus, languages beyond C, runtime profiling ingestion, multi-LLM output variants, terminal-native charts.

## Post-MVP Roadmap (direction only)
C++ and Python next (shared foundations); runtime-profile ingestion layer; further languages long-term (C++, Rust, Python, JS, TypeScript, Java, Go).

## License & Dependency Policy (constraint)
- Source-available proprietary: **free to use; modification and redistribution require explicit founder permission** (strategy: acquisition target).
- **HARD CONSTRAINT: no copyleft (GPL-family) dependencies** anywhere in the core tool.

## Open Architectural Forks
- **Parser strategy:** tree-sitter (fast/universal/imprecise) vs libclang (precise/C-C++-only/heavy) vs hybrid → ADR-0001, gates everything.
- **Visualization tech:** static SVG/HTML chart generation approach → ADR-0002.

## Known Risks
1. Static analysis ≠ performance ground truth — all outputs framed as *optimization candidates*, never certainties; runtime validation is post-MVP.
2. Heuristic noise may drive LLM over-optimization — confidence/severity fields mandatory in schema.
3. Scope creep (8+ language ambition) — MVP ruthlessly locked to C.