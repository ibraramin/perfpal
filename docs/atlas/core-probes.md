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

- **Commits (each reproducible with `git show --stat <sha>`):**

  | Commit | Scope | Files (stat) |
  |---|---|---|
  | `5faacb7` | core emitter + tests | `src/cli/mod.rs` +9, `src/reporter/atlas.rs` +49/−6, `src/reporter/atlas_layout.rs` new, `src/reporter/atlas_render.rs` +328/−124, `src/reporter/mod.rs` +20, `tests/pipeline_e2e.rs` +49 |
  | `deb2cc2` | sim + spec + docs | `sim/from-layout.py` new (236), `sim/extract.py` deleted (−884), `sim/make-source.sh` +22/−?, `sim/data-{hyperfine,cjson}.json` regenerated, `docs/specs/atlas-story.md` new (149), `docs/atlas/core-probes.md` new (121), `docs/research/atlas-run.md`, `docs/activeContext.md`, `state/tracks/atlas-sim/handoff.md`, evidence |
  | `996d47e` | panel fixes: column geometry, provenance, gate | `sim/from-layout.py` +45/−?, `sim/gate.sh` new (112), `sim/sim.js`, `sim/verify.py`, `src/reporter/atlas_layout.rs`, `src/reporter/atlas_render.rs`, `tests/pipeline_e2e.rs` +80, docs (16 files) |
  | `462f4f4` | stable metrics content hash | `sim/from-layout.py` +19, `sim/data-{hyperfine,cjson}.json`, `docs/evidence/atlas-sim/NOTES.md` |
  | `43a0841` | canvas height, record, stale docs | `sim/from-layout.py` +13, `sim/gate.sh` +7, `tests/pipeline_e2e.rs` +18, `docs/atlas/core-probes.md` +66, `docs/research/atlas-run.md` +11, `docs/specs/atlas-story.md` +7, `docs/activeContext.md`, `CHANGELOG.md`, `state/tracks/atlas-sim/handoff.md` |
  | `0461f75` | path-independent provenance, reproducible gate, record | `sim/from-layout.py` (root_path strip, node extents), `sim/gate.sh`, `sim/make-source.sh`, `sim/verify.py`, `sim/data-*` (6), `docs/atlas/core-probes.md`, `docs/research/atlas-run.md`, `docs/evidence/atlas-sim/NOTES.md`, `CHANGELOG.md`, `docs/activeContext.md`, `state/tracks/atlas-sim/handoff.md` |
  | `9ad6ea2` | hard gate assertions, tree attestation, µs check, contract clarity | `sim/gate.sh`, `sim/verify.py`, `sim/make-source.sh`, `src/reporter/atlas_layout.rs`, `src/reporter/mod.rs`, `docs/report/output-contract.md`, `README.md`, registry/research/NOTES/CHANGELOG/state |
  | `7170cad` | gate self-attestation: dirty count excludes the transcript, `verdict=` recorded | `sim/gate.sh` |
  | `63914f9` | gate fails on projection errors; record order/version marker | `sim/gate.sh`, `docs/atlas/core-probes.md`, `docs/specs/atlas-story.md`, `state/handoff/latest.md`, `state/tracks/atlas-sim/handoff.md`, `docs/activeContext.md` |

  **Authoritative for any gate run:** the `Commit at run` (and `Git tree`) line in
  `docs/evidence/atlas-sim/GATE.txt`. Commits that add a transcript are docs/evidence only; this table
  records the development sequence and is extended at session end.
- **Deviations (recorded, not silent):**
  1. **P4 + P5 share one emitter and shipped in one commit.** The one-probe-per-commit rule is waived
     because neither is independently useful; cherry-pick the pair.
  2. **`stages` live in `atlas-layout.json`, not the `flow` block.** Spec §4 corrected 2026-10-24.
  3. **CLI is `--story` on the existing command.** `perfpal story <repo>` remains the S4 entry point.
  4. **The artifact carries more than geometry:** presentation strings, state flags, top functions and
     KV1 actions. The beats need them; re-deriving them in the sim would be a second source of truth.
- **Emitted by:** `perfpal <repo> -o <dir> --story` → `atlas-layout.json` (story-only; no other
  artifact changes).
- **Fields (complete):**
  - top level: `layout_version`, `canvas{w,h}` (height is layout-dependent; the approved fixtures are
    1400×718, deep paths grow it), `meta{entry_text, hottest_path, hottest_text, path_text, caps_text,
    footer_text, findings_included, findings_total}`, `state{empty,no_entry,library,all_test,capped,
    partial_parse,single_file}`.
  - `columns[]{depth,x,width,label,count_text,unreached,folded,file_count,nonproduction_unreached}` —
    `x` is the **SVG header centre**, not the column's left edge.
  - `hulls[]{x,y,w,h,label}`.
  - `nodes[]{path,name,x,y,w,h,column,depth,drawn_depth,entry,hottest,unreached,production,path_step,
    partial_parse,fill,stroke,stroke_width,dash,badge,cost,cost_text,ratio_text,top_text,
    findings{hot,warn,info},functions[]{id,name,line,cost,loop_depth,trip_source,unknown_trip_loops,
    fan_in,fan_out,recursive}}`.
  - `edges[]{from,to,d,count,back,on_path}` — `d` is the SVG path.
  - `aggregates[]{column,x,y,w,h,count,incoming_calls}`.
  - `cap_chips[]{column,x,y,w,h,production_unreached,nonproduction_unreached}`.
  - `stages[]{order,label,depth,files,leaf}`.
  - `actions[]{location,rule,function,severity_confidence,production,cost_anchor}` (KV1).
- **Shared layout math:** `compute_layout`, `edge_shape`, `hull_boxes`, `aggregate_rect`,
  `cap_chip_rect`, `node_style`, `node_rank_lines`, `node_badges`, `column_header_lines` live in
  `atlas_render.rs` and are used by both the SVG and the JSON. No geometry is recomputed.
- **Additive proof (`docs/evidence/atlas-sim/GATE.txt`):** `report.html` byte-identical with/without
  `--story` on hyperfine and cJSON; `metrics.json` content-equal after removing
  `scan.duration_ms`/`scan.started_at`.
- **Output (fixtures):** hyperfine 19 nodes / 49 edges / 3 stages, findings 148/348; cJSON 7 / 5 / 2,
  153/837.
- **Definitions pinned:** `findings_included` = sum of severity counts on drawn nodes (the converter
  asserts it); `stages` = one per `view.path` step, first `ENTRY`, last `leaf`.
- **Post-panel fix:** the first cut emitted `columns[].x` as the left edge while the SVG draws headers at
  the centre (90 px drift; caught by R1). Now shared. **Follow-up:** the e2e test asserts
  `node.x + node.w/2 == column.x`; keep it.
- **Real implementation must do:** P5 = one layout function with two outputs (never fork into the
  story); P4 = extend stages to multi-entry/scene order in S4; both stay story-only until `flow`
  graduates.
- **Known limits:** `--story` builds the view twice (single-pass later); no data-movement fields yet
  (P1–P3 = S2/S3); no fixture with a capped/aggregated column; the JSON carries pre-fit header text
  while the SVG applies `fit_header_line` (equal today, may diverge at 11 columns).

## Status legend

`not started` → `prototype` (emitted, fidelity documented) → `implemented` (real probe, tests) →
`verified` (golden + determinism + F-gate). The founder flips the last two when the real probe lands.
