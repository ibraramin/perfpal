# Perfpal

> **Beta — `0.4.0`.** Pre-release evaluation build (C, Rust, Python, JavaScript and TypeScript/TSX backends), provided for **noncommercial evaluation only — not for commercial or production use at this beta**. Use is governed by the [`EULA`](EULA.md), which supplements the PolyForm Strict 1.0.0 [`LICENSE`](LICENSE). See also the [privacy statement](PRIVACY.md). **Binaries are unsigned** — see [Beta status, code signing, and privacy](#beta-status-code-signing-and-privacy).

Perfpal is a static-analysis CLI that scans C, Rust, Python, JavaScript and TypeScript/TSX codebases for performance-relevant patterns and writes a self-contained `perfpal-report/` directory: a versioned, LLM-consumable `metrics.json`, a human-readable `report.md`, a designed **Atlas** human map (`report.html#map`), LLM-facing `agent-report.md` / `agent-actions.json`, and deterministic SVG architecture charts. It is built for AI-generated codebases — high file volume, frequently non-compiling trees, missing headers and build configuration — where a compiler-based pipeline is often not viable. Parsing runs on a tree-sitter syntactic tier (ADR-0001): no build flags, compile database, or macro expansion are required. All output is framed as **optimization candidates for review, never certainties**.

## How it works

- **Scan** — recursive discovery in lexicographic order (dot-directories skipped, symlinks not followed): `.c`/`.h` files are parsed by the bundled `tree-sitter-c` backend, `.rs` by `tree-sitter-rust`, `.py`/`.pyi` by `tree-sitter-python`, `.js`/`.jsx`/`.mjs`/`.cjs` by `tree-sitter-javascript` (token `js`; JSX is embedded), and `.ts`/`.mts`/`.cts`/`.tsx` by `tree-sitter-typescript` (tokens `ts`/`tsx`). Declaration-only `.d.ts`/`.d.mts`/`.d.cts` files are skipped. Discovery also skips `node_modules`, `bower_components`, `dist`, `build`, `out`, `coverage`, plus JS/TS `*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js`/`*.min.ts`/`*.min.mts`/`*.min.cts` (seven suffixes) and files over 2 MiB. All five backends are live: 78 catalog heuristics in total (23 C + 22 Rust + 22 Python + 11 JS/TS); the Rust module/`use` graph resolves with Cargo.toml-aware crate roots, Python imports resolve best-effort, and JS/TS relative imports resolve with `package.json` entry declarations. Parse health (ok / partial / failed) is captured per file; quoted includes are resolved against the filesystem, and in-repo angle-bracket hits also count as internal coupling (ADR-0002); unresolvable angle-bracket includes are bucketed as external.
- **Analyze** — 78 catalog heuristics (23 `PERF-C-*` C + 22 `PERF-RS-*` Rust + 22 `PERF-PY-*` Python + 11 `PERF-JS-*` shared by JavaScript/TypeScript/TSX) run as pure functions over the parser-agnostic IR. Every finding carries mandatory `confidence` and `severity`; documented downgrade hooks (macro origin, parse-error overlap, name-based cross-file matching) cap confidence and set flags instead of silently dropping or trusting data.
- **Aggregate** — the include graph, the cross-file call graph (name-based, confidence locked LOW), component ranking, and per-function statistics are assembled into one `metrics.json` document with every sort order explicitly owned.
- **Render** — `report.md` and the Atlas page are rendered from fixed templates; the SVG charts are pure functions of `metrics.json` structures and are byte-deterministic.
- **Write** — the report directory is swapped into place atomically; an interrupted run never leaves a half-written report.

## Install

Perfpal is not published to crates.io (`publish = false` in `Cargo.toml`). It ships as release assets from the private repository `github.com/ibraramin/perfpal`; because the repo is private, releases are visible only to repository collaborators.

Release binaries follow the naming pattern `perfpal-<rust-target>[.exe]` with a sibling `<name>.sha256` checksum file. Each release ships nineteen assets:

- **six target binaries** (table below) + **six per-file `.sha256`** checksums;
- **`SHA256SUMS`** — one aggregate manifest over all six binaries — and its **minisign signature `SHA256SUMS.minisig`**;
- **`perfpal-<version>.spdx.json`** — an SPDX 2.3 SBOM of the dependency closure;
- **`THIRD-PARTY-NOTICES.md`** — attribution for the bundled third-party crates;
- **`EULA.md`** — the beta End User License Agreement (supplements `LICENSE`);
- **`PRIVACY.md`** — the privacy statement (no telemetry, no network calls);
- **`LICENSE`** — the PolyForm Strict License 1.0.0.

(The v0.1.0 release shipped ten assets: the five targets below minus `gnu`, which joined post-v0.1.0 as the Linux semantic-tier vehicle, and none of the manifest, signature, SBOM, notices, or legal document files.)

| Asset | Platform |
|---|---|
| `perfpal-x86_64-unknown-linux-musl` + `perfpal-x86_64-unknown-linux-musl.sha256` | Linux x86-64, statically linked (musl) |
| `perfpal-aarch64-unknown-linux-musl` + `perfpal-aarch64-unknown-linux-musl.sha256` | Linux arm64, statically linked (musl) |
| `perfpal-x86_64-unknown-linux-gnu` + `perfpal-x86_64-unknown-linux-gnu.sha256` | Linux x86-64, glibc (semantic-tier vehicle) |
| `perfpal-x86_64-apple-darwin` + `perfpal-x86_64-apple-darwin.sha256` | macOS x86-64 |
| `perfpal-aarch64-apple-darwin` + `perfpal-aarch64-apple-darwin.sha256` | macOS arm64 (Apple Silicon) |
| `perfpal-x86_64-pc-windows-msvc.exe` + `perfpal-x86_64-pc-windows-msvc.sha256` | Windows x86-64 (MSVC) |

Example install and per-file checksum verification (the `.sha256` embeds the shipped binary filename, so `sha256sum -c` works as-is):

```
gh release download v0.2.0-beta.1 --repo ibraramin/perfpal --pattern 'perfpal-x86_64-unknown-linux-musl*'
chmod +x perfpal-x86_64-unknown-linux-musl
sha256sum -c perfpal-x86_64-unknown-linux-musl.sha256
```

For the signed aggregate manifest, download `SHA256SUMS` and `SHA256SUMS.minisig` alongside the binaries, then verify the manifest's signature against the committed [`minisign.pub`](minisign.pub) and every binary against the manifest:

```
minisign -Vm SHA256SUMS -p minisign.pub   # verifies the manifest's signature
sha256sum -c SHA256SUMS                    # verifies all shipped binaries
```

`minisign` (the reference implementation, ISC-licensed) or a compatible verifier such as the MIT `minisign-verify` crate can be used. `SHA256SUMS` covers the six binaries only — the SBOM and notices, as well as the legal documents (`EULA.md`, `PRIVACY.md`, `LICENSE`), are not covered by the manifest. The manifest stays binary-only so the documented single-binary `sha256sum -c SHA256SUMS` convenience keeps working; treat a binary that is not covered by the signed manifest — or by its own `.sha256` — as untrusted.

Post-v0.1.0 (plan `semantic-resolver`), the release matrix grows to six targets: `x86_64-unknown-linux-gnu` joins as the Linux semantic-tier vehicle (`perfpal-x86_64-unknown-linux-gnu` + checksum, same naming pattern). The gnu, both darwin, and Windows-MSVC builds compile with the default-OFF `semantic-clang` cargo feature and activate the semantic tier when libclang is installed (`LIBCLANG_PATH`); without libclang they degrade gracefully to the syntactic tier. The musl x86_64/aarch64 builds stay syntactic-only by design (static musl cannot dlopen) and remain fully functional.

To build from source instead:

```
cargo build --release --locked
```

The toolchain is pinned to Rust 1.92.0 (`rust-toolchain.toml`).

## Quickstart

Scan a directory (a single `.c`/`.h`/`.rs`/`.py`/`.pyi`/`.js`/`.jsx`/`.mjs`/`.cjs`/`.ts`/`.mts`/`.cts`/`.tsx` file also works; declaration-only `.d.*` files are skipped silently):

```
$ perfpal path/to/your-c-project
scanned 214 files (210 ok, 3 partial, 1 failed) · 57 findings (3 hot, 21 warn, 33 info)
report: perfpal-report
```

The stdout line above is illustrative, not a stability contract. Use `-o` to relocate the report directory:

```
perfpal . -o my-report
```

A completed scan leaves exactly this layout (default `perfpal-report/`):

```
perfpal-report/
├── metrics.json              # raw machine-readable data (schema v1.8)
├── metrics.schema.v1.8.json  # byte-copy of the exact schema for this schema_version
├── report.md                 # human-readable report (answer-first key views)
├── report.html               # static HTML sibling (key views + Explorer)
├── findings-detail.md        # complete ranked action list + full finding detail
├── findings-detail.html      # same detail as HTML tables
├── profile-queue.md          # complete cost-ranked function list (KV3 drill)
├── profile-queue.html        # same list as an HTML table
├── atlas-weight.html         # Atlas weight drill (complete file/component share detail)
├── atlas-q2.html             # Atlas blast-radius drill (dependents + component DSM)
├── agent-report.md           # bounded LLM-facing triage report
├── agent-actions.json        # complete machine action list
└── charts/                   # static visualizations (set defined by ADR-0002)
    └── <chart-name>.svg      # e.g. include-graph.svg
```

The **Atlas page** (`report.html#map`) is the product's human map (beta artifacts; publish gate founder-owned): a key-metrics strip (entry, HOTTEST file, three START actions, one caveat, one legend/unit definition) above a graph-only entry-flow canvas with a right-hand detail rail, followed by four designed charts — weight share, blast radius, file detail, and coverage/links. Depth is one click away: `atlas-weight.html` (complete weight detail) and `atlas-q2.html` (reverse drill: what breaks if I touch the file), plus the complete ranked action list (`findings-detail.*`) and the complete cost-ranked function list (`profile-queue.*`). The page also links `agent-report.md` and `agent-actions.json`, the bounded and complete LLM-facing artifacts. The canvas draws the default-tier **structural** call graph (`LOW name-based + MEDIUM semantic`) and every view is framed as optimization candidates to confirm with a profiler — never runtime truth. The page is fluid from 1280 to 3072 CSS px by default; `--report-width <auto|1280|1440|1920|2560|3072>` pins a design width as a layout-only hint (it changes only `report.html` — its CSS and the `data-report-width` layout marker — never data or other artifacts).

The **Atlas Story** is the temporal companion to the map — not a second map — and is **always embedded** in `report.html` below the Atlas charts (ADR-0014). It renders the inferred run (entry → stages → loops → I/O → stop) with the payload crossing each file boundary labelled, from a fixed `STORY_SCRIPT` plus an inline `application/json` payload: no `fetch`/XHR/`eval`/HTTP, offline from `file://`, with a JS-off fallback to the static canvas plus a `<noscript>` beat narrative and reduced-motion/print at the final frame. It is static inference, never a runtime trace. `perfpal <repo> --story` no longer gates the story; it only adds `atlas-layout.json` — the canvas geometry the SVG already draws plus syntactic `flow`, `purpose`, `profile` and `scenes` blocks — and changes no shared artifact. A12 is amended to exactly three script elements (ADR-0014); the standalone `sim/` player remains the reference/evidence implementation, with the in-report S4 panels presented as a closed `<details>`. `layout_version`/`flow_version` stay `1`.

The eight Explorer stems (`#explorer`) are `include-graph.svg`, `layer-file.svg`, `layer-entry.svg`, `component-coupling.svg`, `hot-components.svg`, `layer-workspace.svg`, `layer-bands.svg`, and `layer-findings.svg`, plus per-component detail pages (`component-<rank>-<slug>.svg`) and the `graph.dot` export; each chart is emitted only when its source section is present and non-empty (with deterministic stubs where the source exists but is empty), so `charts/` may be empty. Tooling and LLM sessions should start at `metrics.json` (validate against the bundled schema copy) or `agent-report.md` / `agent-actions.json`; humans can start at `report.html#map` or `report.md`. Never parse data out of the SVGs — they are advisory.

## CLI reference

| Flag | Argument | Default | Meaning |
|---|---|---|---|
| `<path>` (positional) | `PATH` | `.` | Scan target: a directory or a single `.c`/`.h`/`.rs`/`.py`/`.pyi`/`.js`/`.jsx`/`.mjs`/`.cjs`/`.ts`/`.mts`/`.cts`/`.tsx` file (declaration-only `.d.*` skipped) |
| `-o`, `--output` | `DIR` | `perfpal-report` | Report directory; internal layout is fixed (output-contract §2) |
| `--force` | — | off | Allow replacing an existing directory that contains no `metrics.json` (safety-guard override) |
| `--story` | — | off | Also emit `atlas-layout.json`, the opt-in data artifact for the Atlas Story companion (layout/flow `1`; report output is byte-identical) |
| `--report-width` | `auto\|1280\|1440\|1920\|2560\|3072` | `auto` | Layout-only width hint for `report.html`: `auto` is fluid 1280→3072; a fixed value pins the page design width. Never changes data, scripts, or other artifacts |
| `--version` | — | — | Print `perfpal <version> (metrics schema <schema_version>)` (e.g. `perfpal 0.4.0 (metrics schema 1.8)`), exit 0 |
| `-h`, `--help` | — | — | Usage text, exit 0; prints a beta evaluation-notice footer |

stdout carries the short plain-text summary shown above; all diagnostics, warnings, and progress go to stderr. The summary and stderr diagnostics carry no color or cursor control — output stays pipe-clean (interactive `--help`/`--version` rendering is clap's own and disables itself when piped). Machine consumption starts at `metrics.json`, never stdout parsing.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success — report written. Findings never affect the exit code (they are data). Zero-file scans succeed with a valid empty report (`scan.languages` = the supported set `["c","js","python","rust","ts","tsx"]`). |
| `1` | Usage error — unknown flag; missing/unreadable target path. |
| `2` | Output error — cannot create output dir; safety-guard refusal without `--force`; disk/write failure. |
| `3` | Internal error — unexpected panic/crash, always a bug (this also covers a `metrics.json` serialization failure). |

## Heuristics

Perfpal's catalog defines **78** heuristics — 23 C (`PERF-C-*`), 22 Rust (`PERF-RS-*`), 22 Python (`PERF-PY-*`), and 11 shared JavaScript/TypeScript (`PERF-JS-*`) — all of which fire on the syntactic resolver tier. On `semantic-clang` builds with libclang present, proven semantic facts upgrade C findings above their catalog defaults — resolved ARCH-003 cross-file edges report MEDIUM instead of LOW (the syntactic tier stays LOW); the Rust, Python and JS/TS backends are **syntactic-tier only** (the libclang resolver is C-only), so no Rust, Python or JS/TS finding is ever upgraded. Without libclang every build degrades gracefully to the syntactic tier. `PERF-TS-*` is reserved for TypeScript-only constructs and v1 ships none: JS/TS/TSX share the `PERF-JS-*` ids because the V8/Node mechanism is identical. Every finding carries mandatory `confidence` (HIGH / MEDIUM / LOW) and `severity` (`hot` / `warn` / `info`); both are prioritization hints, and every finding remains an optimization candidate. The tables show each rule's catalog defaults; documented per-rule hooks shift severity (e.g. `free()` in a loop escalates warn → hot when allocation co-occurs in the same function; conditionally guarded I/O lowers hot → warn), and downgrade rules cap confidence (macro origin or parse-error overlap caps at MEDIUM; name-based cross-file edges are LOW).

Thresholds are named constants documented in the catalogs (`docs/heuristics/catalog-v1.md` for C, `docs/heuristics/rust-catalog.md` for Rust, `docs/heuristics/js-catalog.md` for JavaScript/TypeScript; the Python per-rule notes and disposition table live in `docs/architecture/python-backend.md`) and are not runtime-configurable.

### ALLOC — allocation churn

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-ALLOC-001` | `malloc`/`calloc`/`realloc`/`aligned_alloc`/`strdup` call nested anywhere inside a loop body | hot | HIGH | 1 |
| `PERF-C-ALLOC-002` | `free()` directly inside a loop body | warn | HIGH | 1 |
| `PERF-C-ALLOC-003` | `realloc(p, n+k)` additive-growth shapes nested in loops (quadratic memmove behavior; multiplicative growth is the documented fix, not a finding) | hot | MEDIUM | 1 |

### ARCH — architecture coupling

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-ARCH-001` | Include-graph fan-out/fan-in hubs (≥12), deep include chains (≥8), cyclic includes, unresolvable quoted includes | warn | HIGH | 1 |
| `PERF-C-ARCH-002` | Hot-component aggregation — files/components ranked by a composite of complexity sum, max loop depth, and alloc-site density | hot | HIGH | 1 |
| `PERF-C-ARCH-003` | Cross-file call-graph edges from name matching (confidence locked LOW on the syntactic tier; `semantic-clang` builds with libclang report USR-resolved edges at MEDIUM/semantic) | info | LOW | 1 |

### CALL — function-level signals

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-CALL-001` | Direct recursion — a function body calling its own name (presence-only; recursion depth is not statically bounded in v1) | info | HIGH | 1 |
| `PERF-C-CALL-002` | Cyclomatic complexity ≥ 15 per function (promoted from the baseline stat) | warn | HIGH | 1 |

### LOOP — loop-body patterns

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-LOOP-001` | Deep loop nesting — nesting depth ≥ 3 (O(n^k) candidate zones) | warn | HIGH | 1 |
| `PERF-C-LOOP-002` | Expensive setup / I/O calls nested in loops (printf-family, `fopen`/`fclose`/`open`/`close`/`read`/`write`, `getenv`, `system`, `regcomp`) | hot | MEDIUM | 1 |
| `PERF-C-LOOP-003` | Char-wise stdio loop — `fgetc`/`getc`/`getchar`/`fputc`/`putc` as the primary statement of a loop body (vs block reads) | info | MEDIUM | 1 |
| `PERF-C-LOOP-004` | Integer `/` or `%` with a variable (non-literal) divisor inside the innermost loop body | info | MEDIUM | 1 |
| `PERF-C-LOOP-005` | Assignment to a file-scope/global identifier inside a loop body | info | MEDIUM | 2 |
| `PERF-C-LOOP-006` | Indirect call through a function pointer inside an innermost loop body | info | LOW | 2 |
| `PERF-C-LOOP-007` | Linear membership scan — comparison against an array element indexed by an outer loop's induction variable (hand-rolled O(n²) lookup), at nesting depth ≥ 2 | info | LOW | 2 |
| `PERF-C-LOOP-008` | Pointer-chasing chain — member-access chain of depth ≥ 3 (`a->b->c->d`) inside an innermost loop body | info | MEDIUM | 2 |

### MEM — memory operations

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-MEM-001` | sizeof-misuse shapes in allocation arguments (e.g. `sizeof` of a pointer parameter, `sizeof(T)*sizeof(U)`) | warn | MEDIUM | 2 |
| `PERF-C-MEM-002` | `memcpy`/`memmove`/`memset` with a non-literal size argument inside a loop (escalates to warn in an innermost loop) | info | MEDIUM | 2 |

### STR — string operations

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-STR-001` | String accumulation — `strcat`/`strncat` with the destination reused across loop iterations (O(n²) rescan) | hot | HIGH | 1 |
| `PERF-C-STR-002` | `strlen(s)` inside a loop condition/body where `s` is not reassigned in the loop | warn | HIGH | 1 |
| `PERF-C-STR-003` | Unbounded copy family — `strcpy`, `strcat`, `sprintf`, `gets`, `scanf("%s…")` | warn | HIGH | 1 |

### TYPE — data layout

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-C-TYPE-001` | Large struct passed by value — estimated size ≥ 32 bytes (one aggregated finding per function; approximate LP64 size model) | warn | LOW | 2 |
| `PERF-C-TYPE-002` | Struct member ordering — estimated padding waste ≥ 8 bytes (one full LP64 word) | info | LOW | 2 |

### Rust backend (`PERF-RS-*`)

The Rust backend mirrors the C families as **22** `PERF-RS-*` rules — `PERF-RS-STR-001` was **retired** by the M1 precision gate (the C `strcat` O(n²) premise does not transfer to Rust's amortized-O(1) `push`/`push_str`, and the rule's real effect was the `Vec::push` name collision; the C `PERF-C-STR-001` rule is unchanged). **Rust is syntactic-tier only** — the libclang `semantic-clang` resolver is C-only, so every Rust finding keeps its catalog confidence (no resolved-edge / resolved-format upgrades). Several rules are **honest reframings** of their C counterparts (Rust has no `realloc`, `sizeof`, or unbounded `strcpy`); the full per-rule implementation notes and reframings live in `docs/heuristics/rust-catalog.md`.

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-RS-ALLOC-001` | Heap-allocation call or macro (`Vec::new`, `Box::new`, `String::with_capacity`, `to_string`, `vec!`, `format!`, …) in the **innermost** loop (`in_loop_depth ≥ 1` + `is_innermost`); **one aggregated finding per innermost loop** (`alloc_count` + short callee list); literal-receiver `"x".to_string()`/`"x".to_owned()` suppressed | hot | HIGH | 1 |
| `PERF-RS-ALLOC-002` | Direct, non-macro `drop()` at `in_loop_depth ≥ 1` (escalates warn → hot when an alloc-family site also fires in the same function) | warn | HIGH | 1 |
| `PERF-RS-ALLOC-003` | Direct, non-macro `reserve`/`reserve_exact` at `in_loop_depth ≥ 1` (additive buffer growth — the `realloc` analogue) | hot | MEDIUM | 1 |
| `PERF-RS-ARCH-001` | `use`-import-graph hubs (≥12), deep chains (≥8), cycles over resolved internal edges (`mod` declarations excluded) | warn | HIGH | 1 |
| `PERF-RS-ARCH-002` | Hot-component aggregation — Rust component dirs ranked by complexity sum, max loop depth, and alloc-site density | hot | HIGH | 1 |
| `PERF-RS-ARCH-003` | Cross-module call-graph edges from name matching; a **name-based edge requires the callee name to be defined in exactly one file** (ambiguous names such as `new` emit nothing) | info | LOW | 1 |
| `PERF-RS-CALL-001` | Direct recursion — a function body calling its own name (presence-only; recursion depth is not statically bounded) | info | HIGH | 1 |
| `PERF-RS-CALL-002` | Cyclomatic complexity ≥ 15 per function (promoted from the baseline stat) | warn | HIGH | 1 |
| `PERF-RS-LOOP-001` | Deep loop nesting — nesting depth exactly 3 (one finding per maximal nest entry) | warn | HIGH | 1 |
| `PERF-RS-LOOP-002` | Curated I/O / setup calls nested in loops — print/write macros, read/write methods, `File::open`, `fs::read`, `Command::new`, `TcpStream::connect`, `env::var`, … | hot | MEDIUM | 1 |
| `PERF-RS-LOOP-003` | Char/byte-at-a-time iterator or reader method (`chars`, `bytes`, `char_indices`, `read_byte`) as the loop body's primary statement | info | MEDIUM | 1 |
| `PERF-RS-LOOP-004` | Integer `/` or `%` with a non-literal divisor inside the innermost loop body | info | MEDIUM | 1 |
| `PERF-RS-LOOP-005` | Assignment to a collected `static`/`const` global inside a loop body (dedup per target and enclosing loop) | info | MEDIUM | 2 |
| `PERF-RS-LOOP-006` | Indirect call inside an innermost loop body (closure-field / trait-object dispatch classifies Direct, so it does not fire) | info | LOW | 2 |
| `PERF-RS-LOOP-007` | Linear membership scan — comparison against a subscript indexed by an outer loop's induction variable at nesting depth ≥ 2 | info | LOW | 2 |
| `PERF-RS-LOOP-008` | Member/index chain of depth ≥ 3 **containing at least one `Subscript` element** inside an innermost loop body (pure member chains such as `ctx.ops.fun` stay silent) | info | MEDIUM | 2 |
| `PERF-RS-MEM-001` | `with_capacity(size_of::<T>())` — a byte count passed where an element count is expected | warn | MEDIUM | 2 |
| `PERF-RS-MEM-002` | Block-copy family (`copy_from_slice`, `copy_nonoverlapping`, `extend_from_slice`) inside a loop (escalates info → warn innermost); `clone`/`to_vec` are no longer included (needs receiver-type evidence) | info | MEDIUM | 2 |
| `PERF-RS-STR-001` | **RETIRED (M1)** — String accumulation via `push_str`/`push` was C-premised; no longer emitted for Rust (the C `PERF-C-STR-001` rule remains) | — | — | — |
| `PERF-RS-STR-002` | `len`/`is_empty`/`capacity` on a bare identifier inside a loop where the receiver is not reassigned in the loop | warn | HIGH | 1 |
| `PERF-RS-STR-003` | Unbounded `unsafe` memory operation (`copy_nonoverlapping`, `copy`, `from_raw_parts`, `from_utf8_unchecked`, `set_len`), at any loop depth | warn | HIGH | 1 |
| `PERF-RS-TYPE-001` | Large struct passed by value — estimated size ≥ 32 bytes (one aggregated finding per function; approximate LP64 size model) | warn | LOW | 2 |
| `PERF-RS-TYPE-002` | Struct member ordering — estimated padding waste ≥ 8 bytes (`repr(Rust)` may reorder fields; deliberately lower-confidence) | info | LOW | 2 |

Rows are ordered by family: ALLOC, ARCH, CALL, LOOP, MEM, STR, TYPE. Every finding remains an optimization candidate.

### Python backend (`PERF-PY-*`)

The Python backend ships **22** `PERF-PY-*` rules: **13 transferred/reframed mirrors** of the C/Rust catalog plus **9 Python-native additions**. Ten C/Rust ids are **retired** for Python because their premise does not transfer to CPython (`TYPE-001`/`TYPE-002` — no by-value struct layout; `MEM-001` — no `sizeof`; `ALLOC-002` — no `free`; `STR-002` — `len()` is O(1); `STR-003` — memory-safe; `LOOP-004` — bytecode-era div/mod; `LOOP-005` — a "global store" is a dict write; `LOOP-006` — every call is dynamic dispatch; `LOOP-008` — attribute chains are idiomatic and PEP 659-cached). **Python is syntactic-tier only** (T1: AST + best-effort binding/alias inference, `resolver_tier = syntactic`): the libclang `semantic-clang` resolver is C-only, and the cross-module resolver (`python-resolver`, T2) and runtime-profile ingestion (T3) are out of scope. Inference-gated findings are capped at MEDIUM and never `semantic`. Messages cite the CPython mechanism (per-object header allocation, list re-scan, immutable strings, event-loop blocking, N+1 round-trips). Per-rule notes and the full disposition table live in `docs/architecture/python-backend.md`.

| Rule | Detects | Severity | Confidence |
|---|---|---|---|
| `PERF-PY-ALLOC-001` | Loop-invariant container/object construction (all-literal args) inside a loop — an f-string in a loop is **not** a finding | info | MEDIUM |
| `PERF-PY-ALLOC-003` | Superlinear list accumulation in a loop — non-in-place `acc = acc + [x]`, or `append`/`extend` into an accumulator never rebuilt in the loop | warn | MEDIUM |
| `PERF-PY-ALLOC-004` | Eager re-materialisation (`list()`/`dict()`/`set()`/`tuple()` of a non-literal iterable) inside a loop | info | MEDIUM |
| `PERF-PY-ARCH-001` | Import cycle over resolved internal edges | info | MEDIUM |
| `PERF-PY-ARCH-002` | Hot-component aggregation — directories ranked by cyclomatic-complexity sum (conservative floor; not decision-grade) | info | MEDIUM |
| `PERF-PY-ARCH-003` | Cross-file call coupling from name matching (needs exactly one other-file definition; duck typing makes this noisy) | info | LOW |
| `PERF-PY-ASYNC-001` | Blocking call inside `async def` — `time.sleep`, a blocking HTTP client, a sync DB-driver method, or the builtin `open` | warn | MEDIUM |
| `PERF-PY-ASYNC-002` | Sequential `await` inside a loop (serialises latency `asyncio.gather` could overlap) | warn | MEDIUM |
| `PERF-PY-CALL-001` | Direct recursion — framed as `RecursionError`/interpreter depth, not C stack cost | info | HIGH |
| `PERF-PY-CALL-002` | Cyclomatic complexity ≥ 15 per function | info | HIGH |
| `PERF-PY-COMPREHENSION-001` | Loop body collapsible to a list comprehension (single `append` statement) | info | MEDIUM |
| `PERF-PY-EXECUTOR-001` | Executor / event-loop construction per iteration (`ThreadPoolExecutor`, `ProcessPoolExecutor`, `new_event_loop`) | info | MEDIUM |
| `PERF-PY-LOOP-001` | Deep loop nesting — nesting depth exactly 3 | info | HIGH |
| `PERF-PY-LOOP-002` | File/network/subprocess/socket I/O inside a loop (`print`/`input` deliberately excluded) | warn | MEDIUM |
| `PERF-PY-LOOP-003` | `range(len(seq))` positional iteration inside a loop | info | HIGH |
| `PERF-PY-LOOP-007` | Linear list scan inside a loop — membership (`x in seq`), index (`seq[i]`), or `seq.index(x)`; set/dict containers excluded | warn | MEDIUM |
| `PERF-PY-LOOP-009` | `try`/`except` inside a loop body | info | MEDIUM |
| `PERF-PY-MEM-002` | Container copy (`copy`/`deepcopy`/`.copy()`) inside a loop | info | MEDIUM |
| `PERF-PY-N1-001` | DB/HTTP call inside a loop (the N+1 pattern) | warn | MEDIUM |
| `PERF-PY-PANDAS-001` | Row-wise pandas iteration (`iterrows`/`itertuples`/`apply(axis=1)`) | info | MEDIUM |
| `PERF-PY-PANDAS-002` | `pd.concat`/`DataFrame.append` inside a loop (O(n²) frame rebuild) | info | MEDIUM |
| `PERF-PY-STR-001` | `str +=` / `str = str + …` accumulation in a loop (honest about CPython refcount==1 amortisation) | warn | MEDIUM |

Every finding remains an optimization candidate. A `try`/`except`-in-loop (`LOOP-009`) is an info-grade readability/robustness signal: the zero-cost-`try` premise is version-dependent (CPython ≥ 3.11).

### JavaScript / TypeScript backend (`PERF-JS-*`)

JavaScript, TypeScript and TSX share **11** `PERF-JS-*` rules: the TS/TSX backend reuses the JS visitor (a grammar/token/flavor dialect) rather than duplicating rule implementations. The token follows the grammar boundary (`js` for `.js`/`.jsx`/`.mjs`/`.cjs`, `ts` for `.ts`/`.mts`/`.cts`, `tsx` for `.tsx`); declaration-only `.d.*` files are skipped. **The JS/TS tier is syntactic-only** — no type information, no `node_modules`/`tsconfig` path resolution, no framework semantics. Type land is never event-scanned (interfaces, type aliases, ambient `declare` blocks and overload signatures emit no events), `as`/`satisfies`/`!`/`<T>expr` are transparent, and type-only imports produce no include edge (so a type-only cycle is not reported). Per-rule gates, evidence keys and accepted false-positive classes live in `docs/heuristics/js-catalog.md`.

| Rule | Detects | Severity | Confidence |
|---|---|---|---|
| `PERF-JS-ALLOC-001` | Object/array literal in a loop; silent when a constructed name is the loop induction variable or is reassigned in that loop (loop-invariance gate) | info | MEDIUM |
| `PERF-JS-ALLOC-003` | `concat`/`slice`/spread accumulation whose accumulator root the same loop reassigns; the spread literal must be that root's assignment RHS, and immutable snapshots/member-target spreads stay silent | info | MEDIUM |
| `PERF-JS-STR-001` | String/template-literal `+=` or `x = x + "…"` in a loop (numeric/identifier RHS and canonical <8-trip loops silent) | warn | MEDIUM |
| `PERF-JS-LOOP-001` | Loop nesting depth ≥ 2 (one finding per function at the earliest loop; `max_depth` scoped to that loop) | info | MEDIUM |
| `PERF-JS-LOOP-002` | Known synchronous Node API (`readFileSync`, `execSync`, sync crypto, …) in a loop | warn | MEDIUM |
| `PERF-JS-LOOP-007` | Membership scan (`indexOf`/`includes`/`find`) in a loop with a non-empty receiver (canonical <16-trip loops silent) | warn | MEDIUM |
| `PERF-JS-ASYNC-001` | `await` inside a loop, only inside an `async` function | warn | MEDIUM |
| `PERF-JS-ASYNC-002` | Known synchronous Node API anywhere inside an `async` function (event-loop stall; no loop requirement) | warn | MEDIUM |
| `PERF-JS-CALL-001` | Bare direct self-call with no visible base-case guard: exact-name `callee_path`, no conditional ancestor, lexical-binding record, and no earlier comparison (member/method-name collisions are rejected) | info | MEDIUM |
| `PERF-JS-ARCH-001` | Cycle over resolved relative-import edges (one finding per broken edge) | info | MEDIUM |
| `PERF-JS-TYPE-001` | `delete obj.prop` with a member operand (subscript form and bare identifier excluded); silent when the receiver is a same-function object/array/`new` binding | info | MEDIUM |

Unresolved internal JS/TS imports stay data-only, so `PERF-JS-ARCH-001` is structurally blind in alias-based monorepos (bare specifiers are external by design). Minified bundles and declaration files are skipped rather than analysed. Every JS/TS finding remains an optimization candidate.

## Output contract

- **One scan → one directory.** A completed report directory contains exactly the artifacts of that run; nothing stale survives.
- **`metrics.json` is the source of truth.** `report.md` and `charts/` are derived artifacts — pure functions of `metrics.json` (plus the reporter's frozen built-in catalog for Appendix A). If they ever disagree, `metrics.json` wins.
- **Single version marker.** The `schema_version` inside `metrics.json` governs the entire directory; there is no separate report-format version. An optional artifact may carry an internal format marker (the opt-in `atlas-layout.json` has `layout_version: 1`). The bundled `metrics.schema.v1.8.json` is a byte-copy of the exact schema for that version (its `$id` is the canonical URI, not the local filename — a mismatch is expected).
- **Atomic regeneration.** The report is built in a temporary sibling directory and swapped into place (rename-aside-then-swap). A crashed or interrupted run leaves the previous complete report intact; stale temp directories are removed on the next successful run.
- **Safety guard.** If the target directory exists and does not contain `metrics.json`, Perfpal refuses to replace it unless `--force` is given — user data is never silently destroyed.
- **Encoding and paths.** All text artifacts are UTF-8, LF line endings, trailing newline. Paths are repo-relative POSIX paths reused verbatim; embedded paths/titles/messages are escaped so scanned filenames cannot inject Markdown or XML.
- **Consumer obligations.** Treat unknown files anywhere in the directory as ignorable; treat `charts/` as advisory; pin the exact minor schema when strict-validating.

Full contract: `docs/report/output-contract.md`.

## Determinism

Perfpal's report artifacts are deterministic. For identical inputs scanned with the same tool version from the same root path, `metrics.json` and `report.md` are byte-identical except the run-provenance values (`started_at`, `duration_ms`) that record the scan itself, and `charts/*.svg` are byte-identical. Determinism is owned, not hoped for: every ordering obligation (file discovery, `files[]`, `findings[]`, graphs, components, chart coordinates) has exactly one owner in the pipeline, and the JSON serializer emits sorted map keys with declaration-order structs.

This claim covers the report directory only. Release binaries make no bit-for-bit reproducibility claim.

## Beta status, code signing, and privacy

Perfpal `0.4.0` is a non-GA pre-release evaluation build. It is provided for noncommercial evaluation only and is not for commercial or production use. Use is governed by the bundled [`EULA`](EULA.md), which supplements the PolyForm Strict 1.0.0 [`LICENSE`](LICENSE).

**Binaries are unsigned.** No Windows Authenticode signing and no macOS notarization is applied at beta, so Windows SmartScreen and macOS Gatekeeper may warn about the download. Azure Trusted Signing is deliberately not wired at beta. Verify authenticity instead via the signed checksum manifest: `minisign -Vm SHA256SUMS -p minisign.pub` followed by `sha256sum -c SHA256SUMS`.

**No telemetry.** Perfpal makes no network calls and collects no data — it runs entirely offline. See [`PRIVACY.md`](PRIVACY.md) for the full statement.

**Known beta limitation — Atlas first-page fit (library class).** A library-class scan with a large UNREACHED column (the published `memchr` crate payload is the reference) renders an Atlas canvas whose bottom extends below the 1440×900 fold (**1,282 px**). The strip, rail and charts still yield three concrete actions (A2: 4/4 blind lanes, 87–120 s), and the limitation is **accepted for this beta** with a named fix trigger in [`docs/evidence/accepted-limitations.md`](docs/evidence/accepted-limitations.md) §D. Application-class scans fit (postgres: 871 px).

## Project docs

- `docs/architecture/modules.md` — module decomposition, determinism ownership, CLI surface (§8)
- `docs/architecture/rust-backend.md` — the Rust backend design record (plan `rust-backend`, R0–R5)
- `docs/report/output-contract.md` — the `perfpal-report/` directory contract
- `docs/specs/atlas-feature.md`, `docs/specs/atlas-layout.md`, `docs/specs/atlas-metrics.md` — the Atlas human-map contract (feature, composition, display metrics)
- `docs/atlas/AGENT-BRIEF.md` — the Atlas contributor brief (visual system, rules, residuals)
- `docs/specs/agent-report.md` — the LLM-facing `agent-report.md` / `agent-actions.json` contract
- `docs/specs/atlas-story.md`, `docs/adr/ADR-0014-in-report-atlas-story.md` — the Atlas Story data and in-report contract (always-on self-contained story; A12 as amended); `docs/adr/ADR-0012-atlas-story-companion.md` is superseded on graduation/report placement (D1/D2/D5)
- `docs/heuristics/catalog-v1.md` — the full 23-rule C catalog with thresholds and false-positive classes
- `docs/heuristics/rust-catalog.md` — the full 22-rule Rust (`PERF-RS-*`) catalog with reframings and false-positive classes
- `docs/schema/metrics-json-v1.md` + `docs/schema/metrics.schema.v1.8.json` — the `metrics.json` data contract and versioning policy
- `docs/heuristics/js-catalog.md` — the full 11-rule JavaScript/TypeScript (`PERF-JS-*`) catalog with gates and false-positive classes
- `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, `docs/adr/ADR-0011-js-ts-backend.md` — the JS/TS backend spec, plan and decision record
- `docs/architecture/js-ts.md` — the JS/TS backend design record and node-kind → IR map
- `docs/adr/ADR-0001-parser-strategy.md` — tree-sitter-first behind a parser-agnostic IR
- `docs/adr/ADR-0002-visualization.md` — the deterministic SVG chart set
- `docs/architecture/ir-v1.md` — the parser-agnostic IR the heuristics consume
- `docs/roadmap.md` — the authoritative post-v0.1.0 roadmap
- `EULA.md` — the beta End User License Agreement (supplements `LICENSE`)
- `PRIVACY.md` — the privacy statement (no telemetry, no network calls)
- `CHANGELOG.md` — release history

## License

Perfpal is source-available proprietary software licensed under the **PolyForm Strict License 1.0.0**. Free to use for **noncommercial** purposes; **commercial use, modification, and redistribution require the owner's explicit permission**. The crate is marked `LicenseRef-PolyForm-Strict-1.0.0` in `Cargo.toml`. The repository is private; there is no external contribution process — proposals and modifications are handled by permission only. The distributed beta binaries are additionally governed by the [`EULA`](EULA.md); see [`PRIVACY.md`](PRIVACY.md) for the no-telemetry statement.
