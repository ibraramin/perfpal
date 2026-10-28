# H3 style contract — 2026-10-28 @ `8544706`

**Scope.** One formatting contract across `report.md`, `agent-report.md`, `profile-queue.md` and the
HTML pages, with the Atlas warm-paper design as the visual reference. Implemented as shared helpers,
not per-renderer copies.

## What changed

- **One formatter home.** `src/reporter/format.rs` now owns `MD_ESCAPE_CHARS`, `escape_md`,
  `escape_html`, and the Markdown helpers `md_single_line` / `md_code` / `md_text` / `md_cell`.
  `agent_report.rs` imports them instead of carrying a second family.
- **One HTML shell.** `html_page_head` + the shared token/base CSS are used by `report.html`,
  `findings-detail.html` and `profile-queue.html`; only `report.html` carries a script. The Atlas
  drill pages consume the same palette constants (their own drill markup is unchanged).
- **Palette (single source):** paper `#f4f1ea`, ink `#202b35`, muted `#6f6a61`, rule `#d6d0c4`,
  card `#fffdf8`, accent `#7c2d12`, table head `#ece8df`, zebra `#fbf9f4`.
- **Typography:** sans-serif body; monospace only for code/paths; one heading scale (h1 1.6rem,
  h2 1.15rem with a top rule, h3 1rem, h4 .85rem uppercase muted); tabular numerals; links accent
  with underline on hover; shared table chrome/zebra.
- **Escaping hardening:** Markdown flattening of CR/LF/TAB (`\r`, `\n`, `\t`) and neutralization of
  `<script` / `</script` in code-span values; HTML escapes `&<>"'`. JSON keeps the original bytes.
- **Section/empty-state parity:** `profile-queue.md` gains `## Complete cost-ranked list` and the
  `No functions recorded.` stub to match its HTML sibling; report/detail/profile stubs stay named.
- **JS-off final frame.** The Atlas reveal animations start columns at `opacity:0` with a delay; a
  `<noscript><style>` block now forces `animation:none; opacity:1; transform:none;
  stroke-dashoffset:0` on `#map #atlas-canvas *`, so the static map paints all nodes immediately
  with scripting disabled. The ranked-action title/location/function cells wrap (the rule is scoped
  to `#ranked-actions`), and the profile `rank` header is nowrap/fixed-width.
- **Scripts:** `report.html` keeps exactly three A12 elements (the fixed `REPORT_SCRIPT` for
  pan/zoom + the fixed story script + the inline story JSON). `findings-detail.html`,
  `profile-queue.html` and the Atlas drills carry **zero** scripts, and no page has a filter bar
  (`#filter-bar` / `filter-label` / `filter-clear` are gone; `data-path` / `data-prefix` remain only
  as stable row keys).

## Evidence

| Claim | Evidence | Result |
|---|---|---|
| Heading order per artifact (no skips, no Appendix C) | `tests/style_contract.rs::heading_order_*` | passed |
| Shared warm-paper shell; one inline style; no external assets | `shared_warm_paper_shell_and_no_external_assets` | passed |
| Hostile input neutralized in every MD/HTML renderer | `hostile_input_is_neutralized_everywhere` | passed |
| Empty states are named stubs | `empty_states_are_named_stubs` | passed |
| Markdown tables rectangular; LF + trailing newline | `tables_are_rectangular_and_all_text_is_lf_with_trailing_newline` | passed |
| JS-off final frame + ranked-action wrap | `js_off_map_forces_the_final_frame_and_ranked_actions_wrap` | passed |
| Focused suite | `cargo test --locked --test style_contract` → `style-contract.txt` | 8/8 passed |
| H2 acceptance preserved (5 widths × DPR 1/2 + JS-off, auto + pinned 1440) | `sim/report_check.py` → `report-check.txt` | PASS, exit 0 |
| Size/FCP budget preserved | `sim/report_budget.py` → `budget.txt` | hyperfine 338,050 B/124–132 ms; cJSON 269,313/112–136; 3k 514,788/128–132 — PASS |
| Story gate (304/304 + 3k 44/44, clean tree) | `docs/evidence/atlas-sim/GATE.txt` → `story-gate.txt` | PASS |
| Vision gate | `VISION.md` | **PASS** (JS-off closed; detail wrap verified) |

## Measured deltas vs H2

- `report.html`: 338,100 → 338,050 B on hyperfine (−50 B: filter UI/script removed, noscript
  final-frame rule added), still under the 384 KiB ceiling; cJSON 269,363 → 269,313; 3k
  514,838 → 514,788 (9,500 B / 1.8% headroom).
- `agent-report.md`: +2 B per rendered action row from the `findings-detail.html` anchor rename
  (fixtures +18..+50 B; synthetic 77,710 → 77,763 B), no ranking/wording change; §12 pins updated.
- `report.md`: Appendix C removed, §4 pointer added, §1 link corrected to “all N ranked actions
  (M findings)”; no per-finding rows.
