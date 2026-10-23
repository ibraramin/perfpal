# Handoff — Atlas build session 3 (steps 7–8 + V8 vision)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. This session closed
**step 7** (test/golden lock) and **step 8** (V8 vision gate) of `atlas-build`; the V8 evidence is
committed. The post-step-8 visual audit then found the composition is **not A2-ready** (`R-pf-109`),
so the plan is reordered around a v2 visual rework. **No drift.**

**Phase:** Atlas **steps 7–8 GREEN; v2 visual rework planned (2026-10-23)**. Next session: **v2 visual
rework steps 9–10** (G baseline/composition + first-page fit), **not A2**. A2/ship moved to steps
14–15 and are blocked until the founder visual gate.

**Commits:** `0021cd1` (step 7/8 code + V8 evidence); `996fb35` (steps 7–8 docs/state); this
audit/v2-plan commit follows. Prior: `9a0e592` (steps 5–6 handoff),
`d31f283`/`a2a4c99`/`672e4b6` (steps 5–6).

## This session — steps 7–8

- **Step 7 (test/golden lock).** The full plan list is covered: entry election ×4, structural
  reachability/BFS (pre-cap), caps/aggregation, path trace, M-01…M-18 anchors/formatting, zero-state
  stubs, MEDIUM/LOW confidence mix, noise footer, M-18 annotations — **106 Atlas unit tests + 5
  integration locks** (`tests/atlas_render.rs`, `tests/atlas_states.rs`) + the 8-state fixture matrix
  + double-render determinism + A9/A12/A4. `tests/atlas_vision.rs` (ignored) renders the eight full
  report pages on demand for the V8 matrix. The four committed `report.html` goldens are current
  (`PERFPAL_REGEN_GOLDEN=1 ... regenerate_goldens` is a no-op). **Reconciliation:** no
  cJSON/hyperfine/tmux `report.html` goldens exist in the repo — those repos are V7 prototype HTML
  only; the committed golden set is the four fixture trees, all regenerated.
- **Step 8 (V8 vision).** Eight states rendered at **1440×900** final frames (`prefers-reduced-motion`
  + `animations=disabled`; HTML + PNG + `manifest.json` under `docs/evidence/map-vision/v8/`).
  **16/16 A1 reads pass** (2 fresh blind `@vision` reads per state, one image per task) and
  **16/16 strip-specific reads pass** (2 per state on the `#atlas-weight` crop). The weight-strip
  inclusion rule holds: every read answers N4; 11/16 name a top file/component in the `action`
  field and 14/16 name a top item/band in `mass_answer`; the five `none` actions are the correct
  empty/all-test/capped-tie answers; no share-vs-multiple or time confusion. Evidence: `reads.json` (image hashes + every read) + `NOTES.md`.
- **Gate-found fixes (real defects).** Report body width 72rem→90rem (the SVG had scaled to 0.83×,
  text 5–8px); SVG text floors (name ≥10px, meta ≥8.5px, badges ≥8px, headers ≥10.5px), four-line
  nodes with a 46px minimum and 12px row gap; two-line column headers (the `UNREACHED` header had
  collided with depth-0); compact `×N` edge pills with the full M-10 text in `<title>` and a legend
  key; deterministic pill/node avoidance (the library back edge covered `src/caller1.c`); hull-label
  clearance; a first-screen `.atlas-keys` line (units/× median/edge/path/caption); tooltip-free
  capture (mouse parked at 1,1). Only the four `report.html` goldens changed.

## Verification

| Claim | Command / evidence | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | 55 suites, exit 0 |
| Licenses | `cargo deny check licenses` | `licenses ok`, exit 0 |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 55 suites, exit 0 |
| A4/A12/A9/A10 | targeted `atlas_render`/`atlas_states` tests | 4/4 pass |
| No schema | `git status --short src/schema/ docs/schema/` | empty |
| Goldens | `git status --short tests/golden` | only the four `report.html` |
| Step 7 golden regen | `PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens` | no-op |
| V8 A1 | `docs/evidence/map-vision/v8/reads.json` | 8/8 states, 16/16 reads PASS |
| V8 strip | same | 8/8 states, 16/16 reads PASS |
| Vision preflight | `render_v8.py` DOM gate | pass (rendered name ≥9.9px, meta ≥8.4px, badge ≥7.9px, column ≥10.4px, aggregate ≥8.4px, hull ≥9.0px; containment; no pill >32px; no tooltip; first row above fold) |
| Gate transcript | `docs/evidence/map-vision/v8/gate.txt` | commands + exits + evidence hashes |

## Risks / open

- **A2 (R-pf-108):** founder timed test on application **and** library classes with animation on is
  the decisive gate; a failure supersedes ADR-0009, never silent tuning. The structural LOW/MEDIUM
  edges remain the main honesty risk.
- **Capped framing:** the depth-1 column is taller than the 1440×900 first screen (canvas viewBox
  1248, rendered 1255px; 20 of 26 shown, 6 aggregated, printed). The first screen carries entry,
  HOTTEST and PATH; A2 must judge large-repo usability.
- **Component label:** `src` was read as `SIC` in 3/16 strip reads; values/shares/actions were
  correct. Watch in A2.
