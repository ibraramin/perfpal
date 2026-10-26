# Track D — Atlas Story · Session D2 + S1 + S2 + S3 + S4 handoff

**Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` · branch `feat/atlas-sim`
**Track:** `/home/shafin/Opencode/Perfpal/docs/tracks/atlas-run.md`
**Status:** **D2 (D1 review) done + S1 committed (`5faacb7..63914f9`) + S2 committed and gated
(`2108117..b8359ae`) + S3 committed (`0ddf4ad..325571e`; docs `89f4feb`) + S4 committed and gated
(`9263c19..71d0d04`; evidence `2ebeabd`), not pushed.** D1 presentation was accepted by the founder;
the requirement was re-sealed as a core-fed temporal showcase (`docs/specs/atlas-story.md`); S1 made
the story read core JSON, S2 added data movement on the edges, S3 added I/O effects (P2) with the
stores/trust render, and S4 added directory scenes at 3k files, the README purpose line and the
whole-repo KV3 queue (P6/P7/P8), with the 3k scale gate (F2–F7). F8 (founder A2) is deferred. No
push, no merge to `main`.

## D2 outcome

- Founder watched D1 at 1440×900: *"It looks good, as well as the animation. Just the info isn't very
  insightful."* Composition and motion pass; the **information** does not.
- Diagnosis (measured, not opinion): the story re-sequences file-level facts the static report already
  states. It never names a function, a finding, a payload, or a comparison. cJSON's climax is
  `testunity.c · 2,000,468 u · 1,000,234× median` (99.8% of ranked cost is non-production); hyperfine's
  hottest file is `command.rs` but the biggest function is `options.rs#from_cli_arguments` (165 u), and
  2 of 3 KV1 actions sit on files the canvas does not draw.
- Founder direction: **Atlas is Perfpal's #1 priority**; it must consume **Perfpal core data**, not a
  separate pipeline; new data needs → add a probe to the core **and record it** for later merge.
- Sealed requirement (`docs/specs/atlas-story.md`): a temporal showcase — entry → stages → loops → I/O
  → stop — with the payload crossing each file boundary labelled; vibecoder-first, senior drill;
  static inference labelled; scenes by directory at scale; shipped as `perfpal <repo> --story`.
- A 15-item questionnaire was drafted and then withdrawn: the docs already specify the requirement
  (A1/A2, KV1/KV3, `AUDIT.md:51-56`). It was never committed.

## What S1 shipped

- **Core (Perfpal):** `perfpal <repo> -o <dir> --story` now also writes `atlas-layout.json` (`layout_version: 1`;
  `src/reporter/atlas_layout.rs`, `--story` in `src/cli/mod.rs`). It carries the exact geometry the SVG
  draws — canvas, columns, node rects, edge `d` paths, hulls, aggregate boxes, cap chips — plus node
  presentation strings, state flags, drawn/scan findings counts, KV1 start actions and derived
  `stages`. Layout math is **shared** with the renderer (helpers extracted in `atlas_render.rs`), not
  copied. Additive: `report.html` is byte-identical and `metrics.json` differs only in scan provenance.
- **Story (sim):** `sim/from-layout.py` builds `sim/data-<case>.json` from `atlas-layout.json` +
  `metrics.json`, asserts the drawn-findings count against `findings_included`, and embeds the layout
  SHA-256 in `source.sha256`. `sim/extract.py` (HTML scraping) is **deleted**; `make-source.sh` scans
  with `--story` and rebuilds the data. The page itself is unchanged.
- **Record:** `docs/atlas/core-probes.md` — P4/P5 are `prototype`; P1–P3 later landed in S2/S3 (records below).
  Merge protocol: one probe per commit, additive only, story-only emission, no golden churn.

## What S2 shipped

- **Core (P3 signatures, `2108117`):** new `src/analysis/flow.rs` re-parses **only the drawn files**
  under `--story` and emits `flow = {flow_version: 1, fidelity: "syntactic", signatures[]}`. Signature
  ids are the canonical IR ids (`path#name#line:col`), so they join `nodes[].functions[].id`. Return
  types and parameter names/types/direction are recorded as written; `impl Trait`, variadics and
  function pointers are `unresolved`; `out` is deliberately unused.
