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
RETRY of a timed-out Phase 0 bootstrap for **Perfpal** at `/home/ibrar/Perfpal` (linux, git repo already initialized). Today is August 2026. RESEARCH + PLANNING ONLY — zero implementation code.

A previous bootstrap run created the state skeleton but TIMED OUT before finishing research/reporting. Existing files (verify content, fill gaps — do NOT blindly recreate): `.git/`, `.opencode/`, `CHANGELOG.md`, `state/session-state.json`, `state/handoff/`, `docs/activeContext.md`, `docs/specs/`, `docs/plans/`, `docs/adr/`, `todos.md`.

## PROJECT VISION (from founder — preserve intent faithfully)

A tool that scans codebases and produces:
1. **Performance heuristics:** (a) simple rudimentary language-specific stats; (b) a very complicated, insightful VISUAL output exposing bottlenecks — which components/processes can be optimized. Founder: "it is all very complex, so keep asking questions so you get the entire idea."
2. **LLM-consumable quantifiable metrics:** outputs feed an LLM (same or different model) so it can optimize the codebase; thesis: LLMs lack explicit knowledge of writing performant code, quantified metrics fix that.
3. **AI-generated codebase comprehension:** clean visual way to understand AI-generated codebases (sheer volume makes manual validation impossible). Related to 1b.
Audience: primarily devs working with AI-generated code; also human-written codebases.
Languages: long-term many (c++, c, rust, python, js, "tp" [typo — clarify], java, golang); MVP belief: **C, C++, Python** (C/C++ similar; CPython written in C → shared foundations).

## YOUR TASKS (timeboxed — prior run died on timeout)
1. Read existing skeleton files; update `state/session-state.json` (append-only arrays), regenerate `state/handoff/latest.md`, refresh `docs/activeContext.md`. Note the retry in decisions/artifacts.
2. TIMEBOXED landscape research: max ~6 websearch/gh_grep calls total. Cover: complexity-metric tools (lizard, radon), C/C++ static analysis (clang-tidy, cppcheck), platform tools (SonarQube, CodeScene), visualization prior art (CodeCharta, Sourcetrail), and anything LLM-facing/metrics-for-LLM-optimization. Tight ≤10-bullet summary: what exists, Perfpal's gap, what to borrow.
3. Produce the full numbered clarifying-question list for the founder, grouped by theme: product form (CLI/library/service/web?), concrete meaning of "visual output" (interactive web UI? static reports? graph types?), priority metrics per language (C/C++/Python), metric→LLM delivery format (JSON schema/MCP/report), static-only vs runtime/profiling scope, MVP definition + success criteria, licensing/open-source, meaning of "tp".
4. DO NOT commit to git (user approval required).

## RETURN in final message
(1) ≤10-bullet landscape summary; (2) full grouped question list; (3) files updated/created this run; (4) risks/blockers. Keep it structured and compact — do not pad.

---

ASSISTANT:
I'll start by reading the existing skeleton files, then proceed with research and question generation. Let me read everything in parallel first.

---

ASSISTANT:
Now I'll conduct the timeboxed landscape research and update files in parallel. Let me start with web searches and file updates:

---

ASSISTANT:
Now I have all the research I need. Let me update the project files and produce the deliverables.

---

ASSISTANT:
## Perfpal Phase 0 — Research & Clarifying Questions (Retry Complete)

---

### 1. Landscape Summary (≤10 bullets)

