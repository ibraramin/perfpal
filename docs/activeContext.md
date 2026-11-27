# Active Context

## Current Task
**v0.5.1 public-release prep (2026-11-27).** The founder explicitly waived the publish gate: the
repository is now **PUBLIC**, and the release will be unsigned (checksum-only assets, no minisign
secrets). Version is **0.5.1**, metrics schema stays **1.9**. The CLI's `report.html` now emits the
**Atlas sim only** via `render_sim_report_at_width` (two scripts: inline JSON + fixed player); the
previous full composition is preserved in `render_html_with_story_at_width` for the audit contract
and a later finalization. `sim/sim.js` gained an SVG-namespace fallback so the player works when the
page has no static map SVG. All other report artifacts are unchanged.

CV-facing README leads with hero → brag-video placeholder (founder-owned) → showcase GIF
(`docs/assets/perfpal-demo.gif`, real CLI scan + real player frames) → technical contract.
EULA/PRIVACY contacts are real; `release.yml` publishes 18 unsigned assets; CHANGELOG `[0.5.1]`.

Gates: fmt / clippy / `cargo test` / `cargo deny` / semantic feature gate all green; 6 golden trees
regenerated; rendered sim page inspected via headless Chromium screenshots. **No commit, push, tag or
GitHub release yet** — commit and tag are the next step after the review panel.

## Recent Decisions
- **Founder waiver (2026-11-27):** publish gate B-pf-001/B-pf-002 waived; repo public; no minisign
  signature at v0.5.1; checksum-only verification.
- **D-pf-281:** v0.5.1 sim-only main report; full composition kept as a library function for later
  finalization; `sim.js` namespace fix; version/sample/golden re-baseline.
- Earlier: D-pf-280 (H3 closed), D-pf-279 (H2), D-pf-278 (H1), ADR-0014 (in-report story), ADR-0015.

## In Progress
- Parallel review panel on the uncommitted v0.5.1 change set; then commit/push/tag/release.

## Next
1. Reconcile panel, commit, fast-forward `main`, push, tag `v0.5.1`, verify the release workflow.
2. Founder inserts the launch film into the README placeholder.
3. Later: `polish-report` H4/H5 finalization of the full composition; map/ranking follow-ups.

## Blockers
- None for the release. The tag/release is pending only the commit+push step and review-panel sign-off.
