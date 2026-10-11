# Dogfood + `examples-python/` — evidence (plan `python-backend`, W4)

> **Scope.** The Python tier's R4-equivalent proof. Unlike the Rust tier (which can
> dogfood Perfpal's own `src/`), Perfpal is written in Rust, so the Python tier's
> dogfood corpus is the committed **`examples-python/`** package. This document
> records the measured evidence: the 22/22 rule-coverage gate, the silent
> controls, the precision-KPI ratchet, the benchmark coverage, and the
> determinism/golden locks. Everything here is measured by the real full
> pipeline (`scanner::scan → analyzer::analyze → aggregator → writer/reporter`),
> never asserted on paper.

## 1. `examples-python/` — a real Python package

`examples-python/` is a **20-file** package (`__init__.py` + numbered `0N_*.py`
modules grouped by heuristic family + a `pkg/` subpackage). The subpackage holds
the import-cycle pair `pkg/cycle_a.py` ↔ `pkg/cycle_b.py` (ARCH-001) and the
cross-module helper `pkg/arch_helpers.py` (ARCH-003). The numbered modules cannot
be module-declared (Python identifiers cannot start with a digit) and are
discovered by file extension — the same pattern as `examples-rust/`.

Scanning the committed tree with the real pipeline:

- **20 files scanned** (20 ok / 0 partial / 0 failed).
- **29 findings: 0 hot / 12 warn / 17 info.**
- `scan.languages == ["python"]`; every `files[].language == "python"`.
- **Zero** `PERF-C-*` / `PERF-RS-*` leakage.

### 1.1 Per-rule counts (all 22 `PERF-PY-*` rules fire)

| Rule | Count | Rule | Count |
|---|---|---|---|
| `PERF-PY-ALLOC-001` | 1 | `PERF-PY-EXECUTOR-001` | 1 |
| `PERF-PY-ALLOC-003` | 3 | `PERF-PY-LOOP-001` | 1 |
| `PERF-PY-ALLOC-004` | 1 | `PERF-PY-LOOP-002` | 1 |
| `PERF-PY-ARCH-001` | 1 | `PERF-PY-LOOP-003` | 1 |
| `PERF-PY-ARCH-002` | 1 | `PERF-PY-LOOP-007` | 3 |
| `PERF-PY-ARCH-003` | 3 | `PERF-PY-LOOP-009` | 1 |
| `PERF-PY-ASYNC-001` | 2 | `PERF-PY-MEM-002` | 1 |
| `PERF-PY-ASYNC-002` | 1 | `PERF-PY-N1-001` | 1 |
| `PERF-PY-CALL-001` | 1 | `PERF-PY-PANDAS-001` | 1 |
| `PERF-PY-CALL-002` | 1 | `PERF-PY-PANDAS-002` | 1 |
| `PERF-PY-COMPREHENSION-001` | 1 | `PERF-PY-STR-001` | 1 |

`PERF-PY-ALLOC-003` = 3 because the flagship accumulation rule has two gated
shapes (non-in-place `acc = acc + [x]`; `append` into an accumulator never
rebuilt in the loop) and the `COMPREHENSION-001` fixture's single-`append` body
is also a genuine accumulator. `PERF-PY-ARCH-003` = 3 (three cross-module call
edges) and `PERF-PY-LOOP-007` = 3 (membership, index, and `.index()` shapes).

### 1.2 The coverage gate

`tests/examples_python_coverage.rs` runs the real pipeline over a hermetic copy
of the committed corpus and **locks**, by set equality and exact counts:

1. the emitted `PERF-PY-*` id set **equals** the 22-rule catalog (no missing, no
   unexpected rule);
2. **zero** `PERF-C-*` / `PERF-RS-*` findings;
3. `scan.languages == ["python"]`, `files.len() == 20`, every file `.py` +
   `language == "python"`;