- **Core (P1 call payloads, `ea72a86`):** `flow.calls[]` records `{from, to, to_id, caller, callee,
  line, args[{expr,type,dir,named,literal}], unresolved}` from a local type table (params + explicit
  locals). Per-edge additive `edges[].payload` / `edges[].label` (`type-only` | `literal-observed` |
  `unresolved`) is computed **in Rust**; the sim never re-classifies. Conservative resolution: unique
  direct names only (`crate`/`self`/`super` for scoped paths); ambiguous/method/fn-pointer/macro =>
  `unresolved`. `layout_version` stays 1; normal scans are byte-identical.
- **Story (sim, `cac8be3` + `c99fade`):** every drawn edge carries a visible label (payload + class,
  grey dashed `unresolved`); deterministic collision-free placement; legend names the three classes.
  Payoff beats: `hottest` (file **and** globally hottest drawn function with `L<line>` + path),
  `profile` (drawn-slice, production-only, "static candidates, verify"), `start-here` (KV1's three
  actions, on/off-canvas), `trust` (caps, partial parse, non-production share, unknown-trip floor).
  cJSON badge overprint fixed. F2/F4/F5 checks added; final-frame DOM hashed across two loads.
- **Fix commits:** `ee74206` (C pointer returns, `dir` consistency, global macro exclusion, scoped
  resolution, depth cap), `eef9d99` (function-pointer-return parameter list). 853 tests.
- **Record:** `docs/atlas/core-probes.md` P3/P1 `prototype` + S2 fixes; evidence in
  `docs/evidence/atlas-sim/` (GATE, checks 162/162, VISION Read 6 PASS).

## What S3 shipped

- **Core (P2, `0ddf4ad` `core(P2): io effects`):** `src/analysis/flow.rs` emits the story-only
  `flow.io[] = {file, line, callee, op, target, literal}` from a fixed **v1** call-name table:
  `read`/`write` effects on `file`/`env`/`net`/`stdio`. Guards: a bare C name defined in the drawn
  set, a macro or a local function-pointer is never matched; Rust matches the complete written path;
  `fopen`/`open` direction comes from the written mode/flags, and a non-literal mode/flags expression
  emits no row. A literal is recorded only when the argument is a plain `string_literal`/
  `raw_string_literal` (never `format!`, `concat!`, adjacent C strings or `sprintf`); a missing
  literal stays `null`. Stdio rows never carry a literal. Deterministic sort
  `(file,line,callee,op,target,literal)`; `flow_version`/`layout_version` stay 1; no new dep.
- **Story (`9e23aa6` `story(S3): io stores and trust line`):** `from-layout.py` projects per-node `io`
  summaries and `trust.io`; `sim.js` adds `Stores:` to trace beats, a DATA STORES inspector section
  (literal or `target unknown`; stdio = stream), and the I/O trust line with up to two literal
  examples; `verify.py` adds 55 checks (data/core/DOM, row-level and cap) and stores/trust/inspector keyframes;
  `gate.sh` prints the io table summary.
- **Fixtures:** hyperfine 8 rows / 5 drawn files (5 read, 3 write; literals `TERM`, `NO_COLOR`,
  `/dev/null`; 5 target-unknown); cJSON 24 rows / 4 drawn files (4 read, 20 write; 0 literal,
  2 target-unknown; 22 stdio). Trace beats render one stores line each; the inspector carries up to
  6 rows per file (`showing N of M`) and the trust line carries the aggregate, stdio count and up to
  two literal examples with a `+N` marker.
