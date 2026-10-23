# Handoff — Atlas v2 Session A1 (step 11: detail rail)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. Step 11 restores the lost
per-file detail layer **without** turning the canvas into a dashboard: the first page is the main
graph + a small key-metrics strip + a G-style right rail of file detail; the rail is the answer-first
detail layer, not a second metric wall. **No drift.** Step-13 founder visual gate remains pending;
A2/ship (steps 14–15) remain blocked by R-pf-109.

**Phase:** Atlas v2 **steps 9–11 implemented** (2026-10-23/24); step-10 founder check APPROVED
2026-10-24; **step 11 implemented 2026-10-24, uncommitted — founder commit approval pending**;
step-12 (progressive charts/F candidate) next; step-13 final gate PENDING; A2/ship BLOCKED.

**Commits:** none for step 11. The track requires founder approval before commit (`atlas-beta.md:28`).
Worktree: 15 modified/untracked source/test/evidence/doc paths; `.playwright-mcp/` is scratch and is
not part of the change. No push.

## This session — step 11

- **Rail:** `#atlas-rail` (530px) is a DOM sibling of the 868px `#atlas-canvas` inside a new
  `.atlas-stage`; the canvas stays graph + one footer (A9). Up to **12** cards: elected entry first,
  then the shipped production-first `AtlasView.files` order; role `ENTRY` > `UNREACHED` > path class
  (`TEST`/`BENCH`/…) > `PRODUCTION`; M-04 cost anchor (`× median function` + `top P% of files`);
  top-3 functions (`name:line`, `(anonymous)` for empty names); dominant finding rule.
- **Reachable:** file → weight-drill row (`#atlas-…`), function → exact `profile-queue.html#fn-…`
  (deterministic per-function row ids added), finding → exact `findings-detail.html#finding-…`
  (prefers the dominant severity occurrence). All links resolve; `+N more` links to the weight detail.
- **Responsive:** ≥1360 side-by-side (1360–1439 wraps the rail name instead of tail-truncating);
  <1360 stacks below the graph; print stacks; 1280×800 no horizontal scroll.
- **Canvas:** node labels are the file name (spec §3.2) with a parent segment on basename collision
  (`export/mod.rs`, `time/mod.rs`, `quantity/mod.rs`); full path in `<title>` and rail. Standalone
  SVG artifact keeps the 1400 viewBox.
- **Chrome:** footer phrases are `nowrap`; rail floors 8.5px (name 10); role badges colour-coded;
  contrast fixed for unreached/test roles.
- **Tests:** new `tests/atlas_rail.rs` (9 tests: eight states, placement, card contents, link-target
  identity, empty stub, entry-first/order, exact cap + remainder, all five roles, determinism/no-JS);
  new unit guards for the severity-occurrence link and the role-colour rule order; `canvas_body`
  slice tightened. Goldens: 4 `report.html` + 4 `profile-queue.html` regenerated.
- **Vision:** `VISION-STEP11.md` — **PASS** (initial read + post-fix re-read + collision re-check).
- **Panel:** round 1 R1/Q1 APPROVED, R2/Q2 BLOCKED on the evidence transcript; fixes applied. Round 2
  R2/Q2 APPROVED + fresh R3 APPROVED; final R4 confirmation on the polished tree (see below).

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | 56 suites, 383 lib tests, exit 0 |
| Licenses | `cargo deny check licenses` | `licenses ok`, exit 0 |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 56 suites, 383 lib tests, exit 0 |
| Rail suite | `cargo test --locked --test atlas_rail` | 9/9, exit 0 |
| A9/A12/A4 | `cargo test --locked --test atlas_render` | 5/5, exit 0 |
| A10 stack | `cargo test --locked --test atlas_states` | 2/2, exit 0 |
| No schema | `test -z "$(git status --short src/schema/ docs/schema/)"` | exit 0 |
| No preserve_order | `grep -n preserve_order Cargo.lock` | exit 1 (absent) |
| Evidence | `python3 docs/evidence/atlas-visual/v2/render_step11.py` | exit 0 |
| 1440×900 fit | `step11.json` | canvas 869 ≤900; rail 834/818; 12 cards; scrollW 1440 |
| 1280×800 | `step11.json` | 1280/1280 scrollW; rail stacks (879 ≥ 869) |
| Text floors | `step11.json` | rail title/role/anchor/fn/find 8.5; name 10; node 11.5/9 |
| Links | `step11.json` + `tests/atlas_rail.rs` | 0 dead same-page/profile/findings anchors |
| Page budget | `step11.json` | `#map` 8,895/14,598 (+8/+7 footer wrap); page 22,918/34,808 |
| Vision | `VISION-STEP11.md` | PASS (step-11 render; not the founder gate) |
| Panel | `gate-step11.txt` + lane reports | R1/Q1/R2/Q2/R3/R4 **APPROVED**; zero blocking findings |
| Commit | `git log` | not committed — founder approval pending |

