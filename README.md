# Perfpal

Perfpal is a static-analysis CLI that scans C codebases for performance-relevant patterns and writes a self-contained `perfpal-report/` directory: a versioned, LLM-consumable `metrics.json`, a human-readable `report.md`, and three deterministic SVG architecture charts. It is built for AI-generated codebases — high file volume, frequently non-compiling trees, missing headers and build configuration — where a compiler-based pipeline is often not viable. Parsing runs on a tree-sitter syntactic tier (ADR-0001): no build flags, compile database, or macro expansion are required. All output is framed as **optimization candidates for review, never certainties**.

## How it works

- **Scan** — recursive discovery of `.c`/`.h` files in lexicographic order (dot-directories skipped, symlinks not followed), parsed by the bundled `tree-sitter-c` backend. Parse health (ok / partial / failed) is captured per file; quoted includes are resolved against the filesystem, and in-repo angle-bracket hits also count as internal coupling (ADR-0002); unresolvable angle-bracket includes are bucketed as external.
- **Analyze** — 23 catalog heuristics (`PERF-C-*`) run as pure functions over the parser-agnostic IR. Every finding carries mandatory `confidence` and `severity`; documented downgrade hooks (macro origin, parse-error overlap, name-based cross-file matching) cap confidence and set flags instead of silently dropping or trusting data.
- **Aggregate** — the include graph, the cross-file call graph (name-based, confidence locked LOW), component ranking, and per-function statistics are assembled into one `metrics.json` document with every sort order explicitly owned.
- **Render** — `report.md` is rendered from a fixed template; the three SVG charts are pure functions of `metrics.json` structures and are byte-deterministic.
- **Write** — the report directory is swapped into place atomically; an interrupted run never leaves a half-written report.

## Install

Perfpal is not published to crates.io (`publish = false` in `Cargo.toml`). It ships as release assets from the private repository `github.com/ibraramin/perfpal`; because the repo is private, releases are visible only to repository collaborators.

Release assets follow the naming pattern `perfpal-<rust-target>[.exe]` with a sibling `<name>.sha256` checksum file. The v0.1.0 release (`perfpal v0.1.0`) ships ten assets — five target binaries and five checksums:

| Asset | Platform |
|---|---|
| `perfpal-x86_64-unknown-linux-musl` + `perfpal-x86_64-unknown-linux-musl.sha256` | Linux x86-64, statically linked (musl) |
| `perfpal-aarch64-unknown-linux-musl` + `perfpal-aarch64-unknown-linux-musl.sha256` | Linux arm64, statically linked (musl) |
| `perfpal-x86_64-apple-darwin` + `perfpal-x86_64-apple-darwin.sha256` | macOS x86-64 |
| `perfpal-aarch64-apple-darwin` + `perfpal-aarch64-apple-darwin.sha256` | macOS arm64 (Apple Silicon) |
| `perfpal-x86_64-pc-windows-msvc.exe` + `perfpal-x86_64-pc-windows-msvc.sha256` | Windows x86-64 (MSVC) |

Example install and checksum verification (checksum files embed the shipped binary filename, so `sha256sum -c` works as-is):

```
gh release download v0.1.0 --repo ibraramin/perfpal --pattern 'perfpal-x86_64-unknown-linux-musl*'
chmod +x perfpal-x86_64-unknown-linux-musl
sha256sum -c perfpal-x86_64-unknown-linux-musl.sha256
```

To build from source instead:

```
cargo build --release --locked
```

The toolchain is pinned to Rust 1.92.0 (`rust-toolchain.toml`).

## Quickstart

Scan a directory (a single `.c`/`.h` file also works):

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
├── metrics.json              # raw machine-readable data (schema v1.0)
├── metrics.schema.v1.0.json  # byte-copy of the exact schema for this schema_version
├── report.md                 # human-readable report
└── charts/                   # static visualizations (set defined by ADR-0002)
    └── <chart-name>.svg      # e.g. include-graph.svg
