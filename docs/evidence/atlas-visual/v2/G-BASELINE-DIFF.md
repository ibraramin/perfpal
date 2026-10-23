# G-baseline diff — Atlas v1 vs the approved V7 G prototype

**Date:** 2026-10-23 · **Viewport:** 1440×900 (canonical), reduced motion, `report.html#map`
anchor. **Sources:** `docs/evidence/map-vision/v7/g/g-hyperfine.html` + `g-hyperfine-final.png`
(approved G), `docs/evidence/atlas-audit/{audit.json,atlas-*-final.png}` (shipped Atlas v1),
`docs/evidence/atlas-audit/AUDIT.md`, fresh comparative `@vision` read in `VISION-READ.md`.

This is a **visual-system and composition** diff, not a content diff. The model/data are unchanged
and correct (53 files · 353 functions · 348 findings · 342 reachable on hyperfine; the data is
identical to V7). The proposed target is **not a G revert**: port G's visual language and composition
discipline, keep Atlas's genuine improvements, then add the founder's key-metrics strip.

## 1. First impression

| | G (approved) | Atlas v1 (shipped) |
|---|---|---|
| What it is | A designed page: named subject, one map, one rail, one legend, one footer | An instrument panel: no masthead, six raw chrome lines, one canvas, then a report |
| Eye lands on | `hyperfine` wordmark → dark ENTRY node → orange HOTTEST outline → rail | Orange metric line → dark `src/main.rs` node → orange-outlined `src/command.rs` + path badges |
| First-page fit | Everything fits in 900px; no clipping | Canvas bottom **981px**; the last node row is cut mid-card; legend/footer invisible |
| Detail layer | 19 file cards in a right rail: role, cost, top functions, finding ID | No rail; nodes carry only `INFO ×N`/`WARN ×N`; details are 3,367/3,219px below |
| Page shape | One 900px artboard | 22,942px (hyperfine) / 34,831px (cJSON) document; `#map` alone 8,982/14,685px |

## 2. Token diff

G tokens are from `g-hyperfine.html:6`; Atlas v1 tokens are from
`src/reporter/atlas_render.rs:77-91` and `src/reporter/mod.rs` body CSS.

| Role | G (approved) | Atlas v1 | Proposed v2 |
|---|---|---|---|
| Page paper | `#f4f1ea` | `#ffffff` | `#f4f1ea` |
| Map/panel | `#fbf9f4` | `#ffffff` | `#fbf9f4` |
| Ink | `#202b35` (entry `#1f2d3a`) | `#0f172a` / `#111111` | `#202b35` / entry `#1f2d3a` |
| Muted text | `#6f6a61` | `#334155` / `#475569` | `#6f6a61` |
| Hairline | `#d6d0c4` | `#e2e8f0` | `#d6d0c4` |
| Hot accent | `#c2410c` | `#9a3412` (text) / `#d97706` (HOTTEST stroke) | `#c2410c` |
| Path trace | `#c2410c` solid, 2.3px | `#ea580c` dashed, 3px | `#c2410c` solid, 2.3px |
| Back edge | `#b45309`, thin dashed, low opacity | `#b45309` dashed + amber labels, same hue family as the path | **`#8d8577`** dashed, 0.45 opacity (neutral; no amber/orange collision) |
| Forward edge | `#8d8577`, 0.34 opacity | `#6b7280` solid | `#8d8577`, 0.34 opacity |
| Production node | fill `#fdfbf6`, stroke `#8b8173` | fill `#f8fafc`, stroke `#475569` | `#fdfbf6` / `#8b8173` |
| Test node | fill `#eef2f7`, stroke `#7f93aa`, dashed | hatch, stroke `#64748b` | `#eef2f7` / `#7f93aa`, dashed hatch stays |
| Bench node | fill `#f7f0f7`, stroke `#8a6f9e`, dashed | hatch, stroke `#64748b` | `#f7f0f7` / `#8a6f9e`, dashed |
| Unreached | fill `#e8e5df`, stroke `#b8b2a7` | grey fill/text, #d1d5db/#6b7280 | `#e8e5df` / `#b8b2a7` |
| Hull / module band | one light hull outline, 6% fill | pastel module bands **plus** hulls | **one light hull only** (drop the pastel bands) |

