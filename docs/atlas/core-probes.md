# Core probes — the data Perfpal must emit for Atlas Story

- **Status:** DRAFT (2026-10-24) · **Owner:** orchestrator · **Seal:** founder
- **Spec:** `docs/specs/atlas-story.md` · **Track:** Atlas Run
- **Purpose:** every non-visual data need of the story, recorded here so the real probe can be
  implemented in Perfpal core later and so the prototype branch merges cleanly.

## Rules (so merging later is easy)

1. **One probe = one commit**, message `core(<id>): <what>`, touching the smallest file set.
2. **New files before edits.** Edits to existing code are limited to the emitter registration and the
   schema declaration; never reformat or reorder untouched code.
3. **Additive only.** No existing field is renamed, retyped, reordered or repurposed. The `flow` block
   is optional and omitted unless story output is requested → existing `metrics.json` goldens stay
   byte-identical (acceptance F6).
4. **Goldens:** add new fixtures; never regenerate existing goldens to mask a diff.
5. **Each probe row records:** commit sha · `git show --stat` file list · emitted field · fidelity ·
   known limits · what the real implementation must do.
6. **Any change to an existing field stops the work** and needs an ADR first.
7. No new dependency. No network. Byte-determinism preserved (sorted arrays, no map iteration).

## Evidence that constrains the design

- Static "execution order" without a trace has no successful prior art: Jeliot required an
  instrumented interpreter; Replay.io requires recorded execution; CodeCity/Gource animate structure
  or VCS history, not execution. Sequence/DFD views scale only per-slice at 1k+ nodes.
- Static dataflow tools that ship are explicit about over-approximation: Joern taints all-args to
  all-args for unknown externals; Semgrep documents no path-sensitivity; CodeQL splits local vs global
  flow. None claim observed values.
- Therefore: `fidelity: "syntactic"`, per-edge `type-only` / `literal-observed` / `unresolved`, no
  weights from counts.

## Registry

| # | Probe | Emits | Site (prototype) | Fidelity | Status |
|---|---|---|---|---|---|
| P1 | call-site payload | `flow.calls[]`: from, to, callee, line, args[{expr,type,dir,named}], unresolved | `src/analysis/flow.rs` (new) | syntactic + local type table | not started |
| P2 | I/O effects | `flow.io[]`: op read/write, target file/env/net/stdio, literal | same module | call-name + string-literal match | not started |
| P3 | signature summary | `flow.signatures[]`: params[{name,type,dir}], ret, unresolved[] | same module | types as written; macros/generics opaque | not started |
| P4 | stage order | `stages[]`: order, label, files, depth, leaf | `src/reporter/atlas_layout.rs` (derived, no new parse) | derived | **prototype** |
| P5 | layout emission | `atlas-layout.json`: canvas, columns, nodes, edges, hulls, aggregates, cap chips | `src/reporter/atlas_layout.rs` + `pub(crate)` layout helpers in `atlas_render.rs` | exact (same math as the SVG) | **prototype** |

### P1 — call-site payload

- **Question:** what value/type crosses this file boundary?
- **Prototype:** for every call edge already in the call graph, walk the call expression in the caller,
  record the callee's resolved id (existing resolver), the verbatim argument source slice, its written
  type when a local signature is available, and a `named` payload when the argument is a single named
  variable/field (`config`, `metrics`).
- **Must not:** guess a type from a name; resolve indirection; merge two call sites with different args.
- **Real implementation:** points-to / type inference for C function pointers and Rust `dyn`/closures;
  macro expansion. Prototype leaves those `unresolved: true` and the story greys them.

### P2 — I/O effects

- **Question:** which files, env vars and endpoints does this stage read or write?
- **Prototype:** call-name match on a fixed, versioned table (`fopen`, `open`, `read`, `write`,
  `std::fs::*`, `File::open`, `std::env::*`, `connect`, `TcpStream`, `reqwest`), plus literal
  path/URL when the argument is a string literal.
- **Must not:** classify a wrapper as I/O by name alone; claim a runtime path; show a literal that is
  built by formatting.
- **Real implementation:** syscall-level probes (post-beta, D3), or a curated interprocedural summary.

### P3 — signature summary

- **Question:** what does this function take and return, in what direction?
- **Prototype:** tree-sitter parameter list + return type as written; `&mut`/pointer direction; `dir`
  = in / out / inout.
- **Must not:** expand macros; resolve `impl Trait` or generic bounds to concrete types.

### P4 — stage order

