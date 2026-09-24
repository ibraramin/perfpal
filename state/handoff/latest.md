# Handoff — Latest

## Objective
Session 34: plan `beta-release` **B6 — unblocked half COMPLETE** (the founder-only half deferred). Made the GitHub **pre-release publish draft-safe** and `--prerelease --latest=false`; recorded tag discipline; handed the 19-asset inventory to the website milestone; pre-flight smoke-tested the `0.2.0-beta.1` musl binary. **NOT yet committed** (founder approval pending).

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · **`beta-release` — B0+B1 ✅ (`ce9b4cd`); B2+B3 ✅ (`daacdd5`); B4+B5 ✅ (`a51abce`); B6 🟡 (unblocked half done, founder-gated half remaining).**

## Version snapshot
- **Published: `v0.1.0`** (tag, target `f368133`) — the only published release.
- **`Cargo.toml` = `0.2.0-beta.1`** (unreleased); **`metrics.json` schema = 1.2**.
- `main` unreleased: … + B0+B1 (`ce9b4cd`) + B2+B3 (`daacdd5`) + B4+B5 (`a51abce`) + **B6 (uncommitted)**. **Not pushed.**

## What B6 changed (uncommitted)
- **`.github/workflows/release.yml`** — publish job rewritten to **create-as-DRAFT (only when the tag has no release) → upload assets ONLY while still a draft → `gh release edit --draft=false --prerelease --latest=false` → fail-closed convergence assert**. The `--clobber` upload is gated on `isDraft` so a re-run can never rewrite live published assets. Header documents the Distribution policy + **tag discipline**.
- **`docs/roadmap.md`** — status snapshot refreshed (session 23 → 34); M2 Distribution + Tag-discipline bullets.
- **`docs/website/scope-and-content.md` §8.1** — release asset inventory (19 assets) + verification + feature split + smoke-test evidence.
- **`EULA.md` / `PRIVACY.md`** — contact-placeholder comment reworded to record the deliberate deferral (placeholder line unchanged).
- **`CHANGELOG.md`** — `[Unreleased]` B6 Changed bullet.

## Key Decisions (B6)
- **Scope split (founder):** execute only the unblocked half; the contact address ("will buy an address later, so keep it blank for now") and the `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` secrets are founder-only, so the actual tag/publish waits.
- **Publish pattern (founder-approved):** draft-then-flip (recoverable) over one-shot create. gh semantics verified by @librarian (draft-aware `view`/`upload`; `edit --draft=false --prerelease --latest=false` valid; only `--prerelease --latest=true` is server-rejected 422).
- **Contact placeholder deferred**, not resolved (founder decision); governing law = Bangladesh (already confirmed).

## Evidence
- **Pre-flight smoke test** (locally built `x86_64-unknown-linux-musl`, exact workflow RUSTFLAGS): `examples/` → exit 0, 13 files, **83 findings (16/25/42)**, 23 `PERF-C-*`; `examples-rust/` → exit 0, 14 files, **41 findings (17/11/13)**, 23 `PERF-RS-*`, zero `PERF-C-*`; full report tree each.
- **Gates:** `cargo fmt --check` ✅ · `cargo clippy --all-targets --locked -- -D warnings` ✅ · `cargo test --locked` **393 pass / 0 fail** ✅ · YAML ✅ · actionlint v1.7.12 ✅ · all bash `run:` blocks `bash -n` ✅.
- **Dual review:** @requirements-reviewer CHANGES_REQUESTED (all 4 clauses met or founder-deferred; only EOL-bookkeeping gaps, now closed) · @reviewer rev-1 SHIP-WITH-FIXES (0 Critical, 1 Major) → fix wave → **rev-2 APPROVE** (0 Critical / 0 Major; two non-blocking minors applied: atomic single-read convergence assert + already-published-tag re-point note).

## Risks
- **The new draft-safe publish path has NEVER run on GitHub** — the first `v0.2.0-beta.1` tag validates it (plus B2/B3 sign+verify and B4/B5 staging/notes). Windows `pwsh` leg unverified locally.
- **Follow-up (@reviewer n1, not actioned):** `minisign.pub` is not attached to the release and the repo is private, so non-collaborators cannot independently verify `SHA256SUMS.minisig`.
- Carried: R5 cross-language ARCH-003 substitution still needs founder sign-off.

## Blockers
- **Founder must** (1) supply the EULA/privacy **contact address** (to be purchased later), and (2) set the two GitHub **secrets** `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` (must match `minisign.pub`) — both before any tag.

## Next Actions
1. **Approve the B6 ritual commit** (change set below); `main` is also unpushed.
2. **Next session (`/new`) = plan `beta-release` B6 finish** (founder-gated): replace the contact placeholder → confirm secrets → commit → ensure `v0.2.0-beta.1` points at the commit containing the current `release.yml` → tag + push → smoke-test the **PUBLISHED** musl binary vs `examples/` + `examples-rust/` (expect 83 / 41) → finalize the §8.1 inventory with the real release URL. Read state files + `readPlan beta-release` FIRST.

## B6 Change Set (uncommitted, 5 files)
- **Modified:** `.github/workflows/release.yml`, `docs/roadmap.md`, `docs/website/scope-and-content.md`, `EULA.md`, `PRIVACY.md`, `CHANGELOG.md` (**6** incl. CHANGELOG).
- **State (EOL):** `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Commit message:** `feat(beta-release): B6 draft-safe pre-release publish (--prerelease --latest=false) + tag discipline + 19-asset website inventory; contact placeholder deferred; dual review APPROVED`.
- **Exclude** `.opencode/**` from the commit.
- **Gate status:** fmt ✅ · clippy ✅ · **393 tests / 0 fail** ✅ · YAML ✅ · actionlint v1.7.12 ✅ · musl smoke test 83/41 ✅.
