# Track D — Atlas Story · Session D2 + S1 + S2 + S3 handoff

**Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` · branch `feat/atlas-sim`
**Track:** `/home/shafin/Opencode/Perfpal/docs/tracks/atlas-run.md`
**Status:** **D2 done + S1 committed (`5faacb7..63914f9`) + S2 committed and gated
(`2108117..b8359ae`) + S3 committed (`0ddf4ad..325571e`; docs `89f4feb`), not pushed.** D1 presentation was accepted
by the founder; the requirement was re-sealed as a core-fed temporal showcase
(`docs/specs/atlas-story.md`); S1 made the story read core JSON, S2 added data movement on the edges,
S3 added I/O effects (P2) with the stores/trust render. S4 (scenes at 3k files) is next. No push, no
merge to `main`.

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

Full command/exit/hash transcript: `docs/evidence/atlas-sim/GATE.txt` (regenerated by `bash sim/gate.sh`).

## Open risks / follow-ups

- **S3 limits:** a wrapper defined outside the drawn set can still match; `read`/`write` are
  fd-based (`target: file` is the call class, may be pipe/socket); method calls
  (`OpenOptions::open`, `write_all`) are not matched; `env::args*` is an `env` read; `connect` is the
  outbound `write` net effect; literal values are source text (quotes stripped, escapes not decoded,
  display-capped at 160); stdio rows carry no literal.

- `--story` builds the Atlas view twice (render + projection). Costs one extra O(files) pass; fix in a
  single-pass artifact assembly later.
- `findings_included` is defined as the drawn-node severity sum (matches the D1 extraction). It is a
  story-slice count, not a cap disclosure.
- P3/P1 fidelity is syntactic: drawn files only (second parse, story-only), local types only, no
  cross-file/points-to inference; field-access args are untyped; a pointer-to-array return is
  best-effort; `unresolved` includes "every argument untyped".
- The `profile` beat is the drawn-slice top-3-per-file candidate set, not a whole-repo KV3 queue
  (labelled as such); S4 owns the full queue.
- Vision nits carried: cJSON `cJSON · type-only` detached ~50 px beside the hull label; dense
  hyperfine top band; display-truncated `…Option<Information>`; border graze; `PATH · 3 steps · re…`
  chrome truncation; cJSON ~1/3 dead space.
- Evidence keyframe PNGs are captured 250 ms after a seek and are not byte-stable (known D1 nit;
  transitions still running). Capture after `transitionend` if byte-diffable frames are needed.
- Companion vs in-report is deferred until S2 produces evidence; an interactive in-report story would
  need a new ADR + A12 amendment (A12 already permits CSS/SMIL, not a second script).

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

## Next

- **S4:** scenes/aggregation at 3k files + F1–F8; then the companion-vs-report ADR (+A12 if
  interactive). S3 is done and recorded.
- Atlas v2 step 11 stays queued; A2/ship still blocked by the step-13 founder gate (R-pf-109).
