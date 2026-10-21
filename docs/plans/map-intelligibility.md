---
plan name: map-intelligibility
plan description: Make the birdseye map legible
plan status: active
---

## Idea
Successor to map-build after the I5 human A5 FAIL ("I can't understand anything"). I5 was the last node (I1–I5; no I6). This plan diagnoses whether the problem is missing data (H1) or visuals that do not do the data justice (H2), using a mandatory back-and-forth with @vision, then redesigns the key views so a maintainer unfamiliar with a repo can name 3 concrete actions from the map alone in under 10 minutes. Visual work has two explicit tiers: answer-first key views (must be legible) and explorer/info-dump stems (allowed but labeled). Every visual change is vision-gated before it is claimed done. The plan also folds in the queued map-quality findings R-pf-098/099/100 (degenerate entry strata, test/literal-bound cost ranking, tiny-scope density) and precedes the roadmap's JS/TS + paper nodes.

## Implementation
- V0 — evidence capture: **pre-register the answer key before looking at any chart.** From source + I5 triage (not from the map), write the expected 3 actions (where / not-touch / profile) for tmux, hyperfine and cJSON. Then screenshot the canonical reading path (`report.html#map` and each stem) at **1440×900, 100% zoom** with Playwright and send the **PNGs** to @vision (never raw SVG text); record per view what it communicates, what it cannot answer, and which maintainer question it serves → docs/evidence/map-vision-diagnosis.md.
- V1 — data-coverage audit + known-answer recall: map each of the 3 maintainer questions (where to look first / what breaks if touched / where time plausibly goes) to the fields actually extracted (findings, function_cost_v2, entry/reachability, resolved call edges, include edges, components, parse health). **Recall experiment:** for every pre-registered action and the 9 I5 GENUINE findings, test whether any key view surfaces it, at what rank, and under what label — data exists but never surfaces → H2; the field needed to express it is absent → H1. Decide per question and list every missing field with a schema additivity note.
- V2 — methodology + spec lock: add the two-tier visual rule (answer-first key views vs labelled explorer stems), the mandatory @vision gate, and plain-language requirement to docs/specs/maintainer-map.md and AGENTS.md. **One primary view, progressive disclosure:** at most 3 answer-first views, one question each; the first screen of `report.html` answers Q1 in plain language; the 8 current stems move under an explicit `Explorer` divider. **One encoding per key view** (severity OR cost OR coupling — never all three); every number carries a unit and a comparison anchor (`× median`, `top 1%`), no bare cost numbers. Define the acceptance view set and remove/demote orphan views.
- V3 — idiom A/B before code: prototype 2–3 candidate idioms per key question (ranked action cards with callout snippets, component DSM/matrix, treemap) **plus a text-first ranked maintainer brief** (3 actions with `path:line`, why, confidence, supporting chart). Run a blind @vision A/B on which answers the question fastest; choose and record the decision and what would change it. Design on tmux/hyperfine/cJSON; keep one **holdout repo** (django or postgres) unseen until V6.
- V4 — implement the chosen key views + data gaps: render the answer-first views, wire the report.md/html drill path, keep the explorer stems as a labelled secondary tier; add any missing field additively (schema note + tests) or downscope the question explicitly. If the brief wins the A/B, it becomes the primary answer view and the charts become its evidence.
- V5 — vision validation loop: each key view gets two fresh @vision reads (blind: no source, no sibling views; read-aloud description first, then strict JSON `{question_answered, action, blockers, confidence}`); iterate until both state the question answered and one concrete action; commit the transcripts as evidence.
- V6 — founder timed eye-test: founder opens the key views on an unfamiliar repo (**including the holdout**) and names 3 actions in under 10 minutes; record verbatim. FAIL re-opens V3 with a distinct idiom. **Stop criterion:** two distinct V3 idioms failing the founder test → stop and escalate to an ADR-0008 amendment (text-first brief or a different primary view); no endless polishing.
- V7 — re-triage the I5 map-quality findings against the new visual/data decisions: R-pf-098 entry strata, R-pf-099 production-path + known-trip ranking, R-pf-100 density denominator; fix or explicitly accept each with the founder.
- V8 — closeout: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `cargo deny check licenses`, feature clippy/tests, golden double-regen byte-identical, 9-repo double-scan, explorer stems remain byte-deterministic; output-contract updated; dual review APPROVED; state/handoff/CHANGELOG committed.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
- evidence-beta-spec
- divergence-study
- maintainer-map
- birdseye-map
- map-intelligibility
<!-- SPECS_END -->