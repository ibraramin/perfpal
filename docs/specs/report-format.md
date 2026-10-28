# Spec: report-format

Scope: feature

# Spec: report-format (feature)

**Status:** proposed (design lock for the `polish-report` program)
**Owns:** the contract for every human/agent-facing artifact Perfpal emits into `perfpal-report/` —
roles, rank keys, formatting, Atlas composition, display scaling, and the in-report Atlas story
(ADR-0014).
**Does not own:** the metrics schema (`docs/schema/metrics.schema.v1.8.json`), the analyzer, or the
standalone prototype player in `sim/` (evidence/reference only).

## 1. Principles

1. **One audience per artifact.** Every emitted file names its consumer. No file exists "because we
   already had it". No two artifacts carry the same complete list.
2. **One ranking.** All action artifacts use the shipped KV1 order; all function/profile artifacts use
   KV3. No renderer may re-rank.
3. **No silent caps.** Every bounded view prints `N of M` and names the cap in
   `data_quality.caps`; the complete list lives in exactly one machine contract
   (`agent-actions.json`).
4. **Canonical data once.** `metrics.json` is the only data source. Markdown/HTML renderings are pure
   functions of it; they never introduce facts.
5. **Static means static.** No network, no telemetry, no external assets, no timestamps/durations in
   byte-deterministic surfaces, no absolute paths except the verbatim CLI `root_path` provenance.
6. **The map is the product.** The Atlas section answers "where do I look first, what breaks if I
   touch it, where might time go"; everything else supports or drills into that answer.

## 2. Artifact roles (binding matrix)

One row per emitted filename; the matrix is the source of truth for "who opens this, and what do
they do with it". An artifact without a consumer is deleted, not kept.

| Artifact | Audience | Consumer / action | Unique role | Cap | Canonical? |
|---|---|---|---|---|---|
| `metrics.json` | tools / LLM / downstream | parsed and validated by tooling | the only data source; every rendering is derived from it | none | **yes** |
| `agent-actions.json` | agents / tools | machine action execution and inspection | complete action list (uncapped) + profile top 100 (`profile_queue_cap`) | actions uncapped; profile 100 | complete action list |
| `agent-report.md` | LLM / human triage | read without loading the full report | bounded digest: top 25 actions, per-rule rollup, top 25 profile rows, limits | 25 / 25 / 100 | no (digest) |
| `report.md` | human | read the narrative + Atlas summary, follow links | narrative report + Atlas summary + methodology; **no full findings appendix** | summary caps named | no |
| `report.html` | human | map-first browsing; click-to-filter | designed Atlas-first page + always-on story (self-contained; fixed report script + fixed story script + inline JSON data) | summary caps named | no |
| `findings-detail.html` | human | opened from `report.md` §4 / `#ranked-actions` for the complete ranked list and per-finding rows; filter | **canonical complete human detail**: ranked actions + all findings, filterable via the fixed `REPORT_SCRIPT` | none | human detail |
| `profile-queue.{md,html}` | human | complete KV3 drill from the key views | complete KV3 function list (cost-ranked, honest badges) | none | KV3 detail |
| `atlas-weight.html`, `atlas-q2.html` | human | drill from the Atlas charts | drill pages: complete per-metric weight / blast-radius detail | complete per metric | no |
| `charts/*.svg` | human / diff | visual overview; never parsed | deterministic chart stems (workspace/file/entry/coupling/hot/bands/findings/component-*), `graph.dot` | top-50 / named | chart data |
| `metrics.schema.v1.8.json` | tools | offline validation without the repo | shipped schema copy; discovered by mapping `schema_version` → filename | n/a | mirrors `src/schema/` |

Rules:
- `report.md` Appendix C (full findings) is **removed**; §4 instead carries
  `All <N> of <M> findings: [findings-detail.html](findings-detail.html) — complete ranked action
  list and per-finding detail (<R> rules).` Section order is 1..5, Appendix A, Appendix B.
