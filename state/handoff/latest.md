# Handoff — Perfpal v0.5.1 public-release prep

**Date:** 2026-11-27 · **State:** v0.5.1 / metrics schema 1.9 · branch `feat/structural-0.5.0`
(uncommitted work) · repository **PUBLIC** (`gh repo view` confirmed). The founder explicitly waived
the publish gate for this release; B-pf-001 (contact) and B-pf-002 (minisign secrets) are closed by
decision / by removing the signing step. **No commit, push, tag or GitHub release has been made yet.**

## What changed
- `src/reporter/mod.rs`: new `render_sim_report_at_width` — the CLI's `report.html` is now the Atlas
  sim only (shell + scan line + artifact nav + player + footer, exactly two scripts). The full
  map/charts/Explorer/Findings composition is preserved unchanged in
  `render_html_with_story_at_width` for the audit contract and later finalization.
- `sim/sim.js`: fallback SVG namespace constant so the player works on a page with no static map
  SVG (literal `http://` avoided so the fixed-script safety guard still passes).
- `src/cli/mod.rs`, `src/reporter/atlas_brief.rs`, `src/reporter/atlas_story_render.rs`: CLI switch,
  drill/back links retargeted to `report.html#atlas-story`, JS-off narrative reworded.
- Version 0.5.1 (Cargo.toml/lock), samples/goldens re-baselined; EULA/PRIVACY contact set to
  `ibrarshafin2002@gmail.com`; `release.yml` now ships checksum-only assets (no minisign);
  release notes updated.
- CV-facing README: hero → brag-video placeholder (founder-owned) → showcase GIF → technical
  contract. `docs/assets/perfpal-demo.gif` is 960×540, ~369 KB, built from a real scan of
  `hyperfine` and real browser frames of the sim player at beats 1/4/7/10/13/16.
- CHANGELOG `[0.5.1]` entry; session-state compacted and updated (7,522 B).

## Verification (claim → evidence)
| Claim | Command / artifact | Observed |
|---|---|---|
| Full suite green | `cargo test --offline --no-fail-fast` | EXIT 0; 74 `test result: ok` lines |
| Format clean | `cargo fmt --all -- --check` | EXIT 0 |
| Lint clean | `cargo clippy --locked --all-targets -- -D warnings` | EXIT 0 |
| Licenses clean | `cargo deny check licenses` | `licenses ok` |
| Feature gate | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings`; `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | EXIT 0 |
| Byte determinism | `grep preserve_order Cargo.lock` | no match |
| Golden parity | `PERFPAL_REGEN_GOLDEN=1 cargo test --offline --test p6_hardening regenerate_goldens` | ok; 6 trees regenerated |
| Real CLI scan | `perfpal ../perfpal-test/hyperfine -o /var/tmp/opencode/hyperfine-report` | exit 0; 53 files / 348 findings |
| Sim renders correctly | headless Chromium at `file:///var/tmp/opencode/hyperfine-report/report.html`, screenshot inspected | no page errors; 19 SVG nodes; 16 beats; player visible — `/var/tmp/opencode/shots/hyperfine-sim.png`, `demo-sim-final.png` |
| GIF | `docs/assets/perfpal-demo.gif`, 8 frames, frames inspected | 960×540, ~369 KB, real CLI + beats |
| Repo public | `gh repo view ibraramin/perfpal --json visibility` | `PUBLIC` |

## Review panel
R1 (correctness) **APPROVED**, Q1 (verbatim scope) **APPROVED**. R2 and Q2 were dispatched but failed
with upstream provider 400 errors on every retry; the founder explicitly approved proceeding with
R1+Q1 plus orchestrator-run gates. All non-blocking R1 notes were fixed (duplicated tool name in
the header, stale module comments, namespace-split comment) and gates re-run green.

## Next
1. Commit on the current branch.
2. Fast-forward `main`, push `main`, create/push tag `v0.5.1`; confirm `release.yml` publishes the
   unsigned 18-asset set for Linux + Windows (and the rest of the six-target matrix).
3. Founder inserts the brag video into the README placeholder.
4. Tool track: `polish-report` H4/H5 finalization of the full composition; map/ranking follow-ups.

**Uncommitted files:** all items in `git status` (58 modified + `docs/assets/`). No push, no tag.