- **M-17/M-02:** functions-in-header and answer-line reachability remain doc-reconciliation
  follow-ups. The forbidden-claim scan stays scoped to the Atlas surfaces.
- **Treemap deferred** (Explorer unchanged); weight detail links to `#explorer`.
- **Do not mark `map-build` done; A1b stays open.** Holdouts vaulted. M5 needs B-pf-001 + B-pf-002.

## Audit addendum — 2026-10-23 (BLOCKER; A2/ship moved to steps 14–15)

The real `report.html#map` (hyperfine/cJSON, current release) was audited after the founder's
"looks like shit" signal. **Atlas is not A2-ready.** Evidence:
`docs/evidence/atlas-audit/AUDIT.md`, `audit.json`, `atlas-{hyperfine,cJSON}-{final,full}.png`;
comparative blind vision read of G vs Atlas.

- **First screen clips the canvas:** `#atlas-canvas` 980px tall (bottom 981); fold 900; legend/footer
  cut; the answer line/brief starts at 981. G fit 1440x900.
- **Visual language replaced:** G warm cream/charcoal/burnt-orange + sans chrome/mono data + masthead
  + right-rail file cards → Atlas white/slate all-monospace + six raw header lines + no rail.
- **Detail rail lost:** the canvas shows only `INFO ×N`/`WARN ×N`; per-file functions/finding IDs
  moved into a 3,367/3,219px below-graph brief. Page 22,942/34,831px; `#map` 8,982/14,685; the Q2 and
  weight drills are inline in `#map`.
- **Hygiene:** structural caveat 6×, median definition 2×, footer legend clipped, leading-dot
  truncation (`..tity/statistics.rs`), 49 edge labels with a path/back-edge hue collision, unreached
  label ambiguity (footer 21 production + 11 non-production; column reads 11).
- **F treemap is Explorer-only;** the page's weight strip is text.
- **Gate blind spot:** V8 16/16 A1 + 16/16 strip passed; A1 tests comprehension, not composition. No
  founder visual gate, no G baseline, no canvas-bottom/clipping/duplication/page-height assertions.

**A2/ship (steps 14–15) are BLOCKED.** Next: the v2 visual rework (steps 9–13) against the G
baseline + founder visual gate; re-run V8; then A2. Holdouts stay vaulted; `map-build` NOT done; A1b open.

**New agents: read `docs/atlas/AGENT-BRIEF.md` first.** Founder model (2026-10-23): first page =
main graph + some key metrics; below, more key charts slowly; restore G's warm/sans/masthead visual
language; F stays undecided (keep the treemap/components as candidates).

## Session design — v2 visual rework (2026-10-23)

The plan is reordered: **steps 9–13 = visual rework; A2/ship = steps 14–15.** Steps 1–8 remain green.
The guiding rule is **easy to understand beats exhaustive metrics**: first page = main graph + a
small key-metrics strip; below = a few designed charts revealed progressively; detail rail; no
3,000px brief. Restore G's visual language; founder sign-off before pixel work; founder visual gate
+ G-baseline diff before A2.

- **Session 4 (next) — steps 9–10:** G/Atlas side-by-side, founder composition decision; first-page
  fit + palette/type/masthead + one-definition/chrome hygiene. DoD: 1440×900 canvas bottom ≤ 900; no
  clipping/duplicates/leading-dot truncation; key-metrics strip; founder sign-off on the visual
  system; G-baseline diff recorded.
- **Session 5 — steps 11–13:** detail rail; progressive weight/blast/file charts (+ F treemap
  candidate); founder visual gate; V8 A1 + strip rerun.
- **Session 6 — steps 14–15:** A2 (holdouts) + ship/docs/EOL.

## Next actions — new-session signposts

1. **`/new` → Atlas v2 visual rework Session 4: plan steps 9–10 only.**
2. Read `docs/atlas/AGENT-BRIEF.md` → `docs/evidence/atlas-audit/AUDIT.md` → V7 G prototype.
3. Step 9 founder composition sign-off; step 10 first-page fit + visual language + chrome hygiene.
4. Do **not** run A2 (step 14) or ship (15); holdouts vaulted; `map-build` not done; A1b open.

## Change set (this session)

- **New:** `tests/atlas_vision.rs`, `docs/evidence/map-vision/v8/render_v8.py`,
  `docs/evidence/map-vision/v8/{html/*.html,*-final.png,*-weight-strip.png,manifest.json,reads.json,NOTES.md,gate.txt,verify_evidence.py}`.
- **Modified:** `src/reporter/atlas_render.rs` (typography/layout/headers/pills/keys/hull clearance),
  `src/reporter/mod.rs` (report body width), four `tests/golden/*/report.html`,
  `docs/plans/atlas-build.md` (steps 7–8 DONE + golden reconciliation), `CHANGELOG.md`,
  `docs/activeContext.md`, `state/session-state.json` (D-pf-246/247; A-pf-118 blocked; A-pf-119 next),
  this handoff, `docs/atlas/AGENT-BRIEF.md`, and `docs/evidence/atlas-audit/`.
- **No schema/CLI/metrics.json change; no new dependency; no new Atlas script.**