- `findings-detail.md` is **dropped** (decided H3, 2026-10-28): no concrete consumer for a Markdown
  mirror was ever named, so the `.md` duplicate is not emitted. `findings-detail.html` is the
  canonical complete human detail; the fixed artifact set is **9 files** (output-contract §2).
- The schema copy is **kept** (decided H3): contractually required — an offline consumer validates
  `metrics.json` without the Perfpal repo, and discovery maps `schema_version` to the filename
  (`"1.8"` ⇒ `metrics.schema.v1.8.json`).
- The 25-action / 25-profile caps are unchanged (ratified D1, `docs/specs/agent-report.md:27`); every
  capped view prints `N of M` and names the cap in `data_quality.caps`. `agent-actions.json` remains
  the complete machine contract.
- Remaining MD/HTML pairs (`report.{md,html}`, `profile-queue.{md,html}`) are intentional format
  variants of the same content — they must be generated from the same section model, never
  hand-written twice. `findings-detail.html` is HTML-only by decision.

## 3. Rank keys and caps

- **KV1 (actions)** — verbatim shipped order. `agent-actions.json.actions` is complete; every Markdown
  view that shows fewer must print `showing top N of M actions; the complete list is
  agent-actions.json` (or the per-artifact equivalent).
- **KV3 (profile)** — verbatim shipped order. `profile-queue.*` is complete; JSON profile is capped at
  100 with the cap named in-file.
- **Cap policy:** 25 detail actions / 25 profile rows in `agent-report.md` is ratified D1
  (`docs/specs/agent-report.md:27`). Rationale: an inline complete index is ~170k tokens on a
  7.5k-finding scan; the digest must stay loadable. Keep 25 as the default; do not add a flag unless a
  real consumer asks. Disclose the cap and the full-list pointer in the header, not only the footer.

## 4. Formatting contract

One style contract, shared by every Markdown/HTML artifact (implemented H3, 2026-10-28):

- **Palette (one palette, one token source for the shell).** HTML artifacts use the Atlas warm-paper
  tokens only — paper `#f4f1ea`, ink `#202b35`, muted `#6f6a61`, rule `#d6d0c4`, card `#fffdf8`,
  accent `#7c2d12`, table head `#ece8df`, zebra `#fbf9f4`. The report-family shell declares them once
  as constants and CSS custom properties (`src/reporter/format.rs`), and the Atlas drill pages consume
  those constants; the Atlas `#map` renderer's CSS/SVG embeds the same literal values in its own
  block. There is no second palette or alternate value for any token.
- **Typography:** sans-serif body; monospace reserved for code and paths; one heading scale (H1
  artifact title, H2 numbered top-level sections, H3 subsections, H4 row labels; no skipped levels,
  and no heading-shaped content generated from user data — escape or fence it); tabular numerals
  for counts and costs.
- **One HTML shell.** `report.html`, `findings-detail.html` and `profile-queue.html` share one
  head/style/header/table markup; detail pages do not fork the report's CSS and carry exactly one
  fixed `REPORT_SCRIPT` (the filter/pan-zoom enhancement). The Atlas drill pages
  (`atlas-weight.html`, `atlas-q2.html`) consume the same palette constants and type rules but keep
  their drill-specific markup and CSS, and remain script-free.
- **Escaping:** every interpolated path/title/message passes the shared escaper
  (`escape_md`/`escape_html` — no per-artifact escaper). Markdown renderers structure-escape
  backticks/pipes/headings and flatten CR/LF/TAB to `\r`/`\n`/`\t`; agent-report code-span values
  neutralize `<script` / `</script` to `&lt;script` / `&lt;/script`, while `report.md` /
  `profile-queue.md` backslash-escape `<` / `>`. A hostile scanned value therefore cannot inject a
  line, section or executable script tag anywhere in a Markdown artifact; HTML renderers escape
  `&<>"'`.
  Quoted analyzer provenance is verbatim in JSON, escaped in Markdown/HTML (JSON keeps the original
  bytes).
