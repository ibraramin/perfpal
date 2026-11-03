# Perfpal 0.5.0 ICC reliability harness (reference cross-check)

Deterministic output (no timestamps). Perfpal is rater 1 and the
independent parser is rater 2. `loop_count` and `loop_body_loc` use
ICC(2,1), two-way random absolute agreement with k = 2.
`max_loop_depth` is reported as exact agreement percentage.

## Commands

```sh
# release binary (0.5.0); -o is the report directory
target/release/perfpal examples        -o /var/tmp/opencode/icc/perfpal/c      --force
target/release/perfpal examples-rust   -o /var/tmp/opencode/icc/perfpal/rust   --force
target/release/perfpal examples-python -o /var/tmp/opencode/icc/perfpal/python --force
target/release/perfpal examples-js     -o /var/tmp/opencode/icc/perfpal/js     --force
target/release/perfpal examples-ts     -o /var/tmp/opencode/icc/perfpal/ts     --force
target/release/perfpal tools/icc/extra -o /var/tmp/opencode/icc/perfpal/extra  --force

python3 tools/icc/sample.py
python3 tools/icc/make_labels.py
python3 tools/icc/python_ref.py
python3 tools/icc/c_ref.py
python3 tools/icc/compute_icc.py
# or all of the above:
python3 tools/icc/run_icc.py
```

## Sample

Synthetic `<module>` records are excluded. Selection is deterministic:
the 188-function base sample (all loop-bearing functions plus no-loop
functions in `(path, start_line)` order) plus all 12 loop-bearing
functions under `tools/icc/extra`. `dead.c` is excluded from the loop/
alloc sample because clang drops its `#if 0` branch.

| language | base | extra | total | loop fns | no-loop fns |
|---|---:|---:|---:|---:|---:|
| c | 38 | 4 | 42 | 29 | 13 |
| rust | 40 | 2 | 42 | 22 | 20 |
| python | 40 | 2 | 42 | 28 | 14 |
| js | 33 | 2 | 35 | 17 | 18 |
| ts | 37 | 2 | 39 | 17 | 22 |
| **total** | **188** | **12** | **200** | | |

The sample is exactly 200 real functions; the extra corpus supplies
C 4, Rust 2, Python 2, JavaScript 2 (`jsNested` and the nested arrow
`f`), and TypeScript 2 loop-bearing records.


## Per-language ICC (loop_body_loc, loop_count)

| language | n matched | loop_body_loc ICC | mean diff | loop_count ICC | mean diff | depth exact % |
|---|---:|---:|---:|---:|---:|---:|
| c | 42 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |
| rust | 42 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |
| python | 42 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |
| js | 35 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |
| ts | 39 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |
| **pooled** | 200 | 1.0000 | 0.0000 | 1.0000 | 0.0000 | 100.00 |

## Optional fields

`alloc_sites_in_loops`, `io_site_count` and `io_sites_in_loops` are
computed only where both sides provide values. The C reference now
supplies `alloc_sites_in_loops`; the Rust reference supplies it too
when built with the allocation field. I/O fields remain n = 0 because
the reference loop contract omits them.

| language | field | n | ICC | mean diff |
|---|---|---:|---:|---:|
| c | alloc_sites_in_loops | 42 | 1.0000 | 0.0000 |
| c | io_site_count | 0 | n/a | n/a |
| c | io_sites_in_loops | 0 | n/a | n/a |
| rust | alloc_sites_in_loops | 42 | 1.0000 | 0.0000 |
| rust | io_site_count | 0 | n/a | n/a |
| rust | io_sites_in_loops | 0 | n/a | n/a |
| python | alloc_sites_in_loops | 0 | n/a | n/a |
| python | io_site_count | 0 | n/a | n/a |
| python | io_sites_in_loops | 0 | n/a | n/a |
| js | alloc_sites_in_loops | 0 | n/a | n/a |
| js | io_site_count | 0 | n/a | n/a |
| js | io_sites_in_loops | 0 | n/a | n/a |
| ts | alloc_sites_in_loops | 0 | n/a | n/a |
| ts | io_site_count | 0 | n/a | n/a |
| ts | io_sites_in_loops | 0 | n/a | n/a |

## Allocation sites in loops (C / Rust)

| language | n | ICC | mean diff | gate |
|---|---:|---:|---:|---|
| c | 42 | 1.0000 | 0.0000 | pass |
| rust | 42 | 1.0000 | 0.0000 | pass |
| **pooled** | 84 | 1.0000 | n/a | PASS |

## Manual labels

| language | n | loop_body_loc ICC | loop_count ICC | depth exact % |
|---|---:|---:|---:|---:|
| c | 24 | 0.9904 | 1.0000 | 100.00 |
| rust | 20 | 0.9935 | 1.0000 | 100.00 |
| python | 26 | 0.7575 | 1.0000 | 100.00 |
| js | 15 | 1.0000 | 1.0000 | 100.00 |
| ts | 15 | 1.0000 | 1.0000 | 100.00 |
| **pooled** | 100 | 0.9812 | 1.0000 | 100.00 |
* Python definitional note: the annotator excluded leading comments because they are not inside the tree-sitter body-node span; the spec's body-node definition is the locked one. Python passes at 0.7575, pooled at 0.9812.

