# Showcase repro packet — ripgrep `14.1.1` (Rust, MIT / Unlicense)

> Part of plan `evidence-beta` **M4** (AI-code credibility showcase), the ADR-0005 publish-gate
> precondition. Umbrella record: `docs/evidence/ai-code-showcase.md`. Raw counts:
> `docs/evidence/showcase/ripgrep/counts.json`; full verdict rows:
> `docs/evidence/showcase/ripgrep/verified.json`.

## 1. Corpus (pinned)
- **Repo:** https://github.com/BurntSushi/ripgrep
- **Tag → commit:** `14.1.1` → `4649aa9700619f94cf9c66876e9549d83420e16c`
- **License:** MIT / Unlicense
- **Cloned outside the product repo** at `ripgrep/` (CWD-relative) via:
  ```bash
  git clone --depth 1 --branch 14.1.1 https://github.com/BurntSushi/ripgrep.git ripgrep
  git -C ripgrep rev-parse HEAD   # 4649aa9700619f94cf9c66876e9549d83420e16c
  ```

## 2. One-command scan
```bash
perfpal ripgrep -o out-ripgrep
```
**Expected summary (exit 0):** `scanned 98 files (98 ok, 0 partial, 0 failed) · 1315 findings (40 hot, 90 warn, 1185 info)`

## 3. Per-rule counts (13 distinct rules)
| rule | total | hot | warn | info |
|---|---:|---:|---:|---:|
| `PERF-RS-ALLOC-001` | 25 | 25 | 0 | 0 |
| `PERF-RS-ARCH-001` | 7 | 0 | 7 | 0 |
| `PERF-RS-ARCH-002` | 1 | 1 | 0 | 0 |
| `PERF-RS-ARCH-003` | 849 | 0 | 0 | 849 |
| `PERF-RS-CALL-001` | 324 | 0 | 0 | 324 |
| `PERF-RS-CALL-002` | 28 | 0 | 28 | 0 |
| `PERF-RS-LOOP-001` | 3 | 0 | 3 | 0 |
| `PERF-RS-LOOP-002` | 27 | 14 | 13 | 0 |
| `PERF-RS-LOOP-008` | 2 | 0 | 0 | 2 |
| `PERF-RS-STR-002` | 26 | 0 | 26 | 0 |
| `PERF-RS-STR-003` | 2 | 0 | 2 | 0 |
| `PERF-RS-TYPE-001` | 11 | 0 | 11 | 0 |
| `PERF-RS-TYPE-002` | 10 | 0 | 0 | 10 |

## 4. Determinism proof
Two scans into separate output dirs; `metrics.json` compared **byte-identical after masking the
documented provenance fields** (`scan.started_at`, `scan.duration_ms`).
The two runs differ only in the masked provenance fields (`started_at` and/or `duration_ms`); the masked copies are byte-identical.
- Run-1 masked `metrics.json` sha256: `8acf8c19f5fce1dda738e6d5eb6e880133b7a021f18fd136d066a9d55c5727cf`
- Result: **IDENTICAL** across both runs.

## 5. Hand-verified findings (stratified sample, n=26)
Sample = up to 3 findings for the dominant rule + up to 2 per other rule, preferring
`hot` → `warn` → `info`. Verdicts: **TP** = claim holds in the real code; **FP** = claim is
wrong; **AMBIG** = technically present but harmless/context-dependent.
**Tally: TP=11 · FP=14 · AMBIG=1** (42% TP of sample).

