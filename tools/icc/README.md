# ICC reliability harness (Perfpal 0.5.0, part A)

This directory cross-checks Perfpal 0.5.0's function-level structural metrics
against independent parsers and reports intraclass correlation (ICC) as a
reliability measure.

It is a **measurement harness only**: no production code is modified, and the
outputs are deterministic (no timestamps).

## Quick start

```sh
# from the Perfpal repo root
python3 tools/icc/run_icc.py            # reuse existing scans under /var/tmp
python3 tools/icc/run_icc.py --force-scans
```

`run_icc.py` is the single entry point. It:

1. runs `target/release/perfpal <tree> -o /var/tmp/opencode/icc/perfpal/<lang>
   --force` when a scan is missing (reuses it otherwise), including the
   `tools/icc/extra` tree,
2. builds the exact-200 `sample.json`,
3. generates the blind `labels/` packets,
4. runs the Python and C reference tools,
5. consumes `ref/{rust,js,ts}.json` from the parallel Rust/JS/TS agent,
6. runs the statistics and exits non-zero if a gate fails.

Individual steps:

```sh
python3 tools/icc/sample.py
python3 tools/icc/make_labels.py
python3 tools/icc/python_ref.py
python3 tools/icc/c_ref.py
python3 tools/icc/compute_icc.py
```

Requirements: Python 3 (stdlib only), clang 23 at `/usr/bin/clang`. No network
and no third-party Python packages are used.

## Corpus and sample

Perfpal example trees, with the 0.5.0 release binary:

| language | tree | base sample | extra sample | total |
|---|---|---:|---:|---:|
| C | `examples/` + `tools/icc/extra` | 38 | 4 | 42 |
| Rust | `examples-rust/` + `tools/icc/extra` | 40 | 2 | 42 |
| Python | `examples-python/` + `tools/icc/extra` | 40 | 2 | 42 |
| JavaScript | `examples-js/` + `tools/icc/extra` | 33 | 2 | 35 |
| TypeScript | `examples-ts/` + `tools/icc/extra` | 37 | 2 | 39 |
| **total** | | **188** | **12** | **200** |

Synthetic `<module>` records are excluded. The base selection is all
loop-bearing functions plus no-loop functions in `(path, start_line)` order up
to each target; the extra corpus contributes all 12 loop-bearing records. No
RNG is used (`seed: null`). `tools/icc/extra/dead.c` is excluded from the
loop/alloc sample because clang drops its `#if 0` branch; it is used only for
the site packet.

The sample is exactly 200 functions. JavaScript contributes `jsNested` and the
nested arrow `f`; the authored `jsCallback` outer record has no loops of its
own and is therefore not selected.

## Reference output contract

Every reference tool writes:

```json
{
  "language": "python",
  "functions": [
    {
      "path": "examples-python/foo.py",
      "name": "f",
      "start_line": 10,
      "start_col": 0,
      "param_count": 2,
      "loop_count": 2,
      "loop_body_loc": 4,
      "max_loop_depth": 2
    }
  ]
}
```

Paths are relative to the Perfpal repo root. Reference tools may add fields
(e.g. `dead_code_excluded`). `tools/icc/ref/rust.json`, `ref/js.json` and
`ref/ts.json` are owned/produced by the parallel Rust/JS/TS agent; this harness
does not generate them.

## Statistics

* `loop_count` and `loop_body_loc`: ICC(2,1), two-way random effects, absolute
  agreement, k = 2 raters (Perfpal, reference), using the standard MSR/MSE/MSC
  formula in pure Python. Mean difference (Perfpal - reference) and n are
  reported.
* `max_loop_depth`: exact agreement percentage (gate: >= 99%).
* `alloc_sites_in_loops`: ICC(2,1) for C and Rust against their reference
  tools; gate requires n >= 10 and ICC >= 0.70 in each. `insufficient_n` is a
  failure, never a silent pass. The C reference counts direct calls to
  `malloc`, `calloc`, `realloc`, `aligned_alloc`, `strdup` at loop depth >= 1
  from the clang AST (its own list, not Perfpal's table).
* `io_site_count`, `io_sites_in_loops`: ICC(2,1) where both sides provide a
  value. The reference loop contract omits these, so they currently have n = 0.
* `loop_body_loc` gate: PASS only if every language has a reference, n >= 10,
  and ICC >= 0.75. Missing references fail the gate.
* Manual labels (if `labels/loop_labels.json` / `site_labels.json` exist):
  loop ICC >= 0.75; per-site-field Cohen's kappa and ICC(2,1) >= 0.70 for
  `alloc_sites_in_loops`, `io_sites_in_loops` (C `cfg_gated` only). Raw
  agreement is descriptive only. Per the frozen prereg decision rule 4, the
  100-site blind audit drops `io_sites_in_loops` (recall 0.55); `io_site_count`
  remains descriptive. If labels are absent the manual gates report `pending`
  and do not fail.

Matching order: `(path, name, start_line)`, then `(path, name)` when unique on
both sides. Unmatched functions are listed, never silently dropped.

Outputs:

* `results.json` - per field / per language and pooled n, ICC, mean difference,
  exact agreement, unmatched lists, allocation gate, manual-label scores, gate
  verdicts.
* `results.md` - human-readable tables, commands and limitations.
* `labels/loop_packet.json`, `labels/site_packet.json` - blind annotation
  packets; `labels/site_comparison.json` - non-blind scoring key;
  `labels/README.md` - annotation schema and instructions.

## Known limitations

* The example corpus is human-written. AI-arm stratification is deferred.
* C: clang drops `#if 0` / `#if (0)` regions while Perfpal counts them
  syntactically. A sampled function whose source range contains such a region
  is marked `dead_code_excluded` and excluded from the comparison instead of
  being compared silently. (The current corpus has none.)
* The C reference supplies empty stubs for deliberately unresolved quoted
  includes (e.g. `10_cross_file_and_arch.c`) so clang can build the AST.
* Python: only module-level `def` / `async def` are emitted; nested defs,
  lambdas and classes are skipped, matching Perfpal's no-nested-records policy.
  Comprehension generators count as loops and use the element expression as
  their body span; `tools/icc/extra/extra.py` exercises this path.
* The `dead.c` fixture contains only four call sites (two live, two `#if 0`),
  so the site packet includes four rather than the requested ten dead.c
  candidates; both `cfg_gated` values are still exercised.
* Rust/JS/TS reference outputs come from a separate agent. When absent, the
  gate fails with `missing_reference`.
