# Handoff — Latest

## Objective
I4 (map-build) DONE: findings now ride as annotations on the map — severity×confidence
dots on five stems, a new `layer-findings.svg` severity map (8th overview stem), and a
bounded `report.html` drill path; `findings-detail.html/.md` stay byte-identical.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1 ✅ I2 ✅ I3 ✅ I4 ✅ (`f56d072`);
I5 proof + demo next. Paper + JS/TS parked behind the map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, with findings as map
  annotations (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; I4 closes the "findings as a list to cross-reference by hand"
  gap in the map.
- **How this session serves the north-star:** "where do I look first" gains a
  severity-weighted findings map plus severity×confidence dots on every map node, and the
  report header drills header → map → component/file → detail anchors.
- **Drift flag: NONE.** Build sequence per ADR-0008.

## This session — I4 burst result
- **Shared annotation model** (`src/charts/mod.rs`): `FindingIndex`/`FindingDot`
  (per-file + exact-`parent_dir` per-component roll-up) built ONCE in `charts::render`
  from `doc.findings`; `BTreeMap` only (deterministic). Dot = worst-severity fill
  (hot/warn/info) + **lowest-confidence ring** (LOW 0.6 / MEDIUM 1.2 / HIGH 2.2 — the
  weakest link, so one LOW candidate never reads as thick HIGH), count numeral when
  total ≥ 2, full counts in `<title>`, one pinned legend line per chart. No dot is ever
  a bare severity colour.
- **Dots wired into** `layer-workspace` (component roll-up), `layer-file`, `layer-entry`,
  `layer-bands`, and per-component pages (file paths). `include-graph.svg` stays
  legacy/undotted and byte-identical.
- **New 8th overview stem `layer-findings.svg`** (`render_layer_findings`): include-graph
  nodes ∪ finding paths, same file-DAG pipeline (cycle-break → topo layers → barycenter),
  top-50 by `(3*hot + 2*warn + 1*info desc, path asc)`, weighted heat + dots + single
  hottest outline, honest `showing top S of M` / `…+N more`, zero-findings stub. Emitted
  after `layer-bands`, before component pages.
- **report.html drill path:** breadcrumb `Map → Components → Files → Findings detail`;
  new `#map` section after nav / before `#architecture` with the workspace figure, the
  annotation legend, and a top-10 component drill table (component-page link,
  severity/confidence summary, ≤3 anchors into `findings-detail.html` using the SAME
  `finding_anchor_assignments` map); `#files` rows gain an emitted component-page link;
  `report.html` still carries ZERO per-finding rows.
- **Honesty fix from review (B1):** rev-1 caught that the first implementation selected
  the HIGHEST confidence present for the ring, overstating certainty; fixed to the
  approved plan's lowest-confidence-present rule and re-pinned in code, legend, test
  (three precedence cases), docs, and goldens.
- **Registry/docs:** `KNOWN_OVERVIEW_STEMS` + golden artifact list (14→15) + caption
  registry + `docs/report/output-contract.md` + `docs/architecture/modules.md` + CHANGELOG;
  component pages now label their cost ramp. `findings-detail` untouched.

## Verification
| Claim | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| tests | `cargo test --locked` | **50 suites / 687 passed / 0 failed** |
| licenses | `cargo deny check licenses` | `licenses ok` (exit 0) |
| determinism / locks | `cargo test --locked --test p4_determinism --test pipeline_e2e --test i4_annotations --test p6_hardening --test charts_render --test i3_map_layer --test i2_cost_v2` | 7 suites / 64 passed |
| detail byte-identity | `git status --porcelain -- 'tests/golden/*/findings-detail.*' 'tests/golden/*/charts/include-graph.svg' 'tests/golden/*/metrics.json' 'tests/golden/*/charts/graph.dot'` | **empty** |
| new stem goldens | `git status --porcelain -- 'tests/golden/*/charts/layer-findings.svg' \| wc -l` | 4 (one per golden tree) |
| B1 evidence | code + goldens | 0.6 for any LOW, 1.2 MEDIUM-only, 2.2 HIGH-only |
| review (code) | `@reviewer` round 2 | **APPROVED** (B1 fixed) |
| review (requirements) | `@requirements-reviewer` round 2 | **APPROVED** (F1 stale CHANGELOG fixed) |
| working tree | `git status --porcelain` | 53 paths at closeout: 49 committed in `f56d072` (code/tests/goldens/docs) + 4 state files in this closeout; no secrets |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact), **`R-pf-082`** (medium; synthetic corpora),
  **`R-pf-007`** (medium; differentiation), **`R-pf-094`** (low), **`R-pf-095`**
  (low; Windows residuals), **`R-pf-096`** (low; default scans have zero non-LOW call
  edges), **`R-pf-097`** (low; I4 residuals).
- **R-pf-097 detail:** no django corpus exists in-repo, so the plan's "django humans
  bounded" is **structural only** (`HUMAN_CAP=10`, ≤3 links/row, zero per-finding rows in
  report.md/html) — no measured size was produced and none is claimed. Pre-I4 chart-set
  prose drift in `README.md:94`, `docs/website/scope-and-content.md:69`,
  `docs/adr/ADR-0002-visualization.md:32`; four layer heat captions + `hot-components`
  still lack the word "candidate"; workspace figure is emitted twice in `report.html`
  (`#map` + `#architecture`).
- **Blockers (unchanged):** **`B-pf-001`** (contact); **`B-pf-002`** (secrets).
- **Follow-ups:** C octal bound `i < 010` parses decimal; IR/schema prose still stops at
  v1.3; old `layer-file` reserve branch partly untested; static-SVG label thresholds need
  the I5 eyeball.

## Next Actions
1. **NEXT: I5 proof + demo** (A-pf-097) — full gate, double-scan byte-identical goldens,
   9-repo demo eyeball with the 3-action maintainer test, A-pf-095 true GENUINE manual
   triage, semantic-tier check that makes `layer-entry` depth strata non-trivial.
2. Close R-pf-097 residuals opportunistically (doc drift + heat-caption wording).
3. M5 publish needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Change set (this session)
- **Code:** `src/charts/{mod,layers,component}.rs` (annotation model, dots, new stem,
  component ramp label), `src/reporter/mod.rs` (breadcrumb, `#map` drill section, `#files`
  component links, registry), `src/cli/mod.rs` (comment: five layered stems).
- **Tests:** new `tests/i4_annotations.rs` (8 locks: dots/ring precedence/legend/roll-up/
  new stem caps+stub/order/drill anchors/detail untouched); frozen-set updates in
  `charts_render`, `i2_cost_v2`, `i3_map_layer`, `p4_determinism`, `pipeline_e2e`,
  `p6_hardening`, `benchmark`.
- **Goldens:** 4 new `layer-findings.svg`; annotated SVGs + `report.html`/`report.md`
  refreshed; `findings-detail.*`, `include-graph.svg`, `metrics.json`, `graph.dot` untouched.
- **Docs/state:** `docs/report/output-contract.md`, `docs/architecture/modules.md`,
  `CHANGELOG.md`, `state/session-state.json` (D-pf-222, R-pf-097, A-pf-097; A-pf-096 →
  ledger), this handoff, `docs/activeContext.md`.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted); first periodic
audit 2026-10-16 (counter 0).
