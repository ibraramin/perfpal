# H3 artifact matrix and consumer checks — 2026-10-28 @ `8544706`

**Scope.** One role per emitted artifact; no artifact without a named consumer; no two human
artifacts carry the same complete list. The **default fixed set is 9 files** plus the two Atlas drill
pages, `charts/graph.dot`, and the chart set. `atlas-layout.json` is the **opt-in** story artifact
(`--story` only) and is not part of the default set. `findings-detail.md` is removed (no consumer) and
`report.md` Appendix C is removed in favour of the §4 complete-list pointer.

## Binding matrix (default rows mirror `tests/artifact_matrix.rs`; the opt-in row does not)

| Artifact | Audience | Consumer / action | Unique role | Cap | Canonical? |
|---|---|---|---|---|---|
| `metrics.json` | tools / LLM / downstream | parsed + validated by tooling | the only data source | none | **yes** |
| `metrics.schema.v1.8.json` | tooling | offline JSON-schema validation | schema mirror discovered by `schema_version` → filename | n/a | mirror |
| `report.md` | human | terminal / Markdown reading, follow links | bounded narrative + Atlas summary; no full findings appendix | summary caps named | no |
| `report.html` | human | browser, map-first; pan/zoom on the legacy linked figures | Atlas map + story + bounded findings; no filter UI | summary caps named | no |
| `findings-detail.html` | human | drill from `report.md` §4 / `#ranked-actions`; complete static list (no filter) | **canonical complete human detail**: ranked actions + every finding | none | human detail |
| `profile-queue.md` | human | terminal / diffing | complete KV3 cost-ranked function list | none | KV3 detail (md) |
| `profile-queue.html` | human | browser drill | complete KV3 cost-ranked function list | none | KV3 detail (html) |
| `atlas-weight.html` | human | drill from the weight chart | complete weight detail | complete per metric | no |
| `atlas-q2.html` | human | drill from the blast chart | complete reverse/blast detail | complete per metric | no |
| `agent-report.md` | LLM / agent | triage without the full report | bounded digest: top 25 actions, rule rollup, top 25 profile rows | 25 / 25 / 100 | no |
| `agent-actions.json` | LLM / agent | machine action execution | **complete** action list + profile top 100 (`profile_queue_cap`) | actions uncapped; profile 100 | complete actions |
| `charts/graph.dot` | tooling | Graphviz / graph tools | deterministic graph export | n/a | graph export |
| `charts/*.svg` | human / diff | browser / visual diff | overview + Atlas map charts | top-50 / named | chart data |
| `charts/component-*.svg` | human | per-component drill | capped component detail page | top-50 | chart data |
| `atlas-layout.json` (opt-in, `--story`) | tooling / story consumer | `sim/` reference player + external story consumers; overlay/project the run story | story geometry (canvas, flow, purpose, profile, scenes) | `layout_version` 1 | no |

Uniqueness owners (one per content class and format): complete findings → `findings-detail.html`;
complete actions → `agent-actions.json`; complete profile → `profile-queue.md` + `profile-queue.html`
(md/html pair); canonical metrics → `metrics.json`; digest → `agent-report.md`; schema mirror →
`metrics.schema.v1.8.json`; graph export → `charts/graph.dot`; story geometry → `atlas-layout.json`
(opt-in).

## Decisions

- **`findings-detail.md` dropped.** No code read it; the only references were tests, link text and
  docs. It duplicated the HTML detail (the canonical anchor target) and the ranked-action rows in
  `agent-actions.json`. The 10-file set becomes 9 (ADR-0015).
- **`report.md` Appendix C removed.** §4 now ends with
  `All <N> of <M> findings: [findings-detail.html](findings-detail.html) — complete ranked action
  list and per-finding detail (<R> rules).` Section order: 1..5, Appendix A, Appendix B.
- **`agent-actions.json` stays complete**, `profile-queue.*` stays the distinct KV3 list, the schema
  copy stays (offline validation + discovery), and the 25-action / 25-profile caps are unchanged.
- **Dead click-to-filter UI removed.** After the C-split, `report.html` has no per-finding rows and
  `findings-detail.html` has no file/component rows, so the filter had nothing to act on. The
  `#filter-bar` / `filter-label` / `filter-clear` controls and the filter half of `REPORT_SCRIPT`
  are removed; rows keep `data-path` / `data-prefix` only as stable keys. `report.html` keeps
  pan/zoom + story (3 scripts); `findings-detail.html`, `profile-queue.html` and the Atlas drills are
  script-free.
- **Cap disclosure:** `report.md` §4 prints `N of M`; `agent-report.md` prints
  `showing the top 25 of N actions` / `showing the top 25 of N functions` when capped;
  `agent-actions.json.data_quality.caps` names `actions_detail` and `profile_inline`.

## Evidence

| Claim | Evidence | Result |
|---|---|---|
| Matrix matches the default emitted set exactly (`atlas-layout.json` excluded) | `cargo test --locked --test artifact_matrix` → `artifact-matrix.txt` | 6/6 passed |
| Opt-in story geometry is emitted only with `--story` | `story_geometry_is_opt_in_only` | passed |
| One owner per class/format | `one_owner_per_uniqueness_class_and_format` | passed |
| Caps are disclosed | `caps_are_disclosed_in_every_consumer` | passed |
| Every relative link resolves; no stale `.md` detail reference | `every_relative_link_resolves_and_no_md_detail_reference_survives` | passed |
| Invalid `--report-width` exits 1 end-to-end | `invalid_report_width_exits_one` | passed |
| Pipeline emits the 9-file set + drills + charts | full gate at `8544706` → `gate.txt` | 2,250 passed (1,124 default + 1,126 semantic) / 0 failed |

Consumer/link coverage: `report.md`, `report.html`, `findings-detail.html`, `profile-queue.md/html`,
`agent-report.md`, `atlas-weight.html`, `atlas-q2.html` → every relative target exists and every
`#fragment` resolves to an anchor/id in the target page. `findings-detail.md` appears in no emitted
text artifact.