- **Review fixes (`b63c6f6` + `27569a9` core, `f3e8804` + `325571e` story):** `freopen` reads mode argument 1; only `0` and
  symbolic access flags are decoded (non-zero numeric bits/variables emit no row); any local/parameter
  value (including a typedef'd function pointer) blocks the I/O table; C bare-name rules are
  language-gated; `std::fs::copy` is read+write with no single literal; the trust line adds stdio and a
  `+N` literal-remainder marker; inspector stdio rows read `stream · no path`; the gate asserts the
  full dirty count is 0 and masks `root_path` in F6; `fs::rename`/`std::fs::rename`/`fs::copy` and
  `hard_link` carry no single literal (source vs destination would be a guess); the inspector checks
  read row-level data attributes and the row cap.
- **Record:** `docs/atlas/core-probes.md` P2 `prototype` (S3 record with commit, table rules, guards,
  fixture counts, limits, real implementation); spec §9 S3 done; output contract names `flow`.
- **Evidence:** `docs/evidence/atlas-sim/GATE.txt` (final transcript on the docs commit), `checks.json`
  217/217, `hyperfine/cjson-stores.png`, `-trust.png`, `-io-inspector.png`, `VISION.md` Read 7 PASS.

## What S4 shipped

- **Core P6 (`9263c19`):** story-only `purpose` (`purpose_version: 1`) from the scan-root README's first
  prose paragraph (64 KiB/200-char caps, front matter/badges/TOC/fences skipped) or the existing entry
  line as a structural fallback. `layout_version` stays 1; story-off is untouched. The sim labels the
  source (`from README.md` / structural fallback) and withholds a README sentence that trips the F4
  claim/time filter.
- **Core P7 (`e55221c`):** story-only `profile` (`queue_version: 1`) — the authoritative KV3 queue
  (`views::complete_profile_queue`) filtered to production, non-demoted rows, top 12 in queue order,
  plus `total_ranked` / `production_ranked` / `candidate_total` / `demoted_production`. No second
  ranking.
- **Core P8 (`3c80ae9`):** story-only `scenes` (`scene_version: 1`) — depth-2 directory partition
  (root files, top-level scene, split into `d1`/`d1/d2` with >1 child dir), shared `atlas::file_cost`,
  production-cost-first ordering, 24 scenes × 5 top files, explicit omitted totals; additive
  `nodes[].scene` / `actions[].scene`.
- **Story (`30a3bba`, `49627b2`, `5f13b04`):** sim projects purpose/profile/scenes + unresolved-call
  summaries; `PURPOSE · <README sentence> · from README.md`; enter-beat scene context; whole-repo
  profile beat; scene-aware start-here; compact trust narration; below-fold SCENES / PROFILE QUEUE /
  TRUST ACCOUNTING panels; inspector UNRESOLVED CALLS; `window.__sim.ready`; 1280/1440 wrap fixes.
- **3k fixture + scale gate (`4006315`, `5f7579d`):** deterministic `sim/fixtures/gen-3k.py` (3,000
  scanned files, 2,999 C + 1 Rust, 2,731 production + 269 test/bench, C fn-pointer + Rust `dyn` in
  the drawn slice) and `sim/scale.py`/`scale.sh` (F2–F7). F7: page **463,490 bytes** (budget 5 MB),
  first frame **under the 2,000 ms budget** (final transcript), no horizontal scroll at 1280×800, 24 scene cards /
  12 queue rows. F2/F3/F4/F5/F6 pass at 3k. F8 (founder A2) **deferred**, not claimed.
- **Record:** `docs/atlas/core-probes.md` P6/P7/P8 `prototype` (S4 record with commits, fields,
  fidelity, limits, real implementation); spec §9 S4 done; output contract names the three blocks.
- **Evidence:** `GATE.txt` (final transcript at `71d0d04`, tree `51548f3`, dirty 0, PASS; evidence `2ebeabd`), `checks.json`
  304/304, 3k scale 44/44, `scale-{final,full,1280}.png`, `VISION.md` Read 8 (two independent reads;
  PASS-WITH-NITS, no blockers). The final 4-lane panel result is recorded below at close.

## Verification

| Claim | Command / artifact | Result |
|---|---|---|
| Core gate | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests (S2) | `cargo test --locked` | 853 passed, 0 failed, 1 ignored |
| Licenses | `cargo deny check licenses` | exit 0 (`docs/evidence/atlas-sim/GATE.txt` §1) |
| Existing outputs unchanged | `sha256sum` report.html, story-off vs story-on | identical: hyperfine `bd0b2055…`, cJSON `373e8087…` |
| metrics.json unchanged | field diff of story-off vs story-on | content-equal after removing `scan.duration_ms`, `scan.started_at`, `scan.root_path` |
| Story artifact | `perfpal … --story` → `atlas-layout.json` | 1400×718; hyperfine 19/49/3 stages, findings 148/348; cJSON 7/5/2, 153/837 |
| Column geometry matches the SVG | `columns[].x` vs the renderer header centre | hyperfine 250/430/610/790/970/1150; cJSON 520/700/880 (the S1 first cut was 90 px off; fixed) |
| Data from core, not HTML | `sim/from-layout.py` | 2 files regenerated byte-stable; all 6 data files unchanged by the gate; `source.sha256` matches |
| Story gate (S2) | `SIM_PORT=8770 python3 sim/verify.py` | **162/162, exit 0** |
| sim parity vs D1 | field diff of regenerated `data-*.json` | nodes/edges/findings/hulls/columns(x)/trace identical |
| Worktree clean | `git status --porcelain src/ docs/schema/` | empty after the commit (scope containment is proven by the story-off/on additive test, not by this command) |
| S2 flow emit | `atlas-layout.json` on hyperfine | 191 signatures, 1586 call sites; labels 44 `unresolved` / 4 `type-only` / 1 `literal-observed` on 49 edges; cJSON 5 edges 1/2/2 |
| S2 final frame | two independent page loads → `#stage` DOM SHA-256 | identical (hyperfine `2f1a3834…`, cJSON `65cd5446…`); double scan layout/flow identical |
| S2 full gate | `bash sim/gate.sh` | `b8359ae`, tree `8889ecac…`, dirty 0, **verdict=PASS** |
| S2 vision | `VISION.md` Read 6 | **PASS** (card-text collisions, grey contrast, de-stack, legend fixed; nits carried) |
| S3 flow emit | `flow.io[]` on hyperfine | 8 rows / 5 files: read/env 3, read/file 2, write/file 3; 3 literal (`TERM`, `NO_COLOR`, `/dev/null`), 5 target-unknown |
| S3 flow emit | `flow.io[]` on cJSON | 24 rows / 4 files: read/file 2, read/stdio 2, write/stdio 20; 0 literal, 2 target-unknown |
| S3 guards | unit + e2e fixtures | drawn `read` wrapper, macro `fopen`, typedef fn-pointer and `sprintf` emit no row; `format!`/concat/non-literal path ⇒ `null`; stdio never literal; `freopen` mode arg 1; `fopen "r+"`/`O_RDWR` ⇒ both ops; non-zero numeric open flags ⇒ no row |
| S3 story gate | `SIM_PORT=8770 python3 sim/verify.py` | **217/217**, exit 0 (data/core/DOM + stores/trust/inspector frames) |
| S3 full gate | `bash sim/gate.sh` | final `GATE.txt` on the S3 close commit: dirty 0, 862 tests, **verdict=PASS** (the script fails on a non-zero full dirty count) |
| S3 vision | `VISION.md` Read 7 | **PASS** (first pass PASS-WITH-NITS; three nits fixed and re-read PASS) |
| S3 review | R1/R2/R3 + Q1/Q2 | **5/5 APPROVE, zero unresolved blocking findings.** Round 1: `freopen` index, numeric flags, typedef pointer, stale docs; fixed in `b63c6f6`/`f3e8804`/`27569a9`/`325571e`; R3 docs-only corrections re-confirmed |
| S4 core probes | `git show --stat 9263c19 e55221c 3c80ae9` | P6 purpose (712 new), P7 queue (373 new), P8 scenes (607 new + shared `file_cost`); all additive/story-only |
| S4 purpose emit | `--story` on hyperfine/cJSON | `A command-line benchmarking tool.` / `Ultralightweight JSON parser in ANSI C.`; structural fallback e2e |
| S4 queue emit | `profile` on hyperfine/cJSON/3k | hyperfine 12/35 eligible, 353 ranked; cJSON 12/45, 1055; 3k 12/2,730, 3,001 ranked; authoritative order asserted |
| S4 scenes emit | `scenes` on hyperfine/cJSON/3k | hyperfine 11/51 files (35 production/16 test); cJSON 3/81 (6/75); 3k 32/3,000 (2,731/269), shown 24, omitted 8/816; sums reconcile |
| S4 scene cost parity | unit test vs every drawn `AtlasNode.cost` | identical via shared `atlas::file_cost` |
| S4 story gate | `SIM_PORT=8770 python3 sim/verify.py` | **304/304**, exit 0 (data/core/DOM + purpose/scenes/queue/trust/unresolved/PATH/1280) |
| S4 3k scale gate | `bash sim/scale.sh` | **44/44**, exit 0; page 463,490 bytes; first frame under the 2,000 ms budget (final transcript); no h-scroll 1280/1440; F2–F6 PASS |
| S4 full gate | `bash sim/gate.sh` | `71d0d04` · tree `51548f3…` · dirty 0 · 897 tests · 304/304 · 3k 44/44 · **verdict=PASS** (`GATE.txt`; evidence `2ebeabd`) |
| S4 vision | `VISION.md` Read 8 | **PASS-WITH-NITS, no blockers** (two independent reads; purpose/1280/scene/queue/trust fixes landed; carried nits listed) |
| S4 review | 4-lane panel (R1/R2/Q1/Q2) + fresh Q3 | **5/5 APPROVE** (R1 APPROVE, R2 APPROVED, Q1 APPROVE, Q2 APPROVE, fresh Q3 APPROVE); zero unresolved blocking findings |

Full command/exit/hash transcript: `docs/evidence/atlas-sim/GATE.txt` (regenerated by `bash sim/gate.sh`).

## Open risks / follow-ups

- **S3 limits:** a wrapper defined outside the drawn set can still match; `read`/`write` are
  fd-based (`target: file` is the call class, may be pipe/socket); method calls
  (`OpenOptions::open`, `write_all`) are not matched; `env::args*` is an `env` read; `connect` is the
  outbound `write` net effect; literal values are source text (quotes stripped, escapes not decoded,
  display-capped at 160); stdio rows carry no literal.

- `--story` builds the Atlas view twice (render + projection). Costs one extra O(files) pass; fix in a
  single-pass artifact assembly later.
- `sim/verify.py`/`sim/scale.sh` are not CI jobs; the story gate needs Playwright + the pinned
  Chromium and runs locally (carried from S2/S3, not new).
- `findings_included` is defined as the drawn-node severity sum (matches the D1 extraction). It is a
  story-slice count, not a cap disclosure.
- P3/P1 fidelity is syntactic: drawn files only (second parse, story-only), local types only, no
  cross-file/points-to inference; field-access args are untyped; a pointer-to-array return is
  best-effort; `unresolved` includes "every argument untyped".
- **S4 limits:** P6 reads the first README with usable prose and the first prose paragraph (64 KiB/200-char caps; symlinked/no-prose candidates are skipped);
  a stale/marketing README is quoted as written and labelled); P7 carries 12 production non-demoted
  rows only (startup/no-loop demoted rows are counted, not shown); P8 caps scenes at 24 / 5 files and
  depth-2 (3k: the `src` direct-files scene with `main.c` is omitted while the canvas shows it; the
  269-file `tests` scene is omitted by production-first ordering but its counts are in the omitted
  block and the scan-wide test/bench count); scene cost is static cost, not runtime work; scene scale
  can be `mixed` on v1/v2 mixtures; the 3k fixture is synthetic (one Rust `dyn` file is forced into
  the drawn slice as the unreached hottest because the C entry cannot reach Rust across the language-
  scoped call graph).
