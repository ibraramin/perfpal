# Accepted limitations & false-positive classes

> Required by ADR-0005 / `evidence-beta-spec` §6: *"an accepted-limitations page — an explicit, honest
> list of the tool's approximate/FP classes."* Perfpal is **static analysis**: every finding is an
> *optimization candidate*, never a proven bottleneck. This page is the cranky truth, consolidated from
> the M4 showcase (`docs/evidence/ai-code-showcase.md`), the M1 dogfood
> (`docs/evidence/dogfood-rust-backend.md`), the W0 canary (`docs/evidence/python-canary.md` §9), and the
> Python tier notes (`docs/architecture/python-backend.md` §10).
>
> **Framing.** Disclosing these is the product's differentiator (honesty), not a defect. A finding's
> `confidence` (`HIGH`/`MEDIUM`/`LOW`) and `severity` (`hot`/`warn`/`info`) are the prioritization hints;
> the FP classes below concentrate in the **`LOW`-confidence, name-matching** rules.

## A. Rule-level FP classes observed in the M4 showcase

`TP/FP` = hand-verified in the M4 stratified sample (`docs/evidence/showcase/*/verified.json`).

| Rule | Mechanism of the FP | M4 evidence | Verdict |
|---|---|---|---|
| `ARCH-003` (PY/C/RS) | **Name-only** cross-file coupling: a call to `x.foo()` is matched to any definition `foo` reachable by name, ignoring the receiver type. Duck-typing (Python), libc/std same-names (C/Rust) collide. | `print` vs `InputOutput.print` (aider); `strdup` ↔ `src/tool_strdup.c` (curl); `PathBuf::from` ↔ a local `from` (chunkhound) | mixed; always `LOW`/`info`; **72% of all M4 volume** |
| `CALL-001` (PY/RS) | Direct-recursion test matches a **method name** against the enclosing function name, but not the receiver type → name collisions count as "self-calls". | `super().start_section` (aider); `HashSet::new`, `Connection::open`, `self.progress.add_task` (chunkhound); `GlobSetBuilder::build` (ripgrep) | PY `1/6`, RS `0/4` TP |
| `CALL-001` (C) | (C) same name-collision risk between static functions in different TUs. | not hit in the curl sample (both TP) | watch |
| `TYPE-001` (RS) | Flags a **by-value parameter** as a copy, but Rust *moves* by-value args (ownership transfer) unless `Copy`; a single startup call is not hot. | `from_low_args(mut low: LowArgs)`, `new(config: DbConfig)` (ripgrep/chunkhound) | `0/4` TP |
| `TYPE-002` (RS) | Claims field-ordering **padding** on structs that are **not** `#[repr(C)]`/FFI — Rust reorders fields, so declared-order padding is meaningless. | `struct HiArgs`, `struct PipelineConfig` | `0/3` TP |
| `STR-002` (RS) | "Loop-invariant length" FPs on `O(1)` calls (`len()`, `is_empty()`) whose operand is loop-varying. | `e.is_empty()`, `e.len()` (chunkhound); `col1.len()` reassigned each iter (ripgrep) | `0/4` TP |
| `STR-002` (C) | Flags `strlen()` of a string that is **reassigned** each iteration (hoisting would be wrong). | `strlen(link)` (curl `crawler.c:151`) | 1 FP of 2 |
| `STR-003` (RS) | A string-conversion smell that over-matches `File::set_len` (a bounded file truncate). | `walk.rs:1922`, `tests/util.rs:119` | `0/2` TP |
| `LOOP-002` (RS/PY) | "Blocking I/O in loop" over-matches **in-memory** `write!` to a `String`, `Command::new`, and **async** (non-blocking) `await readline()`. | ripgrep `help.rs:102`; chunkhound `client_proxy.py:300` | 2 FPs of 4 |
| `LOOP-004` (C/RS) | "Non-constant divisor" FPs when the divisor is a compile-time constant (`RAND_MAX`, `sizeof(...)`, a `const`). | curl `crawler.c:142`, `cf-https-connect.c:361`; chunkhound `differ.rs:122` | `0/3` TP |
| `ALLOC-001` (PY) | "Container literal in loop" over-matches `open()` (file) and `Event.set()` (method) — the name heuristic is too loose. | chunkhound `hyde.py:233`, `client_proxy.py:180` | 2 FPs of 4 |
| `MEM-002` (PY) | Matches any `copy`-named call, including **`shutil.copy`** — a *filesystem* copy, not `copy.copy`/`deepcopy`. | aider `benchmark.py:768/1014` | `0/2` TP |
| `COMPREHENSION-001` (PY) | Flags a loop whose body appends to a list when the body also contains other statements (e.g. an early `break`) — not a comprehension candidate. | chunkhound `ia.py:328` | 1 FP of 5 |
| `ARCH-001` (PY/RS) | Import-cycle FP when the import is `TYPE_CHECKING`-gated (Python) or `#[cfg(test)]`-gated (Rust) — no **runtime** cycle. | chunkhound `indexing_coordinator.py:27`; ripgrep `flags/mod.rs:18` (FP) and `flags/parse.rs:9` (AMBIG) | 2 FP + 1 AMBIG |
| `LOOP-006` (C) | Indirect-call FP on a raw `#ifdef` preprocessor line — a **macro-parse artifact** (the file parsed partially). | curl `socketpair.c:224` | 1 FP of 2 |
| `LOOP-008` (RS, carried) | Member/deref-chain modelling (`R-pf-078`): chains are flagged even when the base is loop-invariant and hoistable — TP but low action value. | (not re-checked in M4 sample) | open |