### Per-rule tally
| rule | TP | FP | AMBIG | n |
|---|---:|---:|---:|---:|
| `PERF-RS-ALLOC-001` | 2 | 0 | 0 | 2 |
| `PERF-RS-ARCH-001` | 0 | 1 | 1 | 2 |
| `PERF-RS-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-RS-ARCH-003` | 2 | 1 | 0 | 3 |
| `PERF-RS-CALL-001` | 0 | 2 | 0 | 2 |
| `PERF-RS-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-RS-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-RS-LOOP-002` | 0 | 2 | 0 | 2 |
| `PERF-RS-LOOP-008` | 2 | 0 | 0 | 2 |
| `PERF-RS-STR-002` | 0 | 2 | 0 | 2 |
| `PERF-RS-STR-003` | 0 | 2 | 0 | 2 |
| `PERF-RS-TYPE-001` | 0 | 2 | 0 | 2 |
| `PERF-RS-TYPE-002` | 0 | 2 | 0 | 2 |

### Full verdict rows
| rule | location | verdict | decisive span | note |
|---|---|---|---|---|
| `PERF-RS-ARCH-003` | `crates/core/flags/doc/help.rs:151` | **TP** | `super::render_custom_markup(doc, "flag"` | real cross-file call; defined in doc/mod.rs:18 via explicit super:: path |
| `PERF-RS-ARCH-003` | `crates/core/flags/doc/man.rs:66` | **TP** | `super::render_custom_markup(doc, "flag"` | same confirmed call to doc/mod.rs:18; coupling real, not name-only |
| `PERF-RS-ARCH-003` | `build.rs:40` | **FP** | `Command::new("git").args(args).output()` | `.output()` is std::process::Command method, not tests/util.rs `output` |
| `PERF-RS-CALL-001` | `crates/cli/src/decompress.rs:65` | **FP** | `glob_builder.build()` | calls GlobSetBuilder::build, not DecompressionMatcherBuilder::build; no recursion |
| `PERF-RS-CALL-001` | `crates/cli/src/decompress.rs:168` | **FP** | `DecompressionMatcherBuilder::new()` | calls another type's `new`, not self-recursion |
| `PERF-RS-CALL-002` | `crates/core/flags/hiargs.rs:112` | **TP** | `fn from_low_args(mut low: LowArgs)` | huge fn (~50 decision points; if15/else8/match7/arms18/for3), far over 15 |
| `PERF-RS-CALL-002` | `crates/core/flags/hiargs.rs:459` | **TP** | `fn matcher_rust(&self)` | ~16 branches, exceeds threshold 15 |
| `PERF-RS-LOOP-002` | `crates/cli/src/decompress.rs:182` | **FP** | `let mut cmd = Command::new(&decomp_cmd.bin);` | Command::new is not I/O; loop also returns on 1st iter |
| `PERF-RS-LOOP-002` | `crates/core/flags/doc/help.rs:102` | **FP** | `write!(out, "  ");` | `out` is a String; write! is in-memory fmt, no syscall/blocking I/O |
| `PERF-RS-STR-002` | `crates/core/flags/doc/help.rs:32` | **FP** | `maxcol1 = maxcol1.max(col1.len());` | col1 re-assigned each iter (line 31); not a hoistable loop-invariant length |
| `PERF-RS-STR-002` | `crates/core/flags/doc/help.rs:33` | **FP** | `maxcol2 = maxcol2.max(col2.len());` | col2 re-assigned each iter; the 'no in-loop assignment' claim is false |
| `PERF-RS-ALLOC-001` | `crates/core/flags/complete/bash.rs:83` | **TP** | `format!("--{}", flag.name_long())` | loop body allocates via format!/to_string/join every iteration |
| `PERF-RS-ALLOC-001` | `crates/core/flags/complete/fish.rs:14` | **TP** | `format!("-s {}", char::from(byte))` | loop body allocates (format!/replace/to_string) every iteration |
| `PERF-RS-TYPE-001` | `crates/core/flags/hiargs.rs:112` | **FP** | `from_low_args(mut low: LowArgs)` | by-value param is a move (ownership transfer), not a copy; single startup call |
| `PERF-RS-TYPE-001` | `crates/ignore/src/dir.rs:666` | **FP** | `fn types(&mut self, types: Types)` | `types` moved into Arc::new; no per-call copy; builder setter called once |
| `PERF-RS-TYPE-002` | `crates/core/flags/hiargs.rs:36` | **FP** | `pub(crate) struct HiArgs {` | no #[repr(C)]/FFI; Rust auto-reorders fields, so declared-order padding is moot |
| `PERF-RS-TYPE-002` | `crates/core/flags/lowargs.rs:33` | **FP** | `pub(crate) struct LowArgs {` | no #[repr(C)]/FFI; compiler reorders fields already; padding not actionable |
| `PERF-RS-ARCH-001` | `crates/core/flags/mod.rs:18` | **FP** | `generate_complete_bash` | bash.rs imports crate::flags::defs, not mod.rs; no mod.rs<->bash.rs cycle |
| `PERF-RS-ARCH-001` | `crates/core/flags/parse.rs:9` | **AMBIG** | `use crate::flags::{` | mutual import exists but defs->parse is #[cfg(test)]-gated; no production cycle |
| `PERF-RS-LOOP-001` | `crates/ignore/src/types.rs:486` | **TP** | `for ext in exts {` | triple-nested for (DEFAULT_TYPES->names->exts), depth 3 confirmed |
| `PERF-RS-LOOP-001` | `crates/printer/src/standard.rs:1209` | **TP** | `while !line.is_empty() {` | inside for(matches)->while(stepper)->while; depth 3 confirmed |
| `PERF-RS-STR-003` | `crates/ignore/src/walk.rs:1922` | **FP** | `file.set_len(size).unwrap();` | bounded file-truncate in a test helper; not a string-conversion smell |
| `PERF-RS-STR-003` | `tests/util.rs:119` | **FP** | `nice_err(&path, file.set_len(filesize));` | File::set_len (bounded, test util); not a string conversion smell |
| `PERF-RS-LOOP-008` | `crates/searcher/src/searcher/glue.rs:114` | **TP** | `while !self.slice[self.core.pos()..].is_empty()` | self.core.pos() is a depth-3 member chain re-evaluated in the loop |
| `PERF-RS-LOOP-008` | `crates/searcher/src/searcher/glue.rs:164` | **TP** | `while !self.slice[self.core.pos()..].is_empty() && keepgoing` | same depth-3 self chain in run()'s loop condition |
| `PERF-RS-ARCH-002` | `crates/core/flags/hiargs.rs:112` | **TP** | `fn from_low_args` | flags dir is huge (defs.rs 7675 LOC, ~980 fns); hub/hottest plausible |

