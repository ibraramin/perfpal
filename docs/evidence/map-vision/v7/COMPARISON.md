# F vs G — prototype comparison (blind-read evidence)

**Status:** prototype A/B input; the founder timed test decides. Design repos only; holdout vaulted.
**Evidence:** `f/reads.json` (6 valid), `g/reads.json` (6 valid), `f/manifest.json`, `g/manifest.json`.
**Animation:** both CSS/SMIL declarative, no JS, pause + reduced-motion/print final frames.

## What each candidate is

- **F — weight map:** component treemap (area = static cost, one cost ramp, production-first, test/bench
  hatched), KV1 brief strip, Q2 1-hop drill.
- **G — entry-flow map:** entrypoint (production `main`; library root otherwise) at depth 0, file
  nodes by resolved call edges in BFS rings, per-file key-function cards, component hulls, unreached
  tail, numbered path trace, Q2 reverse drill.

## Blind reads at a glance

| Repo | F action (both reads) | F blockers | G action (both reads) | G blockers |
|---|---|---|---|---|
| **cJSON** | `cJSON_Utils.c:1188` — hoist `sprintf` out of loop (KV1 brief) | **Treemap hijacked by tests:** `tests/unity/test/tests` = 93.5% of area; production = 0.2%, unreadable slivers; answer comes only from the brief | entry `cJSON.c:253 cJSON_Delete` (library root); **no forward path** — 1 reachable node, 5 callers greyed | **Dead-end flow** (no production main); LOW-confidence edges; no downstream path |
| **hyperfine** | `src/command.rs:148` — hoist per-iteration allocations (HOT/HIGH) | truncated remediation hint; no enclosing function name | `src/command.rs` HOTTEST (611 units, 87× median); action `build`/`spawn` | none (one read chose a different top function inside the same file) |
| **tmux** | `compat/getline.c:85` — realloc in loop (HOT/HIGH) | small-cell labels truncated; hatched `other (107)` = 16.7%; legend swatches unlabeled | `format.c` HOTTEST (6,030 units); action `format_replace`/`format_expand1` | LOW-confidence edges; depths 4–5 collapsed; 20/106 files drawn |

Both candidates pass the A1-style bar on hyperfine/tmux (question + one concrete action). **Both fail
cJSON, for different reasons:** F shows a test-dominated weight map; G shows a one-node dead-end
flow. The brief rescues F on cJSON; nothing rescues G's forward flow on a library.

## Criterion matrix

| Criterion | F | G | Edge |
|---|---|---|---|
| **Q1 — where first?** | Brief headline + top card; the map shows mass, not the start | Entry label + HOTTEST node + card; the start is *in* the map | F clearer text; G more map-native |
| **Q2 — what breaks?** | 1-hop drill from the hottest component; target may be a test component | Reverse drill from the entry/dependents (`cJSON.h`, `tmux.h`); target is a production hub | G's default target is more useful; drills not blind-read yet |
| **Q3 — where time?** | Whole-map area = cost; strongest "where is the mass" | Node/card cost along the execution path; strongest "which stage carries cost" | Split: F for mass, G for path |
| **Whole-codebase grasp** | One object; works for applications; **fails** when tests dominate | One object; works for applications; **fails** for libraries (no main) | Repo-dependent |
| **Data honesty** | Test dominance disclosed; units/anchors present | LOW-confidence call edges disclosed; caps/aggregates printed | Tie |
| **Animation value** | Rank reveal + hot pulse; decorative relative to the brief | BFS progression + path trace; the animation *is* the story | G |
| **Robustness** | Degrades to "brief + test wall" on cJSON | Degrades to "one-node + unreached" on cJSON | F slightly (brief still answers) |
| **Application fit** | Strong brief; map secondary | Strong flow map + brief | G |
| **Library/test-heavy fit** | Brief carries the answer; map weak | No forward story | F |

## Hypothesis (to be tested, not assumed)

- **G is the core graph** (founder decision, 2026-10-22): the program story is the object, and the
  animation is meaningful. It is closest to the founder's stated idea.
- **F is not a fallback and is not discarded.** F-derived components (weight/treemap mass,
  production-first area, component cells) are **candidate supporting elements** around the G core,
  included only if the developer-needs research justifies them. No mode switch.
- The combined feature still has to prove the G core on the holdout; the A2 timed test is the gate.
- **Biggest risk:** G's static call graph is LOW-confidence and can fragment (cJSON proves it).
  **Second risk:** F's area policy lets test cost hijack the hero (cJSON proves it). Neither is
  polished away; both are recorded.

## Founder timed A/B (not run)

- The proposed protocol (10 minutes per candidate, 3 actions, understandable yes/no) was **not
  run**; the founder made the call from the prototypes + the 12/12 blind reads.
- The A2 timed test now moves to the **shipped combined feature** on the holdout.
- Next: research/planning (`map-shipped`) — developer needs, metrics standardization, data gaps,
  full-page combined layout, component-inclusion rule, naming, spec/ADR, build plan.
- The holdout (django/postgres) stays vaulted until the shipped feature is ready.
