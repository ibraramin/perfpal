# Perfpal C example corpus (`examples/`)

Small, deliberately noisy C programs used to exercise Perfpal end to end and
to eyeball real output. Each file's header comment lists the findings it is
expected to produce (rule id + severity); if a run disagrees with a header,
the header is stale or the tool regressed — investigate, don't shrug.

## Running it

From the repo root:

```sh
cargo run --locked -- examples -o /tmp/pf-ex
```

Expect exit `0` and a full report tree in `/tmp/pf-ex`
(`metrics.json`, `report.md`, `charts/`). Reference shape (syntactic tier,
default build): **13 files, 83 findings (16 hot / 25 warn / 42 info)**.
The semantic tier (`--features semantic-clang` with libclang present) upgrades
some ARCH-003 edges from LOW to MEDIUM — finding *counts* stay the same.

## What each file demonstrates

| File | Demonstrates |
|---|---|
| `01_allocator_churn.c` | Heap allocation/teardown in loops (`ALLOC-001/002/003`), `sprintf` in loop, unbounded `sprintf`, non-literal `memcpy` size |
| `02_string_building.c` | Quadratic string building (`STR-001/002/003`), `printf`/`putc` in loops |
| `03_deep_nesting.c` | Loop nesting depth, variable-divisor `%`/`/`, globals written in loops. Defines `scale_checksum()`, an ARCH-003 call target (anchored at the caller, never here) |
| `04_stdio_churn.c` | Per-byte/per-iteration stdio (`LOOP-002/003`), unbounded `sprintf` |
| `05_recursion_complexity.c` | Direct recursion (`CALL-001`), high cyclomatic complexity (`CALL-002`, anchored at `classify()`), and the ARCH-002 hot-component pick for the whole `examples/` component (anchored at `classify()`, this component's highest-complexity function) |
| `06_memory_misuse.c` | `sizeof` misuse shapes (`MEM-001`: pointer-param, multiply, divide, nested `sizeof`) and non-literal `memcpy`/`memset` sizes in loops (`MEM-002`) |
| `07_struct_by_value.c` | Large structs passed by value (`TYPE-001`, one finding per function) and padding waste from member ordering (`TYPE-002`). Sizes use the documented LP64 approximation |
| `08_globals_and_chains.c` | Globals written in loops (`LOOP-005`), indirect calls in loops (`LOOP-006`), deep member-access chains (`LOOP-008`), `printf` in loop, nesting depth |
| `09_membership_scan.c` | Hand-rolled O(n²) membership scans (`LOOP-007`), nesting depth, variable-divisor modulo |
| `10_cross_file_and_arch.c` | Architecture signals: unresolved quoted includes + the include cycle (`ARCH-001`), cross-file call edges to `01_`/`03_` (`ARCH-003`), variable-divisor modulo |

Shared headers:

| Header | Role |
|---|---|
| `perf_common.h` | Real internal include edge — several files include it, so the include graph shows internal fan-in. Declares the ARCH-003 targets |
| `cycle_a.h` / `cycle_b.h` | Deliberate include cycle (`ARCH-001`, warn/HIGH). Both includes sit inside `#ifndef` guards and the scanner collects guarded content, so the cycle reports, anchored at the lowest-sorted broken edge (`cycle_b.h` → `cycle_a.h`, listed under `cycle_b.h`) |

## Known planted-but-silent sub-case

`10_cross_file_and_arch.c` also plants an ARCH-001 **hub fan-out** case that
does NOT fire by design: the threshold (≥ 12) counts *distinct RESOLVED*
include targets, unresolved quoted includes don't count, and the file resolves
only 2 (`perf_common.h`, `cycle_a.h`). Making it fire needs ≥ 12 distinct
resolved includes (12 real headers, or 12 includers of one header) — a
deliberate corpus extension, not a bug. See `10_cross_file_and_arch.c`.
