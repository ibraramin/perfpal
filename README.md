# Perfpal

> **Beta — `0.2.0-beta.1`.** Pre-release evaluation build (C + Rust backends), provided for **evaluation only — not for commercial or production use at this beta** (see the [License](#license) section for terms). Licensed under PolyForm Strict 1.0.0 (see `LICENSE`).

Perfpal is a static-analysis CLI that scans C and Rust codebases for performance-relevant patterns and writes a self-contained `perfpal-report/` directory: a versioned, LLM-consumable `metrics.json`, a human-readable `report.md`, and three deterministic SVG architecture charts. It is built for AI-generated codebases — high file volume, frequently non-compiling trees, missing headers and build configuration — where a compiler-based pipeline is often not viable. Parsing runs on a tree-sitter syntactic tier (ADR-0001): no build flags, compile database, or macro expansion are required. All output is framed as **optimization candidates for review, never certainties**.

## How it works

- **Scan** — recursive discovery in lexicographic order (dot-directories skipped, symlinks not followed): `.c`/`.h` files are parsed by the bundled `tree-sitter-c` backend and `.rs` files by the bundled `tree-sitter-rust` backend. Both backends are fully live: C ships 23 `PERF-C-*` catalog heuristics, Rust ships 23 `PERF-RS-*` catalog heuristics, and the Rust module/`use` graph resolves with Cargo.toml-aware crate roots. Parse health (ok / partial / failed) is captured per file; quoted includes are resolved against the filesystem, and in-repo angle-bracket hits also count as internal coupling (ADR-0002); unresolvable angle-bracket includes are bucketed as external.
- **Analyze** — 46 catalog heuristics (23 `PERF-C-*` C + 23 `PERF-RS-*` Rust) run as pure functions over the parser-agnostic IR. Every finding carries mandatory `confidence` and `severity`; documented downgrade hooks (macro origin, parse-error overlap, name-based cross-file matching) cap confidence and set flags instead of silently dropping or trusting data.
- **Aggregate** — the include graph, the cross-file call graph (name-based, confidence locked LOW), component ranking, and per-function statistics are assembled into one `metrics.json` document with every sort order explicitly owned.
- **Render** — `report.md` is rendered from a fixed template; the three SVG charts are pure functions of `metrics.json` structures and are byte-deterministic.
- **Write** — the report directory is swapped into place atomically; an interrupted run never leaves a half-written report.

## Install

Perfpal is not published to crates.io (`publish = false` in `Cargo.toml`). It ships as release assets from the private repository `github.com/ibraramin/perfpal`; because the repo is private, releases are visible only to repository collaborators.

Release assets follow the naming pattern `perfpal-<rust-target>[.exe]` with a sibling `<name>.sha256` checksum file. The current release matrix ships twelve assets — six target binaries and six checksums (the v0.1.0 release shipped ten: the five targets below minus `gnu`, which joined post-v0.1.0 as the Linux semantic-tier vehicle):

| Asset | Platform |
|---|---|
| `perfpal-x86_64-unknown-linux-musl` + `perfpal-x86_64-unknown-linux-musl.sha256` | Linux x86-64, statically linked (musl) |
| `perfpal-aarch64-unknown-linux-musl` + `perfpal-aarch64-unknown-linux-musl.sha256` | Linux arm64, statically linked (musl) |
| `perfpal-x86_64-unknown-linux-gnu` + `perfpal-x86_64-unknown-linux-gnu.sha256` | Linux x86-64, glibc (semantic-tier vehicle) |
| `perfpal-x86_64-apple-darwin` + `perfpal-x86_64-apple-darwin.sha256` | macOS x86-64 |
| `perfpal-aarch64-apple-darwin` + `perfpal-aarch64-apple-darwin.sha256` | macOS arm64 (Apple Silicon) |
| `perfpal-x86_64-pc-windows-msvc.exe` + `perfpal-x86_64-pc-windows-msvc.sha256` | Windows x86-64 (MSVC) |

Example install and checksum verification (checksum files embed the shipped binary filename, so `sha256sum -c` works as-is):

```
gh release download v0.1.0 --repo ibraramin/perfpal --pattern 'perfpal-x86_64-unknown-linux-musl*'
chmod +x perfpal-x86_64-unknown-linux-musl
sha256sum -c perfpal-x86_64-unknown-linux-musl.sha256
```

Post-v0.1.0 (plan `semantic-resolver`), the release matrix grows to six targets: `x86_64-unknown-linux-gnu` joins as the Linux semantic-tier vehicle (`perfpal-x86_64-unknown-linux-gnu` + checksum, same naming pattern). The gnu, both darwin, and Windows-MSVC builds compile with the default-OFF `semantic-clang` cargo feature and activate the semantic tier when libclang is installed (`LIBCLANG_PATH`); without libclang they degrade gracefully to the syntactic tier. The musl x86_64/aarch64 builds stay syntactic-only by design (static musl cannot dlopen) and remain fully functional.

To build from source instead:

```
cargo build --release --locked
```

The toolchain is pinned to Rust 1.92.0 (`rust-toolchain.toml`).

## Quickstart

Scan a directory (a single `.c`/`.h`/`.rs` file also works):

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
├── metrics.json              # raw machine-readable data (schema v1.2)
├── metrics.schema.v1.2.json  # byte-copy of the exact schema for this schema_version
├── report.md                 # human-readable report
└── charts/                   # static visualizations (set defined by ADR-0002)
    └── <chart-name>.svg      # e.g. include-graph.svg
```

The chart set is `include-graph.svg`, `component-coupling.svg`, and `hot-components.svg`; each chart is emitted only when its source section is present and non-empty, so `charts/` may be empty. Tooling and LLM sessions should start at `metrics.json` (validate against the bundled schema copy); humans can start at `report.md`. Never parse data out of the SVGs — they are advisory.

## CLI reference

| Flag | Argument | Default | Meaning |
|---|---|---|---|
| `<path>` (positional) | `PATH` | `.` | Scan target: a directory or a single `.c`/`.h`/`.rs` file |
| `-o`, `--output` | `DIR` | `perfpal-report` | Report directory; internal layout is fixed (output-contract §2) |
| `--force` | — | off | Allow replacing an existing directory that contains no `metrics.json` (safety-guard override) |
| `--version` | — | — | Print `perfpal <version> (metrics schema <schema_version>)` (e.g. `perfpal 0.2.0-beta.1 (metrics schema 1.2)`), exit 0 |
| `-h`, `--help` | — | — | Usage text, exit 0 |

stdout carries the short plain-text summary shown above; all diagnostics, warnings, and progress go to stderr. The summary and stderr diagnostics carry no color or cursor control — output stays pipe-clean (interactive `--help`/`--version` rendering is clap's own and disables itself when piped). Machine consumption starts at `metrics.json`, never stdout parsing.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success — report written. Findings never affect the exit code (they are data). Zero-file scans succeed with a valid empty report (`scan.languages` = the supported set `["c","rust"]`). |
| `1` | Usage error — unknown flag; missing/unreadable target path. |
| `2` | Output error — cannot create output dir; safety-guard refusal without `--force`; disk/write failure. |
| `3` | Internal error — unexpected panic/crash, always a bug (this also covers a `metrics.json` serialization failure). |

## Heuristics

Perfpal's catalog defines 46 heuristics — 23 C (`PERF-C-*`) plus 23 Rust (`PERF-RS-*`) — all of which fire on the syntactic resolver tier. On `semantic-clang` builds with libclang present, proven semantic facts upgrade C findings above their catalog defaults — resolved ARCH-003 cross-file edges report MEDIUM instead of LOW (the syntactic tier stays LOW); the Rust backend is **syntactic-tier only** (the libclang resolver is C-only), so no Rust finding is ever upgraded. Without libclang every build degrades gracefully to the syntactic tier. Every finding carries mandatory `confidence` (HIGH / MEDIUM / LOW) and `severity` (`hot` / `warn` / `info`); both are prioritization hints, and every finding remains an optimization candidate. The tables show each rule's catalog defaults; documented per-rule hooks shift severity (e.g. `free()` in a loop escalates warn → hot when allocation co-occurs in the same function; conditionally guarded I/O lowers hot → warn), and downgrade rules cap confidence (macro origin or parse-error overlap caps at MEDIUM; name-based cross-file edges are LOW).

Thresholds are named constants documented in the catalogs (`docs/heuristics/catalog-v1.md` for C, `docs/heuristics/rust-catalog.md` for Rust) and are not runtime-configurable.

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

The Rust backend mirrors the C families one-for-one as 23 `PERF-RS-*` rules. **Rust is syntactic-tier only** — the libclang `semantic-clang` resolver is C-only, so every Rust finding keeps its catalog confidence (no resolved-edge / resolved-format upgrades). Several rules are **honest reframings** of their C counterparts (Rust has no `realloc`, `sizeof`, or unbounded `strcpy`); the full per-rule implementation notes and reframings live in `docs/heuristics/rust-catalog.md`.

| Rule | Detects | Severity | Confidence | Tier |
|---|---|---|---|---|
| `PERF-RS-ALLOC-001` | Heap-allocation call or macro (`Vec::new`, `Box::new`, `String::with_capacity`, `to_string`, `vec!`, `format!`, …) at `in_loop_depth ≥ 1` | hot | HIGH | 1 |
| `PERF-RS-ALLOC-002` | Direct, non-macro `drop()` at `in_loop_depth ≥ 1` (escalates warn → hot when an alloc-family site also fires in the same function) | warn | HIGH | 1 |
| `PERF-RS-ALLOC-003` | Direct, non-macro `reserve`/`reserve_exact` at `in_loop_depth ≥ 1` (additive buffer growth — the `realloc` analogue) | hot | MEDIUM | 1 |
| `PERF-RS-ARCH-001` | `use`-import-graph hubs (≥12), deep chains (≥8), cycles over resolved internal edges (`mod` declarations excluded) | warn | HIGH | 1 |
| `PERF-RS-ARCH-002` | Hot-component aggregation — Rust component dirs ranked by complexity sum, max loop depth, and alloc-site density | hot | HIGH | 1 |
| `PERF-RS-ARCH-003` | Cross-module call-graph edges from name matching (LOW; the symmetric resolved branch is unreachable — no Rust resolver) | info | LOW | 1 |
| `PERF-RS-CALL-001` | Direct recursion — a function body calling its own name (presence-only; recursion depth is not statically bounded) | info | HIGH | 1 |
| `PERF-RS-CALL-002` | Cyclomatic complexity ≥ 15 per function (promoted from the baseline stat) | warn | HIGH | 1 |
| `PERF-RS-LOOP-001` | Deep loop nesting — nesting depth exactly 3 (one finding per maximal nest entry) | warn | HIGH | 1 |
| `PERF-RS-LOOP-002` | Curated I/O / setup calls nested in loops — print/write macros, read/write methods, `File::open`, `fs::read`, `Command::new`, `TcpStream::connect`, `env::var`, … | hot | MEDIUM | 1 |
| `PERF-RS-LOOP-003` | Char/byte-at-a-time iterator or reader method (`chars`, `bytes`, `char_indices`, `read_byte`) as the loop body's primary statement | info | MEDIUM | 1 |
| `PERF-RS-LOOP-004` | Integer `/` or `%` with a non-literal divisor inside the innermost loop body | info | MEDIUM | 1 |
| `PERF-RS-LOOP-005` | Assignment to a collected `static`/`const` global inside a loop body (dedup per target and enclosing loop) | info | MEDIUM | 2 |
| `PERF-RS-LOOP-006` | Indirect call inside an innermost loop body (closure-field / trait-object dispatch classifies Direct, so it does not fire) | info | LOW | 2 |
| `PERF-RS-LOOP-007` | Linear membership scan — comparison against a subscript indexed by an outer loop's induction variable at nesting depth ≥ 2 | info | LOW | 2 |
| `PERF-RS-LOOP-008` | Member-chain dereference of depth ≥ 3 inside an innermost loop body | info | MEDIUM | 2 |
| `PERF-RS-MEM-001` | `with_capacity(size_of::<T>())` — a byte count passed where an element count is expected | warn | MEDIUM | 2 |
| `PERF-RS-MEM-002` | Block-copy/clone family (`copy_from_slice`, `copy_nonoverlapping`, `extend_from_slice`, `clone`, `to_vec`) inside a loop (escalates info → warn innermost) | info | MEDIUM | 2 |
| `PERF-RS-STR-001` | String accumulation — `push_str`/`push` on a bare-identifier receiver reused across loop iterations | hot | HIGH | 1 |
| `PERF-RS-STR-002` | `len`/`is_empty`/`capacity` on a bare identifier inside a loop where the receiver is not reassigned in the loop | warn | HIGH | 1 |
| `PERF-RS-STR-003` | Unbounded `unsafe` memory operation (`copy_nonoverlapping`, `copy`, `from_raw_parts`, `from_utf8_unchecked`, `set_len`), at any loop depth | warn | HIGH | 1 |
| `PERF-RS-TYPE-001` | Large struct passed by value — estimated size ≥ 32 bytes (one aggregated finding per function; approximate LP64 size model) | warn | LOW | 2 |
| `PERF-RS-TYPE-002` | Struct member ordering — estimated padding waste ≥ 8 bytes (`repr(Rust)` may reorder fields; deliberately lower-confidence) | info | LOW | 2 |

Rows are ordered by family: ALLOC, ARCH, CALL, LOOP, MEM, STR, TYPE. Every finding remains an optimization candidate.

## Output contract

- **One scan → one directory.** A completed report directory contains exactly the artifacts of that run; nothing stale survives.
- **`metrics.json` is the source of truth.** `report.md` and `charts/` are derived artifacts — pure functions of `metrics.json` (plus the reporter's frozen built-in catalog for Appendix A). If they ever disagree, `metrics.json` wins.
- **Single version marker.** The `schema_version` inside `metrics.json` governs the entire directory; there is no separate report-format version. The bundled `metrics.schema.v1.2.json` is a byte-copy of the exact schema for that version (its `$id` is the canonical URI, not the local filename — a mismatch is expected).
- **Atomic regeneration.** The report is built in a temporary sibling directory and swapped into place (rename-aside-then-swap). A crashed or interrupted run leaves the previous complete report intact; stale temp directories are removed on the next successful run.
- **Safety guard.** If the target directory exists and does not contain `metrics.json`, Perfpal refuses to replace it unless `--force` is given — user data is never silently destroyed.
- **Encoding and paths.** All text artifacts are UTF-8, LF line endings, trailing newline. Paths are repo-relative POSIX paths reused verbatim; embedded paths/titles/messages are escaped so scanned filenames cannot inject Markdown or XML.
- **Consumer obligations.** Treat unknown files anywhere in the directory as ignorable; treat `charts/` as advisory; pin the exact minor schema when strict-validating.

Full contract: `docs/report/output-contract.md`.

## Determinism

Perfpal's report artifacts are deterministic. For identical inputs scanned with the same tool version from the same root path, `metrics.json` and `report.md` are byte-identical except the run-provenance values (`started_at`, `duration_ms`) that record the scan itself, and `charts/*.svg` are byte-identical. Determinism is owned, not hoped for: every ordering obligation (file discovery, `files[]`, `findings[]`, graphs, components, chart coordinates) has exactly one owner in the pipeline, and the JSON serializer emits sorted map keys with declaration-order structs.

This claim covers the report directory only. Release binaries make no bit-for-bit reproducibility claim.

## Project docs

- `docs/architecture/modules.md` — module decomposition, determinism ownership, CLI surface (§8)
- `docs/architecture/rust-backend.md` — the Rust backend design record (plan `rust-backend`, R0–R5)
- `docs/report/output-contract.md` — the `perfpal-report/` directory contract
- `docs/heuristics/catalog-v1.md` — the full 23-rule C catalog with thresholds and false-positive classes
- `docs/heuristics/rust-catalog.md` — the full 23-rule Rust (`PERF-RS-*`) catalog with reframings and false-positive classes
- `docs/schema/metrics-json-v1.md` + `docs/schema/metrics.schema.v1.2.json` — the `metrics.json` data contract and versioning policy
- `docs/adr/ADR-0001-parser-strategy.md` — tree-sitter-first behind a parser-agnostic IR
- `docs/adr/ADR-0002-visualization.md` — the deterministic SVG chart set
- `docs/architecture/ir-v1.md` — the parser-agnostic IR the heuristics consume
- `docs/roadmap.md` — the authoritative post-v0.1.0 roadmap
- `CHANGELOG.md` — release history

## License

Perfpal is source-available proprietary software licensed under the **PolyForm Strict License 1.0.0**. Free to use for **noncommercial** purposes; **commercial use, modification, and redistribution require the owner's explicit permission**. The crate is marked `LicenseRef-Proprietary` in `Cargo.toml`. The repository is private; there is no external contribution process — proposals and modifications are handled by permission only.