**Ambiguous-but-technically-present** (`AMBIG`) cases are also disclosed, not hidden: a `dict()` inside a
loop that runs ≤1× (aider `base_coder.py:193`); a deliberate **retry** `while True` with per-attempt I/O
(aider `base_coder.py:1459`); **pagination** HTTP in a `while` (aider `issues.py:89`); a guarded one-time
container init (aider `linter.py:283`); a `+=` whose target is reset each pass (chunkhound `search.py:235`).

## B. Language-tier approximations

- **Python** (`PERF-PY-*`, syntactic only): `ARCH-*` thresholds are conservative and **not
  decision-grade**; inference is single-assignment and best-effort; the attribute-path key is
  **root-only**; `.ipynb` is excluded. **Root-package relative imports** (a `.py` directly under the scan
  root doing `from .sibling import …`) do **not** resolve — the candidate is built as
  `"{package}.{rest}"`, so an empty package yields a leading-dot candidate that never matches
  (`R-pf-094`). T2 cross-module resolver and T3 runtime profiles are out of scope.
- **Rust** (`PERF-RS-*`, syntactic only): `TYPE-002` `repr(C)`/layout reasoning and the residual
  name-based coupling above are the known gaps; a `r#`-raw-identifier receiver nit (`R-pf-080`) and the
  `LOOP-008` deref-chain model (`R-pf-078`) remain open.
- **C** (`PERF-C-*`, syntactic, optional `semantic-clang`): libclang upgrades ARCH-003 cross-file edges to
  MEDIUM only when present; a USR-ambiguity guard (`R-pf-077`) is a known resolver limit. Macro-heavy
  code parses **partially** (curl: 236/923 files) and partial regions can host parse artifacts.
- **`.pyi` / `.h`**: stub/header files are scanned but carry no bodies, so most rules are silent there.

## C. Global modelling limits (all languages)

- **Static ≠ runtime.** A flagged pattern may sit on a cold path, be provably bounded, or be dominated by
  I/O the tool cannot see. Runtime profiling (T3) is post-MVP.
- **Severity hooks are local.** Downgrade hooks (macro origin, parse-error overlap → MEDIUM) and severity
  hooks (allocation co-occurrence → hot; guarded I/O → warn) operate within a single function/loop scope.
- **No cross-language edges** (deferred, ADR-0004): mixed C/Rust/Python call graphs are separate;
  architecture findings never span languages.
- **Determinism over cleverness.** Every traversal is sorted for byte-identical output; where two
  resolutions are equally plausible the tool reports the conservative one.

## D. Disposition

None of the classes above **blocks** the M4 evidence record (the gate asks for an honest showcase, and
this page is that honesty). The M4-observed rule-level FPs are **follow-up candidates** — the strongest
arguable precision work is the name-matching family: `CALL-001` (PY/RS recursion name collisions),
`ARCH-003` receiver-type awareness, and the Rust `TYPE-001/002`, `STR-002/003`, `LOOP-002` over-matches.
Any fix must be precision-measured against the M1 KPI + M2 benchmark before landing (never lowered).
