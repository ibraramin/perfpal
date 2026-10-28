# H3 style contract — 2026-10-28 @ `c5ded97`

**Scope.** One formatting contract across `report.md`, `agent-report.md`, `profile-queue.md` and the
HTML pages, with the Atlas warm-paper design as the visual reference. Implemented as shared helpers,
not per-renderer copies.

## What changed

- **One formatter home.** `src/reporter/format.rs` now owns `MD_ESCAPE_CHARS`, `escape_md`,
  `escape_html`, and the Markdown helpers `md_single_line` / `md_code` / `md_text` / `md_cell`.
  `agent_report.rs` imports them instead of carrying a second family.
- **One HTML shell.** `html_page_head` + the shared token/base CSS are used by `report.html`,
  `findings-detail.html` and `profile-queue.html`. The Atlas drill pages consume the same palette
  constants (their own drill markup is unchanged).
- **Palette (single source):** paper `#f4f1ea`, ink `#202b35`, muted `#6f6a61`, rule `#d6d0c4`,
  card `#fffdf8`, accent `#7c2d12`, table head `#ece8df`, zebra `#fbf9f4`.
- **Typography:** sans-serif body; monospace only for code/paths; one heading scale (h1 1.6rem,
  h2 1.15rem with a top rule, h3 1rem, h4 .85rem uppercase muted); tabular numerals; links accent
  with underline on hover; shared table chrome/zebra.
- **Escaping hardening:** Markdown flattening of CR/LF/TAB (`\r`, `\n`, `\t`) and neutralization of
  `<script` / `</script` in code-span values; HTML escapes `&<>"'`. JSON keeps the original bytes.
- **Section/empty-state parity:** `profile-queue.md` gains `## Complete cost-ranked list` and the
  `No functions recorded.` stub to match its HTML sibling; report/detail/profile stubs stay named.
- **Scripts:** `report.html` keeps exactly three A12 elements; `findings-detail.html` keeps exactly
  one fixed `REPORT_SCRIPT` (its filter bar stays live); `profile-queue.html` keeps the same fixed
  script; the Atlas drills stay script-free.

## Evidence

| Claim | Evidence | Result |
|---|---|---|
| Heading order per artifact (no skips, no Appendix C) | `tests/style_contract.rs::heading_order_*` | passed |
| Shared warm-paper shell; one inline style; no external assets | `shared_warm_paper_shell_and_no_external_assets` | passed |
| Hostile input neutralized in every MD/HTML renderer | `hostile_input_is_neutralized_everywhere` | passed |
| Empty states are named stubs | `empty_states_are_named_stubs` | passed |
| Markdown tables rectangular; LF + trailing newline | `tables_are_rectangular_and_all_text_is_lf_with_trailing_newline` | passed |
| Focused suite | `cargo test --locked --test style_contract` → `style-contract.txt` | 7/7 passed |
| H2 acceptance preserved (5 widths × DPR 1/2 + JS-off, auto + pinned 1440) | `sim/report_check.py` → `report-check.txt` | PASS, exit 0 |
| Size/FCP budget preserved | `sim/report_budget.py` → `budget.txt` | hyperfine 339,271 B/128–132 ms; cJSON 270,534/144–152; 3k 516,009/124–136 — PASS |
| Story gate (304/304 + 3k 44/44, clean tree) | `docs/evidence/atlas-sim/GATE.txt` → `story-gate.txt` | PASS |
| Vision gate | `VISION.md` | see verdict |

## Measured deltas vs H2

- `report.html`: 338,100 → 339,271 B on hyperfine (+1,171 B of shared CSS), still under the
  384 KiB ceiling; cJSON 269,363 → 270,534; 3k 514,838 → 516,009 (8,279 B / 1.6% headroom).
- `agent-report.md`: +2 B per rendered action row from the `findings-detail.html` anchor rename
  (fixtures +18..+50 B; synthetic 77,710 → 77,763 B), no ranking/wording change; §12 pins updated.
- `report.md`: Appendix C removed, §4 pointer added, §1 link corrected to “all N ranked actions
  (M findings)”; no per-finding rows.
