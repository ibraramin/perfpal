---
plan name: agent-report
plan description: LLM artifact contract and renderer
plan status: active
---

## Idea
Implement Track C, the LLM-facing depth layer: always-emitted `agent-report.md` (bounded, token-efficient) + `agent-actions.json` (complete machine action list), both pure deterministic renderers of the final `MetricsDocument`/`metrics.json`. No LLM calls, no network, no JS, no schema change, no new dependencies, no CLI flag. The contract is `docs/specs/agent-report.md` (contract_version 1, session C1): actions ranked by the shipped KV1 key (production-first, severity, confidence, containing-function cost_v2 desc, finding_id asc; -ARCH-002 excluded and counted); profile queue by the shipped KV3 key; cost anchors reused verbatim from `views::ScaleOrdering`; `why`/`evidence` verbatim from the finding; curated fix hints from a C-owned 67-rule table with a completeness lock; data-quality block with parse health, unresolved includes, structural edge mix, cost-scale mix, unknown-trip count, excluded aggregates and named caps; clock provenance deliberately omitted so repeated runs of the same command are byte-identical with no normalization (`root_path` is verbatim CLI provenance). `agent-report.md` keeps a fixed seven-section skeleton (header, how-to-use/rank key, data quality, ranked actions top 25, per-rule rollup, profile queue top 25, limits) with named stubs in every scan state and an 80 KB hard budget; the JSON is the complete action list with a 100-row profile cap named in-file. C owns `src/reporter/agent_report.rs`, its tests, `docs/specs/agent-report.md`, `docs/plans/agent-report.md`, `docs/evidence/agent-report/`; A adds the human-page link at step 12 and owns the output-contract row; no edits to A/D surfaces, main state, CHANGELOG or schema. C3 also records a manual downstream eval (top-1/top-3 hit, hallucination, time-to-fix) because that is the north-star's agent-loop metric.

## Implementation
- C2 (module + unit tests) — **DONE 2026-10-24**: `src/reporter/agent_report.rs` (views + 67 hints + JSON + MD + 24 unit tests); visibility-only `mod.rs`; panel R1/R2/R3/R5 + Q1/Q2 APPROVED; no wiring. C2 CLI emission and the C3 suite remain deferred until A step 12.
- C1 — contract lock (DONE 2026-10-24): write `docs/specs/agent-report.md` (envelope, action/function/profile/data_quality fields, KV1/KV3 rank keys, 67 fix hints, formatting/units rules, fixed MD skeleton, determinism + size bounds, state matrix, eval protocol, acceptance), this plan, `state/tracks/agent-report/handoff.md`, and the reproducible sample under `docs/evidence/agent-report/sample/` derived from `tests/golden/report-mixed/metrics.json`. No production code; the only tooling is the founder-ratified, evidence-only reproducer (spec §2 D8).
- C2 — action + profile view model in `src/reporter/agent_report.rs`: reuse `views::complete_actions` and `views::complete_profile_queue` (no second ranking), build the `function` sub-object and the data-quality aggregates, and add the `AGENT_FIX_HINTS` table with a unit test asserting it covers exactly the 67 `HEURISTIC_CATALOG_V1` ids (unknown → null).
- C2 — renderers: `agent-actions.json` envelope + the §6.2 profile-row objects + `agent-report.md` fixed seven-section skeleton with the 25/25 caps, `showing top N of M` lines, named stubs, sorted evidence keys, the §9.9 escaping (single-line, backtick/HTML/table safety), `null` handling for absent `parse_health`/`loc_total`, integral `cost` serialization (`15`, never `15.0`), and the byte budget. Unit tests: rank parity with the shipped views, anchor byte-parity, JSON completeness invariants, profile-row schema, text-safety fixture, size ≤ 80 KB on every committed golden fixture + the §12 synthetic at-envelope fixture (25 actions + 67 rollup rules + 25 profile rows), forbidden time claims only in C-authored strings, and double-render byte equality.
- C2 wiring (only after A step 12): `src/cli/mod.rs` emits both artifacts via `Artifact::text`; expose the shipped `finding_anchor_assignments` as `pub(crate)` (or duplicate with an equality test) so `detail_anchor` matches `findings-detail.html`; update the artifact-set assertions that currently count the eight fixed files; no schema, no dependency, no CHANGELOG (A adds it at merge).
- C3 — integration suite `tests/agent_report.rs`: double run of the same command on the same fixture tree → byte-identical `agent-report.md` + `agent-actions.json` with no normalization; completeness (`actions.len() == complete_action_count`, profile cap arithmetic); the state matrix renders the fixed skeleton with stubs; no schema/Cargo/deny changes; no `<script>`/network; `detail_anchor` resolves in `findings-detail.html`.
- C3 — evidence + eval: regenerate `docs/evidence/agent-report/sample/*` from the Rust renderer and **byte-diff** against the committed C1 sample (empty diff, asserted by a named test); write `docs/evidence/agent-report/README.md` (contract, sample provenance, determinism hashes); run the §13 downstream eval (cJSON/hyperfine/bat, N=6, thresholds) and record transcripts under `docs/evidence/agent-report/eval/`, or record the founder's explicit deferral.
- C3 — A link slot: verify A's step-12 human-page link resolves to `agent-report.md` (A owns the link line; C only verifies and reports), then run the full DoD, the parallel review panel (2 reviewers + 2 requirements-reviewers), update only the C track handoff, and request founder approval to merge after A step 12. No push.

