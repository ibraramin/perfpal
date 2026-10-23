# Step 11 — G-style per-file detail rail

**Status:** implemented 2026-10-24 (Session A1); step-13 founder gate still pending; A2/ship remain
blocked (R-pf-109). **No schema/JS/dependency change.**

## What was built

The first page is now a **two-column stage**: the graph canvas (`#atlas-canvas`, 868px) plus a
**right-hand per-file detail rail** (`#atlas-rail`, 530px) — the G-style detail layer restored
without turning the canvas into a dashboard (A9). The rail is a DOM sibling of the canvas; it is
detail, not a metric panel.

- **Selection:** the elected entry file first, then the shipped production-first `AtlasView.files`
  order, deduplicated by path, capped at **12** cards (`RAIL_FILE_CAP`). `+N more source files` is a
  link to the weight detail below when the scan exceeds the cap.
- **Roles:** `ENTRY` > `UNREACHED` > path class (`TEST`/`BENCH`/…) > `PRODUCTION`; every role value is
  pinned by the `roles_demo` fixture (`tests/atlas_rail.rs`).
- **Per card:** role badge (`ENTRY` / `PRODUCTION` / `TEST` / `BENCH` / other path class /
  `UNREACHED`), the full-path label (mid-ellipsis, full path in `title`), the shipped file cost anchor
  (`file total … static cost units (v2) · N× median function · top P% of files`), the top-3 functions
  (`name:line`; anonymous functions render `(anonymous)`), and the dominant finding ID.
- **Reachability:** every file name links to its weight-detail row (`#atlas-…`); every function links
  to its exact `profile-queue.html#fn-…` row (deterministic `fn-…` ids added to the profile queue);
  every dominant finding links to its exact `findings-detail.html#finding-…` occurrence. No card link
  is dead — asserted in `tests/atlas_rail.rs` and measured in `step11.json`.
- **Responsive:** ≥1360px = side-by-side; <1360px stacks the rail below the graph (1280×800: no
  horizontal scroll); print stacks too.
- **Canvas:** stays graph-only. The graph viewBox is the narrower `PAGE_GRAPH_W = 868`; node labels
  now show the **file name** (spec `atlas-layout` §3.2; the rail and node `<title>` carry the full
  path) so identities stay readable at 868px, with a **collision-aware parent segment** when two
  drawn files share a name (`export/mod.rs`, `time/mod.rs`, `quantity/mod.rs`). The standalone SVG
  artifact keeps `SVG_W = 1400`.
- **Footer:** each metric phrase is a `nowrap` span, so the two-line footer never splits
  `33/238 unresolved includes`. Rail text floors: title/role/anchor/function/finding 8.5px, name 10px.
- **Plan wording:** `docs/plans/atlas-build.md` says "without leaving the page"; the binding track DoD
  is "reachable **from** the first page". The rail shows the function name/line and finding rule
  in-page and links to the existing `profile-queue.html` / `findings-detail.html` drill rows; the
  track wording governs.

## Measured evidence (`step11.json`, 1440×900 + 1280×800, reduced motion)

| Claim | hyperfine | cJSON |
|---|---|---|
| canvas bottom (≤900) | **869** | **869** |
| rail bottom (≤900) | **834** | **818** |
| rail beside canvas / width | 894 ≥ 884 · 530px | 894 ≥ 884 · 530px |
| rail cards (cap 12) | 12 | 12 |
| role mix | ENTRY/PRODUCTION/UNREACHED | ENTRY/PRODUCTION/UNREACHED |
| broken rail links (`#`, profile, findings) | 0 | 0 |
| text floors: node name / meta / rail name / rail rows | 11.5 / 9.0 / 10 / 8.5 | 11.5 / 9.0 / 10 / 8.5 |
| chrome counts: legend / footer / caveat / median / leading-dot | 1 / 1 / 1 / 1 / 0 | 1 / 1 / 1 / 1 / 0 |
| canvas panels / `REPORT_SCRIPT` | 0 / 1 | 0 / 1 |
| `#map` height (step-10 baseline) | 8,895 (8,887) | 14,598 (14,591) |
| page height (step-10 baseline) | 22,918 (22,911) | 34,808 (34,800) |
| 1280×800 | 1280/1280 scrollW · rail stacks below (879 ≥ 869) | same |

The only growth is **+8px hyperfine / +7px cJSON**: the graph is 5px shorter, the 868px canvas wraps
the footer from one line to two (+12px). The evidence script therefore allows a bounded **+12px
footer-wrap delta** against the step-10 baseline (`render_step11.py` `mapHeight_le_step10_plus_footer_wrap`);
nothing else grew and the real `#map` cut (~4–5k) is step 12. Booleans and artifact hashes are in
`step11.json`; screenshots `atlas-v2-step11-{hyperfine,cJSON}-{final,full,1280x800}.png`.
**Goldens:** the four committed `report.html` trees now embed the rail and the four
`profile-queue.html` trees carry the new `fn-…` row ids; no other artifact changed.

## Verification

| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | 56 suites, 383 lib tests, exit 0 |
| Licenses | `cargo deny check licenses` | `licenses ok`, exit 0 |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0 |
| Rail suite | `cargo test --locked --test atlas_rail` | 9/9, exit 0 |
| A9/A12/A4 | `cargo test --locked --test atlas_render` | 5/5, exit 0 |
| A10 stack | `cargo test --locked --test atlas_states` | 2/2, exit 0 |
| No schema | `test -z "$(git status --short src/schema/ docs/schema/)"` | exit 0 |
| Evidence | `python3 docs/evidence/atlas-visual/v2/render_step11.py` | exit 0 |

Full transcript: `docs/evidence/atlas-visual/v2/gate-step11.txt` (every suite line, exits, mtimes,
artifact hashes). `gate.txt` remains the step-10 transcript.

## Vision (step-13 proxy — not the founder gate)

`VISION-STEP11.md` — **PASS** for both 1440×900 renders (two reads: initial + post-fix re-read).
Graph-first holds (hyperfine) and the rail reads as G-style file cards, not a dashboard. The first
read found node-name elision, rail basename ambiguity, a split footer phrase and bare `:line`
anonymous functions; all were fixed and the re-read confirmed them. The collision-aware `mod.rs`
labels were then verified in a third read: `export/mod.rs`, `time/mod.rs`, `quantity/mod.rs` — no
duplicate canvas label remains. Residual (deferred to step 13/A2): two 16-char basenames still
elide by one glyph (`executor_tests.rs`, `macos_counters.rs`), mid-identifier wraps in some rail
function rows, edge-label/group-border touches, and the sparse cJSON graph void.

## Residuals / deferred

- **cJSON sparse state:** with 2 of 81 files reachable, the 720px graph region has a large empty
  area while the rail is full — the graph is still the first read, but step 13 should judge the
  sparse-state composition (disclosed V8 item).
- **Edge-label/group-border touches** remain (canvas chrome, step 13/A2).
- **Node label granularity:** basenames can repeat (`mod.rs` in different modules); the rail and
  `<title>` disambiguate. Founder call at step 13.
- Step 12 owns the `#map` ≤ ~5k cut and the progressive charts/F candidate; A2/ship stay blocked.