Perfpal is the prediction and the manual labels are truth.
`agreement` is descriptive only; gates use kappa and ICC.

| field | n | TP | FP | FN | TN | precision | recall | kappa | ICC | gate |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| alloc_sites_in_loops | 100 | 8 | 1 | 1 | 90 | 0.8889 | 0.8889 | 0.8779 | 0.8790 | pass |
| io_sites_in_loops | 100 | 11 | 0 | 9 | 80 | 1.0000 | 0.5500 | 0.6617 | 0.6639 | DROPPED |
| cfg_gated_c | 15 | 2 | 0 | 0 | 13 | 1.0000 | 1.0000 | 1.0000 | 1.0000 | pass |

* `dropped_fields`: io_sites_in_loops
* Drop reason: The 100-site blind audit found 9 external-I/O calls Perfpal missed (C fgetc/putc; JS/TS bare readFileSync; Rust write_all; Python DB/wrapper calls): recall 0.55. Per the frozen prereg decision rule 4, io_sites_in_loops is dropped from the confirmatory family; io_site_count remains descriptive only.
* `io_site_count` remains descriptive only.

## Gates

* `loop_body_loc` gate (n >= 10 and ICC >= 0.75 in every language): **PASS**
  * c: n=42, ICC=1.0000, pass
  * rust: n=42, ICC=1.0000, pass
  * python: n=42, ICC=1.0000, pass
  * js: n=35, ICC=1.0000, pass
  * ts: n=39, ICC=1.0000, pass
* `max_loop_depth` exact agreement (>= 99.0% in every language): **PASS**
* `alloc_sites_in_loops` gate (C and Rust each n >= 10 and ICC >= 0.7): **PASS**
  * c: n=42, ICC=1.0000, pass
  * rust: n=42, ICC=1.0000, pass
* manual loop gate: **PASS**
* manual site gate: **PASS**
  * dropped: io_sites_in_loops

## Freeze decision

| component | verdict |
|---|---|
| loop_body_loc | PASS |
| max_loop_depth | PASS |
| alloc_sites_in_loops | PASS |
| manual_loop | PASS |
| cfg_gated_c | PASS |
| io_sites_in_loops | DROPPED |

* `freeze_ok`: **True**
* Rule: frozen prereg decision rule 4
* Dropped fields: io_sites_in_loops
* `io_site_count`: descriptive only
* Drop reason: The 100-site blind audit found 9 external-I/O calls Perfpal missed (C fgetc/putc; JS/TS bare readFileSync; Rust write_all; Python DB/wrapper calls): recall 0.55. Per the frozen prereg decision rule 4, io_sites_in_loops is dropped from the confirmatory family; io_site_count remains descriptive only.

## Matching and unmatched counts

Match order: `(path, name, start_line)` first, then `(path, name)` when
unique on both sides. Nothing is silently dropped.

| language | matched | unmatched Perfpal | unmatched reference | dead-code excluded |
|---|---:|---:|---:|---:|
| c | 42 | 0 | 0 | 0 |
| rust | 42 | 0 | 0 | 0 |
| python | 42 | 0 | 1 | 0 |
| js | 35 | 0 | 1 | 0 |
| ts | 39 | 0 | 0 | 0 |

### Unmatched details

* python unmatched reference: `examples-python/14_async_blocking.py` `await_once` line 31 (not_selected_in_sample)
* js unmatched reference: `tools/icc/extra/extra.js` `jsCallback` line 11 (not_selected_in_sample)

## Limitations

* The example corpus is human-written. AI-arm stratification and
  per-author grouping are deferred to a later phase.
* C loop bodies are measured from clang's body range, which includes
  braces for compound bodies (matching Perfpal's span convention).
  Clang removes `#if 0` / `#if (0)` code while Perfpal counts it
  syntactically; any such sampled function is flagged
  `dead_code_excluded` and excluded from the comparison rather than
  compared silently.
* Python reference emits module-level `def` / `async def` only.
  Perfpal does not emit nested function records, so nested defs and
  lambdas are skipped when counting an outer function; loops inside
  a nested def are not attributed to the outer function.
* Comprehension generators count as loops; their body span is the
  element expression. The extra corpus (`extra.py` `p_comp`) now
  exercises this path (Perfpal and the Python reference agree).
* The extra corpus adds C 4, Rust 2, Python 2, JavaScript 2 and
  TypeScript 2 loop-bearing records, making the sample exactly 200.
  `dead.c` is excluded from the loop/alloc comparison because clang
  drops its `#if 0` branch; it is used only in the site packet.
* The C reference counts direct calls to `malloc`, `calloc`,
  `realloc`, `aligned_alloc`, `strdup` at loop depth >= 1. It uses
  the clang AST and does not import Perfpal's allocation table.
* Perfpal exposes line-level `resource_sites` only for C; non-C
  site-comparison entries are therefore `neither`. Site agreement is
  reported across all packet languages as specified, but only the C
  portion has a line-level Perfpal reference.
* Manual-label packets are blind (no Perfpal values). The dead.c
  fixture contains only four call sites (two live, two `#if 0`), so
  the site packet includes four rather than ten dead.c candidates;
  cfg_gated is still exercised in both states.
* Reference outputs for Rust/JavaScript/TypeScript are produced by a
  separate agent; this harness consumes them when present and fails
  the gate when they are missing.
