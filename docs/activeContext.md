# Active Context

## Current Task
Session 17 (post-MVP): **MVP RELEASED as v0.1.0.** Repo published to github.com/ibraramin/perfpal (private) after founder chose "reuse + preserve old" for the pre-existing perfpald MCP prototype (preserved on `legacy/perfpald-prototype` at `2f32378`; main lease-protected force-push). `ci.yml` + `release.yml` validated by first-ever real execution through a diagnose→fix→re-tag loop: (1) dtolnay/rust-toolchain needs an explicit `toolchain` input (does NOT auto-read rust-toolchain.toml — action.yml verified) → `696ce67`; (2) P6 authored no publish job → added (`c336cc8`: needs-gated, job-scoped `contents: write`, download-artifact v4.3.0 pinned, in-CI sha256 re-verification); (3) `gh release create` without git context → `GH_REPO` env (`02e1e77`); (4) reviewer MINORs: re-run-safe publish (`view`→`upload --clobber`, else `create`) + header accuracy (`f368133`). GitHub Release **"perfpal v0.1.0"** live: 10 assets (5 binaries + 5 .sha256), all checksums OK, published musl binary smoke-tested (exit 0, 15 findings, full report tree). Tag re-pointed per fix (GitHub runs the workflow from the tagged commit) → final target `f368133` = main tip. Cross-run binary checksums: musl ×2 + aarch64-darwin byte-identical; x86_64-darwin + windows-msvc differ (MSVC timestamps / darwin-x64 cause unknown) — documented, not a spec violation.

## Recent Decisions
- Private visibility (no LICENSE file yet — codify terms before going public).
- Publish job re-run-safe per reviewer (partial-release recovery path validated by the final re-tag run 33568277020).
- Determinism scope reaffirmed: charts/metrics artifacts only; release binaries never claimed bit-reproducible.

## In Progress
- Nothing — task boundary reached; awaiting ritual-commit approval.

## Next
- Founder approves ritual commit → `chore: session 17 — MVP released as v0.1.0; ci/release pipelines validated on GitHub; state + docs + changelog ritual` (files in `state/handoff/latest.md` §Uncommitted Files; exclude `.opencode/memory/**`).
- Fresh session (`/new`): founder picks the next milestone — semantic resolver tier (ADR-0001), runtime-profile ingestion adapters (CONFIRMED-1), housekeeping batch (docs-writer pass, STR-003 notes, stats.distributions proposal, LICENSE), C++/Python backend, or release follow-ups (Node-24 pins, reproducible builds, toolchain drift guard). Signposts: **MVP RELEASED as v0.1.0 (tag f368133)**; read state files first.

## Blockers
- None.