- **S4 vision nits carried:** the third START item's scene-attribution tail at 1280/1440; `PATH` meta
  ellipsis at 1440 (full text in the title/summary); dense 3k depth-1 column + clipped orange edge
  labels; trust findings/edge-coverage run-on; cJSON unreached START + bottom canvas space
  (pre-existing); hyperfine edge-label density (pre-existing). The F1 purpose + first `path:line` and
  the 1280 legend/START/footer are readable.
- **F8:** founder A2 is deferred and not claimed; A2/ship remains behind the Atlas v2 step-13 founder
  gate (R-pf-109 for the map).
- **Non-blocking gate limits:** F4 regexes can match code identifiers (a path/function named
  `execution`); Playwright falls back to the bundled Chromium if the pinned path is absent; the 3k
  fixture reuse checks counts, not a content digest; the first frame is measured in cold browser
  contexts against a warmed local server.
- Evidence keyframe PNGs are captured 250 ms after a seek and are not byte-stable (known D1 nit;
  transitions still running). Capture after `transitionend` if byte-diffable frames are needed.
- Companion vs in-report: **DECIDED 2026-10-26 (ADR-0011, D-pf-259) — companion stays**; A12
  unchanged. The earlier "deferred until S2" note is resolved; an interactive in-report story would
  still need a new ADR + A12 amendment (A12 permits CSS/SMIL, not a second script).

