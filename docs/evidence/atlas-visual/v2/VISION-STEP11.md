# Vision — step 11 detail rail (step-13 proxy, not the founder gate)

**Verdict: PASS** for both 1440×900 renders (`atlas-v2-step11-{hyperfine,cJSON}-final.png`).

Reads: (1) initial render, (2) post-fix re-read, (3) targeted re-check of the collision-aware canvas
labels. The render was judged graph-first, with the rail reading as G-style file cards rather than a
metrics dashboard.

## Fixed after the first read

- **Canvas identity:** path mid-elision (`src/co…and.rs`, three `src/tim…` nodes) → file-name labels
  at 868px, with collision-aware parent segments (`export/mod.rs`, `time/mod.rs`,
  `quantity/mod.rs`). No duplicate canvas label remains; the full path stays in the node `<title>`.
- **Rail identity:** basenames (`mod.rs`) → path labels (`src/quantity/statistics.rs`), so same-named
  files are distinguishable without leaving the page.
- **Anonymous functions:** bare `:line` → `(anonymous):line`.
- **Footer:** the two-line wrap no longer splits `33/238 unresolved includes`; each metric phrase is
  a single `nowrap` span.
- **Roles:** ENTRY / PRODUCTION / UNREACHED badges are colour-coded (with text), distinguishable at a
  glance.

## Residual (deferred to step 13 / A2, disclosed)

- Two 16-char basenames still elide by one character in the graph at 11.5px (`executor_tests.rs`,
  `macos_counters.rs`); the rail and `<title>` carry the full name.
- Some rail function rows wrap mid-identifier at the card edge (full name is preserved, not clipped).
- Edge labels still touch dashed group-box borders (canvas chrome; unchanged by step 11).
- The sparse cJSON state (2 of 81 files reachable) leaves a large empty graph region while the rail
  is full; graph-first still holds but the sparse composition is a step-13 call.