- **Tables:** same column conventions (rank, location, class, cost/why), same alignment, same
  `<table>`/`<thead>` markup in HTML, same `N of M` header note. Links use one style (relative
  paths, `#anchors`, no trailing punctuation).
- **Empty states:** every artifact renders a named stub (e.g. `no findings recorded`), never a blank
  section or a broken table. `profile-queue.md` gains the `## Complete cost-ranked list` section and
  renders `No functions recorded.` when the queue is empty.
- **Section order:** fixed per artifact and asserted by tests. Markdown and HTML siblings render the
  same sections in the same order.
- **Determinism:** no clock, no random, no set iteration; sorted traversals; byte-identical output for
  the same inputs on repeated runs (the existing determinism suite is the arbiter).
- **Style-contract test:** `tests/style_contract.rs` asserts the heading order per artifact,
  hostile-input escaping, and the table/link/empty-state contract; `tests/artifact_matrix.rs`
  asserts the role matrix, one owner per content class/format, cap disclosures and link consumers.

## 5. Atlas composition (report.html)

Answer-first order, top to bottom:

1. Header: schema/version + file/finding counts (no clock).
2. **Answer line** (restored): one sentence naming the elected entry, why, the largest static-cost
   file, the single first action, and the structural caveat.
3. **Atlas map**: canvas + detail rail (current step-11/12 composition), then the four charts
   (weight, blast radius, file detail, coverage).
4. **Ranked actions** (restored): top KV1 actions with severity/confidence, path class, anchor,
   why/trip provenance, same-location dedupe; link to the complete list.
5. Components → files → findings (group-and-fold), with honest `N of M` notes.
6. Footer: noise/confidence detail restored (`files_without_functions`, `partial_parse_files`, plus
   the existing seven phrases), caveats.

`#explorer` is **removed from the report's main flow**. The eight legacy chart stems move to a
separate `explorer.html` (breadcrumb back to `report.html#map`), linked once from the footer or the
coverage line. No chart is deleted from `charts/`.

**Branch parity (binding).** The pre-merge `feat/atlas-sim` report (schema 1.7) exposed seven
`data-atlas-block` surfaces — `answer`, `actions`, `weight`, `blast`, `cards`, `noise`, `links` —
plus the inline `atlas-drill-q2` / `atlas-drill-weight` sections and a 1400px graph-only hero. Every
one must have a main equivalent after H1: rendered in place, or exposed through an equivalent linked
surface with the same content. Nothing branch-authored may be silently dropped. Main's rail and four
charts stay as additions; they must not replace a branch block.

| Branch block | Required main equivalent |
|---|---|
| `answer` | one-sentence answer line (entry, why, largest cost, first action, caveat) |
| `actions` | ranked-actions block: top KV1 rows with severity/confidence, path class, anchor, why/trip, dedupe |
| `weight` | weight chart **plus** the branch share/coverage prose (not chart-only) |
| `blast` | blast-radius chart **plus** the branch dependent/coverage prose |
| `cards` | file cards with per-function list + dominant finding (rail stays as a summary) |
| `noise` | noise/confidence disclosure incl. `files_without_functions` / `partial_parse_files` |
| `links` | drills block linking the complete lists and drill pages |
| hero | full-width canvas (fluid; rail below or beside at wide widths) |
| inline drills | inline collapsible sections **or** the existing separate pages with a prominent link block — content-equivalent, decided in H1 |

