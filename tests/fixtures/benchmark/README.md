# Benchmark fixtures (`evidence-beta` M2)

This directory holds the **ratchet baseline** for the M2 benchmark suite. The
corpus itself is generated in code (`tests/benchmark/corpus.rs`), not committed
as files here — this keeps every rung deterministic, materializable into a
fresh tempdir, and free of stale on-disk artifacts.

## Files

| File | Purpose |
|------|---------|
| `baseline.json` | Per-rule intended precision/recall ratchet (1.0/1.0). Must cover **exactly** the rules the catalog plants. |

## Generator contract

- **`GenFile { rel_path, contents }`** — one generated source file. Paths are
  POSIX-relative; `materialize` creates parent dirs.
- **`Label { rule_id, rel_path, line, expect }`** — one planted ground truth.
  `expect: Fire` requires a finding whose `(rule_id, path, span.start_line)`
  matches; `expect: Silent` names the path carrying the rule's near-miss
  construct. A silent control is documentation of the intended negative (any
  unmatched scoped finding is already an FP); the harness additionally asserts
  every `Silent` rule is inside the scored set.
- **`Rung { name, files, labels }`** — one labeled corpus.
- The `small` rung plants ONE true positive for **every active rule in the
  67-rule catalog** (23 C + 22 Python + 22 Rust) and an explicit `Silent`
  control for every one of those rules, plus the retired `PERF-RS-STR-001`
  always-silent control. The harness asserts that the set of rules carrying a
  `Fire` label **equals** the 67 active rule ids and that `Silent` coverage is
  exactly the 67 active rules plus `PERF-RS-STR-001`, so deleting an extension
  unit can never silently drop a rule from scoring. The nine existential gate
  constructs are lifted **verbatim** from the committed M1 corpus
  (`tests/fixtures/precision/`), retaining every M1 fire anchor (including the
  six Python KPI fixtures); the other rules use dedicated minimal files (with
  unique `_cNN` / `_rNN` / `py_` names) so a construct can never trip a sibling
  rule unlabelled.
- `medium`/`large` replicate the NINE gate rules **unchanged** (the M1 KPI
  set) with a unique `_k` suffix on every function/definition name, so
  replicas can never trip the cross-file name-uniqueness gate (ARCH-003),
  create a false cross-file call edge, or form an accidental
  include/component hub.

## How labels are derived

1. Every template marks its **anchor line** with a stable comment token
   (`/* TP PERF-C-… */`, `// TP PERF-RS-…`, `# TP PERF-PY-…`; the Rust ALLOC-001
   anchor is the `for` line because that rule anchors at the loop span).
2. `anchor_line(contents, marker)` collects every 1-based line containing that
   token and **asserts exactly one match** (a duplicate or misplaced token
   fails loudly instead of silently anchoring on the first hit), then records
   it. Labels are therefore computed from the exact bytes that get written —
   they cannot drift from a hand-maintained line number.
3. Silent controls are emitted without a line (`0`); they document the intended
   negative. The `small` rung labels one for every active rule (wherever the
   template contains a genuine near-miss construct), and the harness asserts
   that coverage is exactly the 67 active rules plus `PERF-RS-STR-001`.

## Scoring

For each scoped rule: `TP` = the set intersection of fire labels and actual
findings on `(rule_id, path, span.start_line)` (a duplicate finding at a fire
coordinate is not double-counted); `FN` = unmatched fire labels; `FP` =
**any** finding of a scoped rule with no matching fire label (subsuming every
silent control). A scored rule with **no** `Fire` label would measure 0/0/0 and
ratchet vacuously to 1.0/1.0, so the harness asserts every scored rule carries a
`Fire` label — the only exemption is the retired always-silent
`PERF-RS-STR-001`. Measured `precision`/`recall` must be `>=` `baseline.json` —
the baseline is the intended 1.0/1.0 target and must **never** be lowered to
make a test pass. A shortfall is a real analyzer gap or a corpus bug.

The ratchet is **per rung**, scoped to the rules that rung actually plants:
`small` ratchets all 67 active rules (plus `PERF-RS-STR-001`), while
`medium`/`large` ratchet only the nine gate rules they replicate. Every rule a
rung plants MUST appear in `baseline.json`, and every `baseline.json` key MUST
be planted by at least one rung (no stale entries).

## Extending the catalog

The rule set is pinned in `corpus.rs`: `gate_c_unit` / `gate_rust_unit` for
the nine replicated gate rules, and `c_ext` / `rust_ext` / `py_ext` (plus the
collateral labels folded into `small`) for the rest. To add another rule:
1. add a template (TP + silent control) in `corpus.rs`;
2. add its fire/silent labels;
3. add the rule id to `active_rule_ids()` in `tests/benchmark/mod.rs` (the
   `assert_small_covers_catalog` lock asserts the Fire-label set equals the
   67 active ids, so a new rule fails loudly until listed);
4. add the rule to `baseline.json`.
The in-crate reporter test `catalog_covers_every_registry_rule_id`
(`src/reporter/mod.rs`) locks the registry, catalog and count together.
`tests/benchmark.rs` needs no change.

## Semantic-only sub-cases (not plantable from plain source)

Three catalog sub-cases require resolver facts and are covered by the IR
fixtures (`tests/fixtures/ir/`) and the `semantic-clang` build rather than by
the plain-source benchmark: the proven-`fclose` ALLOC-002 path
(`PERF-C-ALLOC-002__fclose-in-loop.json`), the resolved-format STR-003 path
(`PERF-C-STR-003__resolved-format.json`), and the resolved ARCH-003 edge
(`PERF-C-ARCH-003__resolved-edge.json`). The ARCH-001 deep-chain (≥8) and hub
fan-out (≥12) sub-cases are likewise IR-fixture-covered
(`…__deep-chain.json`, `…__fanout-hub.json`); the plain-source benchmark
plants the required cycle true positive instead.
