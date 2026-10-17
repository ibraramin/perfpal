---
plan name: map-research
plan description: Exhaustive map research sessions
plan status: active
---

## Idea
Four exhaustive read-only research sessions feeding the maintainer birdseye map, each with a named doc deliverable so nothing evaporates: R1 layout algorithms for static codebases (Sugiyama limits, force-directed ceiling, treemap+edge-bundling, static flame graphs, code-city, DSM/matrix) plus deterministic-layout constraints; R2 precedent teardown (Sourcetrail ego-graph, code2flow honest estimates, NDepend/Structure101/Lattix, CodeSee/Gource, madge/depcruise) judging maintainer-helpful vs eye-candy with sources; R3 cost+uncertainty visualization and maintainer information-needs literature; R4 constraint-mapping decision matrix against Perfpal non-negotiables (static-only, no-build, musl, byte-identical, offline SVG/HTML, top-50 caps) ending in an ADR that gates all implementation.

## Implementation
- R1 algorithms: DONE (evidence-backed redo, commit `8b26c63`, LL-012). Do not re-research; R2–R4 build on `docs/research/map-algorithms.md`.
- R2 precedents: teardown 6+ tools, verdict helpful-vs-eyecandy with FETCHED sources -> docs/research/map-precedents.md. Per tool (Sourcetrail, code2flow, NDepend/Structure101/Lattix, CodeSee/Gource, madge/depcruise, +1 sourced extra only with fetched source): open repo README + user docs + archive/shutdown status (dead tools: record last commit/date); `gh_grep` real-world configs in the wild; every verdict quotes its backing section with [URL + access date].
- R3 honesty: cost-without-lying idioms, cap/omission markers, maintainer-needs literature -> docs/research/map-honesty.md. Literature from named venues (ICSE/CHI/TOSEM/CSCW developer-information-needs studies — fetched, never cited-blind); uncertainty idioms (CI bands, cap markers, provenance footnotes) with fetched examples; cap/omission marker patterns with doc quotes.
- R4 decision: matrix vs non-negotiables, recommended idiom per map level + rejected alternatives -> ADR + docs/research/map-decision.md. Every matrix cell cites R1/R2/R3 sections; rejected alternatives carry fetched reasons; no new external claims (all evidence already fetched in R1–R3).
- Gate review: reviewer + requirements-reviewer verdicts on R1-R4 packet before any I-session starts. STRICT BRIEF: any external claim without a fetched [URL + date] citation is BLOCKING.

## Evidence discipline (LL-012 — binding on R2–R4, maximum priority)
This is maximum-priority exhaustive work: scrape every inch of the open web. Procedure per lane, no exceptions: (1) OPEN each source with webfetch/websearch (`save_binary` for PDFs); search-result snippets alone do not support claims. (2) QUOTE the backing section for every factual claim. (3) CITE inline as [full URL + access date] on the claiming bullet — shorthand re-cites must resolve to a full dated cite in the same section. (4) DEAD ENDS: every unopenable page (paywall, 403, fetch-limit, dead link) is listed and supports NOTHING; gaps are explicit, never silent. (5) Memory citations are FORBIDDEN. Minimums per session: 3+ parallel research lanes; R2 ≥3 fetched sources per tool; R3 ≥5 fetched venue papers/examples; R4 zero new external claims. Enforcement is defense-in-depth: this block is the backstop — every dispatch prompt MUST also paste the fetch→quote→cite procedure verbatim, and any review caveat on evidence ("not fetched", "not verified") is a BLOCKING finding, never a note.

## Required Specs
<!-- SPECS_START -->
- maintainer-map
- evidence-beta-spec
- perfpal-core-spec
- divergence-study
<!-- SPECS_END -->