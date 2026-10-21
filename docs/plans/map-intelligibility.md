---
plan name: map-intelligibility
plan description: Make the birdseye map legible
plan status: active
---

## Idea
Successor to map-build after the I5 human A5 FAIL ("I can't understand anything"). I5 was the last node (I1–I5; no I6). This plan diagnoses whether the problem is missing data (H1) or visuals that do not do the data justice (H2), using a mandatory back-and-forth with @vision, then redesigns the key views so a maintainer unfamiliar with a repo can name 3 concrete actions from the map alone in under 10 minutes. Visual work has two explicit tiers: answer-first key views (must be legible) and explorer/info-dump stems (allowed but labeled). Every visual change is vision-gated before it is claimed done. The plan also folds in the queued map-quality findings R-pf-098/099/100 (degenerate entry strata, test/literal-bound cost ranking, tiny-scope density) and precedes the roadmap's JS/TS + paper nodes.

## Implementation
- V0 — evidence capture: run @vision over all 8 overview stems for tmux, hyperfine and cJSON (fresh read, no source); record per view what it communicates, what it cannot answer, and which of the 3 maintainer questions it serves; append the transcripts to docs/evidence/map-vision-diagnosis.md.
- V1 — data-coverage audit: map each of the 3 maintainer questions (where to look first / what breaks if touched / where time plausibly goes) to the fields actually extracted (findings, function_cost_v2, entry/reachability, resolved call edges, include edges, components, parse health); decide H1 vs H2 per question and list every missing field with a schema additivity note.
- V2 — methodology + spec lock: add the two-tier visual rule (answer-first key views vs labelled explorer stems), the mandatory @vision gate, and plain-language requirement to docs/specs/maintainer-map.md and AGENTS.md; define the acceptance view set and remove/demote orphan views.
- V3 — idiom A/B before code: prototype 2–3 candidate idioms per key question (e.g. ranked action cards with callout snippets vs component DSM/matrix vs treemap) as static mockups; run a blind @vision A/B on which answers the question fastest; choose and record the decision (and what would change it).
- V4 — implement the chosen key views + data gaps: render the answer-first views, wire the report.md/html drill path, keep the explorer stems as a labelled secondary tier; add any missing field additively (schema note + tests) or downscope the question explicitly.
- V5 — vision validation loop: each key view gets a fresh @vision read (view only, no source, no sibling views); iterate until it states the question answered and one concrete action; commit the transcripts as evidence.
- V6 — founder timed eye-test: founder opens the key views on an unfamiliar repo and names 3 actions in under 10 minutes; record verbatim. FAIL re-opens V3; PASS plus dual review clears the node.
- V7 — re-triage the I5 map-quality findings against the new visual/data decisions: R-pf-098 entry strata, R-pf-099 production-path + known-trip ranking, R-pf-100 density denominator; fix or explicitly accept each with the founder.
- V8 — closeout: determinism double-scan, full gate (fmt/clippy/test/deny + feature), goldens regenerated, output-contract updated, dual review APPROVED, state/handoff/CHANGELOG committed.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- evidence-beta-spec
- divergence-study
- maintainer-map
- birdseye-map
- map-intelligibility
<!-- SPECS_END -->