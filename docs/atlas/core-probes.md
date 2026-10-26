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
| P1 | call-site payload | `flow.calls[]`: from, to, to_id, caller, callee, line, args[{expr,type,dir,named,literal}], unresolved; per-edge `payload`/`label` | `src/analysis/flow.rs` (new) | syntactic + local type table | **prototype** (`ea72a86`) |
| P2 | I/O effects | `flow.io[]`: op read/write, target file/env/net/stdio, literal | same module | call-name + string-literal match | **prototype** (`0ddf4ad`) |
| P3 | signature summary | `flow.signatures[]`: id, name, path, line, params[{name,type,dir}], ret, unresolved[] | `src/analysis/flow.rs` (new) | types as written; macros/generics opaque | **prototype** (`2108117`) |
| P4 | stage order | `stages[]`: order, label, files, depth, leaf | `src/reporter/atlas_layout.rs` (derived, no new parse) | derived | **prototype** |
| P5 | layout emission | `atlas-layout.json`: canvas, columns, nodes, edges, hulls, aggregates, cap chips | `src/reporter/atlas_layout.rs` + `pub(crate)` layout helpers in `atlas_render.rs` | exact (same math as the SVG) | **prototype** |
| P6 | purpose line | `purpose`: text, source, kind | `src/reporter/atlas_purpose.rs` (new) | first README prose paragraph, else structural fallback | **prototype** (`9263c19`) |
| P7 | whole-repo profile queue | `profile`: queue_version, counts, rows[] | `src/reporter/atlas_queue.rs` (new; projects `views::complete_profile_queue`) | authoritative KV3 order; top 12 non-demoted production | **prototype** (`e55221c`) |
| P8 | directory scenes | `scenes`: rows, caps, omitted totals; `nodes[].scene`, `actions[].scene` | `src/reporter/atlas_scenes.rs` (new) + shared `atlas::file_cost` | directory partition + shared file cost | **prototype** (`3c80ae9`) |

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
- **S4 update:** directory scenes are a separate **P8** block; `stages` stay the representative path
  (entry → steps → leaf). Multi-entry/scene-ordered stages are **not** implemented and are not
  claimed by S4.

### P5 — layout emission

- **Question:** where is each node, so the story does not re-implement layout?
- **Prototype:** emit the exact geometry the SVG renderer already computes (x/y/w/h, column x,
  edge path `d`), and the cap/aggregate boxes.
- **Must not:** drift from the SVG. One layout function, two outputs.
- **Why:** without it the story must either scrape HTML (current D1 shortcut, rejected) or fork the
  layout in JS (drift). This is the single most important probe for mergeability.

### P6 — purpose line

- **Question:** what does this scanned app do, in one plain sentence?
- **Prototype:** read the scan-root README (`README.md`, `README.rst`, `README.txt`, `README`;
  first match, 64 KiB cap), take the first prose paragraph after the title (skip front matter,
  headings, badges/images, TOC/link-only blocks, HTML, fences and tables), strip inline markdown,
  collapse whitespace and cap at 200 chars + ellipsis (≤201). If there is no README or no usable paragraph, emit a
  structural fallback built from the existing entry line. `purpose_version: 1`,
  `kind: "readme" | "structural"`, `source: <filename> | null`.
- **Must not:** invent a purpose; summarise code as intent; follow links or read outside the scan
  root; treat a README as a runtime claim. The story labels the source (`from README.md` /
  `structural fallback`) and withholds text that trips the F4 claim filter.
- **Real implementation:** a README/metadata parser with provenance and a human-confirmed purpose
  override; keep the quote-vs-inference distinction visible.

### P7 — whole-repo profile queue

- **Question:** what are the top non-test, production candidates in the whole scan (KV3)?
- **Prototype:** project the **authoritative** `views::complete_profile_queue` ordering. Count
  `total_ranked`, `production_ranked`, eligible production non-demoted `candidate_total` and
  `demoted_production`; carry the first 12 eligible rows in queue order with rank, canonical id,
  name, `path:line`, cost + rendered anchor, unknown/known trip fields and demotions. `queue_version: 1`.
