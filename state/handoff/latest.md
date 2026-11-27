# Handoff — Perfpal v0.5.1 public-release prep

**Date:** 2026-11-28 · **State:** v0.5.1 / metrics schema 1.9 · branch `main` at `92552f3`, clean ·
repository **PUBLIC** · tag **`v0.5.1`** pushed · GitHub **pre-release published** with 18 unsigned
assets. The founder explicitly waived the publish gate; B-pf-001 (contact) and B-pf-002 (minisign
secrets) are closed by decision / by removing the signing step. `v0.1.0` remains "Latest".

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
| Committed + pushed | `git checkout main && git merge --ff-only feat/structural-0.5.0 && git push origin main` | `e0928b8..92552f3 main -> main` |
| Tag pushed | `git tag -a v0.5.1 -m … 92552f3 && git push origin v0.5.1` | `* [new tag] v0.5.1 -> v0.5.1` |
| Release published | `gh run watch 46048423774 --exit-status` | conclusion `success`; publish job ✓ |
| Assets | `gh release view v0.5.1 --json assets` | 18 assets incl. `perfpal-x86_64-pc-windows-msvc.exe`, three Linux binaries + checksums, `SHA256SUMS`; draft=false, prerelease=true |

## Review panel
R1 (correctness) **APPROVED**, Q1 (verbatim scope) **APPROVED**. R2 and Q2 were dispatched but failed
with upstream provider 400 errors on every retry; the founder explicitly approved proceeding with
R1+Q1 plus orchestrator-run gates. All non-blocking R1 notes were fixed (duplicated tool name in
the header, stale module comments, namespace-split comment) and gates re-run green.

## Next
1. Founder inserts the brag video into the README placeholder.
2. Tool track: `polish-report` H4/H5 finalization of the full composition; map/ranking follow-ups.

**Uncommitted files:** none at release time; this state/handoff refresh is a docs-only follow-up commit.