## 6. Limitations observed on this repo
- **This is the weakest repo in the showcase.** Sample tally **11 TP / 14 FP / 1 AMBIG** — the Rust backend's false-positive classes dominate on human-written Rust.
- `PERF-RS-CALL-001` (recursion) fired **0/2 TP** — it matches **method-name collisions** (`GlobSetBuilder::build`, `DecompressionMatcherBuilder::new`).
- `PERF-RS-TYPE-001` (0/2) flags by-value **moves**, not copies; `PERF-RS-TYPE-002` (0/2) claims field padding on non-FFI structs (meaningless in Rust).
- `PERF-RS-STR-002` (0/2) flags O(1) `len()` calls; `PERF-RS-STR-003` (0/2) flags `File::set_len` (a bounded truncate, not a string conversion).
- `PERF-RS-LOOP-002` (0/2 here) flags in-memory `write!` to a `String` and `Command::new` — not blocking I/O.
- `PERF-C/`-style positives absent; the strong Rust rules (`CALL-002`, `LOOP-001`, `LOOP-008`, `ALLOC-001`, `ARCH-002`) are TP in the sample.

## 7. Provenance
- Plan `evidence-beta` M4; spec `evidence-beta-spec` §6; ADR-0005.
- Binary: `perfpal 0.2.0-beta.1 (metrics schema 1.3)`, release build (`--locked`).
- Scan tier: **syntactic** (default). Python/Rust are syntactic-only; C here is also syntactic (no libclang).
- Generated from `counts.json` + `verified.json`.