- **Must not:** re-rank in the story; include non-production rows; hide the startup/no-loop
  demotions; call the rows runtime hot spots. They are static candidates.
- **Real implementation:** emit the complete queue on demand (or a stable page), keep KV3 and the
  story on one ranking function, and add a per-row drill identity.

### P8 — directory scenes

- **Question:** how is a 1k–10k-file repo organised, and where is the mass per directory?
- **Prototype:** partition source files (`function_count > 0`) into directory scenes: root files →
  `(root)`; a top-level dir with one child dir → one scene; with more than one child dir → split
  into `d1` (direct files) + `d1/d2` scenes; deeper files stay in their depth-2 scene. Per scene:
  files, production files, findings, cost and production cost (shared `atlas::file_cost` rule),
  scale (`v2`/`v1`/`mixed`), entry/hottest flags, top 5 files by cost. Order production-cost↓,
  cost↓, path↑; carry 24 scenes plus an explicit omitted totals block (`scenes`, `files`, `cost`,
  `production_cost`, `findings`). Emit `scene_version: 1`; annotate `nodes[].scene` and
  `actions[].scene` with the same partition.
- **Must not:** claim the scene cost is runtime work; hide omitted scenes/files; use a different
  cost formula from the canvas; treat a directory as a call-graph component; infer execution order
  from the tree.
- **Real implementation:** configurable scene depth/caps for very large repos, component-aware
  scene grouping where it is justified, and a source-level scene index for drill links.

## What the D1 prototype faked (and where it stands after S1/S4)

| Fake | D1 | Status |
|---|---|---|
| Geometry | `sim/extract.py` parsed `report.html` | **resolved** — P5 `atlas-layout.json`, HTML extraction deleted |
| Story data | `extract.py` read rendered text | **resolved** — P4 + `AtlasView` |
| Data movement | absent | **resolved (S2/S3)** — P1/P3 (`ea72a86`, `2108117`) + P2 (`0ddf4ad`) |
| Scene aggregation | none (19-node canvas only) | **resolved (S4/P8)** — directory scenes, 24-scene/5-file caps, counts + omitted totals; 3k fixture |
| Whole-repo queue | drawn-slice only | **resolved (S4/P7)** — authoritative KV3 top 12 + totals |
| Purpose | absent | **resolved (S4/P6)** — README first prose paragraph, structural fallback |

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
  3. **CLI is `--story` on the existing command; the `perfpal story <repo>` subcommand is deferred.**
     Decision 4 named it the S4 target, but the approved S4 plan and build shipped only the flag; no
     CLI subcommand was added. Recorded deviation — the subcommand remains a later CLI slice.
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

## S2 record (2026-10-24) — P3 + P1 `prototype`

### P3 — signature summary (`2108117`)

- **Commit:** `2108117` `core(P3): signature summaries` · `git show --stat 2108117`:
  `src/analysis/flow.rs` new (563), `src/analysis/mod.rs` new (6), `src/lib.rs` +1,
  `src/reporter/atlas_layout.rs` +18/−5, `src/reporter/mod.rs` +14, `tests/pipeline_e2e.rs` +74.
- **Emitted:** `flow.flow_version = 1`, `flow.fidelity = "syntactic"`,
  `flow.signatures[] = {id, name, path, line, params[{name, type, dir}], ret, unresolved[]}`.
  `calls` and `io` were omitted from the first S2 cut; P1 landed in S2 (`ea72a86`) and P2 `io` in S3 (`0ddf4ad`). `layout_version` stays 1.
- **Identity:** `id` is the canonical IR id (`path#name#start_line:start_col`, 1-based) copied
  from the backend builders, so a signature joins `nodes[].functions[].id` without a translation
  table (e2e asserts every id exists in `metrics.json` `stats.functions[].id`).
- **Fidelity:** syntactic, types **as written**; local re-parse of the **drawn node files only**
  under `--story` (C/Rust extensions). Nothing executes; no runtime claim.
