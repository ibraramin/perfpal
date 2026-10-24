# F treemap — candidate prototype (not shipped)

**Status:** candidate prototype, not shipped. **F remains UNDECIDED.** The Explorer treemap is
untouched. This is evidence for the step-12 below-graph weight-slot decision, not a founder decision.
This session made no `src/`, `tests/`, schema or Cargo change; no commit; no JavaScript in the
artifacts, no network, no new dependency.

## Founder decision (2026-10-24)

**Keep W1; F stays undecided.** No F code ships in step 12. The Explorer treemap and all F data
remain untouched; this prototype is retained as evidence for a later decision. Signed: founder (via
the step-12 decision prompt).

## What this is

Two standalone pages that put the F treemap in the slot the shipped weight panel occupies: a 1,408px
content band (1,440px page with 16px paper gutters) at 1440×900, with a 260px panel, heading above,
caption and legend below. The Atlas warm tokens are used throughout: paper `#f4f1ea`, panel `#fbf9f4`,
ink `#202b35`, muted `#6f6a61`, hairline `#d6d0c4`, production `#3f6d6a`, hot/path `#c2410c`; sans
chrome, mono identifiers and figures.

- `f-render.py` — self-contained deterministic renderer. Reads exactly the two real scans and refuses
  any path containing django/postgres (and any path that is not one of the two allowed inputs).
- `f-treemap-hyperfine.html` / `f-treemap-cJSON.html` — the candidate panels.
- `f-treemap-hyperfine.png` / `f-treemap-cJSON.png` — 1440×900 viewport captures (Playwright was
  available; the renderer skips these with a note if it is not).

**Data contract.** Component = longest matching `components[].path_prefix`, `.` fallback. Area =
the component's share of summed `function_cost_v2` (the whole-repo mass). Production-first top-50
cell selection; a residual `other` cell would carry any omitted mass (not exercised here: 12/12 and
20/20 components fit). Production cells use one teal cost ramp; test/bench cells are neutral
hatched and class-labelled where the area allows (the `<title>` always carries the class), but they
still keep their area. Top-3 cells get rank badges. A cell is labelled only when its area allows;
every cell `<title>` carries label, class, `static cost units (v2)`, share, file count, multiple and
top P%. The caption is `area = static cost units (v2); colour = the same scale`; the anchor note
names `× median component` and its denominator.

## Measured shape of the two scans

| Scan | Files | Functions | Findings | Parse ok/partial/failed | Total cost | Production | Components with cost | Median component | Top cell |
|---|---:|---:|---:|---|---:|---:|---:|---:|---|
| hyperfine | 53 | 353 | 348 | 53 / 0 / 0 | 4,535 | 2,806 (61.87%) | 12 of 12 | 202 | `src` 35.66% (production) |
| cJSON | 101 | 1,055 | 837 | 63 / 38 / 0 | 2,138,769 | 3,877 (0.18%) | 20 of 21 | 24 | `tests/unity/test/tests` 93.53% (test) |

cJSON is the stress case: one Unity test component is 2,000,485 static cost units (v2); the second
`tests` component is 6.19%; root production is only 3,724 units and `fuzzing` adds 153. The map makes
that test mass visible, but production becomes a 57×11px root strip plus a sub-10px `fuzzing` sliver
at the bottom edge.

## How it differs from the shipped weight panel (W1)

| | Shipped weight panel (W1) | F treemap candidate |
|---|---|---|
| Read | Ranked, production-first rows/bars: top 5 production files + top 3 production components by `% of scanned static cost`, plus the labelled `non-production (test/bench/other)` band. | One whole-repo area map: every component's area is its share; production-first top-50 cells; rank badges are the only order cue. |
| Scope | Exact top-N rows, file and component detail, with the long tail represented as a labelled band. | Whole-repo mass at a glance, component-level only; the long tail is visible as area, not as rows. |
| Non-production | A labelled band; never area (M-15). | Hatched and class-labelled, but it still occupies area. |
| Spec status | Allowed by `atlas-layout` §4. | Would need to amend `atlas-layout` §4 ("no area encoding, no treemap") and M-15 ("never area"). |

## What F answers better / worse

**Better.** It answers "where does the whole repo's weight sit?" in one look. On hyperfine it shows
production at 62% spread across nine components, with `src` heavy but not a lone hot spot. On cJSON
it makes the answer line's test-dominated claim visual: the 93.53% test component is the map, not a
row in a table. The production-first selection keeps production cells present, and the rank badges
plus titles give a second read.

