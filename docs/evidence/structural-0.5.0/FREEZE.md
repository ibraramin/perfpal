# Perfpal 0.5.0 / metrics schema 1.9 — structural-suite freeze

**Status:** FROZEN. **Date:** 2026-11-04. **Owner:** orchestrator.
**Branch:** `feat/structural-0.5.0` (local only; no push).
**Code commit:** `8d68209` (Stage 1 `e441fc1`, Stage 2 `fd77588`, ICC `8d68209`).
**Tag:** `perfpal-v0.5.0-structural` (annotated, local). The tag is moved by the
orchestrator after this re-freeze.
**Binary:** `target/release/perfpal` — `perfpal 0.5.0 (metrics schema 1.9)`.
**Binary SHA-256:** `b89d75027d8b238ad8febab1c8dc3a6687f0a4fd731c86b4e951257d84780ee0`.

**Re-freeze note (2026-11-04):** re-frozen after R1 panel findings; no main
scan; old binary/SHA `f254de14…c240` retired. Corrected the C `#if 0` dead-arm
span (the live `#else` arm is no longer over-gated), the 0-based
`resource_sites[].column` contract, and the ICC site matcher
(`(path, line, 0-based column)`); `run_icc.py` now propagates a failed
recomputation instead of trusting a stale `freeze_ok: true`.

This is the study instrument for the five-language structural suite. No
production code changes after the first main-study scan; a bug fix increments
to 0.5.1 and **re-scans every cell** (never mix binary versions across arms).

## Frozen field set (schema 1.9, additive from 1.8)

| Field | Scope | Status |
|---|---|---|
| `loop_count` | C, Rust, Python, JS, TS | frozen |
| `loop_body_loc` | C, Rust, Python, JS, TS | frozen; **primary outcome** |
| `max_loop_depth` | C, Rust, Python, JS, TS | frozen (existing field) |
| `parse_health` (per function) | all five | frozen (missingness control) |
| `alloc_sites_in_loops` | C, Rust | frozen (secondary) |
| `io_site_count` / `io_sites_in_loops` | all five | `io_site_count` descriptive; `io_sites_in_loops` **dropped** (see ICC) |
| `resource_sites[]` | in-loop sites | frozen; per-site `cfg_gated` C-only |
| per-site `cfg_gated` | C bool; Rust/Python/JS/TS null | frozen |

Schema copies `src/schema/metrics.schema.v1.9.json` and
`docs/schema/metrics.schema.v1.9.json` are byte-identical. No new required
function-level property; 1.8 documents validate unchanged.

## I/O table versions

- `IO_TABLE_VERSION = 2` (C/Rust shared table; Rust stdio macro rows added,
  `macro_only` rows are skipped by the flow projection). Targets filtered to
  `file`, `stdio`, `net`; `env` excluded.
- `PY_IO_TABLE_VERSION = 1` (Python external-I/O table).
- `JS_IO_TABLE_VERSION = 1` (JavaScript/TypeScript external-I/O table).

## Verification (all exit 0, run at the re-freeze working tree on `8d68209`)

| Gate | Command | Result |
|---|---|---:|
| Format | `cargo fmt --all -- --check` | 0 |
| Clippy | `cargo clippy --locked --all-targets -- -D warnings` | 0 |
| Tests | `cargo test --locked` | 0; **1,142 passed**, 0 failed |
| Licenses | `cargo deny check licenses` | 0; `licenses ok` |
| Semantic clippy | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | 0 |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 0; **1,144 passed**, 0 failed |
| Release build | `cargo build --release --locked` | 0 |
| Version | `target/release/perfpal --version` | `perfpal 0.5.0 (metrics schema 1.9)` |
| Binary hash | `sha256sum target/release/perfpal` | `b89d7502…0ee0` |
| Whitespace | `git diff --check` | 0 |
| ICC harness | `python3 tools/icc/run_icc.py` | 0; `freeze_ok=true` |

## ICC / reliability evidence (`tools/icc/`)

