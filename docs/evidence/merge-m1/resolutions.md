# Merge-train M1 — `feat/js-ts` → `main` (v0.3.0)

- **Date:** 2026-10-27
- **Worktree / branch:** `/home/shafin/Opencode/Perfpal` · `main`
- **Merge:** `git merge --no-ff feat/js-ts` → merge commit `cda9d5f`
- **Version:** `Cargo.toml` / `Cargo.lock` **0.3.0**; metrics schema **1.8**
- **Gate:** js-ts B6 DoD + the full gate on the merged tree — transcript
  [`gate.txt`](./gate.txt). **Final rerun:** every command exits 0. The first run failed and was
  fixed (see below); the transcript keeps both runs.
- **Boundaries:** no `--force`, `-X ours/theirs`, `--no-verify`; no push/tag.

## Conflict resolutions (read both sides, combined features)

| Path | Main side | `feat/js-ts` side | Resolution |
|---|---|---|---|
| `README.md` | Atlas v2 + agent-report sections; schema 1.7 references | five-language intro; 78-rule catalog; schema 1.8 | Union: kept main's Atlas/agent-report sections and the beta limitation, took the five-language/78-rule intro and all 1.8 references; `--version` example is `0.3.0 (metrics schema 1.8)`. |
| `docs/report/output-contract.md` | Atlas page contract; 1.7 schema names | discovery/language-token rule; 1.8 schema names | Union: added the discovery/language-token rule (global directory skips + JS/TS-only skips), all schema names at 1.8; kept the Atlas page contract. |
| `docs/tracks/README.md` | C row: agent-report C1–C4 DONE/merged | B row: B5 committed + merge gate; stale C row | Kept main's C row; took the B B5/merge-gate row and recorded B6 merged into `main` `v0.3.0`; updated the start message and track-state paragraph. |
| `src/reporter/mod.rs` | `pub(crate) HEURISTIC_CATALOG_V1` (agent_report depends on it); 67-rule text; Atlas v2 modules | 78-rule catalog with 11 `PERF-JS-*` entries; JS label wiring | Kept `pub(crate)` visibility and the 78-rule text/entries; both the Atlas v2 modules and the JS `import/require` label wiring are present (7 `pub mod`, one catalog). |
| `state/archive/ledger.jsonl` | 14-line tail (`D-pf-*`, `A-pf-*`, `R-pf-109`) | 37-line tail (`D-tb-*`, `R-tb-*`, `B-tb-*`, `A-tb-*`) | Append-only union: main tail then branch tail. All 724 lines parse as JSON; no cross-tail ID reuse
  (append-only status pairs such as `A-tb-005` pending → done are intentional history). |
| `tests/p6_hardening.rs` | agent-report prune block (`AGENT_REPORT_ARTIFACTS`); 17-artifact comment | six golden trees; 19-artifact set; both Atlas pages | Kept the prune block and six trees; took the 19-artifact set/both-Atlas-page comment; corrected the stale 17 count to 19. |

## Gate failure and integration fix (not a gate weakening)

The first post-merge `cargo test --locked` failed in 5 `agent_report` unit tests and 3 `agent_report`
integration tests. Main's Track C landed after the js-ts base and hardcoded the old 67-rule catalog and
schema 1.7 sample.

- Added **11 curated `PERF-JS-*` fix hints** to `AGENT_FIX_HINTS` (data, ≤20 words, no time/certainty
  claims) and updated the completeness lock to **78** catalog ids.
- Updated `docs/specs/agent-report.md` §8 (hint table + counts) and §12 (synthetic envelope:
  67 → 75 rollup rules, 76,416 → **77,713 B**, 4,207 B headroom).
- Updated `derive_sample.py` to the 78-row hint table.
- The synthetic-envelope count assertions now use the filtered catalog (75 = 78 − three ARCH-002
  aggregates).
- Regenerated the six committed golden trees and the agent-report sample at **0.3.0**. The golden
  delta is `tool.version` (`0.2.0-beta.1` → `0.3.0`) in every tree plus main's agent-report nav links
  restored in the two new JS/TS `report.html` files; finding counts are unchanged
  (15/9/32/29/11/11).

No conflict was resolved by deleting a side; no gate was skipped or relaxed. The first
`cargo test --locked` run was **stopped at the first failure and reported**; the fix is the
merge-train binding rule 5 integration fix (fix the tree, never weaken the gate), after which the
full gate was rerun from the top.

## Verification

| Claim | Command | Result |
|---|---|---|
| Branch merged | `git merge-base --is-ancestor feat/js-ts main` | exit 0 |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | exit 0 (all suites; agent-report 24 unit + 9 integration green) |
| Licenses | `cargo deny check licenses` | exit 0 |
| Semantic gate | semantic clippy + `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0 |
| JS/TS determinism | double scan of `examples-js` / `examples-ts` (masked provenance; charts/schema byte-equal) | both deterministic |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | exit 0 |
| Version | `cargo run --locked --quiet -- --version` | `perfpal 0.3.0 (metrics schema 1.8)` |
| Clean status | `git status --porcelain` | empty (observed; `.playwright-mcp/` ignored by `8f7c5c4`) |
| No tag | `git tag --points-at HEAD` | empty |
| Hot state | `wc -c state/session-state.json` | 5,614 B (≤ 8,192) |
| Ledger | JSON parse of every line | 735 lines valid (724 union + 11 archive records) |
| Plan | `js-ts` status | done |
