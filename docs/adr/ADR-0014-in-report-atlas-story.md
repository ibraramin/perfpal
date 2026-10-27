# ADR-0014: The Atlas Story is embedded in `report.html`

- **Status:** Accepted (founder decision 2026-10-27)
- **Date:** 2026-10-27
- **Supersedes:** `ADR-0012` Decision 5 (and its Decision 1 stance that the story does not enter
  `report.html`). Amends **A12** (the report's script budget) as stated below.
- **Deciders:** Founder + orchestrator; counter-case by `@oracle`
  (`ses_e83041a08ffeySV4Bsj8K42tul`).
- **Gates:** this ADR changes the report contract, not the metrics schema. `layout_version` and
  `flow_version` stay **1**; schema stays **1.8**; no new dependency; no network.

## Context

- `report.html` today carries exactly one fixed `REPORT_SCRIPT` (`src/reporter/mod.rs:1793-1832`,
  emitted at `:2515-2528`) and the Atlas canvas/SVG artifacts carry zero scripts and zero handlers
  (A12; `D-pf-243`). The report is a single offline file and byte-deterministic.
- The Atlas Story (`sim/index.html` + `sim/sim.js` + `sim/sim.css`) renders the inferred run
  entry → stages → loops → I/O → stop over `atlas-layout.json` + `metrics.json`. S4 evidence is
  strong on mechanism: 897 tests, `sim/verify.py` 304/304, 3k scale 44/44, final gate `71d0d04`
  (`docs/evidence/atlas-sim/GATE.txt`).
- ADR-0012 kept the story a **companion**: its D1 says it does not graduate in v1, D2 says A12 is
  unchanged, and D5 makes a self-contained artifact a precondition, not work. The founder reversed
  the presentation choice on **2026-10-27**: the interactive story is **included in the report**.
  The *engineering* preconditions in ADR-0012 D4(c) remain binding (self-contained, deterministic,
  offline, complete final frame, pause/reduced-motion/print), and this ADR records how they are met.
- The founder also decided, in the same session, that **the story is always embedded** (not gated by
  `--story`), so every shipped report carries it.
- **Oracle counter-case (recorded, not adopted).** `@oracle` argued for shipping a self-contained
  `story.html` first, linked from the report by a constant pointer:
  1. `report.html` must keep exactly one script — the existing test `tests/atlas_render.rs:135-161`
     and `tests/report_html.rs:236-309` assert it; embedding a player forces a contract change.
  2. F6 (`tests/pipeline_e2e.rs:492-535`, `:777-833`) proves `--story` is purely additive: the
     page is byte-identical story-off vs story-on. Inlining retires that guarantee.
  3. A measured **+152 %** report size on the hyperfine fixture (`report.html` 134 KB baseline;
     the story adds ~58 KB of player JS + ~24 KB CSS + the per-scan payload).
  4. `sim.js` is **1,467 lines / 58,261 B** of DOM-heavy player code entering the report's
     audit surface.
  5. Page-height risk: the story is a second full-view canvas below an already tall Atlas page.
  The oracle's conclusion was: keep the report small and auditable; graduate only after a
  self-contained companion exists. The founder weighed reachability above size and chose in-report.
  The fallback below preserves the oracle path if a precondition fails.

## Decision

1. **The story is always embedded in `report.html`.** Every report gains one story section after
   the Atlas charts. `--story` no longer toggles the page; it continues to emit
   `atlas-layout.json` for tooling/companion use and is otherwise side-effect-free on the report.
2. **A12 is amended to exactly three script elements in `report.html`:**
   1. the existing fixed `REPORT_SCRIPT` (unchanged bytes and behaviour);
   2. one fixed `STORY_SCRIPT` whose source is committed in the repository (minified or not is a
      build detail; it is a constant, not generated per scan);
   3. one inline `<script type="application/json" id="atlas-story-data">` carrying that scan's
      payload (no JavaScript execution; not a data fetch).
   No other `<script>` element, no `<script src>`, no `<link>` to an external asset. The Atlas
   canvas/SVG artifacts themselves still add **zero** scripts, and no inline `on*` attributes are
   added. The fixed `STORY_SCRIPT` may attach event listeners programmatically; that is the player,
   not an Atlas artifact handler.
3. **Self-contained and offline.** No `fetch`, `XMLHttpRequest`, `import()`, WebSocket, `eval`,
   `new Function`, `Date`, randomness, `setTimeout`/`setInterval`, or `innerHTML`/`outerHTML`/
   `document.write` in the story script or data. The payload is inline; the page works from
   `file://`. The data channel is parsed once on load. The existing `REPORT_SCRIPT` banned-API
   scan is scoped to the *executable* scripts, not to JSON data (rule IDs in data are content,
   not code).
4. **The story payload is produced once, in Rust, from typed data.** A new
   `src/reporter/atlas_story.rs` projects `&Documents` + `AtlasView` + `Flow` + `Purpose` +
   `ProfileQueue` + `SceneBlock` into the player's camelCase shape (the transform that
   `sim/from-layout.py` performs today). The CLI builds one `StoryBundle` (view, flow, purpose,
   profile, scenes, payload, `atlas-layout.json`) and passes it to `render_html`; `render_html`
   stays a pure function of its inputs and performs no filesystem I/O. `from-layout.py` remains
   the prototype/evidence generator, not a build step. Determinism: all grouping uses ordered
   maps; no set iteration; no clock.