## Commits

- `5faacb7` — `feat(core): atlas-layout story projection behind --story [P4/P5]`
- `deb2cc2` — `feat(sim): story data from Perfpal core, delete HTML extraction [S1]`
- `996d47e` — `fix(story): panel blockers — column geometry, provenance, gate`
- `462f4f4` — `fix(sim): stable metrics content hash across scans`
- `43a0841` — `fix(story): close re-review — canvas height, record, stale docs`
- `0461f75` — `fix(story): path-independent provenance, reproducible gate, record`
- `7170cad` — `fix(gate): exclude the transcript from the dirty count, record the verdict`
- `9ad6ea2` — `fix(story): hard gate assertions, tree attestation, final record`
- `63914f9` — `fix(gate): fail on projection errors; record final commit order`
- `2108117` — `core(P3): signature summaries`
- `ea72a86` — `core(P1): call-site payloads`
- `cac8be3` — `sim(S2): edge payloads and payoff beats`
- `ee74206` — `fix(flow): pointer returns and conservative resolution`
- `c99fade` — `fix(sim): legible labels and honest hottest beat`
- `eef9d99` — `fix(flow): function-pointer parameter list`
- `b8359ae` — `docs(S2): record function-pointer fix` (gate run here); evidence/docs commits follow.
- `0ddf4ad` — `core(P2): io effects`
- `b63c6f6` — `fix(P2): table guards and numeric flags`
- `9e23aa6` — `story(S3): io stores and trust line`
- `f3e8804` — `fix(S3): trust detail, inspector stdio and gate assertion`
- `27569a9` — `fix(P2): rename literal and guard tests`
- `325571e` — `fix(S3): row-level store checks`
- S3 docs/evidence/state commits follow (gate transcript on the docs commit); **not pushed**.
- `c3b40f8` — `docs(plan): add atlas-s4 scale scenes and payoff`
- `4006315` — `test(S4): add deterministic 3k fixture generator`
- `9263c19` — `core(P6): purpose line`
- `e55221c` — `core(P7): whole-repo profile queue`
- `3c80ae9` — `core(P8): directory scenes`
- `30a3bba` — `sim(S4): project purpose, scenes and queue`
- `49627b2` — `sim(S4): scene, queue and purpose payoff`
- `5f7579d` — `test(S4): 3k scale gate`
- `5f13b04` — `fix(S4): 1280 legibility and scan non-production` (post-vision gate run here)
- `61ef8c9` — `docs(S4): record scale probes and payoff evidence`
- `97c4cda` — `docs(evidence): S4 gate transcript and scale frames`
- `21e6057` — `fix(S4): panel round 1 records and evidence`
- `71d0d04` — `fix(S4): panel round 2 symlink and evidence` (final gate here)
- `2ebeabd` — `docs(evidence): S4 final gate at 71d0d04`
- S4 docs/state close commit follows this list; **not pushed, not merged**.