- **Question:** in what order does the inferred run move?
- **Prototype:** derive from existing depth + path steps + entry election; no new parse. Emit stage
  labels and the ordered file list.
- **Must not:** claim runtime order; stages are a static slice, labelled as such.

### P5 — layout emission

- **Question:** where is each node, so the story does not re-implement layout?
- **Prototype:** emit the exact geometry the SVG renderer already computes (x/y/w/h, column x,
  edge path `d`), and the cap/aggregate boxes.
- **Must not:** drift from the SVG. One layout function, two outputs.
- **Why:** without it the story must either scrape HTML (current D1 shortcut, rejected) or fork the
  layout in JS (drift). This is the single most important probe for mergeability.

## What the D1 prototype faked (and where it stands after S1)

| Fake | D1 | S1 status |
|---|---|---|
| Geometry | `sim/extract.py` parsed `report.html` | **resolved** — P5 `atlas-layout.json`, HTML extraction deleted |
| Story data | `extract.py` read rendered text | **resolved** — P4 + `AtlasView` |
| Data movement | absent | **still absent** — P1 + P2 + P3 (S2/S3) |
| Scene aggregation | none (19-node canvas only) | **still absent** — S4 |

## S1 record (2026-10-24) — P4 + P5, status `prototype`

- **Commits:** `5faacb7` (core), `deb2cc2` (sim + docs), plus the post-panel geometry fix
  (`git log --grep 'panel blockers'`). P4 and P5 share one emitter and ship together — a **recorded
  deviation** from the one-probe-per-commit rule; neither is independently useful.
- **Emitted by:** `perfpal <repo> -o <dir> --story` → `atlas-layout.json` (story-only; no other
  artifact changes). The target entry point `perfpal story <repo>` is S4; until then the flag is the
  shipped interface (recorded deviation).
- **Fields:** `canvas`; `columns[]` (depth, **x = the SVG header centre**, width, label, count_text,
  unreached, folded, file_count, nonproduction_unreached); `hulls[]`; `nodes[]` (rect, column, depth,
  drawn_depth, entry/hottest/unreached/production, path_step, partial_parse,
  fill/stroke/stroke_width/dash, badge, cost, cost_text, ratio_text, top_text,
  findings{hot,warn,info}, functions[] with
  loop_depth/trip_source/unknown_trip_loops/fan_in/fan_out/recursive); `edges[]` (from, to, **d = the
  SVG path**, count, back, on_path); `aggregates[]`; `cap_chips[]`; `stages[]` (order, label, depth,
  files, leaf); `actions[]` (KV1); `meta` (entry/hottest/path/caps/footer text, findings counts);
  `state` (the seven flags).
- **Shared layout math:** `compute_layout`, `edge_shape`, `hull_boxes`, `aggregate_rect`,
  `cap_chip_rect`, `node_style`, `node_rank_lines`, `node_badges`, `column_header_lines` live in
  `atlas_render.rs` and are used by both the SVG and the JSON. No geometry is recomputed.
- **Additive proof (`docs/evidence/atlas-sim/GATE.txt`):** `report.html` byte-identical with/without
  `--story` on hyperfine and cJSON; `metrics.json` content-equal after removing
  `scan.duration_ms`/`scan.started_at`.
- **Output:** canvas 1400×718; hyperfine 19 nodes / 49 edges / 3 stages, findings 148/348; cJSON
  7 nodes / 5 edges / 2 stages, findings 153/837.
- **Definitions pinned:** `findings_included` = sum of severity counts on drawn nodes (the converter
  asserts it); `stages` = one per `view.path` step, first `ENTRY`, last `leaf`.
- **Post-panel fix:** the first cut emitted `columns[].x` as the column's left edge while the SVG draws
  the header at its centre (90 px drift; caught by review lane R1). Columns now carry the header centre
  plus width and the renderer's own label/count-text helper. **Follow-up:** add a DOM-vs-JSON geometry
  parity test so this class of drift cannot recur silently.
- **Real implementation must do:** P5 = keep one layout function with two outputs (never fork into the
  story); P4 = extend stages to multi-entry/scene order in S4; both stay story-only until the `flow`
  block graduates.
- **Known limits:** `--story` builds the view twice (single-pass later); no data-movement fields yet
  (P1–P3 = S2/S3); `aggregates`/`cap_chips` have no fixture with a capped/aggregated column.

## Status legend

`not started` → `prototype` (emitted, fidelity documented) → `implemented` (real probe, tests) →
`verified` (golden + determinism + F-gate). The founder flips the last two when the real probe lands.
