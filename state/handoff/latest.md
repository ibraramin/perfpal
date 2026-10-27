# Handoff — polish-report H2 CLOSED; H3 next (`main` @ close commit; 2026-10-28)

**Objective:** complete H2 of `polish-report` — responsive scale from 1280 to 3072 CSS px with a
public layout-only `--report-width` hint, cross-width/DPR acceptance, scrollbar-safe full bleed,
and an evidence-backed A8/page-height re-baseline, while keeping the H1 Atlas/parity/story
contract, byte-determinism and A12.
**Status:** **H2 CLOSED**. Panel **R1/R2/Q1/Q2 + fresh R3 APPROVED** (round 3 after the
classic-scrollbar fix). Code **4d401db**; evidence **1df5286**; schema **1.8**; no new deps.
**No push, no tag.**
**Plan/phase:** `polish-report` H2 · next: H3 artifact consolidation/formatting (plan line 29).

## Delivered

- **Responsive scale:** fluid body/page with `--page-gutter`, full-bleed `#map` anchored to the
  body (not `100vw`, so classic scrollbars cannot overflow), canvas `width:100%` with preserved
  viewBox, Atlas charts/figures fluid, legacy Explorer figures downscaled but never upscaled, and
  a `clamp()` type ramp anchored at 1440 (1.5× ceiling at 3072). Story `.page`/`.canvas-shell`
  full-width, content-driven (`min-height:0`).
- **Public `--report-width <auto|1280|1440|1920|2560|3072>`** (default `auto`): layout-only; only
  `report.html` (CSS + `data-report-width` marker) changes; values pinned in CLI help/README/
  modules/output-contract; invalid values exit 1.
- **Acceptance:** `sim/report_check.py` covers 5 widths × DPR 1/2 interactive + JS-off (auto and
  pinned 1440), canvas fill, no clipped labels, story final, zero network/console errors, and a
  15 px classic-scrollbar stress (`map == body`).
- **Budget:** `sim/report_budget.py --widths` covers 1440/3072 for hyperfine, cJSON and 3k.
- **Goldens:** six report.html regenerated; no other golden artifact changed.
- **Docs:** spec §6/§9 (resolved decisions, A8 re-baseline), output-contract §2, README CLI,
  modules §8.1, CHANGELOG; evidence under `docs/evidence/polish-report/H2/`.

## Verification (claim → evidence → result)

| Claim | Evidence | Result |
|---|---|---|
| Full gate | `docs/evidence/polish-report/H2/gate.txt` @4d401db | fmt/clippy/test **1,111**/deny/semantic clippy/semantic test **1,113** all EXIT=0 |
| Story gate | `docs/evidence/atlas-sim/GATE.txt` @4d401db | 304/304, 3k 44/44, `worktree_dirty_zero`, verdict **PASS** |
| Report acceptance | `docs/evidence/polish-report/H2/report-check.txt` @4d401db | 5×2 interactive + 5×2 JS-off, auto + pinned; no hscroll; canvas fill; no clipped labels; story final; 19/19 JS-off; stress map==body |
| Budget | `docs/evidence/polish-report/H2/budget.txt` @4d401db | hyperfine 338,100 B/120–128 ms; cJSON 269,363/128–132; 3k 514,838/124–132 — PASS |
| A8/page height | `docs/evidence/polish-report/H2/PAGE-HEIGHT.md` @4d401db | doc 16,927 @1440 / 21,241 @3072; story 1,029/1,875; answer bottom ≤436 |
| Vision | `docs/evidence/polish-report/H2/VISION.md` @4d401db | 1440/3072/JS-off **APPROVED-WITH-NITS**; H1 nits resolved |
| Panel | `docs/evidence/polish-report/H2/PANEL.md` | R1/R2/Q1/Q2/R3 **APPROVED** |
| Clean/tag | `docs/evidence/polish-report/H2/close.txt` | tree clean; tag empty; no push/tag |

## Panel / close notes

- R3 caught a real classic-scrollbar overflow that the overlay-scrollbar test browser hid:
  `#map{margin:0 calc(50% - 50vw)}` exceeded the client width by 15 px. Fixed at `4d401db` with a
  body-anchored gutter, a static regression test, and a runtime 15 px-shrink stress; R3 then
  independently reproduced the old failure and the fixed state.
- Accepted non-blocking nits: 1440 Explorer fitted labels ~6 px (`.pz` zoom is the escape);
  full-width body copy; letterboxed story artwork; inert `Pause` in JS-off; PAGE-HEIGHT raw JSON is
  under `/var/tmp` (the committed Markdown table is the evidence).

## Next

1. **H3 — artifact consolidation + formatting** (separate session; plan line 29): role matrix,
   `report.md` Appendix C duplication, one canonical human detail rendering, style contract across
   Markdown/HTML artifacts; update output-contract §2 and goldens; no schema change.
2. **H4** `report.html` final composition (`#explorer` → `explorer.html`, order, A8 re-baseline);
   **H5** finalize (full gate, story gate, docs, state, EOL).
3. **Post-beta map/ranking:** F treemap (D-pf-251), memchr fold (D-pf-256), map-build/A1b recall.
4. **Publish gate (founder-owned):** B-pf-001 contact + B-pf-002 minisign secrets; no push/tag until
   cleared.

## Uncommitted files / state

None at the H2 close; code `4d401db`; evidence `1df5286`; **no push/tag**.