4. `report.md` §4 catalogs every emitted id;
5. the total (29) and the severity split (0/12/17), plus every per-rule count;
6. the silent controls (§2) stay silent — asserted **directly**.

## 2. Silent controls (19, each verified to fire 0 findings)

One benign near-miss is asserted per rule via a `SILENT_CONTROLS` table with
three checks: (a) anti-typo — the control line must carry a distinctive needle;
(b) the target rule must **not** overlap that line; (c) non-vacuity — the rule
must fire elsewhere in the corpus. The controls: constructor outside a loop
(ALLOC-001), receiver rebuilt in the loop (ALLOC-003), materialisation outside a
loop (ALLOC-004), a loop-invariant string read (STR-001), depth-2 nesting
(LOOP-001), `print` in a loop (LOOP-002), plain `range(n)` (LOOP-003), set
membership (LOOP-007), `try` outside a loop (LOOP-009), `deepcopy` outside a
loop (MEM-002), a non-recursive function (CALL-001), a low-complexity function
(CALL-002), a same-file call (ARCH-003), a sync `def` (ASYNC-001), an `await`
outside a loop (ASYNC-002), a multi-statement loop body (COMPREHENSION-001),
an executor created once (EXECUTOR-001), a DB call outside a loop (N1-001), and
a vectorised pandas aggregation (PANDAS-001). ARCH-001/ARCH-002 are
structure-level (asserted by non-vacuity + the pinned counts, not a single
control line).

## 3. Precision KPI — 6 Python rules at 1.0/1.0

`tests/fixtures/precision/` gains a `python/` subdir (13 fixtures) and
`labels.json` gains six Python rules — `ALLOC-001`, `ALLOC-003`, `STR-001`,
`LOOP-007`, `ARCH-003`, `MEM-002` — each with a planted true positive and a
silent control. `tests/precision_kpi.rs` runs the real pipeline and ratchets
per-rule precision/recall; all six measure **precision 1.00 / recall 1.00**,
alongside the ten C/Rust rules.

## 4. Benchmark + goldens

- **Benchmark ladder.** The `small` rung widens from 45 to the full **67-rule
  catalog** (113 files): every active rule gets a planted true positive and a
  silent control, ratcheted against `tests/fixtures/benchmark/baseline.json`
  (all 1.0/1.0). The nine M1 gate rules remain the `medium`/`large` ladder. The
  unlabeled real-code rung now scans `src/` + `examples/` + `examples-rust/` +
  `examples-python/` twice for byte-determinism (PASS).
- **Golden tree.** A 4th committed golden tree `tests/golden/report-python/`
  (metrics + report + charts + schema artifact) is added and byte-stable across
  two regeneration runs.

## 5. Verified commands

| Claim | Command | Result |
|---|---|---|
| Full suite | `cargo test --locked` | green (all test binaries; 0 failures) |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0, 0 warnings |
| Coverage gate | `cargo test --locked --test examples_python_coverage` | 1 passed |
| Precision KPI | `cargo test --locked --test precision_kpi` | 16 rules at 1.0/1.0 |
| Benchmark | `cargo test --locked --test benchmark` | green (small = 68 scoped) |
| Goldens | `cargo test --locked --test p6_hardening` | 4 passed |

## 6. Accepted limitations (unchanged from §10 of `python-backend.md`)

`ARCH-*` thresholds remain conservative and are **not** decision-grade on
Python; inference is single-assignment and best-effort; the attribute-path key
is root-only; `.ipynb` is excluded; T2 (cross-module resolver) and T3 (runtime
profiles) are out of scope. **Root-package relative imports** (a `.py` directly
under the scan root importing `from .sibling import …`) are not resolved by
`resolve_python_edges` — its candidate is built as `"{package}.{rest}"`, so a
root-level module (empty package) yields a leading-dot candidate that never
matches; the corpus therefore places its import cycle inside a `pkg/`
subpackage. This is carried as a low-severity follow-up.