## Definition of Done

### C1 (contract — this session; docs only, no Cargo commands)

All commands run from the repo root and must exit 0:

1. `test -f docs/specs/agent-report.md && test -f docs/plans/agent-report.md && test -f state/tracks/agent-report/handoff.md && test -f docs/evidence/agent-report/sample/agent-report.sample.md && test -f docs/evidence/agent-report/sample/agent-actions.sample.json`
2. `python3 docs/evidence/agent-report/sample/derive_sample.py tests/golden/report-mixed/metrics.json "$(mktemp -d)" docs/specs/agent-report.md` then `diff` both files against `docs/evidence/agent-report/sample/` — empty diffs (the committed sample is reproducible).
3. `jq -e '(.actions|length)==.complete_action_count and (.profile_queue|length)==([.profile_queue_cap,.complete_profile_count]|min) and ([.actions[].rank]==[range(1;(.complete_action_count+1))]) and ([.actions[]|select(.fix_hint==null)]|length)==0 and (.scan|has("started_at")|not) and (.scan|has("duration_ms")|not)' docs/evidence/agent-report/sample/agent-actions.sample.json`
4. `bash docs/evidence/agent-report/c1-gate.sh` — exit 0. The committed gate script asserts: deliverables, sample reproduction, the JSON invariants, **set equality** between the 67 spec hint rows and `HEURISTIC_CATALOG_V1`, the worked example vs action 1, the seven headings, the 64 KB size assertion, and `git status` cleanliness for `src/`, `Cargo.*`, `deny.toml`, `docs/schema/`.
5. `test -z "$(git status --short src/ Cargo.toml Cargo.lock deny.toml docs/schema/)"`
6. `grep -qx 'GATE_EXIT=0' docs/evidence/agent-report/c1-gate.txt` — the transcript is the script's own output, so the exit line is mechanically derived, not hand-written.

### C2–C3 (implementation; run after the post-A-step-12 wiring)

All commands run from the repo root and must exit 0 (the task is NOT done otherwise):

7. `cargo fmt --all -- --check`
8. `cargo clippy --locked --all-targets -- -D warnings`
9. `cargo test --locked`
10. `cargo deny check licenses`
11. `test "$(cargo test --locked --lib agent_report -- --list | grep -c ': test$')" -ge 10` — enforces a real test count (a zero-match filter exits non-zero).
12. `cargo test --locked --lib agent_report` (unit: rank parity, anchors, hint-set lock, profile schema, text safety, size, stubs, forbidden strings, double render)
13. `test "$(cargo test --locked --test agent_report -- --list | grep -c ': test$')" -ge 8 && cargo test --locked --test agent_report` (integration: double-run byte equality, completeness arithmetic, state-matrix skeleton, `detail_anchor` resolution, no script/network, emitted artifact set)
14. `cargo test --locked --test agent_report -- --list | grep -qx 'agent_report_sample_matches_committed_contract: test' && cargo test --locked --test agent_report -- agent_report_sample_matches_committed_contract` (empty diff vs the committed sample; the guard fails on a rename)
15. `cargo test --locked --test agent_report -- --list | grep -qx 'agent_report_size_budget_golden_fixtures: test' && cargo test --locked --test agent_report -- agent_report_size_budget_golden_fixtures` (≤ 80 KB on the committed goldens + the §12 synthetic; the external `perfpal-test` scans run only with `PERFPAL_EXTERNAL_SCANS=1`)
16. `cargo test --locked --test agent_report -- --list | grep -qx 'agent_report_human_page_links_to_agent_report: test' && cargo test --locked --test agent_report -- agent_report_human_page_links_to_agent_report` (A's step-12 link slot resolves; A owns the link line)
17. `test -f docs/evidence/agent-report/eval/README.md || grep -q 'eval deferred by founder' state/tracks/agent-report/handoff.md` (eval recorded or deferral recorded)
18. `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings`
19. `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang`
20. `test -z "$(git status --short src/schema/ docs/schema/ Cargo.toml Cargo.lock deny.toml)"`
21. `@reviewer` and `@requirements-reviewer` both APPROVED on the final tree (parallel panel: 2 + 2 lanes), zero blocking findings.

### Track gate (before any merge to `main`)

22. Founder approval recorded in the track handoff; `git merge` only after A step 12 is committed.
23. `docs/tracks/agent-report.md` status block updated; `state/tracks/agent-report/handoff.md` regenerated with the claim→evidence table.


## Required Specs
<!-- SPECS_START -->
- divergence-study
- evidence-beta-spec
- maintainer-map
- perfpal-core-spec
- agent-report
<!-- SPECS_END -->