- **Complexity metrics (mature)**: Lizard (15+ languages, cognitive complexity, no deps), Radon (Python-specific, MI + Halstead, JSON output), McCabe/Wily/Flake8 — all measure maintainability/complexity, **not performance bottlenecks**.
- **C/C++ static analyzers**: clang-tidy (300+ checks, performance-profiling capable) and cppcheck (fast, multi-threaded, tuning knobs) — focus on correctness/security; performance checks exist but are scattered and not aggregated into optimization guidance.
- **Visualization platforms**: CodeCharta (3D city metaphor, imports SonarQube metrics, web UI), Sourcetrail (interactive source explorer, C/C++/Java/Python) — both visualize *existing* metrics; neither generates performance-specific heuristics.
- **Enterprise platforms**: SonarQube (widely adopted, high false-positive rate, maintainability metrics criticized in peer research), CodeScene (Code Health™ metric, behavioral analysis, 6× more accurate per ICSME 2024 benchmarks) — both are general quality gates, not LLM-optimization pipelines.
- **LLM-facing metrics (emerging)**: DeSlopify (Slop Index 0–100, tree-sitter, LLM-consumable fix instructions), AgentFit (AI-readiness static metrics + live LLM benchmarking), CCG standard (JSON-LD code intelligence for AI agents), adit-code (structural hot spots for AI-edited code) — all focus on *friction/readiness*, not runtime performance optimization.
- **Perfpal's gap**: No existing tool unifies **performance-relevant static heuristics** + **visual bottleneck maps** + **LLM-consumable quantified metrics for optimization** into one pipeline. Existing tools are either analyzer-only, viz-only, or LLM-readiness-only.
- **What to borrow**: Lizard's polyglot metric engine; CodeCharta's shell→web-studio pipeline; CCG's structured JSON-LD layer model; DeSlopify's weighted scoring and `--format llm` pattern; clang-tidy/cppcheck check integration for C/C++.
- **Parser decision pending**: tree-sitter (universal, fast) vs libclang (C/C++ precise, heavy) vs hybrid — major architectural fork requiring founder input on depth vs breadth.
- **Output format gap**: No standard schema for "performance metrics for LLM optimization" exists; CCG is closest but focuses on structural intelligence, not optimization guidance.
- **Risk**: "Performance heuristics" from static analysis are inherently approximate; must be positioned as *optimization candidates* (likely bottlenecks) not *ground truth* — LLM must validate with runtime profiling.

---

### 2. Clarifying Questions for the Founder (Grouped by Theme)

#### A. Product Form & Delivery
1. **What is the product form?** CLI tool? Library (Python/JS/Rust package)? Web service? Desktop app? Multiple forms — e.g., CLI for CI + web UI for exploration?
2. **Who runs it?** The developer locally, integrated into CI/CD, or as a SaaS scanning repos? This affects licensing, architecture, and data privacy.

#### B. "Visual Output" — Concrete Definition
3. **What does "insightful VISUAL output exposing bottlenecks" mean specifically?**
   - Interactive web UI (like CodeCharta's 3D city map)?
   - Static HTML/PDF report with charts?
   - CLI charts (ASCII, terminal graphics)?
   - Graph structures (call graphs, dependency graphs, heatmaps)?
   - All of the above — what's the MVP priority?
4. **What should the visualization show?** Complexity hotspots? Call chains? Memory allocation patterns (static estimates)? Cache locality hints? Data flow? — "Performance" is multidimensional; which dimensions matter most?

#### C. Metrics — What to Measure (C/C++/Python MVP)
5. **For C/C++, which performance-relevant heuristics are highest priority?**
   - Cyclomatic/cognitive complexity?
   - Function length / inline hints?
   - Cache-unfriendly patterns (linked lists, pointer chasing)?
   - Algorithmic complexity estimates (nested loops, recursion depth)?
   - Memory allocation density (malloc/free frequency)?
   - SIMD opportunities missed?
   - Something else?
6. **For Python, same question:** Is it about CPython C-extension opportunities, algorithmic complexity, I/O-bound detection, GIL contention patterns, or something else?
7. **Static-only vs runtime:** The vision says "scans codebases" — is Perfpal strictly static analysis, or should it ingest runtime profiling data (e.g., perf, valgrind, cProfile) to combine static + dynamic insights?

#### D. LLM Interface — Metric Delivery Format
8. **How does the LLM consume the metrics?**
   - JSON file with schema?
   - MCP (Model Context Protocol) tool/server?
   - Plain text report injected into prompt?
   - Structured "optimization instructions" (like DeSlopify's `--format llm`)?
   - Should Perfpal emit *directives* ("refactor function X to reduce complexity from 25 to 10") or *raw data* for the LLM to interpret?
9. **Same LLM or different?** The vision says "outputs feed an LLM (same or different model)" — does Perfpal need to support multiple target LLMs with different context window sizes or preferred formats?

#### E. MVP Scope & Success Criteria
10. **What is the smallest viable Perfpal?** Specifically:
    - One language or all three (C, C++, Python)?
    - One metric or ten?
    - CLI-only or CLI + web UI?
    - What does "working" look like for the MVP — e.g., "scan a repo, produce a JSON report, and an LLM can successfully optimize at least one bottleneck"?
11. **What is the success criteria for the visual output?** "A human can identify the top 3 optimization targets in under 60 seconds"? "An LLM can improve benchmark scores by X%"?

#### F. Licensing & Open Source
12. **What is the intended license?** Open source (MIT/Apache/GPL)? Commercial? Dual-license? This affects dependency choices and contribution strategy.

#### G. Clarifications on Language List
13. **What is "tp"?** You listed "c++, c, rust, python, js, tp, java, golang" — is "tp" TypeScript? Something else?
