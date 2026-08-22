# Perfpal — Specification (Draft)

## Vision
Perfpal is a static-analysis CLI tool that scans codebases and produces three complementary outputs:
1. **Performance heuristics** — language-specific quantitative metrics that flag likely optimization candidates.
2. **Architecture-level bottleneck visualization** — static charts that expose structural issues (dependency structure, component coupling, hot components) so bottlenecks can be seen visually, not just as per-function metric heatmaps.
3. **LLM-consumable quantified metrics** — structured raw data plus a human-readable report, designed to feed an LLM so it can optimize the codebase effectively. Thesis: LLMs lack explicit knowledge of writing performant code; quantified metrics fix that.

Target audience: developers working with AI-generated code (sheer volume makes manual validation impossible); also human-written codebases.

## Scope

### MVP (Minimum Viable Product)
- **Product form**: CLI tool only. Scans a codebase and saves all artifacts to an output directory.
- **Language**: ONE language. Assumed **C** (pending founder confirmation) because C has the most extensive documentation and surrounding projects, and C/C++/Python share parsing foundations.
- **Metrics**: Performance-relevant static heuristics for C. Include all sensible heuristics (cyclomatic complexity, cognitive complexity, function length, nesting depth, parameter count, cache-unfriendly patterns, malloc density, recursion depth, etc.) without prioritization for MVP.
- **Visualization**: Static charts exposing architecture-level issues — dependency graphs, component coupling maps, hot-component identification. Form flexible; choose easiest to develop (likely static HTML/SVG or image files in the output directory).
- **LLM interface**: Output BOTH:
  - Raw machine-readable data (structured JSON with schema).
  - Human-readable report (Markdown or formatted text).
- **Target LLM**: ONE target LLM only; no multi-LLM format variants.

### Post-MVP (explicitly out of scope for now)
- Additional languages: C++, Python, Rust, Java, Go, JavaScript/TypeScript, etc.
- Runtime profiling ingestion (perf, valgrind, cProfile) — architected to bolt on later, but not in MVP.
- Web UI / SaaS / CI integration.
- Auto-fixing code (Perfpal produces metrics; optimization is LLM/human driven).
- Multi-LLM output formats.

## Non-Goals (MVP)
- Web UI, SaaS, or CI pipeline integration.
- Multi-language support.
- Runtime profiling data ingestion.
- Auto-fixing or code rewriting.
- Real-time analysis (batch/CLI only).

## Tech Stack (TBD — open architectural forks)
- **Language parsing**: Undecided between tree-sitter (fast, universal, lighter), libclang (C-precise, heavy, no copyleft issues), or hybrid approach. This is the highest-priority architectural decision.
- **Metric computation**: Custom engine built on top of parser output; may integrate with clang-tidy/cppcheck check results.
- **Visualization**: Static files (HTML/SVG/PNG) written to output directory. City metaphor or dependency graphs; easiest viable option.
- **LLM interface**: Structured JSON report + Markdown report.

## License & Dependency Policy
- **License**: Source-available proprietary. Free to use; modification and redistribution require explicit permission.
- **Dependency constraint**: NO copyleft (GPL-family) dependencies. All direct and transitive dependencies must be permissive (MIT, Apache-2.0, BSD, etc.) or proprietary. Must audit license compatibility before adding any dependency.

## Acceptance Criteria (MVP)
- Perfpal CLI can scan a C codebase and produce an output directory containing:
  - JSON file with structured performance heuristics.
  - At least one static chart showing architecture-level bottleneck analysis.
  - Human-readable Markdown report summarizing findings.
- The JSON output is structured enough that an LLM can consume it and suggest specific optimizations.
- No copyleft dependencies in the dependency tree.

## Open Questions
- Confirm assumption: MVP language = C.
- Confirm assumption: static-analysis-first MVP, runtime profiling bolt-on later.
- Define exact C performance heuristics list (what counts as "sensible"?).
- Choose parser strategy (tree-sitter vs libclang vs hybrid).
- Define JSON schema for LLM-consumable output.

## Known Risks
- **Static analysis is approximate, not ground truth**: heuristics flag "optimization candidates," not guaranteed bottlenecks. LLM must validate with runtime profiling. Framing must be careful.
- **Architecture-level visualization for C is hard**: preprocessor, headers, and implicit dependencies make module/call graph extraction non-trivial.
- **Scope creep**: founder wants "all sensible heuristics" — need a cutoff for MVP.
- **No formal success criteria**: "done" is subjective; use acceptance criteria above as proxy.
