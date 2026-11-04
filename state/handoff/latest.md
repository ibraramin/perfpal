# Handoff — Perfpal structural 0.5.0 frozen (paper track)

**Date:** 2026-11-04 · **State:** Perfpal **0.5.0 / metrics schema 1.9** structural suite
frozen; tag `perfpal-v0.5.0-structural`; branch `feat/structural-0.5.0` (local; no push).
**Code:** `f53a882` (re-freeze; Stage 1 `e441fc1`, Stage 2 `fd77588`, ICC `8d68209`,
R1 fixes `f53a882`; freeze-doc commit follows). **Binary:** `target/release/perfpal`
`perfpal 0.5.0 (metrics schema 1.9)`, SHA-256
`b89d75027d8b238ad8febab1c8dc3a6687f0a4fd731c86b4e951257d84780ee0`.
**Evidence:** `docs/evidence/structural-0.5.0/FREEZE.md`; `tools/icc/` (results,
sample, independent-parser refs, blind labels). **H4 of `polish-report` remains
next for the tool track**; this handoff is the paper-track freeze.

## What changed
- Five-language `loop_count`, `loop_body_loc`, per-function `parse_health`,
  C/Rust `alloc_sites_in_loops`, five-language `io_site_count`/`io_sites_in_loops`,
  `resource_sites[]` (in-loop, sorted, 0-based column; C-only per-site
  `cfg_gated`, null elsewhere), schema 1.9 additive copies.
- C nested-function traversal boundary fixed; Python comprehension body =
  element expression; determinism/invariant/fixture/golden coverage.
- Re-freeze after the R1 panel: C `#if 0` dead regions stop before
  `#else`/`#elif` (live arms are not gated); `resource_sites[].column` is
  0-based; ICC site matching keys `(path, line, 0-based column)`; `run_icc.py`
  propagates `compute_icc.py` failures. New `#else`/column fixtures,
  determinism `#if 0` coverage, table-version assertions.

## Verification
- `cargo fmt`, clippy, `cargo test --locked` (**1,142 passed**, 0 failed),
  `cargo deny check licenses`, semantic clippy, semantic tests
  (**1,144 passed**, 0 failed), release build — all exit 0.
- `python3 tools/icc/run_icc.py --force-scans` — exit 0; `freeze_ok=true`.
  `loop_body_loc` ICC 1.0000 automated / 0.9812 pooled manual (Python 0.7575);
  `max_loop_depth` exact 100%; `alloc_sites_in_loops` ICC 1.0000 / manual
  kappa 0.9357; C `cfg_gated` kappa 1.0000. `io_sites_in_loops` kappa 0.6617,
  ICC 0.6639, recall 0.55 -> **dropped** from the confirmatory family under
  prereg decision rule 4; `io_site_count` descriptive only.
- Tag moved to the re-freeze freeze-doc commit; old binary `f254de14…c240`
  retired. No main scan.

## Next
- **Paper track:** Stage A 500-repo mechanics check with the full-frame sampler,
  then the main scan only after the founder's explicit go-ahead (see
  `/home/shafin/Opencode/Perfpal-paper/docs/specs/` A1/A2).
- **Tool track:** resume `polish-report` H4 (`report.html` final composition)
  when returning to the Perfpal product; do not mix the structural branch into
  the H4 work without a merge decision.

**Uncommitted files:** none after the freeze-doc/handoff commit. No push.