Independent parsers: clang 23 AST (C), Python `ast`, `syn` (Rust), TypeScript
compiler (JS/TS). Sample: **200 real functions** (C 42, Rust 42, Python 42,
JS 35, TS 39; all available loop functions plus deterministic no-loop fills).
The corpus is the committed Perfpal examples plus a 12-function extra corpus;
it is human-written, so AI-arm stratification is deferred (the tool-vs-tool
ICC is arm-independent).

| Field | n | Result | Gate |
|---|---:|---|---|
| `loop_count` ICC(2,1) | 200 | 1.0000 (every language) | PASS (>=0.75) |
| `loop_body_loc` ICC(2,1) | 200 | 1.0000 automated; 0.9812 pooled manual | PASS (>=0.75) |
| `loop_body_loc` manual per language | 100 | C 0.9904, Rust 0.9935, Python 0.7575, JS 1.0000, TS 1.0000 | PASS (Python narrowly) |
| `max_loop_depth` exact | 200 | 100% | PASS (>=99%) |
| `alloc_sites_in_loops` ICC(2,1) | C 42 / Rust 42 | 1.0000 automated; manual kappa 0.9357 (precision 1.000 / recall 0.889; confusion TP 8 / FP 0 / FN 1 / TN 91) | PASS (>=0.70) |
| `cfg_gated` (C sites) | 15 | kappa 1.0000 | PASS (>=0.70) |
| `io_sites_in_loops` (blind site audit) | 100 | kappa 0.6617, ICC 0.6639; precision 1.00, recall 0.55 (unchanged; still below 0.70) | **FAIL -> DROPPED** |

### Pre-registered decision: `io_sites_in_loops` is dropped

The blind 100-site audit found 20 external-I/O calls; Perfpal caught 11
(recall 0.55): missed C `fgetc`/`putc`, JS/TS bare `readFileSync`, Rust
`write_all`, and Python DB/wrapper calls. The frozen main-study pre-registration
(decision rule 4) drops a dimension whose ICC is below its pre-registered
threshold from the confirmatory family. `io_sites_in_loops` is therefore
**not confirmatory** in the main study; `io_site_count` remains descriptive
only, with the audit limitation reported. No table tuning was applied after
the audit.

## Known limitations (reported with every field)

- `loop_body_loc` is a span-line measure, not comment-stripped SLOC; the
  Python manual check differs on leading comments because the tree-sitter
  body node starts at the first statement (the spec's locked definition is the
  body-node span). The automated independent-parser ICC is 1.0000; the manual
  pooled ICC is 0.9812 (Python 0.7575).
- `alloc_sites_in_loops` inherits the name-list, non-type-resolved limitation
  of `alloc_site_count`; the clang/syn ICC is name-based, not HIR-type-resolved.
- Unexpanded macro bodies are opaque; counts are lower bounds where macros
  generate loops/allocations.
- C `#if 0` loops are counted syntactically; clang drops dead code, so any
  such sampled C function is flagged and excluded from the loop ICC rather
  than compared silently.
- The I/O tables are conservative; the blind audit documented the misses above.
- The ICC corpus is human-written; AI-agent functions are not in the
  reliability sample (stratification deferred; measurement agreement is
  parser-level).

## Reproduce

```sh
cargo build --release --locked
target/release/perfpal examples        -o /var/tmp/opencode/icc/perfpal/c      --force
target/release/perfpal examples-rust   -o /var/tmp/opencode/icc/perfpal/rust   --force
target/release/perfpal examples-python -o /var/tmp/opencode/icc/perfpal/python --force
target/release/perfpal examples-js     -o /var/tmp/opencode/icc/perfpal/js     --force
target/release/perfpal examples-ts     -o /var/tmp/opencode/icc/perfpal/ts     --force
target/release/perfpal tools/icc/extra -o /var/tmp/opencode/icc/perfpal/extra --force
python3 tools/icc/run_icc.py
```

Evidence: `tools/icc/results.json`, `tools/icc/results.md`,
`tools/icc/sample.json`, `tools/icc/labels/` (blind packets, labels,
comparison), `tools/icc/ref/` (independent-parser outputs).