**Story integration (ADR-0014; founder decision 2026-10-27).** The interactive story is **always
embedded** in `report.html`; ADR-0012 Decision 5 is superseded. The amended A12 script contract is
exactly three elements: the unchanged fixed `REPORT_SCRIPT`, one fixed `STORY_SCRIPT`, and one
inline `<script type="application/json">` data block. The player is self-contained (no fetch/HTTP,
no eval/`Function`, no `setTimeout`/`setInterval`, no randomness, no `innerHTML`), scoped under `#atlas-story`, starts at
the final frame under reduced motion, prints the final frame, and works from `file://`. The JS-off
fallback is the complete static Atlas canvas (canonical final frame) plus a `<noscript>` beat
narrative and drill links — no second SVG, no raster. The story payload is projected once in Rust
from the typed document/Atlas blocks; `sim/` remains the evidence prototype. `--story` no longer
gates the page; it only emits `atlas-layout.json` and must leave every shared artifact byte-identical.
F6 is redefined as flag purity (F6a), double-render identity (F6b) and inline-data integrity (F6c).
Measured budget (1440×900, `file://`, median of 3): `report.html` ≤ 384 KiB at hyperfine
(134 KB baseline, 108 ms FCP), ≤ 320 KiB at cJSON (105 KB, 128 ms), ≤ 512 KiB at the 3k fixture
(109 KB, 108 ms); story-on FCP ≤ 300 ms (≤ 400 ms at 3k). Any breach triggers the explicit
fallback: a self-contained `story.html` linked by a constant pointer, with the blocker recorded in
ADR-0014. Do not ship a fetch-based report.

## 6. Resolution and display scaling

**Shipped behaviour (H2).** The report is responsive by default. `body` uses a fluid max width
(`min(100%, 100vw)`) and fluid padding; `#map` keeps its full-bleed band with fluid gutters
anchored to the page body (never `100vw`), so a classic scrollbar cannot create a horizontal
overflow; the
Atlas canvas stays `width:100%` with the preserved `868`-wide viewBox; the four Atlas chart blocks
fill their grid; and the map/story chrome uses a `clamp()` type ramp anchored so **1440 px renders
the H1 sizes** and **3072 px caps at 1.5×**. The always-on Atlas story is full-width too: `.page`,
`.story-head` and `.canvas-shell` follow the page width, and the story canvas keeps its
`1400/718` aspect ratio. Linked SVG figures (the legacy Explorer stems) scale down with
`max-width:100%` but are **not upscaled past their intrinsic width** and are centred — upscaling
them would turn the page into a multi-thousand-pixel vertical stretch without adding information.

**Optional fixed width.** `--report-width <auto|1280|1440|1920|2560|3072>` (default `auto`) is a
public, layout-only render hint. It changes only `report.html` CSS: the `--report-max` design width
and the `data-report-width` breakpoint marker. Pinned widths use their own design width for the
type ramp, the rail placement (1280 stacked, 1440 beside, ≥1920 below) and the gutter. Data, the
inline story payload, scripts and every other artifact stay byte-identical. It is not a scan
setting and never detects the scanning machine's display.

**No scan-machine resolution detection as the default.** A report is a static file opened later,
possibly on another machine; baking the scanning display's resolution would be wrong more often
than right. A future "match my display" convenience would be an explicit separate decision.

**High-DPI.** All output is vector; charts and the canvas stay DPR-independent (no rasterization,
no `image-rendering` hacks). Acceptance runs at DPR 1 and 2.

**Page height (A8 re-baseline, 2026-10-28).** The first-page fold target is unchanged: the answer
line must end above the fold at every supported width (measured bottom: 427 px at 1280/1440, 428 at 1920,
424 at 2560, 436 at 3072). The document-height budget is re-baselined with evidence on the hyperfine fixture:
16,927 px at 1440 and 21,241 px at 3072 (H1: 18,747 / 20,646); the story section is 1,029 px at
1440 and 1,875 px at 3072 (full-width canvas). The 1440 height is lower than H1 because the
legacy Explorer pan/zoom figures now downscale to their figure (no internal overflow); the 3072
growth is the full-width story canvas. Wider layouts grow with content, not by stretching legacy
figures; cJSON and the 3k fixture are recorded in
`docs/evidence/polish-report/H2/PAGE-HEIGHT.md`.

**Acceptance.** Render and review at 1280, 1440, 1920, 2560 and 3072 CSS px at DPR 1 and 2:
no horizontal scroll (especially 1280), the Atlas SVG fills its container, no clipped SVG labels,
the story boots to the final summary beat, and the JS-off fallback keeps every static node visible.
`@vision` signs off at 1440 and 3072.

