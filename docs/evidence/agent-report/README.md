# Evidence — Track C (agent report)

**Status:** C1 contract evidence. The C2 Rust renderer now exists (`src/reporter/agent_report.rs`); this
directory still describes the C1 Python-derived sample and is not emitted by `perfpal`. The normative
contract is [`docs/specs/agent-report.md`](../../specs/agent-report.md).

## Files

| File | What it is |
|---|---|
| `sample/derive_sample.py` | Evidence-only reproducer (stdlib Python). **Not** production code, **not** normative (spec §2 D8). Parses the 67 fix hints out of the spec table, re-derives the shipped KV1/KV3 orderings from `metrics.json`, and renders the two sample artifacts. |
| `sample/agent-report.sample.md` | Derived `agent-report.md` for `tests/golden/report-mixed/metrics.json` (4 files, 4 functions, 9 findings). |
| `sample/agent-actions.sample.json` | Complete `agent-actions.json` for the same scan (9 actions, 4 profile rows). |
| `c1-gate.sh` | Executable C1 gate. Exit 0 = green; its stdout is the committed transcript. |
| `c1-gate.txt` | Gate transcript (this script's own output, including `GATE_EXIT`). |

Source fixture: `tests/golden/report-mixed/metrics.json` (`schema_version` 1.7). Its `scan.root_path` is
the fixture placeholder `<ROOT>`, so the sample carries no machine-specific path.

## Reproduce and verify

```bash
tmp=$(mktemp -d)
python3 docs/evidence/agent-report/sample/derive_sample.py \
  tests/golden/report-mixed/metrics.json "$tmp" docs/specs/agent-report.md
diff "$tmp/agent-report.sample.md" docs/evidence/agent-report/sample/agent-report.sample.md
diff "$tmp/agent-actions.sample.json" docs/evidence/agent-report/sample/agent-actions.sample.json
bash docs/evidence/agent-report/c1-gate.sh        # exit 0; stdout = c1-gate.txt
```

The reproducer prints `actions=9 profile=4 hints=67 bytes=8889 ok` and fails closed on:

- a hint table that is not exactly 67 rows, has a duplicate `rule_id`, or has a hint over 20 words;
- an action whose `rule_id` has no hint;
- **any v1-only function** (the chart-side v1 cost is not reproduced here — the sample is v2-only);
- `complete_action_count != actions.len()`;
- a forbidden time claim (`ms`, `µs`, `seconds`, `speedup`, `latency`, `throughput`, `runtime`) in **C-authored**
  text (fix hints, headers, caveats, data-quality lines) — quoted `- why:` / `- what:` analyser
  provenance is exempt by contract (spec §2 D7);
- a rendered MD over the C1 gate budget (64 KB; the C2 contract is 80 KB, spec §12).

## What this sample does and does not prove

**Proves (reviewable by eye and by the gate):** the artifact names, the envelope + action + profile-row
key sets, the fixed seven-section MD skeleton, the `showing top N of M` cap lines, the per-rule rollup
shape, the `unknown-trip` badge placement, the §9.9 escaping as far as this fixture can show it (single-line, backtick substitution), and that the 67-hint table is complete **as a set** against `HEURISTIC_CATALOG_V1`.

**Does not prove:** byte-equality with the future Rust renderer, the 80 KB budget at real scan scale, the
full state matrix, the prose-HTML neutralisation (no `<`/`>` occurs in this fixture's `title`/`why`), the
unknown-`rule_id` hint stub (the reproducer fails closed before rendering it), or the `detail_anchor`
collision path (no slug collisions here). Those are C2/C3 gates (spec §14.10/§14.8). Those are
C2/C3 gates (spec §14). C3 regenerates this sample from `src/reporter/agent_report.rs` and asserts an
**empty diff**.

## Known inherited wording

The shipped `views::ScaleOrdering::anchor` prints `1 static cost units (v2)` (plural on a singular cost)
and no thousands separators. C keeps that wording for cross-artifact parity (spec §2 D6); a fix is a
shared-formatter change outside this contract.