5. **JS-disabled fallback = the complete static final frame already in the page.** The Atlas canvas
   SVG is the canonical complete static frame (`render_canvas_svg`,
   `src/reporter/atlas_render.rs:1334-1340`; `static_svg_is_final_frame_without_animation`). The
   story section renders, without JavaScript:
   - a `<noscript>` narrative of the same run (entry → stages → loops → I/O → stop, plus the
     stop reason and the first action), and
   - direct links to the drill pages and the complete lists.
   No second `<svg>` is inlined; no raster fallback. With JS disabled the report remains fully
   usable and the map remains the final frame.
6. **Scoped playback.** The story keeps play/pause/step/scrub/speed/select, starts at the final
   frame under `prefers-reduced-motion: reduce`, prints the final frame, and is scoped under
   `#atlas-story` so its CSS cannot restyle the Atlas page. At 1280 CSS px the page has no
   horizontal scroll. Two renders of the same scan are byte-identical.
7. **F6 is redefined, not dropped.**
   - **F6a (flag purity):** with the story always embedded, `--story` must change **no shared
     artifact** — `report.html`, `metrics.json` (after masking `scan.started_at`,
     `scan.duration_ms`, `scan.root_path`), `agent-*`, findings/profile artifacts are byte-identical
     with and without `--story`. Only `atlas-layout.json` appears.
   - **F6b (double-render identity):** two scans of the same input produce byte-identical
     `report.html`, including the inline data and story script.
   - **F6c (data integrity):** the inline JSON parses; its `source.sha256` matches the emitted
     `atlas-layout.json`; `source.metricsContentSha256` matches the canonical metrics projection;
     the story section and all three script elements are present in every report.
8. **Measured size / first-paint budget.** Baselines measured 2026-10-27 at 1440×900, headless
   Brave, `file://`, median of three (SHA `020c48a`):

   | Fixture | `report.html` baseline | FCP baseline | Story-on ceiling (bytes) | Story-on FCP ceiling |
   |---|---|---|---|---|
   | hyperfine (`c516152`) | 133,985 B | 108 ms | **≤ 393,216 B (384 KiB)** | **≤ 300 ms** |
   | cJSON (`6d9f244`) | 105,243 B | 128 ms | **≤ 327,680 B (320 KiB)** | **≤ 300 ms** |
   | 3k fixture (`gen-3k.py`) | 108,752 B | 108 ms | **≤ 524,288 B (512 KiB)** | **≤ 400 ms** |

   The ceilings are baseline + player (58 KB) + CSS (24 KB) + the projected payload, with headroom.
   Exceeding a ceiling, or failing any of F6a–F6c, triggers Decision 10.
