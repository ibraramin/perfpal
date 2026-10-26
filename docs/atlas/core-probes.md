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
| Data movement | absent | **resolved (S2/S3)** — P1/P3 (`ea72a86`, `2108117`) + P2 (`0ddf4ad`) |
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

## Status legend

`not started` → `prototype` (emitted, fidelity documented) → `implemented` (real probe, tests) →
`verified` (golden + determinism + F-gate). The founder flips the last two when the real probe lands.
