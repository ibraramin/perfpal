# Step 13 — founder visual gate, G-baseline diff, V8 rerun

**Status: gate evidence complete; founder composition sign-off APPROVED 2026-10-24.** This session did **not**
change code, schema, scripts or dependencies. A2/ship (steps 14–15) remain blocked by `R-pf-109`
until the founder signs the 1440×900 composition. If the founder rejects it, the gate **fails** and
a superseding decision is escalated — no tuning in this session.

**Date:** 2026-10-24 · **Viewport:** 1440×900 canonical (+1280×800) · **Capture:** `report.html#map`
scrolled to the viewport top, `prefers-reduced-motion` + animations disabled · **Source:** a
session-private fresh scan with the current binary (`/tmp/opencode/step13-page/{hyperfine,cJSON}`).

**Provenance / tree:** Track C fast-forward merged into `main` (`c73d6ff`) while this gate was
being prepared. The gate was therefore **re-scanned and re-rendered on the merged tree** with a
release binary built from it. The session-private scans are **byte-identical to the shared
`perfpal-test/out-atlas-v2-*` pages** (hyperfine `a4fade03…`, cJSON `323ee0a8…`), and
`step13.json → capture` records both plus `same_as_private: true`. The merge adds two nav links
(`agent report`, `agent actions`) inside the below-the-fold coverage block: **`-charts.png` and
`-full.png` changed**, while the 900px `-final.png`, the weight crops, the 1280×800 frames and all
eight fixture frames are **byte-identical**, so the 32 blind reads and the G composite remain valid.
No assertion or measurement changed.

The founder page is served privately at **http://127.0.0.1:8769/hyperfine/report.html#map** (and
`/cJSON/report.html#map`). `:8767` also serves the merged page, but the private server is pinned to
the gate capture.

## 1. What was run

1. `render_step13.py` — real-page DOM gate + renders (1440×900, 1280×800), G-vs-step-13 composite,
   `step13.json`.
2. `render_step13_states.py` — the eight synthetic fixture states regenerated from the current tree
   (`step13/html/`), preflight + final frames + weight-panel crops, `step13/manifest.json`.
3. 32 fresh blind `@vision` reads on the reworked page (two A1 + two strip reads per state; neutral
   filenames; protocol = V8) → `step13-reads.json`.
4. G-baseline diff (measured + a fresh comparative read) → `G-BASELINE-DIFF.md` §9,
   `step13/G-vs-step13-hyperfine-1440x900.png`.

## 2. First-page composition (canonical `#map`, reduced motion)

| Claim | hyperfine | cJSON |
|---|---|---|
| `#map` height (budget ≤5,000) | **1,848** | **1,848** |
| canvas top → bottom (fold ≤900) | 120 → **869** | 120 → **869** |
| canvas footer bottom (≤900) | **869** | **869** |
| page chrome bottom vs canvas top | 120 ≤ 120 | 120 ≤ 120 |
| detail rail | 120–834, 530×714, **12 cards** | 120–818, 530×698, **12 cards** |
| first below-canvas block top | 879 (≥ canvas bottom) | 879 |
| page height (whole document) | 15,872 | 22,058 |
| horizontal scroll 1440×900 | none (1,440/1,440) | none (1,440/1,440) |
| horizontal scroll 1280×800 | none (1,280/1,280) | none (1,280/1,280) |
| rail at 1280×800 | stacks below the canvas (y 879, 868 wide) | stacks below the canvas |
| edge labels drawn (`counts.edgeLabels`) | 2 (path/selected only; G drew 49) | 1 |

### DOM/render assertions (all pass, `step13.json`)

| Assertion | Result |
|---|---|
| `#atlas-canvas` bottom ≤ 900 including header/legend/footer | 869 / 869 |
| canvas footer bottom ≤ 900 | 869 / 869 |
| no clipped text (hidden/clip container overflow; node text vs node box; CSS ellipsis without a `title`) | 0 violations |
| no SVG node text overflowing its node rect | 0 violations |
| no duplicate definitions (unit, median, structural caveat text, unreached, legend, footer) | 1 each |
| no duplicate DOM ids (document and `#map`) | 0 |
| no leading-dot truncation (ASCII `..` or leading `…`) | 0 |
| `#map` ≤ 5,000px | 1,848 / 1,848 |
| no horizontal scroll 1440 and 1280 | 1,440/1,440 · 1,280/1,280 |
| canvas purity (A9): tables / bars / rail cards | 0 / 0 / 0 |
| no scripts or event handlers in the canvas (A12) | 0 / 0 |
| first below-canvas block starts below the canvas | 879 ≥ 869 |
| rail stacks at 1280×800 | yes |