9. **Page-height / composition.** The seven restored branch blocks, the full-width hero and the
   rail-below-at-≥1600 px composition are H1 scope; the story sits after the four charts, before
   the footer. Page height is measured at 1440 and 3072 and recorded with the vision evidence; the
   A8 fold target applies to the first page (header → answer → canvas), not to the whole story.
10. **Fallback (explicit).** If the self-contained player, JS-off fallback, determinism, F6a–F6c or
    the budget cannot be met, **stop** and ship a self-contained `story.html` linked from
    `report.html` by a **constant pointer**; keep `report.html` at one fixed script (A12/F6
    unchanged). Record the blocker and the measured numbers in this ADR's addendum. **Never ship a
    fetch-based report.**
11. **No schema, dependency, layout-version or flow-version change.** No telemetry. No external
    assets. The story remains a temporal companion to the map, not a second map: no second metric
    encoding, no runtime claim, no time unit (`atlas-story.md:27-35,71-83`).

## Consequences

**Positive**
- Every user reaches the story from the shipped artifact; no companion server, no CLI subcommand.
- One Rust payload projection is the single source of truth; the Python prototype stays evidence-only.
- The report remains offline, deterministic and auditable; the no-network/no-eval bans are enforced
  against executable code.
- The JS-off page is still the complete Atlas answer; the story is additive.

**Negative / mitigated**
- `report.html` grows ~2.4–3× at hyperfine and ~2.4× at cJSON, and carries 58 KB of player code into
  the audit surface. Mitigation: fixed budgets (Decision 8), minified fixed script, payload filtered
  to drawn paths, no duplicate `atlas-layout.json` copy inside the page.
- The single-script lock becomes a three-element lock; tests must assert the exact set and the fixed
  sources. Mitigation: F6a–F6c + structural tests, and the banned-API scan scoped to executable code.
- Page height grows below the map. Mitigation: story starts at the final frame under reduced motion;
  print prints the final frame; the A8 fold target is unchanged.
- Two renderers exist (Rust projection, Python prototype). Mitigation: `sim/verify.py` remains the
  prototype gate; a Rust golden pins the projected payload against a known fixture.

## Gates

- Report: F6a–F6c; structural parity (seven `data-atlas-block` values + story); no
  `fetch`/XHR/`eval`/`http://`; JSON parses; JS-off fallback present; double-render identity; 1280
  no-hscroll.
- Story prototype: `bash sim/gate.sh` (304/304 + 3k scale). The standalone `sim/` player is not
  the shipped surface; it remains the reference implementation.
- Full CI gate + semantic features as in `AGENTS.md`.
- Vision: `@vision` at 1440 and 3072; panel R1/R2/Q1/Q2.

## References

- `docs/adr/ADR-0012-atlas-story-companion.md` · `docs/adr/ADR-0013-atlas-branch-reconciliation.md`
- `docs/specs/report-format.md` §5, §7, §9 · `docs/report/output-contract.md` §2
- `docs/specs/atlas-story.md` · `docs/specs/atlas-layout.md` · `docs/specs/atlas-feature.md` A12
- `src/reporter/mod.rs:557,1793,1972,2515` · `src/reporter/atlas_render.rs:1334,1483,1573`
- `sim/index.html` · `sim/sim.js` · `sim/sim.css` · `sim/from-layout.py` · `sim/verify.py` · `sim/gate.sh`
- `docs/evidence/atlas-sim/GATE.txt` · `docs/evidence/merge-m3/gate.txt`
- Oracle counter-case: `ses_e83041a08ffeySV4Bsj8K42tul` (recorded in Context).