## ADR-0011 — companion vs in-report (2026-10-26)

**Decision: companion.** The Atlas Story does not graduate into `report.html` in v1; **A12 is
unchanged** (exactly one fixed `REPORT_SCRIPT`; zero scripts/handlers/network in the
canvas/SVG/animation artifacts; no CSS/SMIL story film). No code, schema, layout or dependency
change; `layout_version`/`flow_version` stay 1.

**Why.** S4 proves the mechanism (F1–F7; 3k 44/44; 304/304; page 463,490 B; first frame 1,706 ms;
F6 byte-identity; F5 double-scan determinism), and the F1/scale reads are PASS-WITH-NITS, no
blockers. But the founder has **not used the post-S4 story**: the 2026-10-24 D2 session reviewed
**D1** (presentation accepted, information not insightful: D-pf-251), and F8 is deferred. The
pre-registered **§5 graduation session on the post-S4 artifact** has not run. In-report controls also
require a second script + data channel + deterministic/offline gates and a port of the `sim.js`
player into the Rust emitter. The companion itself is not shipped: it `fetch()`es its data and needs
an HTTP server; `--story` adds only `atlas-layout.json`; the `perfpal story` subcommand is deferred.

**Branch snapshot.** This branch is a pre-merge story snapshot at `8f0249e`. Main has since
recorded Atlas v2 done (`c797004`; step 13 approved, A2 delegated pass, `R-pf-109` closed:
D-pf-255/257/258), while this branch's state says `R-pf-109` open. The ADR does not rely on the
report gate being open; at merge, condition (b) is satisfied. The recorded "61 ahead / 8 behind" was stale: the 2026-10-26
recount is **63 ahead / 23 behind** (`ADR-0012`). Decision IDs collided (branch D-pf-251..255 =
D2/S1/S2/S3/S4; main = F-treemap/archived/step-13/archived/A2 PASS); `ADR-0012` remaps them to
D-pf-260..264 and keeps `D-pf-259` for this ADR.