Smallest text in `#map` is the **8px masthead kicker** (G's signed kicker is 8px/800); node meta is
8.5px, node badges 8.5px — unchanged from the approved step-10/11 system.

## 3. G-baseline diff (summary; full §9 in `G-BASELINE-DIFF.md`)

The reworked page is the **same visual system** as G: warm paper `#f4f1ea` / panel `#fbf9f4`, ink
`#202b35`, muted `#6f6a61`, burnt-orange `#c2410c` accent; `ui-sans-serif` chrome with mono
identifiers/figures; 58px masthead; one legend, one unit definition, one structural caveat, one
footer; a right file-detail rail; mid-ellipsis node labels with full values in the rail/`title`.
Measured tokens match the step-9 target (wordmark 21/800, kicker 8/800, node mono meta 8.5, legend 9).

Deliberate/known differences, all previously signed or disclosed:

- the masthead heroes the page (**Atlas**) rather than the repo name, because the HTML deliberately
  omits `scan.root_path`;
- the rail shows the top 12 of 51 files plus a `+39 more` weight link (G drew 19 cards);
- `#map` is 1,848px, not a 900px artboard, because the founder's model adds progressive charts
  below the first page;
- node labels are basenames (parent segment only on collision) — the step-11 residual below.

## 4. V8 A1 + strip rerun on the reworked page

`step13/manifest.json` + `step13-reads.json`. Two fresh blind reads per state.

| State | canvas bottom | `#map` | rail cards | A1 | strip |
|---|---|---|---|---|---|
| application | 860 | 1,623 | 4 | 2/2 | 2/2 |
| library | 869 | 1,616 | 3 | 2/2 | 2/2 |
| no-entry | 860 | 1,343 | 1 | 2/2 | 2/2 |
| empty | 860 | 1,205 | 0 (named stub) | 2/2 | 2/2 |
| all-test | 860 | 1,313 | 1 | 2/2 | 2/2 |
| capped | **1,398** (V8-disclosed) | 2,372 | 12 | 2/2 | 2/2 |
| partial-parse | 860 | 1,480 | 2 | 2/2 | 2/2 |
| single-file | 860 | 1,387 | 1 | 2/2 | 2/2 |

**A1 16/16 PASS · strip 16/16 PASS**; 14/14 share-bearing strip reads correct (the empty state contributes two named-stub reads, no share), 0 scale confusions, 0 time/runtime claims. Every state carries
entry/HOTTEST/START (or its named stub) in the first screen; the weight panel answers the mass
question in every state.

## 5. Founder gate

Open **http://127.0.0.1:8769/hyperfine/report.html#map** (private, current) and inspect:

- `docs/evidence/atlas-visual/v2/atlas-v2-step13-hyperfine-final.png` (first screen)
- `docs/evidence/atlas-visual/v2/atlas-v2-step13-cJSON-final.png`
- `docs/evidence/atlas-visual/v2/step13/G-vs-step13-hyperfine-1440x900.png` (G beside step 13)
- `atlas-v2-step13-{repo}-full.png` / `-charts.png` (below the fold)

**What only the founder can judge:** does the first page read as the approved designed page (palette,
typography, hierarchy, masthead), and is the composition right for the beta? The DOM gate and the
blind reads cannot substitute for that.

**Decision: APPROVED 2026-10-24.** Step 13 passes; A2 (step 14) may be scheduled as a separate
session. The dated sign-off and its provenance are recorded in `step13/FOUNDER-GATE.md` and
`state/session-state.json` (D-pf-253). No commit or ship is implied.

## 6. Residuals (disclosed, not fixed)

1. **Step-11 canvas truncation:** basenames can mid-elide (`executi_.ets.rs`, `macos_c_.ters.rs`)
   and node labels drop the `src/` prefix (parent segment only on basename collision); one rail
   function row (`src/shell_syntax.rs`) wraps mid-identifier and loses a line number in the visible
   card (full value in `title` / profile queue). Flagged again by the comparative read; **not
   tuned** here.
2. **Sparse cJSON state:** 2 of 81 source files reachable, 38 partial parses, 75 files demoted,
   133/229 unresolved includes. The map is honest but visually thin (one reachable file, large empty
   UNREACHED column) while the rail is full.
3. **Docs leftovers:** `docs/report/output-contract.md` does not list `atlas-weight.html` /
   `atlas-q2.html`; `atlas-feature.md` / `maintainer-map.md` / `map-animation.md` still describe the
   retired seven-block brief; the metrics wording says "whole-tenth" where one doc says "one
   decimal". Step 15 docs pass.
4. **Page height outside `#map`:** 15,872 / 22,058px from the legacy Explorer/appendix; the step-12
   budget targets `#map` only.
5. **Capped synthetic state:** canvas 1,398px > the 900px fold (V8-disclosed, A2-owned). It is
   measured, not asserted; its A1/strip reads still pass.
6. **Fold peek:** the first below-canvas heading ("Where is the mass?", 890–907) is sliced by the
   900px fold — a standard scroll affordance; no element is clipped by its container.
7. **G masthead:** the wordmark heroes "Atlas", not the repo name (root path omitted from the HTML).

## 7. Verification

| Claim | Command / artifact | Result |
|---|---|---|
| Real-page gate | `python3 docs/evidence/atlas-visual/v2/render_step13.py` | PASS, exit 0 |
| Eight-state gate + frames | `python3 docs/evidence/atlas-visual/v2/render_step13_states.py` | PASS, exit 0 |
| Fixture regeneration | `PERFPAL_ATLAS_VISION_DIR=…/step13/html cargo test --locked --test atlas_vision -- --ignored` | 1 passed (in `gate-step13.txt` §1) |
| Currency | private scan sha256 vs shared page | identical (`a4fade03…` / `323ee0a8…`, merged `c73d6ff`) |
| Frames affected by the merge | `-charts.png` / `-full.png` changed; `-final`/weights/1280/8 fixtures identical | 32 reads stand |
| A1 reads | `step13-reads.json` | 16/16 PASS |
| Strip reads | `step13-reads.json` | 16/16 PASS |
| G composite | `step13/G-vs-step13-hyperfine-1440x900.png` | present, sha256 `653c357f…` |
| Founder sign-off | `step13/FOUNDER-GATE.md` + D-pf-253 | **APPROVED 2026-10-24** |
| No source/schema/deps | `git status --short src/ Cargo.toml Cargo.lock` | empty |

Full transcript: `gate-step13.txt`.

## 8. Scope guard

This session made no `src/`, schema, JS, dependency or golden change, and made no commit or push.
The tree advanced during the session via **Track C's fast-forward merge into `main` (`c73d6ff`)**;
the gate was re-run on that tree (see Provenance). `step13/` holds the state frames, manifest and the
F-independent gate package; F remains undecided (`D-pf-251`) and the Explorer treemap is untouched.
