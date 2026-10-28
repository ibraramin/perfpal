# H3 determinism and golden regeneration — 2026-10-28 @ final H3 code (see `gate.txt` header)

## Byte-determinism

| Claim | Evidence | Result |
|---|---|---|
| Double-run artifacts byte-identical modulo provenance (C pipeline) | `tests/p4_determinism.rs` | passed |
| Double-run all JS artifacts | `tests/determinism_js.rs` | passed |
| Double-run all TS artifacts | `tests/determinism_ts.rs` | passed |
| Detail artifacts identical across provenance changes (HTML) | `tests/findings_detail_split.rs` | passed |
| Full pipeline double-run + artifact set (incl. `--report-width` purity) | `tests/pipeline_e2e.rs` | passed |
| Focused determinism batch | `determinism.txt` | 24/24 passed |
| No `preserve_order` / no set iteration | unchanged; `cargo test --locked` | passed |

`findings-detail.html` carries no provenance by construction (schema/file/finding counts only), so
its double-run check is unmasked; `report.md` / `profile-queue.md` keep the existing provenance mask.
`profile-queue.html` and the drill pages are byte-identical unmasked as before.

## Golden regeneration

- Six committed trees (`report`, `report-mixed`, `report-python`, `report-rust`, `report-js`,
  `report-ts`) were regenerated with
  `PERFPAL_REGEN_GOLDEN=1 cargo test --locked --test p6_hardening -- --nocapture`, then compared by
  the normal `p6_hardening` run.
- `GOLDEN_ARTIFACTS` is now **18** (was 19): `findings-detail.md` removed from every tree and from
  the comparator; `findings-detail.html`, `profile-queue.*`, `report.md`, and `report.html` were
  refreshed by the consolidation/shell changes. Charts, `graph.dot`, and the bundled schema are
  byte-identical (no chart change).
- Expected content changes: `report.md` loses Appendix C and gains the §4 pointer; §1 says
  “all 14 ranked actions (15 findings)” instead of the misleading “all 14 findings”;
  `findings-detail.html` / `profile-queue.html` carry the shared shell and are script-free;
  `profile-queue.md` gains the `## Complete cost-ranked list` heading.
- `docs/evidence/agent-report/sample/agent-report.sample.md` was refreshed for the
  `findings-detail.html` anchor rename; the sample byte-parity test passes (`tests/agent_report.rs`).

## Full gate at the committed tree

`gate.txt` — `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked`,
`cargo deny check licenses`, semantic clippy, and `PERFPAL_SEMANTIC_TESTS=1 cargo test`:
**all six exit 0**; 1,125 default + 1,127 semantic = **2,252 passed / 0 failed / 2 ignored**.
This gate runs at the final H3 code commit (`fbda69b`, see `gate.txt` header) and re-runs the
determinism suites listed above at that commit.
Story gate `story-gate.txt`: **304/304 + 3k 44/44, verdict PASS, worktree_dirty_zero**.