**Worse.** On cJSON, production is 0.18% of area; no production-first rule can make a 57×11px root
strip legible without changing the area contract. W1 keeps the production rows and the labelled
non-production band readable — which is exactly why `atlas-metrics` M-15 and `atlas-layout` §4
currently forbid an area encoding on the Atlas page. Area comparisons are also less exact than bars
for the long tail: most cJSON cells are 1–2px and only reachable through `<title>`, which is not
available on touch or print. F is component-level only, so it loses W1's file rows. On cJSON the
hottest two badges are test cells, which is honest to cost but can mislead a glance; the read line
has to correct it.

## Decision options

1. **Keep W1 bars.** Spec-clean, best exact production read, no page change. Status quo.
2. **Replace the weight slot with F.** Strongest whole-repo mass read, but needs an M-15/`atlas-layout` §4 amendment,
   a cJSON production-legibility treatment, and a page-height budget. The cJSON evidence alone does
   not support replacing W1.
3. **Hybrid: F as a second below-graph panel (or Explorer-only).** W1 stays primary; F adds the mass
   and concentration view. Costs page height and a second encoding; needs a founder page-budget call.
4. **Leave undecided.** F stays a candidate; no page change. This is the current state; this artifact
   does not decide it.

**Current lean (not a decision):** keep W1 as the primary weight slot; if F is used, use it as a
second panel or an Explorer view, not a replacement, until cJSON production legibility is solved.
Hyperfine alone would support F more strongly; cJSON is the blocker.

## Weakest assumption / what would change the recommendation

**Weakest assumption:** that F's whole-repo mass is worth a second panel or a spec exception. The
strongest evidence for F is the cJSON test mass, but that fact is already stated in the answer line
and the labelled non-production band; F adds shape, not a new finding. If the founder's job is
"which production rows do I start with?", W1 wins.

**What would change it:** a production-legibility treatment that keeps area honest (a production
inset, a split contract, or a declared log-area scale); a page budget that allows a second panel
without breaking the ~4–5k `#map` target or the 900px first screen; a founder preference for
whole-repo mass over exact ranked rows; more production-dominated scans like hyperfine; or an
explicit Explorer-only placement, which lowers the stakes and leaves the current Explorer treemap
untouched.

## Verification (exact commands, observed)

| Claim | Command | Result |
|---|---|---|
| Renderer runs | `python3 docs/evidence/atlas-visual/v2/step12/f-render.py` | exit 0; wrote 2 HTML + 2 PNGs |
| No script | `grep -c '<script' f-treemap-*.html` | 0 / 0 |
| No URL | `grep -c 'http' f-treemap-*.html` | 0 / 0 |
| No event handlers | `grep -cE ' on[a-z]+=' f-treemap-*.html` | 0 / 0 |
| No links / embeds | `grep -cE '<a \|href=\|<img\|src=\|xlink:' f-treemap-*.html` | 0 / 0 |
| Exact caption | `grep -c 'area = static cost units (v2); colour = the same scale' f-treemap-*.html` | 1 / 1 |
| Holdout guard | `guard_input()` on django/postgres/other paths | refused; only the two allowed paths accepted |
| Determinism | double run + `cmp` on both HTML files | byte-identical; sha256 `2dba5cfa…` (hyperfine), `e732b66a…` (cJSON) |
| Screenshots | script asserts PNG dimensions | both 1440×900 |

The HTML is the deterministic artifact. The PNGs are viewport screenshots, not part of the
byte-determinism guarantee. Inputs are the exact two `out-atlas-v2-*` metrics files; the holdout was
never read. This session wrote only `docs/evidence/atlas-visual/v2/step12/`; it did not commit. The
worktree already contained unrelated modifications under `src/` and `tests/`; this session did not
touch them.

## Limits

The cost is a static heuristic, not a measurement. cJSON's parse is partial on 38 of 101 files, so its
findings and costs are bounded by parse coverage. The top-50 cap and residual `other` are implemented
but not exercised by these two scans; the V7 renderer's root-file expansion (tmux-only) is not used
here because both roots are under 0.3%. Tiny cells are hover-only. There is no holdout claim, no
measured-time claim, and no founder decision: **F remains undecided and the Explorer treemap is
untouched.**
