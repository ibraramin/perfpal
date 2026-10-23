# Fresh comparative vision read — G vs Atlas v1

**Input:** `docs/evidence/atlas-visual/v2/side-by-side-hyperfine-1440x900.png` (labelled composite:
LEFT = approved V7 G hyperfine; RIGHT = shipped Atlas v1 `#map` first viewport, both 1440×900).
**Date:** 2026-10-23. **Model:** `@vision`. **Read type:** independent comparative read, no labels
given to the reader beyond the composite's own captions.

> **Caveat from the reader:** in-panel micro-text is below reading size at composite scale; G's meta
> rows, rail code tags and both panels' smallest footers could not be verified. Titles, buttons, node
> names, depth headers and metric lines were readable. Native-resolution checks remain necessary.

## LEFT — G visual system

- **Palette:** warm cream/ivory page, white detail cards, near-black navy for the `src/main.rs`
  "ENTRY" node and the primary pill button; one amber/orange accent (title underline, "ENTRY ·
  PRODUCTION PATH", the outlined **HOTTEST** node), red-orange hot-path strokes, muted slate body
  text; color appears only as micro-tags inside rail cards and left-border severity stripes.
- **Typography hierarchy:** one large bold **sans** wordmark `hyperfine` dwarfs everything; sans for
  the orange section label, both buttons, and legend prose; **mono** owns all data — entry path
  `src/main.rs»main`, `DEPTH 0…DEPTH 4` column headers, every node filename, every metric line, all
  rail card content, and the footer. Weight/size order: wordmark ≫ orange label ≈ buttons > node
  filenames > metric lines > footer.
- **Masthead:** left cluster = tiny uppercase eyebrow + wordmark + amber underline, with a two-line
  block beside it (`ENTRY · PRODUCTION PATH` over the mono path); right cluster = two pills — dark
  `Q1 reverse drill: src/quantity/mod.rs` and light `Pause animation (15s)`. Beneath: two dense meta
  rows (cost-unit / reachable / aggregated counts, `structural [LOW name-based + MEDIUM semantic]`,
  alternatives; then legend chips **production · root (1 file) search edge · backbone · hot path**
  plus entry-policy and animation/reduced-motion notes).
- **Legend/footer placement:** legend chips live in the masthead (row 2, top-left). Footer = small
  swatch line bottom-left + a tiny mono line bottom-right; both inside the frame.
- **Detail rail:** right ~40%, two columns of ~12 cards each; every card = mono filename, two mono
  metric lines (`611 static cost units (v2)`, `87× median function`, `top 2% of files`), a colored
  micro code tag, and a colored left border as severity key.
- **Figure/ground and fit:** map fills the upper-left ~55% between thin depth-column rules; rail runs
  full height on the right; a noticeable empty cream band sits lower-left under the map. Everything —
  header, meta, map, rail, footer — fits within 900px; **no clipping**.

## RIGHT — Atlas visual system

- **Palette:** white/near-white page, light gray card borders, dark navy `src/main.rs` entry card,
  amber/orange accent carried by the headline line, the outlined `src/command.rs` card, the dashed
  representative path, and numbered badges (01–04); faint tinted column bands (warm/pinkish toward
  depth 3–4, cool gray behind **UNREACHED**); no red/green micro-tags visible.
- **Typography:** flat hierarchy — **no display type at all**; largest text is the orange headline
  line (small-medium). Sans for meta prose and the bold caveat band; mono for depth headers + file
  counts, node filenames, and the uniform three-line metric block per node. Node filename > metric
  lines; nothing else competes.
- **Masthead:** no title/wordmark anywhere. Top-left = lone `Pause animation` control; then four
  stacked lines: orange answer/metric line, `PATH: 3 steps · representative path (static,
  BFS-derived) · 19 of 19 reachable files shown · 0 aggregated`, a units/legend line, and a bold
  caveat band `structural (LOW name-based + MEDIUM semantic), may over/under-approximate`.
- **Legend/footer:** legend is prose-only with no color swatches; **no footer visible**.
- **Detail rail:** none — metrics are inlined into every node as a fixed triple (`611 static cost
  units (v2)` / `87× median function` / `top 2% of files`); the far right instead carries a dashed
  `+11 non-production unreached` box under `UNREACHED · no forward path · 11 files`.
- **Figure/ground and fit:** five depth columns + UNREACHED span the full width; nodes are much larger
  and fewer (~15–16 visible), spacing generous. **Clipped at the 900px bottom edge:** the last-row
  card `src/export/mod.rs` is cut mid-card; below it no footer, no legend is visible.
- **Answer/metric line:** yes, at the top — orange line reads `ENTRY · production main · confidence
  HIGH · 0 alternatives · HOTTEST: src/command.rs · 611 static cost units (v2) · 87× median function
  · top 2% of files`. No bottom answer line exists in-frame.

## Composition delta

- **Identity:** G opens with a named masthead (`hyperfine` + entry path + two action pills); Atlas
  opens with no title — a control and four text rows, so there is no "whose page is this" beat.
- **Above-the-fold text budget:** G spends 2 meta rows before the map; Atlas spends 4 stacked lines
  including a bold caveat band, pushing the first node row lower.
- **Density model:** G concentrates density in a 24-card right rail and keeps map nodes tiny; Atlas
  spreads density into ~15 oversized nodes and deletes the rail — fewer entities, more pixels each.
- **Chrome duplication:** Atlas prints the same headline numbers twice in-frame (orange top line and
  inside `src/command.rs`), and its units/legend line re-explains what the node triple already shows;
  G states aggregates once in the meta rows and per-file detail once in the rail.
- **Where the eye lands first:** LEFT → wordmark → dark ENTRY node → orange HOTTEST outline → rail.
  RIGHT → orange headline metric line → dark `src/main.rs` card → orange-outlined `src/command.rs`
  with badges 01–04 and the dashed path.
- **Edge treatment:** G = thin depth rules, solid card borders with severity left-stripes, gray
  dashed connectors; Atlas = tinted column bands, a highlighted orange dashed representative path
  with numbered badges, dashed UNREACHED boxes.
- **Truncation:** G terminates cleanly with two footer lines inside the frame; Atlas's content
  crosses the 900px edge mid-card, so the first viewport reads as unfinished.
- **Key coverage:** G encodes legend as colored chips (swatch-based); Atlas encodes it as prose — no
  swatches to decode colors against.

## What G does better that a port must preserve

- The named, answer-bearing masthead: wordmark + `ENTRY · PRODUCTION PATH` + `src/main.rs»main`
  establishes subject and focus in one glance.
- Full first-page fit — header, legend, map, rail, and footer all inside 900px, nothing cut.
- Separated detail rail: deep per-file metrics stay available without inflating the map.
- Swatch-based legend chips (production / root search edge / backbone / hot path) that key directly
  to the marks.
- Restraint of figure/ground: small nodes, thin column rules, and the empty lower-left band keep the
  map legible as a map; grouped top-right controls.

## What Atlas genuinely improves and should be kept

- The single orange answer/metric line: `confidence HIGH · 0 alternatives · HOTTEST:
  src/command.rs · 611 static cost units (v2) · 87× median function · top 2% of files` puts the
  conclusion first.
- Explicit accounting above the fold: per-column counts (`depth 1 · 6 files`, `UNREACHED · no forward
  path · 11 files`) and `19 of 19 reachable files shown · 0 aggregated`.
- Methodology surfaced, not buried: `static cost units (v2) · relative scale` and the bold
  `structural (LOW name-based + MEDIUM semantic), may over/under-approximate` caveat.
- Larger nodes with a uniform metric triple — the numbers are readable at viewport scale where G's
  tiny nodes require the rail.
- Clear path grammar: numbered badges (01–04), the orange dashed representative path, and a visually
  distinct UNREACHED zone with `+11 non-production unreached`.

## Visible defects in the composite

- In-panel micro-text is below reading size at composite scale; only titles, buttons, node names,
  depth headers, and metric lines were confidently readable.
- Captures are not in equivalent animation states: the left frame is complete with footer inside
  900px, while the right frame's content is cut mid-card at the bottom edge and only the right frame
  exposes an animation control at top-left (G's is top-right) — a placement/state mismatch.
