---
plan name: atlas-s4
plan description: Scale scenes and payoff
plan status: done
---

## Idea
Implement Atlas Story S4 against the sealed spec docs/specs/atlas-story.md §6–§9: add directory scenes/aggregation for 1k–10k-file repos, the whole-repo profile queue, scene-aware start-here and scale-aware trust, and prove F1–F7 at a 3,000-file synthetic repo; F8 (founder A2) is deferred and must not be claimed. Core-fed only: three additive, story-only probes in atlas-layout.json, one commit each — P6 purpose (README first prose paragraph with structural fallback), P7 whole-repo KV3 queue (authoritative complete_profile_queue rows + totals), P8 directory scenes (depth-2 partition, production-first ordering, 24-scene / 5-file caps, top files, counts, omitted totals, actions[].scene). layout_version and flow_version stay 1; no schema bump, no new dependency, no network; existing scans stay byte-identical. The sim projects the new blocks from core artifacts (never scrapes HTML), renders a masthead purpose line, scene context on enter beats, a whole-repo profile wording, scene-aware start-here, a compact trust narration and a below-fold SCENES / PROFILE QUEUE / TRUST ACCOUNTING section; unresolved calls get a visible inspector/trust representation for F3; the page stays JS-only in the standalone sim and adds no JS to the static report. A deterministic 3k fixture generator (3,000 scanned files, nested dirs, test/bench share, C function pointer + Rust dyn, loops/I/O, README) plus a scale script measure page size ≤5 MB, first frame <2 s, no horizontal scroll at 1280×800, F2/F3/F4/F5/F6 and the 3k double-scan determinism. Evidence: gate PASS with dirty 0, F1–F7 mapped to runnable checks/artifacts, @vision blind read, 4-lane review panel; no push/merge, no holdout edits, no companion-vs-report decision.

## Implementation
- Build the deterministic 3,000-file synthetic fixture generator (sim/fixtures/gen-3k.py) with nested production dirs, tests/ bench share, a C function-pointer file and a Rust dyn file engineered into the drawn slice, loops/I/O and a README purpose sentence; assert exactly 3,000 scanned source files and reuse a cached /var/tmp fixture.
- Core P6: add src/reporter/atlas_purpose.rs (README discovery/paragraph extraction, 64 KB and 200-char caps, structural fallback) and emit purpose{text,source,kind} story-only in atlas-layout.json; unit + e2e tests; layout_version stays 1 and story-off output is byte-identical.
- Core P7: add src/reporter/atlas_queue.rs projecting the top 12 non-demoted production rows from views::complete_profile_queue with rank/path:line/cost/notes plus production and total counts; emit profile{queue_version,total,production_total,rows}; tests assert equality with the authoritative queue.
- Core P8: add src/reporter/atlas_scenes.rs partitioning source files into directory scenes (depth-2 split, root files scene, 24-scene/5-file caps, production-cost-first ordering, omitted totals) and add actions[].scene + nodes[].scene (enter-beat context); emit scenes{scene_version,caps,totals,rows}; unit tests for partition/caps/cost sums and e2e shape.
- Project purpose/profile/scenes/unresolved-call counts in sim/from-layout.py with strict enum/cap/reconciliation validation; regenerate data-hyperfine.json and data-cjson.json; add data/DOM checks to sim/verify.py.
- Render the S4 UI in sim/index.html, sim/sim.js and sim/sim.css: masthead purpose + summary purpose, enter-beat scene context, whole-repo profile lane, start-here scene/path:line, compact trust narration, below-fold SCENES / PROFILE QUEUE / TRUST ACCOUNTING, unresolved-calls inspector section, window.__sim.ready; responsive wrap with no horizontal scroll at 1280×800.
- Add sim/scale.py + sim/scale.sh and wire them into sim/gate.sh: generate/scan the 3k repo twice, project, measure page bytes ≤5 MB, first frame <2 s to ready, no h-scroll at 1280×800, F2/F3/F4/F5/F6 at 3k, and capture 1440×900 + full-page evidence frames.
- Run the DoD gate (fmt, clippy -D warnings, test --locked, cargo deny, bash sim/gate.sh), record F1–F7 claim→evidence and F8 deferred; dispatch @vision for the S4/F1 blind read and fix any blockers with a re-read.
- Dispatch the 4-lane review panel (R1 correctness/determinism, R2 contract/security, Q1 verbatim scope, Q2 acceptance/evidence), reconcile findings, fix blockers, rerun affected lanes plus one fresh confirmation, then commit docs/state/evidence and the final GATE transcript with dirty 0 and verdict=PASS.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- maintainer-map
- evidence-beta-spec
- divergence-study
<!-- SPECS_END -->