- **`dir`:** `in` for values, `const T*`, Rust `&T`/`*const T`; `inout` for non-const C pointers
  and Rust `&mut T`/`*mut T`. `out` is deliberately **unused** in S2: write-only cannot be
  inferred syntactically, and guessing it would break §5.
- **`unresolved`:** only opaque constructs — `impl Trait`, `variadic`, `fn-pointer` (C `(*`).
  Generics are recorded as written, not resolved, and are not by themselves an unresolved entry.
- **Known limits:** second parse of the drawn files (bounded by the canvas cap; story-only);
  macros are not expanded; C parameter text is assembled from the declaration so qualifiers and
  pointers survive (the grammar's `type` field alone drops `const`/`*`); Rust pattern names use
  the leftmost identifier; unnamed C parameters carry `name: null`; unreadable or failed-to-parse
  files are skipped (a partial parse tree can still emit rows); no `io` block yet.
- **Real implementation must:** capture return type + parameter names/direction in the scanner IR
  in the same pass (no second parse), expand macros, and resolve `impl Trait`/generics only with a
  real type analysis. Prototype stays story-only.
- **Evidence:** 853 tests pass; `cargo fmt`/`clippy -D warnings` exit 0; on the hyperfine fixture
  `--story` emits 191 signatures for 19 drawn nodes in 1.78 s; `report.html` byte-identical
  with/without `--story`; `metrics.json` content-equal after masking scan provenance; two runs
  produce identical `flow` bytes.

### P1 — call-site payload (`ea72a86`)

- **Commit:** `ea72a86` `core(P1): call-site payloads` · `git show --stat ea72a86`:
  `src/analysis/flow.rs` +706/−21 (total stat), `src/reporter/atlas_layout.rs` +10,
  `tests/pipeline_e2e.rs` +91.
- **Emitted:** `flow.calls[] = {from, to, to_id, caller, callee, line, args[{expr, type, dir, named, literal}], unresolved}`
  plus **per-edge** additive `edges[].payload` and `edges[].label` on every drawn edge
  (`type-only` | `literal-observed` | `unresolved`). `layout_version` stays 1; no other artifact changes.
  The edge label is computed in Rust — the film does not re-classify in Python/JS.
- **Identity:** `from`/`to` are file paths (the drawn edge endpoints); `caller`/`to_id` are canonical
  function ids so a later drill-down can join `path:line` (the S2 film shows `callee`/`line` in the edge tooltip; no inspector drill yet). `to`/`to_id` are `null` when unresolved.
- **Resolution:** direct names only (`identifier`, Rust `scoped_identifier` final segment) and only when
  the name is **unique across the parsed drawn functions**. Ambiguous names, method/field calls,
  function pointers and macros are `unresolved: true` — never guessed (§2 non-goals).
- **Args / local type table:** function params + explicit locals only (Rust `let x: T`, C `T x`);
  Rust references report the pointee (`&mut Config` ⇒ `Config`); C decimal constants are `int`;
  string literals are typed `str`/`char*` with `literal: true`; `named` is the bare identifier or the
  root of a field access. No cross-file or points-to inference.
- **Edge classification:** any string-literal arg ⇒ `literal-observed` (payload = the literal, ≤24 chars);
  else any typed arg ⇒ `type-only` (payload = most frequent type, ties lexicographic); else
  `unresolved` (payload `null`). No matching call ⇒ `unresolved`.
- **Evidence:** 853 tests pass; fmt/clippy exit 0; hyperfine `--story` in 1.9 s emits 1586 calls
  (22 resolved after the `ee74206` conservatism fix) and 49 labelled edges — 44 `unresolved`, 4 `type-only`, 1 `literal-observed`;
  every `from` is drawn, every `to` is drawn-or-null; `report.html` byte-identical with/without
  `--story`; metrics content-equal after masking scan provenance; two runs identical.
- **Known limits:** drawn files only (second parse, story-only); no cross-file type inference; method
  and function-pointer calls stay unresolved; `named` is `null` for borrowed expressions (`&cfg`);
  literal payloads are display-capped; file-scope initializer calls are not rows; intra-file calls are
  included (the film may ignore same-file edges); C macros are never expanded.
- **Real implementation must:** resolve C function pointers and Rust `dyn`/closures, expand macros,
  infer argument types interprocedurally, and capture all of it in the single scanner pass.

### S2 fixes (`ee74206`)

- **Commit:** `ee74206` `fix(flow): pointer returns and conservative resolution` (panel findings).
- **Fixed:** C return types include the declarator pointer/qualifiers (`char *dup` ⇒ `char *`,
  `const char *name` ⇒ `const char *`, `int **grid` ⇒ `int **`); fn-pointer returns are recorded
  as written and marked `fn-pointer-return`; argument `dir` is expression/type-consistent
  (`&cfg` ⇒ `inout`); macro names are collected across **all** drawn files before call extraction;
  Rust `scoped_identifier` resolves only from `crate`/`self`/`super`, so `std::fs::read` can no
  longer masquerade as a drawn `read`; AST walks are capped at depth 256.
- **Effect:** hyperfine resolved calls 43 → 22 (more conservative); edge labels unchanged
  (44 `unresolved`, 4 `type-only`, 1 `literal-observed`). 853 tests pass.
- **Follow-up fix:** `eef9d99` `fix(flow): function-pointer parameter list` — a function
  returning a function pointer (`int (*f(void))(int)`) now reports its **own** `(void)`
  parameter list, not the returned function's `(int)` list; the test asserts
  `factory.params == []`. Found by the R1 re-review (round 2); this is the only blocking
  residual after `ee74206`.
- **Not changed:** `out` still unused; `unresolved` still means "callee unresolved or every
  argument untyped" (documented in the module).

## S3 record (2026-10-26) — P2 `prototype`

### P2 — I/O effects (`0ddf4ad`)

- **Commit:** `0ddf4ad` `core(P2): io effects` · `git show --stat 0ddf4ad`:
  `src/analysis/flow.rs` +481/−9, `tests/pipeline_e2e.rs` +110/−1.
- **Emitted:** story-only `flow.io[] = {file, line, callee, op, target, literal}`.
  `file` is the **call-site** drawn path (never the target); `op` ∈ `read|write`; `target` ∈
  `file|env|net|stdio`; `literal` is `null` unless the designated argument is a plain
  `string_literal`/`raw_string_literal`. `flow_version`/`layout_version` stay 1; no schema/dep change.
- **Table:** fixed `IO_TABLE_VERSION = 1` in `src/analysis/flow.rs` (v1 is the first recorded
  table; the review fixes below landed before this record, so no released v1 differs). Exact written
  names only
  (C `fopen/freopen/open/openat/creat/...`; Rust `File::open|create`, `std::fs::*`, `std::env::*`,
  `TcpStream::connect`, `reqwest::get`, and exact `fs::…` forms). `fopen` direction comes from the
  written mode (`r` read, `w/a` write, `+` ⇒ both); `open` direction from a written flag expression
  (`O_WRONLY` write, `O_RDWR` both, `O_RDONLY`/`0` read); a variable, a non-zero numeric literal
  (platform-specific bits) or a flags expression without an access mode emits **no row**.
- **Guards (never classify by name alone):** a bare C name is skipped when any drawn file defines a
  function with that name; the C bare-name rules are language-gated; a callee that resolves to any
  local/parameter value (function pointer, typedef'd pointer or closure) is blocked; macros are
  blocked. Rust paths match the complete written path, not a bare `read`/`write` (a drawn local
  `File`/`fs` wrapper with the same path is the documented Rust path-ambiguity limit). `sprintf`/
  `sscanf` are not I/O. Stdio rows never carry a literal (a format string is not a target).
- **Fixtures:** hyperfine **8 rows / 5 drawn files** (5 read, 3 write; 3 literal: `TERM`, `NO_COLOR`,
  `/dev/null`; 5 target-unknown); cJSON **24 rows / 4 drawn files** (4 read, 20 write; 0 literal,
  2 target-unknown; 22 stdio). The story shows `Stores:` on the trace beats; each drawn file gets a
  DATA STORES inspector section with up to 6 rows (disclosed as `showing N of M`), and the trust beat
  aggregates coverage, stdio counts and up to two literal examples (with a `+N` remainder marker).
- **Known limits:** a wrapper defined **outside** the drawn set can still match (documented
  prototype limit); a Rust local `File`/`fs` wrapper with the same written path can still match until
  imports/type resolution lands; `read`/`write` are fd-based so `target: file` is the call class (may
  be a pipe/socket); `OpenOptions::new().open(...)`/`write_all(...)` are method calls and are not
  matched; `env::args*` is recorded as an `env` read; `connect` is the outbound (`write`) net effect;
  `std::fs::copy` emits read+write with no single literal (two path arguments), and `rename`/
  `hard_link` emit no literal (source vs destination would be a guess); literal values are source
  text with outer quotes removed (escapes not decoded, display-capped at 160); non-literal direction
  is omitted rather than guessed.
- **Real implementation must:** infer wrapper summaries/interprocedural effects, resolve imports and
  Rust path ambiguity, capture this in the single scanner pass, and keep the runtime claim at zero
  (syscall probes are post-beta D3).
- **Evidence:** 862 tests; `cargo fmt`/`cargo clippy -D warnings` exit 0; story gate
  `sim/verify.py` 217/217; `report.html` byte-identical with/without `--story`; two scans emit
  identical `flow.io`; `flow.io` sorted `(file, line, callee, op, target, literal)`.

## S4 record (2026-10-26) — P6 + P7 + P8 `prototype`

S4 is the scale + payoff slice (`docs/specs/atlas-story.md` §9): directory scenes at 3k files, the
whole-repo profile queue, scene-aware start-here/trust, and F1–F7 evidence. F8 (founder A2) is
**deferred**, not claimed. The authoritative gate run is the `Commit at run` line in
`docs/evidence/atlas-sim/GATE.txt` (S4 close commit; `dirty 0`; `verdict=PASS`; 897 tests; story gate
304/304; 3k scale 44/44).

### P6 — purpose line (`9263c19`)

- **Commit:** `9263c19` `core(P6): purpose line` · `git show --stat 9263c19`:
  `src/reporter/atlas_purpose.rs` new (712), `src/reporter/atlas_layout.rs` +41/−?, `src/reporter/mod.rs` +8,
  `tests/pipeline_e2e.rs` +71. Addendum: structural fallback is `cap_text`-bounded (same commit after the
  unit-test pass).
- **Emitted:** `purpose { purpose_version: 1, text, source: String|null, kind: "readme"|"structural" }`
  in `atlas-layout.json`; only under `--story`; `layout_version` stays 1.
- **Extraction:** first README at the scan root in `README.md` / `README.rst` / `README.txt` /
  `README` order; 64 KiB read cap; first prose paragraph after the title; skips YAML front matter,
  headings, badges/images, TOC/link-only blocks, HTML comments/tags, fences and tables; strips inline
  links/emphasis/code ticks; collapses whitespace; 200-char word-boundary cap + ellipsis (≤201).
- **Fallback:** no README / no prose → `No README at the scan root; inferred entry: <entry_text>.`,
  `source: null`, `kind: "structural"` (also capped).
- **Honesty:** the core copies text, never invents; the sim labels the source and withholds a README
  sentence that trips the F4 claim/time filter (`purpose.kind: "withheld"` only in the sim data; the
  core block stays raw for provenance). A README quote is **not** an Atlas inference.
- **Known limits:** first README only (a broken `README.md` does not fall through to `README.rst`);
  first prose paragraph only (a marketing sentence is still the author's words); no README freshness
  check; no heading/author provenance beyond the filename; no non-UTF-8 decoding beyond lossy UTF-8;
  a scan root that is a single file has no README.
- **Real implementation must:** parse README metadata with explicit provenance, allow a human-confirmed
  purpose override, and keep the quote-vs-inference distinction on screen.
- **Evidence:** hyperfine `A command-line benchmarking tool.` (README.md); cJSON
  `Ultralightweight JSON parser in ANSI C.` (README.md); structural fallback e2e fixture; 64 KiB/200-char
  caps; story-off has no `atlas-layout.json`; F6 byte-identical.

### P7 — whole-repo profile queue (`e55221c`)

- **Commit:** `e55221c` `core(P7): whole-repo profile queue` · `git show --stat e55221c`:
  `src/reporter/atlas_queue.rs` new (373), `src/reporter/atlas_layout.rs` +65/−?, `src/reporter/mod.rs` +5,
  `tests/pipeline_e2e.rs` +62.
- **Emitted:** `profile { queue_version: 1, total_ranked, production_ranked, candidate_total,
  demoted_production, shown, rows: [{rank, id, name, path, line, path_class, production, cost, cost_text,
  unknown_trip_loops, known_trip_value, demotions}] }`; top 12 eligible rows, in authoritative order.
- **Source:** `views::complete_profile_queue` (the KV3 ranking used by `profile-queue.md`); the module
  only filters `production && !row.is_demoted()` and copies; it never re-sorts. `rank` is the full
  queue rank (demoted rows included).
- **Counts:** `total_ranked` = every row; `production_ranked` = path-class production; `candidate_total`
  = production and non-demoted; `demoted_production` = production but startup/no-loop demoted;
  `shown = min(candidate_total, 12)`.
- **Known limits:** capped at 12 rows; production-only (KV3 is non-test); startup/no-loop demoted rows
  are counted but not carried; no line-level drill identity beyond `path:line`; the queue is still a
  static cost ranking, not a runtime profile.
- **Real implementation must:** emit the complete queue on demand (or a stable page), keep the story
  and KV3 on one ranking function, and join to the drill/diagnostic identity.
- **Evidence:** hyperfine `candidate_total=35`, `production_ranked=194`, `total_ranked=353`,
  top `print` (`src/output/report.rs:19`, 213 units); cJSON `45/161/1055`, top `cJSON_Delete`
  (`cJSON.c:253`, 137 units); 3k `candidate_total=2,730`, `shown=12`; e2e asserts every id exists in
  `metrics.stats.functions` and the top row matches the authoritative first eligible row.

### P8 — directory scenes (`3c80ae9`)

- **Commit:** `3c80ae9` `core(P8): directory scenes` · `git show --stat 3c80ae9`:
  `src/reporter/atlas_scenes.rs` new (607), `src/reporter/atlas.rs` +64/−? (shared `file_cost`),
  `src/reporter/atlas_layout.rs` +106/−?, `src/reporter/mod.rs` +5, `tests/pipeline_e2e.rs` +92.
- **Emitted:** `scenes { scene_version: 1, scene_cap: 24, file_cap: 5, total_scenes, shown_scenes,
  total_files, shown_files, rows: [{path, depth, files, production_files, findings, cost,
  production_cost, scale, entry, hottest, top_files: [{path, name, cost, scale, production, findings,
  function_count, entry, hottest}]}], omitted: {scenes, files, cost, production_cost, findings} }`.
  Additive `nodes[].scene` and `actions[].scene` use `SceneBlock::scene_for`.
- **Partition:** source files = `doc.files` with `function_count > 0`; root files → `(root)`; a
  top-level dir with 0–1 distinct child dirs → one scene at depth 1; with more than one child dir →
  files directly in it form `d1` (depth 1) and `d1/d2/...` form `d1/d2` (depth 2); deeper files stay in
  the depth-2 scene. Every file belongs to exactly one scene.
- **Cost:** shared `atlas::file_cost` — all-`function_cost_v2` files sum v2; otherwise sum
  `charts::function_cost` and round half-up, exactly as the canvas file cost. `scale` is `v2`/`v1`/
  `mixed` (mixed only in the scene aggregate). Findings are counted by `findings[].path`.
- **Ordering/caps:** production cost↓, cost↓, path↑; 24 scenes; top 5 files by cost↓/path↑; omitted
  totals are the complement of the shown rows and are printed. `entry`/`hottest` flag the scene that
  contains the elected entry / global hottest path.
- **Known limits:** depth-2 cap (not recursive); the depth-1 direct-files scene can be small and can
  fall below the 24-scene cap (3k: `src` with `main.c` is omitted while the canvas shows it); caps are
  fixed (24/5), not configurable; production-first ordering can omit a huge test/bench scene (3k:
  `tests`, 269 files) — its counts are in the omitted block and the scan-wide non-production count;
  scene cost is static cost, not runtime work; a scene is a directory, not a component/call-graph
  region; action scene is the directory partition, not the call path. A non-source path
  (e.g. an action on a header with `function_count == 0`) uses a best-effort directory key
  and may not name a shown scene row; source nodes/actions are exact.
- **Real implementation must:** configurable depth/caps for 10k+ repos, component-aware grouping where
  justified, source-level scene drill links, and the same shared cost rule.
- **Evidence:** hyperfine 11 scenes / 51 files (35 production / 16 test-bench), cJSON 3 / 81
  (6/75), 3k 32 scenes / 3,000 files (2,731/269), shown 24, omitted 8 scenes / 816 files; every drawn
  `AtlasNode.cost` equals the shared `file_cost`; all scene sums reconcile; two scans/double projection
  byte-identical.

### S4 deviations (recorded, not silent)

1. **The `perfpal story <repo>` subcommand is deferred.** Spec §11 decision 4 names it the S4 target,
   but the approved S4 plan and build shipped the `--story` flag only; no CLI subcommand was added.
   S4 does not claim it. See also S1 deviation 3 and the spec §11 status note.
2. **`nodes[].scene` is additive beyond the plan's literal `actions[].scene` wording.** It is the
   minimal data for the enter-beat scene context (plan step 6), story-only, and uses the same
   `SceneBlock::scene_for` partition as actions.
3. **Scene caps are fixed (depth-2, 24 scenes, 5 top files).** These are the approved plan defaults,
   not configurable in S4; the omitted totals are printed.

### S4 scale fixture and evidence

- **Fixture:** `sim/fixtures/gen-3k.py` (committed) — deterministic 3,000 scanned source files (2,999 C
  + 1 Rust; 2,731 production + 269 test/bench), 30 modules × 9 subdirs × 10 files + 30 entries + main +
  269 tests, a reachable high-cost C function-pointer file (`src/mod00/sub00/f00.c`) and a Rust `dyn`
  file (`src/mod29/sub08/f09.rs`) engineered into the drawn slice, loops/I/O and a README. Generated
  under `/var/tmp/opencode/atlas-3k-fixture`; never committed.
- **F7:** page **463,490 bytes** (component breakdown in `GATE.txt`) vs
  5 MB budget; first frame **under the 2,000 ms budget** over three cold page loads (final transcript); no horizontal scroll
  at 1440×900 or 1280×800; 24 scene cards / 12 queue rows.
- **F2–F6:** every drawn edge labelled; unresolved C fn-pointer + Rust `dyn` rows are visible and never
  guessed; no forbidden claim/time unit in the data or final frame; two scans → identical layout/flow;
  two loads → identical DOM; story-off vs story-on `report.html` identical and `metrics.json`
  content-equal after provenance masking.
- **F1:** `@vision` Read 8, two independent reads (F1 frames + scale frames): **PASS-WITH-NITS, no
  blockers**. Hyperfine `PURPOSE · A command-line benchmarking tool. · from README.md`, entry
  `src/main.rs:88 main`; cJSON `PURPOSE · Ultralightweight JSON parser in ANSI C. · from README.md`,
  entry `fuzzing/afl.c:85 main`; both first `path:line` actions legible; the 3k purpose is fully
  readable; 1280 legend/START/footer wrap; scan-wide test/bench counts present. Carried nits:
  third START item tail, PATH meta ellipsis at 1440, dense depth-1/edge labels, findings run-on, cJSON
  unreached START/empty canvas (pre-existing).
- **F8:** **deferred** — founder A2 session; not run or claimed. A2/ship remains behind the Atlas v2
  step-13 founder gate (R-pf-109 for the map) and is not part of S4.

## Status legend

`not started` → `prototype` (emitted, fidelity documented) → `implemented` (real probe, tests) →
`verified` (golden + determinism + F-gate). The founder flips the last two when the real probe lands.