**Graduation conditions (ADR-0011 Decision 4).** (a) a founder **§5 graduation session on the
post-S4 story** (distinct from the 2026-10-24 D1 review) on a self-contained artifact, including the
F8/A2 action read; (b) the report's visual gate (`R-pf-109` closed; on main 2026-10-26); (c) a
deterministic, offline, no-network player with a complete final frame and pause/reduced-motion/print;
(d) an explicit A12 amendment + founder ratification.

**Recorded follow-up (not active).** Companion packaging: inline the data (no `fetch`/server) and
add a CLI entry point; that is the precondition for the §5 session. No ship claim.

**Panel record (ADR-0011).** R1 correctness/claims: **CHANGES_REQUESTED** round 1 (D2 wording,
HEAD/evidence reproducibility, no-code proof) → fixed in `6d95411`; R2 contract/A12/scope:
**APPROVE**; Q1 verbatim scope: **APPROVED**; Q2 acceptance/evidence: **CHANGES_REQUESTED** round 1
(same evidence items) → fixed in `6d95411`; fresh confirmation lane: **APPROVE**. Round-2 textual
fixes (CHANGELOG D1/§5 wording, close-record reference, ID list) follow in the final close commit.

## Next

- **S4 close:** complete (panel 5/5 APPROVE, final gate `71d0d04`, evidence `2ebeabd`); **ADR-0011
  decided 2026-10-26 (D-pf-259) — companion, A12 unchanged.** F8 remains founder-deferred.
- **Reconciliation recorded** (`R-pf-112`, `ADR-0012`, Proposed): direction **C** — keep this
  branch separate from `main` until the §5 founder session; no story→main merge now. Next (after
  founder approval): merge `main` into this branch (sync; 7 conflicted paths), run the full story
  gate, then package the self-contained story + `perfpal story` CLI and take it to §5. **Do not
  re-run Atlas v2 step 11 here** — main closed steps 11–15 (`c797004`; D-pf-258). No A2/ship.

## R-pf-112 reconciliation (2026-10-26)

**Record:** `docs/adr/ADR-0012-atlas-branch-reconciliation.md` (Proposed; founder approval required
before any merge/rebase/cherry-pick/push/code change).

**Facts (corrected).** `feat/atlas-sim` @ `988aea2`; `main` @ `c797004`; merge base `821b984`
(v2 steps 9–10). `git rev-list --left-right --count main...HEAD` → **23 63**: 63 story commits
ahead, **23** main commits behind (the recorded 61/8 was stale). Main is v2 DONE: steps 11–15
(`189df91`, `9a94905`, step 13 `86c0537`/D-pf-253, A2 `D-pf-255`, step 15 `57d91a2`/D-pf-256/257/258);
`R-pf-109` closed; `atlas-build` done. This branch is a pre-merge story snapshot; its `atlas-build`
copy is now marked done with a snapshot banner.

**Dry-run sync.** `git merge-tree --write-tree --name-only main HEAD` → 7 conflicted paths:
`README.md`, `docs/activeContext.md`, `src/reporter/atlas_render.rs`, `src/reporter/mod.rs`,
`state/archive/ledger.jsonl`, `state/handoff/latest.md`, `state/session-state.json`. Auto-merges:
`CHANGELOG.md`, `docs/report/output-contract.md`, the Atlas specs, `src/cli/mod.rs`. Known API fix:
main's `strip_entry_line` is private; restore `pub(crate)` for the story projection. `start_actions`
and `build_atlas_view` survive on main.

**ID remap (applied).** `D-pf-251→260`, `D-pf-252→261`, `D-pf-253→262`, `D-pf-254→263`,
`D-pf-255→264`; `A-pf-124→126`, `A-pf-125→127`. `D-pf-259` (ADR-0011) unchanged; `D-pf-265` records
this reconciliation; `D-pf-266` is the hot-state archive pointer for `D-pf-238..243`. `R-pf-109`
is closed on this branch as a main-line record; `R-pf-112` stays open until the sync/packaging lands.
The ledger keeps the old IDs plus a `decision_id_remap` entry.

**Direction (recommended, pending approval).** **C** — no story→main merge until the §5 founder
session; next is a `main`→branch sync, then the self-contained story + CLI packaging slice. A/B are
recorded in ADR-0012 with tradeoffs and "what would change my mind"; A would use the merge-based A′
(not a literal rebase) to preserve evidence SHAs.