The full command/exit/hash transcript is `docs/evidence/atlas-visual/v2/gate-step11.txt`; `gate.txt`
remains the step-10 transcript. `step11.json` carries the DOM geometry, floors, link resolution and
artifact hashes.

## Risks / open

- **Step-13 founder gate PENDING:** the final composition/V8 rerun must judge the 868+530 stage, the
  basename labels and the rail. The `@vision` read was a proxy.
- **Sparse-state composition:** cJSON (2 of 81 reachable) leaves a large empty graph region while the
  rail is full; graph-first still holds but step 13 should judge it (disclosed V8 item).
- **Residual polish:** two 16-char basenames still elide by one glyph in the graph
  (`executor_tests.rs`, `macos_counters.rs`); some rail function rows wrap mid-identifier; edge
  labels can touch group borders; the below-graph brief is still 3,367/3,219px (step 12's cut).
- **Page budget:** `#map` grew +8/+7px (footer wrap on the 868px canvas); the ~4–5k cut is step 12.
- **R-pf-109 open;** A-pf-118 blocked; holdouts vaulted; `map-build` NOT done; A1b open; no schema/
  JS/dependency; no push; **no commit yet** (founder approval required).

## Next actions

1. Founder: review the step-11 renders and **approve the commit** (track gate). On approval, commit
   the code + tests + goldens + evidence + docs/state (exclude `.playwright-mcp/`); no push.
2. `/new` → **Session A2: step 12** (progressive weight/blast/file charts + F treemap candidate;
   `#map` ≤~5k; retire the seven-block text wall; no schema/JS).
3. Do **not** start step 13/A2/ship; holdouts vaulted; `map-build` not done; A1b open.

## Change set

- **Code:** `src/reporter/atlas_render.rs` (PAGE_GRAPH_W/layout width, collision-aware node labels,
  stage/rail CSS + responsive/print, footer phrase spans, standalone 1400 retained),
  `atlas_brief.rs` (`render_rail`, roles, links, plural remainder, anonymous-function labels),
  `mod.rs` (rail wiring + `function_anchor_map` + profile-queue row ids).
- **Tests/goldens:** `tests/atlas_rail.rs` (new), `tests/atlas_render.rs`, `tests/atlas_fixtures/mod.rs`
  (`roles_demo`), 4 `report.html` + 4 `profile-queue.html` goldens.
- **Evidence:** `docs/evidence/atlas-visual/v2/` (`STEP11.md`, `step11.json`, `gate-step11.txt`,
  `VISION-STEP11.md`, `render_step11.py`, 6 PNGs, `README.md`).
- **Docs/state:** `CHANGELOG.md`, `docs/activeContext.md`, `docs/plans/atlas-build.md`,
  `docs/specs/atlas-layout.md`, `docs/atlas/AGENT-BRIEF.md`, `state/session-state.json`,
  `state/archive/ledger.jsonl`, this handoff.
- **No** schema/CLI/metrics.json/dependency/script changes; no push; no commit.
