# Handoff — Latest

## Objective
Post-MVP session (session 17): founder chose **"Tag v0.1.0 + remote"** — publish the repo, push main, tag `v0.1.0`, and exercise `ci.yml` + `release.yml` for real (they had never executed on GitHub). DONE: repo published (private), both pipelines validated by execution through a diagnose→fix→re-tag loop, GitHub Release **`perfpal v0.1.0`** published with 10 verified assets. Dual review reconciled (@reviewer SHIP-WITH-FIXES → both MINORs applied; @requirements-reviewer items closed with direct evidence, zero scope creep). Ritual commit pending founder approval.

## Phase
POST-MVP — MVP **released** as v0.1.0 (tag `f368133`). Plan `mvp-implementation` remains COMPLETE.

## Key Decisions
- **Repo conflict (founder: "reuse + preserve old"):** `ibraramin/perfpal` already existed (private) holding an old `perfpald` MCP prototype (3 commits, Jun 2026). Prototype preserved on branch `legacy/perfpald-prototype` (at `2f32378`) BEFORE force-push; main force-pushed lease-protected (`2f32378...b79b0d1`). Visibility = **private** (no LICENSE file yet — codify terms before going public).
- **First-execution finding #1 (ci.yml):** `dtolnay/rust-toolchain@6c977a6` requires an explicit `toolchain` input — it does **NOT** auto-read `rust-toolchain.toml` (reviewer verified in action.yml source: `required: true`, no fallback). Fixed by `toolchain: "1.92.0"` at both uses (commit `696ce67`). CI then GREEN (check + license-audit, run 33566525470).
- **First-execution finding #2 (release.yml):** P6 authored no publish job — first tag run built all 5 targets but left assets as workflow-artifacts-only. `publish` job added (commit `c336cc8`): `needs: [build-unix, build-windows]`, job-scoped `contents: write`, `download-artifact` v4.3.0 SHA `d3f86a1…` (live-verified) with `merge-multiple`, in-CI `sha256sum -c` before publishing.
- **First-execution finding #3:** `gh release create` died `fatal: not a git repository` (publish job has no checkout → no git context). Fixed with `GH_REPO: ${{ github.repository }}` (commit `02e1e77`). Run 33567514518 fully GREEN → release published.
- **Review fixes (commit `f368133`):** stale "least privilege" header corrected; publish made re-run-safe (`gh release view` → `upload --clobber`, else `create`; checksum gate in both paths). Final re-tag validated the new re-run branch for real (run 33568277020 green, 10 assets re-verified).
- **Tag re-point discipline:** GitHub runs the workflow file from the commit the tag points at → each fix required re-pointing `v0.1.0` (`696ce67→c336cc8→02e1e77→f368133`).
- **Binary reproducibility observed (not a spec violation):** musl x86_64/aarch64 + aarch64-darwin byte-IDENTICAL across independent runs; x86_64-darwin + windows-msvc DIFFER (MSVC embeds link timestamps unless `/Brepro`; darwin-x64 cause uninvestigated). Perfpal's determinism guarantees cover charts/metrics artifacts, never release binaries.
- **Published binary smoke test:** musl release binary scanned `tests/fixtures/e2e` → exit 0, 15 findings, full report tree — matches the P6 local smoke test exactly.

## Reviews
- @reviewer: **SHIP-WITH-FIXES** (zero Critical/MAJOR; 2 MINOR applied in `f368133`, 2 NITs accepted: toolchain pin mirrored 5×, debug `ls` kept). Independently verified: dtolnay action source, pin SHAs, fail-fast→no-partial-publish job graph, token scoping, quoted tag expansion, Windows `.sha256` format compatibility.
- @requirements-reviewer: **CHANGES_REQUESTED** solely on sandbox-unverifiable runtime items — all closed with direct evidence: `private=true`, exact tag message, tag target `f368133`, legacy branch at `2f32378`, release with 10 assets. **Zero scope creep.**

## Risks
- Release binaries x86_64-darwin + windows-msvc not byte-reproducible across runs (post-MVP: `/Brepro`, darwin-x64 investigation).
- Node.js-20 deprecation warnings on all pinned actions (forced to Node 24) — informational; bump pins post-MVP.
- Toolchain `1.92.0` mirrored in 5 places with no CI drift guard (reviewer NIT, accepted).
- Tag was re-pointed 3× during validation — early fetchers hold a stale tag (harmless, no consumers; avoid once consumers exist).
- No LICENSE file while repo is private — blocker only if visibility changes.

## Blockers
- None.

## Next Actions
1. Founder approves ritual commit → commit state/handoff/activeContext/CHANGELOG updates only (the 4 workflow commits `696ce67`/`c336cc8`/`02e1e77`/`f368133` are already pushed). Suggested message: `chore: session 17 — MVP released as v0.1.0; ci/release pipelines validated on GitHub; state + docs + changelog ritual`.
2. Post-MVP paths (founder picks): semantic resolver tier (ADR-0001); runtime-profile ingestion adapters (CONFIRMED-1); housekeeping batch (docs-writer README/docs, STR-003 catalog notes, stats.distributions proposal, LICENSE); C++/Python backend; release follow-ups (Node-24 pins, reproducible builds, toolchain drift guard).
3. Fresh session (`/new`) signposts: **MVP RELEASED as v0.1.0** (tag `f368133`, github.com/ibraramin/perfpal private); read state/session-state.json + state/handoff/latest.md + docs/activeContext.md FIRST.

## Uncommitted Files (pending founder-approved ritual commit)
- Modified: `state/session-state.json` (append-only: 6 decisions, 6 risks, 3 actions, 5 artifacts, phase), `state/handoff/latest.md` (regenerated), `docs/activeContext.md`, `CHANGELOG.md` ([Unreleased] → [0.1.0] - 2026-09-01 + new Unreleased section)
- EXCLUDE from commit: `.opencode/memory/**` (session noise)
