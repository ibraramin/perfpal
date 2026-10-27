# H2 review panel — 2026-10-28

Plan `polish-report` H2 (responsive scale + cross-width acceptance). Base
`7c4c59c`; final code `4d401db`; final evidence `1df5286`.

## Round 1 (uncommitted worktree)

| Lane | Agent | Verdict | Blocking findings |
|---|---|---|---|
| R1 | `@reviewer` | CHANGES_REQUESTED | (1) H2 evidence dir absent while docs cited it; (2) story `.page` forced `min-height:100vh`, undoing the H1 embed override and making page height viewport-dependent; (3) final gate not run on the submitted tree. |
| R2 | `@reviewer` | CHANGES_REQUESTED | `figure.pz img{max-width:none}` prevented the legacy Explorer SVGs from downscaling at 1280/1440, contradicting the spec/CHANGELOG claim and clipping/overflowing the chart. |
| Q1 | `@requirements-reviewer` | BLOCKED | H2 evidence bundle missing (docs cite `docs/evidence/polish-report/H2/`); A8 re-baseline asserted without the evidence artifact. |
| Q2 | `@requirements-reviewer` | CHANGES_REQUESTED | `sim/gate.sh` failed only on the expected uncommitted-worktree check; gates stale/unbound; H2 evidence/vision/tag/clean-tree artifacts absent. |

Non-blocking nits fixed: "CSS only" precision (the flag also emits
`data-report-width`); `value_name`; stale comment; §9 heading; timer wording;
full JS-off DPR matrix; pinned-1440 browser acceptance; evidence naming.

## Round 1 fixes

- Story `.page` restored to `min-height:0`.
- `figure.pz img` set to `max-width:100%` (no upscale; legacy figures fit).
- Full JS-off width × DPR matrix in `sim/report_check.py`.
- Docs corrected; final run4 artifacts and H2 evidence committed.

## Round 2

| Lane | Agent | Verdict |
|---|---|---|
| R1 | `@reviewer` | **APPROVED** — all three blockers resolved; evidence numbers match the raw JSON. |
| R2 | `@reviewer` | **APPROVED** — pz fix verified; A12/security/determinism intact. |
| Q1 | `@requirements-reviewer` | **APPROVED** — evidence bundle present; A8 hold satisfied; no H3/H4 creep. |
| Q2 | `@requirements-reviewer` | **APPROVED** — DoD mapped to committed evidence; gate/story gate at `43dfeb9`. |

## Fresh R3 (final-tree confirmation)

**CHANGES_REQUESTED** — a blocking classic-scrollbar overflow:
`#map{margin:0 calc(50% - 50vw)}` exceeded the client width by the scrollbar
width in Chrome/Edge on Windows/Linux (1280 viewport: clientWidth 1265,
scrollWidth 1272). The overlay-scrollbar test environment had hidden it.

## Scrollbar fix (`4d401db`)

- `:root{--page-gutter:clamp(...)}` uses the design-width gutter for body
  padding; `#map{margin:0 calc(-1*var(--page-gutter,1rem))}` makes the band
  equal the body border box (never `100vw`). The fixed-width margin override was
  removed.
- `tests/report_html.rs` locks the CSS (asserts no `50vw`).
- `sim/report_check.py` injects a 15 px root shrink on every interactive context
  and fails if the map exceeds the body; run5 records `stress_map/body` equality
  at all auto/pinned widths.
- Six report goldens re-generated; the detail goldens are unchanged.

## Round 3 (final)

| Lane | Agent | Verdict |
|---|---|---|
| R1 | `@reviewer` | **APPROVED** — scrollbar math correct; test/stress meaningful; auto state unchanged. |
| R2 | `@reviewer` | **APPROVED** — fix confined to report.html; A12/schema/determinism intact. |
| Q2 | `@requirements-reviewer` | **APPROVED** — DoD re-ran at `4d401db`; budget provenance now `4d401db`; no stale numbers. |
| R3 (fresh) | `@reviewer` | **APPROVED** — independently reproduced the old 50vw failure and the fixed no-overflow state with classic 15 px scrollbars. |

Q1's scope approval stands (the fix added no scope; no H3/H4 work).

**Overall: APPROVED.** Code `4d401db`; evidence `1df5286`; full gate
1,111/1,113; story gate 304/304 + 3k 44/44; vision APPROVED-WITH-NITS.

Accepted non-blocking nits: 1440 Explorer fitted labels ~6 px (`.pz` zoom is the
escape); full-width body copy; letterboxed story artwork; inert `Pause` in
JS-off; the internal comment at `mod.rs:1782` describing the removed fixed-width
override; PAGE-HEIGHT raw JSON lives in `/var/tmp`.
