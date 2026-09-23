# Handoff — Latest

## Objective
Session 33: plan `beta-release` **B4 (EULA + noncommercial evaluation notice + no-telemetry statement) + B5 (code-signing deferred; unsigned-beta caveat documented)**. **Complete — NOT yet committed** (founder approval pending). Residual gates: 393 tests / 0 fail, fmt/clippy/YAML/actionlint clean. Dual review: **@reviewer CHANGES_REQUESTED** (0 Critical; 1 in-scope Major M1) → **fix wave applied**; **@requirements-reviewer APPROVED**.

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ (R0–R6) · **`beta-release` — B0+B1 ✅ (`ce9b4cd`); B2+B3 ✅ (`daacdd5`); B4+B5 🟡 (uncommitted); B6 remaining.**

## Version snapshot
- **Released: `v0.1.0`** (tag, target `f368133`).
- **`Cargo.toml` = `0.2.0-beta.1`**; **`metrics.json` schema = 1.2 — unchanged.** `Cargo.toml`/`Cargo.lock`/`deny.toml`/`ci.yml`/`LICENSE` **untouched** this session.
- `main` unreleased: … + B0+B1 (`ce9b4cd`) + B2+B3 (`daacdd5`) + **B4+B5 (uncommitted)**. **Not pushed.**

## Key Decisions (B4+B5)
- **Evaluation notice in `--help` ONLY** (clap `after_help` footer); **`--version` kept byte-exact** (`perfpal 0.2.0-beta.1 (metrics schema 1.2)`, modules.md §8.1) — founder override of the plan's literal "--version/--help".
- **`EULA.md` + `PRIVACY.md` at repo root, BOTH attached to the release** alongside `LICENSE` (release bundle now 19 assets).
- **Release notes:** committed `docs/release-notes-header.md` is **prepended** via `--notes` to `--generate-notes` (verified gh semantics); the publish step guards it with `test -s` + `set -euo pipefail`.
- **Signing deferred:** Windows Authenticode + macOS notarization deferred (HSM/cost); **Azure Trusted Signing NOT wired**; unsigned-beta caveat documented in README + release-notes header. No ADR.
- **Governing law:** founder deferred → defaulted to **Bangladesh** with an in-file HTML-comment flag. **Contact address** likewise left as an **explicit in-file pre-tag deferred placeholder** (reviewer M1).

## Risks
- **B4 pre-tag blocker:** `EULA.md`/`PRIVACY.md` still contain `Contact: [licensor contact email]` — must be replaced with a real address before the beta tag (founder chose to resolve it in B6). Governing law is **confirmed = Bangladesh** (session 33).
- **B6 forward items (@reviewer M2/M3):** `gh release create` still lacks `--prerelease --latest=false`; the re-run branch cannot recover an unpublished draft. A `# TODO(beta-release B6)` marker is in `release.yml`.
- The new publish staging + notes prepend have **never run on GitHub** — the first `v*` tag validates them (plus the pre-existing B2/B3 sign/verify path). Windows `pwsh` leg unverified locally.
- Carried: repo secrets `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` must exist and match `minisign.pub`; R5 cross-language ARCH-003 substitution still needs founder sign-off; stale `docs/roadmap.md` "Status snapshot (session 23)" block.

## Blockers
- **Founder must** (1) supply a real contact address for `EULA.md`/`PRIVACY.md`, (2) set the two GitHub secrets — both before any tag.

## Next Actions
1. **Approve the B4+B5 ritual commit** (see change set below); `main` is also unpushed.
2. Supply the **contact address** (governing law is confirmed = Bangladesh).
3. **Next session (`/new`) = plan `beta-release` B6** — add `--prerelease --latest=false` + make draft-recovery re-run safe, confirm tag discipline (tag points at the workflow-running commit), confirm secrets, smoke-test the published musl binary vs `examples/` + `examples-rust/`, hand the asset inventory to the website milestone. Read state files + `readPlan beta-release` FIRST.

## B4+B5 Change Set (uncommitted, 9 files)
- **New:** `EULA.md` (root; supplements PolyForm Strict `LICENSE`), `PRIVACY.md` (root; no-telemetry), `docs/release-notes-header.md` (eval + unsigned-beta + no-telemetry caveat).
- **Modified:** `src/cli/mod.rs` (`HELP_FOOTER` + `after_help`; rendered-level tests locking `--version` byte-exactness), `.github/workflows/release.yml` (stage `EULA.md`/`PRIVACY.md`/`LICENSE` into `dist/`; prepend notes header with `test -s` guard; accurate re-run comment + B6 TODO), `README.md` (19-asset inventory, "Beta status, code signing, and privacy" section, beta banner + anchor, manifest-coverage wording), `CHANGELOG.md` (B4/B5 bullets + realigned beta banner), `docs/roadmap.md` (M2 decision record), `docs/architecture/modules.md` (§8.1 `--help` footer note).
- **State (EOL):** `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Commit message:** `feat(beta-release): B4 EULA + PRIVACY + --help evaluation notice + release-bundle legal docs; B5 code-signing deferral + unsigned-beta caveat; dual review APPROVED`.
- **Exclude** `.opencode/**` from the commit.
- **Gate status:** `cargo fmt --check` ✅ · `cargo clippy --all-targets --locked -- -D warnings` ✅ · `cargo test --locked` **393 pass / 0 fail** ✅ · YAML parse ✅ · actionlint v1.7.12 ✅ · `--version` byte-exact ✅ · `--help` footer renders ✅.