## 7. Invariants (must not break)

- Schema stays 1.8 unless a deliberate contract change is approved (prefer none).
- A12 as amended (ADR-0014): exactly three script elements in `report.html` — the unchanged fixed
  `REPORT_SCRIPT`, the fixed `STORY_SCRIPT`, and one inline `application/json` data block; no
  fetch/HTTP, no eval/`Function`, no `setTimeout`/`setInterval`, no randomness; story data inline; JS-disabled static
  fallback (canonical canvas + `<noscript>` narrative); `--story` changes no shared artifact.
- No network/external assets; no telemetry.
- Byte-determinism: repeated runs are byte-identical on deterministic surfaces.
- Output-contract §2 layout: fixed filenames; any removal/merge is an explicit contract change with
  golden regeneration and ADR/spec update.
- Static analysis stays framed as optimization candidates, not ground truth.

## 8. Acceptance tests

1. Artifact matrix (`tests/artifact_matrix.rs`): every emitted filename has a role entry; one owner
   per content class/format; no two artifacts carry the same complete list; every cap is printed and
   named in `data_quality.caps`; every link consumer resolves to an emitted artifact.
2. Style contract (`tests/style_contract.rs`): heading order per artifact, hostile-input escaping,
   and the table/link/empty-state contract.
3. Atlas composition: answer line → map → ranked actions → components/files/findings order asserted;
   `#explorer` absent from `report.html`; `explorer.html` exists and links back.
4. Resolution: render at the five widths above; assert no horizontal overflow at 1280, canvas fills
   its container, and the page is usable with JS disabled.
5. Determinism: existing byte-determinism suite green; no `preserve_order`.
6. Gates: `cargo fmt`, `cargo clippy`, `cargo test`, `cargo deny check licenses`, semantic
   clippy/tests, story gate (`sim/gate.sh`), golden regeneration.
7. Vision: `@vision` APPROVED at 1440 and 3072 before any formatting claim is considered done.

## 9. Open decisions (H1/H2/H3)

- **Decided (founder, 2026-10-27; ADR-0014): include the interactive story in-report, always
  embedded.** `--story` is layout-artifact-only and must not change shared artifacts. The fallback
  (self-contained `story.html` + constant link) is explicit if a precondition or budget fails.
- Inline drill sections vs the existing separate drill pages (content-equivalent): **decided in H1 —
  keep the separate pages plus a prominent links block** (`answer`/`links` blocks link them).
- **Decided (H1, 2026-10-27):** the canvas is full-width; the detail rail sits beside it at 1440 px
  and below it at ≥1600 px (stacked below 1360). Recorded in ADR-0014 and the vision evidence.
- **Decided (H3, 2026-10-28): drop `findings-detail.md`.** No named consumer existed for a Markdown
  mirror; `findings-detail.html` is the canonical complete human detail (ADR-0015).
- **Decided (H3, 2026-10-28): keep the shipped schema copy.** Contractually required for validation
  without the repo, and for `schema_version` → filename discovery.
- **Decided (H3, 2026-10-28): one warm-paper style contract for all human artifacts.** The shared
  HTML shell uses the Atlas palette and typography (§4); the `.md` artifacts share the same heading
  scale, escaping helpers, table/link conventions and named empty-state stubs.
- **Decided (founder, 2026-10-28):** `--report-width` is public in v0.4.x — documented in
  `--help`, the README CLI table and output-contract §2; the accepted values are
  `auto|1280|1440|1920|2560|3072`. It is layout-only and changes no shared artifact.
- **Decided (H2, 2026-10-28):** the A8 first-page fold stays "answer line above the fold" (measured
  ≤ 436 px at every width); the document-height budget is re-baselined per fixture at 3072 with
  evidence (`docs/evidence/polish-report/H2/PAGE-HEIGHT.md`), and legacy linked SVG figures are not
  upscaled.