**Deliberate deviation from G:** the back-edge becomes neutral grey instead of amber. G's amber
back-edge and burnt-orange path are close enough to collide once Atlas draws 49 labels; the audit's
rule is "never amber vs orange for two different edge types." Path = burnt orange solid; back =
neutral dashed; forward = neutral solid.

## 3. Typography diff

| Role | G | Atlas v1 | Proposed v2 |
|---|---|---|---|
| Global body | `ui-sans-serif` stack | `ui-monospace` everywhere (16px) | `ui-sans-serif` chrome; mono only identifiers/figures |
| Masthead wordmark | 21px / 800 | none | 21px / 800 sans + repo sub 12.5px / 750 |
| Kicker | 8px / 800, .16em | none | 8px / 800, .16em |
| Entry path | 12.5px / 750 sans | inside 12px mono head | 12.5px mono, sans label |
| Column header | 7.6px / 800 | 10.5px mono | 10px / 800 sans; count 8.5px mono |
| Node name | 8.8px / 700 mono | 10.0–11.5px / 700 mono | 10.0–11.5px / 700 mono (keep Atlas's legibility) |
| Node meta | 6.8–7px | 8.5–9px mono | 8.5–9px mono (keep) |
| Rail card path | 9.4px / 750 mono | n/a (no rail) | 9.4px / 750 mono (step 11) |
| Rail card meta | 7.4px | n/a | 8px minimum (raise G's floor) |
| Legend/footer | 8px sans + swatches | 11px mono prose, no swatches | 8.5–9px sans with swatches; one occurrence each |

G's 7px text is not a target to copy: the audit counted 132 sub-10px canvas elements in Atlas and
G had small labels too. The proposed floor is **node name ≥10px, meta ≥8.5px, key strip ≥9px**;
hierarchy comes from weight, color and spacing, not from shrinking text further.

## 4. Geometry / composition diff

| | G | Atlas v1 | Proposed v2 (step 10) |
|---|---|---|---|
| Artboard | 1440×900 fixed, `overflow:hidden` | 1440×900 viewport, 22,942/34,831px body | 1440×900 canonical; no horizontal scroll |
| Page padding | 8px 12px | 16px page gutter; 1408px canvas | 8px 12px |
| Masthead | 58px | none (orphan Pause + 4–6 mono lines) | 58px: kicker/wordmark + entry block + [Pause] [Explorer] |
| Stats/legend strip | 30px subhead: two rows (counts + swatch legend) | 5 head spans + a separate `.atlas-keys` row + a 9-item legend row + footer (duplicated) | one **key-metrics strip ≤56px**: entry · hottest · path · caps · 3 actions · one caveat; one swatch legend row; one unit definition |
| Graph stage | 868×796 map (59.5% width) | 1408×722 SVG (full width) | full-width stage in step 10; G-style map+rail in step 11 |
| Detail rail | 538×796, 2-col cards | none | G-style right rail (step 11) |
| Footer | inside 900px, one line | clipped below 900px | one compact footer ≤24px inside 900px |
| Canvas bottom | 900 | **981** | **≤900** |
| Page budget | 900 | 22,942/34,831 (#map 8,982/14,685) | unchanged in step 10; `#map` ≤~5,000 after step 12 |
| Edge labels | only path/selected; counts in title | **49 labels** on 49 edges, path/back hue collision | path/selected only; full counts in `<title>` |
| Truncation | mid-ellipsis + full path in title/rail | leading-dot (`..tity/statistics.rs`) | mid-ellipsis + full path in title/rail |
| Unreached label | one clear stats line | footer 21 production + 11 non-production; column reads 11 | `unreached production: 21 · non-production: 11` (one label) |
| Background systems | depth columns + one hull | depth columns + pastel module bands + unreached column | depth columns + one light hull + unreached column |

**Implementation deltas (2026-10-23, shipped step 10):** the paper band is full-bleed
(`#map` margin `-1rem`, padding `8px 1rem`) with a **1,408px** canvas content width; the strip is
**53px** (≤56 signed); the masthead carries the wordmark/actions only and the entry rides in the
strip (the `<repo>` label is omitted because the HTML deliberately omits `scan.root_path`); test/bench
nodes use G's opaque pastel fills rather than the hatch; the canvas bottom is **862px**. The detail
rail is step 11; the 3,000px brief cut and progressive charts are step 12.

The **step 10** fit math: 8px top padding + masthead 58 + key-metrics strip ≤48 + graph stage
≈735–745 + footer 24 + 8px bottom padding ≤ 900. The exact graph height is computed from the
remaining space; the SVG preserves its aspect ratio and the canvas never scrolls horizontally.

## 5. What Atlas genuinely improves (keep)

1. **Answer-first metric line** — entry, HOTTEST, confidence, alternatives and `× median` in one line.
2. **Explicit accounting** — `19 of 19 reachable files shown · 0 aggregated`, per-column counts,
   `UNREACHED · no forward path · N files`.
3. **Methodology surfaced** — `static cost units (v2) · relative scale` and the structural
   `LOW name-based + MEDIUM semantic` caveat (once, not six times).
4. **Legible nodes** — 10–11.5px names and a uniform metric triple at viewport scale.
5. **Path grammar** — numbered badges, a distinct representative path, a separate unreached zone.
6. **Eight-state coverage, units/anchors, top N% per node, module hulls, unreached column.**

## 6. Defect → action map

| Audit defect | Step 10 (this session) | Later |
|---|---|---|
| F1 canvas clips at 900 | **Fix:** masthead + strip + graph + footer ≤900 | — |
| F2 visual language replaced | **Fix:** G palette/type/masthead; sans chrome + mono data | — |
| F3 rail removed | Decide the rail composition now | **Step 11** implements it |
| F4 page became a report | Do not increase `#map`; 3,000px brief stays out of the first page | **Step 12** cuts `#map` to ~4–5k |
| F5 chrome hygiene | **Fix:** one unit definition, one caveat, one legend, one footer; no clipped text | — |
| F6 leading-dot truncation | **Fix:** mid-ellipsis + full path in `title` | — |
| F7 edge hairball / hue collision | **Fix:** path/selected labels only; neutral back-edge; counts in `<title>` | — |
| F8 two background systems | **Fix:** drop pastel module bands; keep one light hull | — |
| F9 unreached label ambiguity | **Fix:** one `unreached production: N · non-production: M` label | — |
| F10 F visual absent | Decide F as a below-graph candidate now | **Step 12** prototype + founder choice |
| F11 small text | **Fix:** hierarchy + floors (≥10/8.5px); no new sub-8.5px text | — |
| F12 gate blind spot | — | **Step 13** founder gate + G diff + V8 rerun |

## 7. Fresh comparative read (summary)

The independent `@vision` read of the hyperfine composite confirms the audit and adds one useful
point: **G's named masthead is the "whose page is this" beat** that Atlas lacks; Atlas's orange
answer line and node-level metric triple are the improvements worth keeping. Full read:
`VISION-READ.md`. It also flags that the composite's smallest text is below reading size, so
micro-text judgments still need a native-resolution check (not a composite read).

## 8. Weakest assumption / alternative

**Weakest assumption:** that G's palette/type/masthead — not merely its full-page fit and rail — are
what the founder reacted to as "amazing." The audit's alternative is to keep Atlas's cool/slate
system and only fix composition. If the founder prefers that, the fit/strip/hygiene work still
applies; only the token/type layer changes.

**Alternative:** a hybrid (G composition + a new warm-neutral palette) — rejected for now because it
requires a second design pass and the founder's 2026-10-23 directive is explicit: restore G's visual
language.