```

The chart set is `include-graph.svg`, `component-coupling.svg`, and `hot-components.svg`; each chart is emitted only when its source section is present and non-empty, so `charts/` may be empty. Tooling and LLM sessions should start at `metrics.json` (validate against the bundled schema copy); humans can start at `report.md`. Never parse data out of the SVGs — they are advisory.

## CLI reference

| Flag | Argument | Default | Meaning |
|---|---|---|---|
| `<path>` (positional) | `PATH` | `.` | Scan target: a directory or a single `.c`/`.h` file |
| `-o`, `--output` | `DIR` | `perfpal-report` | Report directory; internal layout is fixed (output-contract §2) |
| `--force` | — | off | Allow replacing an existing directory that contains no `metrics.json` (safety-guard override) |
| `--version` | — | — | Print `perfpal <version> (metrics schema <schema_version>)` (e.g. `perfpal 0.1.0 (metrics schema 1.0)`), exit 0 |
| `-h`, `--help` | — | — | Usage text, exit 0 |

stdout carries the short plain-text summary shown above; all diagnostics, warnings, and progress go to stderr. The summary and stderr diagnostics carry no color or cursor control — output stays pipe-clean (interactive `--help`/`--version` rendering is clap's own and disables itself when piped). Machine consumption starts at `metrics.json`, never stdout parsing.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success — report written. Findings never affect the exit code (they are data). Zero-C-file scans succeed with a valid empty report. |
| `1` | Usage error — unknown flag; missing/unreadable target path. |
| `2` | Output error — cannot create output dir; safety-guard refusal without `--force`; disk/write failure. |
| `3` | Internal error — unexpected panic/crash, always a bug (this also covers a `metrics.json` serialization failure). |

## Heuristics

Perfpal's catalog v1 defines 23 heuristics, all on the syntactic resolver tier. Every finding carries mandatory `confidence` (HIGH / MEDIUM / LOW) and `severity` (`hot` / `warn` / `info`); both are prioritization hints, and every finding remains an optimization candidate. The tables show each rule's catalog defaults; documented per-rule hooks shift severity (e.g. `free()` in a loop escalates warn → hot when allocation co-occurs in the same function; conditionally guarded I/O lowers hot → warn), and downgrade rules cap confidence (macro origin or parse-error overlap caps at MEDIUM; name-based cross-file edges are LOW).

Thresholds are named constants documented in the catalog (`docs/heuristics/catalog-v1.md`) and are not runtime-configurable.

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
| `PERF-C-ARCH-003` | Cross-file call-graph edges from name matching (confidence locked LOW until a semantic resolver exists) | info | LOW | 1 |

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

## Output contract

- **One scan → one directory.** A completed report directory contains exactly the artifacts of that run; nothing stale survives.
- **`metrics.json` is the source of truth.** `report.md` and `charts/` are derived artifacts — pure functions of `metrics.json` (plus the reporter's frozen built-in catalog for Appendix A). If they ever disagree, `metrics.json` wins.
- **Single version marker.** The `schema_version` inside `metrics.json` governs the entire directory; there is no separate report-format version. The bundled `metrics.schema.v1.0.json` is a byte-copy of the exact schema for that version (its `$id` is the canonical URI, not the local filename — a mismatch is expected).
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
- `docs/report/output-contract.md` — the `perfpal-report/` directory contract
- `docs/heuristics/catalog-v1.md` — the full 23-rule catalog with thresholds and false-positive classes
- `docs/schema/metrics-json-v1.md` + `docs/schema/metrics.schema.v1.json` — the `metrics.json` data contract and versioning policy
- `docs/adr/ADR-0001-parser-strategy.md` — tree-sitter-first behind a parser-agnostic IR
- `docs/adr/ADR-0002-visualization.md` — the deterministic SVG chart set
- `CHANGELOG.md` — release history

## License

Perfpal is source-available proprietary software licensed under the **PolyForm Strict License 1.0.0**. Free to use for **noncommercial** purposes; **commercial use, modification, and redistribution require the owner's explicit permission**. The crate is marked `LicenseRef-Proprietary` in `Cargo.toml`. The repository is private; there is no external contribution process — proposals and modifications are handled by permission only.
