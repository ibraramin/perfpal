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
You are bootstrapping a BRAND-NEW, multi-month project called **Perfpal** located at `/home/ibrar/Perfpal` (directory exists, currently empty or near-empty, NOT yet a git repo). Today is August 2026. Your job is Phase 0 Discovery & Interrogation Protocol + state skeleton creation. This is RESEARCH AND PLANNING ONLY — write ZERO implementation code.

## PROJECT VISION (from the founder, preserve intent faithfully)

A tool that scans through a codebase and:

1. **Performance-based heuristics:**
   a. Simple rudimentary stats based on the language (e.g., complexity counts, structural metrics).
   b. A very complicated and insightful VISUAL output to see the bottlenecks of the codebase — i.e., which components/processes can be optimized. (Founder stresses: "it is all very complex, so keep asking questions so you get the entire idea" — your question list must dig deep here.)

2. **LLM-consumable quantifiable metrics:** These outputs help human devs, but MORE importantly they should be feedable to an LLM (same or different model) so it can optimize the codebase easily because it has quantifiable metrics. Founder thesis: LLMs generally lack explicit knowledge of how to write performant/optimized code, so quantified metrics will help enormously.

3. **AI-generated codebase comprehension:** A clean and visual approach to easily understand an AI-generated codebase (not necessarily AI-generated, but due to the sheer volume of code AI generates every session it's near-impossible to validate manually). Related to feature 1b.

**Target audience:** Primarily people working with AI-generated code; also usable on classic human-written codebases.

**Language support:** Long-term goal is MANY languages (founder listed: c++, c, rust, python, js, "tp" [likely typo — probably TS or PHP, clarify], java, golang). MVP belief: **C, C++, and Python** are easiest to start (C/C++ very similar; CPython interpreter is written in C → three languages sharing common foundations).

## YOUR TASKS

1. **Research the landscape** (websearch / gh_grep): existing static analyzers, complexity-metric tools, profiling-adjacent tooling, and codebase-visualization tools (e.g., lizard, cppcheck, clang-tidy, radon, SonarQube, CodeScene, CodeCharta, sourcetrail, etc.). Summarize: what exists, what gap Perfpal fills (esp. LLM-facing machine-readable perf metrics + AI-code comprehension), and prior art worth borrowing from. Keep it a tight summary, not a dump.

2. **Create the state skeleton** in `/home/ibrar/Perfpal`:
   - `state/session-state.json` — goal, current_phase="phase0-discovery", decisions[] (append-only), open_risks[], blocked_by[], next_actions[], artifacts[], last_updated.
   - `state/handoff/latest.md` — objective, phase, decisions, risks, blockers, next actions, uncommitted files.
   - `docs/activeContext.md` — lean rolling memory.
   - `docs/specs/`, `docs/plans/`, `docs/adr/` directories (with .gitkeep if empty).
   - `CHANGELOG.md` — Keep-a-Changelog format with `## [Unreleased]`.
   - You MAY run `git init`, but DO NOT commit anything (commits require explicit user approval).

3. **Produce a thorough list of clarifying questions for the founder.** The founder explicitly asked to be interrogated until the full idea is captured. Cover at minimum: product form (CLI/library/service/web app?), what "visual output" concretely means (graphs? flame-graph-like views? dependency/perf heatmaps? interactive web UI vs static reports?), which metrics matter most per language (C/C++/Python), how metrics get fed to LLMs (JSON schema? MCP tool? report format?), static-only vs runtime/profiling integration scope, MVP definition and success criteria, licensing/open-source intent, and the meaning of "tp" in the language list.

## HARD CONSTRAINTS
- Zero implementation code. Research + skeleton + questions only.
- Do not commit to git.
- Return in your final message: (1) ≤10-bullet landscape summary, (2) full numbered question list grouped by theme, (3) exact list of files/dirs created, (4) any risks/blockers spotted.
</conversation_update>
