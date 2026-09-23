# Handoff — Latest

## Objective
Session 32: plan `beta-release` **B2 (minisign-signed SHA256SUMS — attestations skipped) + B3 (SPDX 2.3 SBOM + THIRD-PARTY-NOTICES)**. **Complete — committed as `daacdd5`** (11 files). actionlint clean, all workflow commands simulated locally, gates green (391 tests / 0 fail). Dual review: **@reviewer rev-2 APPROVE** (1 MAJOR + 5 minors fixed) + **@requirements-reviewer APPROVED**.

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ (R0–R6) · **`beta-release` — B0+B1 ✅ (`ce9b4cd`); B2+B3 ✅ (`daacdd5`); B4–B6 remaining.**

## Version snapshot
- **Released: `v0.1.0`** (tag, target `f368133`).
- **`Cargo.toml` = `0.2.0-beta.1`**; **`metrics.json` schema = 1.2 — unchanged.** `Cargo.toml`/`Cargo.lock`/`deny.toml`/`ci.yml`/`LICENSE` untouched this session.
- `main` unreleased: … + B0+B1 (`ce9b4cd`) + **B2+B3 (`daacdd5`)**. **Not pushed** (remote push not requested).

## Key Decisions (B2+B3)
- **Attestations SKIPPED** — private repo needs GitHub Enterprise Cloud; minisign-only path (as pre-decided session 31; `docs/roadmap.md` M2 requalified + visibility recorded).
- **B2:** aggregate `SHA256SUMS` over the 6 binaries + minisign signature `SHA256SUMS.minisig`; keep per-target `.sha256` **and** the in-CI `sha256sum -c` gate; passphrase via **stdin** (never argv), key written `0600` with `trap … EXIT` cleanup; in-CI `minisign -Vm dist/SHA256SUMS -p minisign.pub` verify. minisign installed via **apt** (0.11-1, unpinned — founder decision).
- **B3:** new tag-time `sbom` job → SPDX 2.3 SBOM (`cargo-sbom 0.10.0` pinned prebuilt + SHA256 gate) + `THIRD-PARTY-NOTICES.md` (`cargo-about 0.9.2` via pinned install-action) → merged into `dist/` and attached to the release; a no-copyleft grep assertion fails closed. **`cargo-about --all-features`** (M1 fix — the default closure omitted `clang-sys` et al. that the semantic builds compile). `cargo-deny` (`ci.yml`) stays the per-push enforcement gate.
- **Keypair:** a dedicated perfpal-beta keypair was generated (pub key id `4A06548D8A4EFF90`, committed as `minisign.pub`); the **secret key + passphrase were handed to the founder** for the GitHub secrets — **not** in the repo.

## Risks
- **Operational pre-check for B6:** repo secrets `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` must exist and match the committed `minisign.pub` (mismatch fails closed at the verify step).
- The release pipeline (the new `sbom` job + apt minisign install + sign/verify/merge) has **never executed on GitHub** — the first `v*` tag validates it. Windows `pwsh` leg still unverified locally.
- minisign apt version is **unpinned** (documented, unmonitored). Deferred: verify signature against `origin/main` key (s1); `ubuntu-latest`/apt drift monitoring.
- Carried: R5 cross-language ARCH-003 substitution **still needs founder sign-off**; R5 deferred MINOR test items open; `error_node_overlap` FORCES `Medium`; stale `docs/roadmap.md` "Status snapshot (session 23)" block.

## Blockers
- **Founder must set the two GitHub secrets** (`MINISIGN_SECRET_KEY`, `MINISIGN_PASSWORD`) before any tag is pushed. (Commit `daacdd5` landed; `main` not pushed.)

## Next Actions
1. **Set GitHub secrets** `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` (values delivered in the session-32 close-out).
2. *(optional)* **Push `main`** — B2+B3 (`daacdd5`) is local only; a push triggers `ci.yml` (`release.yml` runs only on `v*` tags).
3. **Next session (`/new`) = plan `beta-release` B4+B5** — EULA (bundled with PolyForm Strict `LICENSE`), non-commercial evaluation notice in `--version`/`--help` + bundle, no-telemetry privacy statement, unsigned-beta caveat (defer Authenticode/notarization). Read state files + `readPlan beta-release` FIRST.
4. Then **B6** — publish the GitHub **PRE-RELEASE** (`--prerelease --latest=false`), tag discipline, smoke-test the published musl binary vs `examples/` + `examples-rust/`, hand the asset inventory to the website milestone.

## B2+B3 Change Set (committed `daacdd5`, 11 files)
- **Workflow:** `.github/workflows/release.yml` (new `sbom` job; `publish` `needs` + checkout + apt minisign + aggregate SHA256SUMS signing + signature/checksum verification). No attestation step.
- **New committed config:** `about.toml`, `about.hbs`, `minisign.pub`; `.gitignore` (+ `/dist/`, `/THIRD-PARTY-NOTICES.md`, `/perfpal-*.spdx.json`).
- **Docs:** `README.md` (16-asset inventory + signed-manifest verification), `CHANGELOG.md` (2 bullets under `[0.2.0-beta.1]`), `docs/roadmap.md` (M2 requalified).
- **State (EOL):** `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Commit message:** `feat(beta-release): B2 minisign-signed SHA256SUMS + B3 SBOM (cargo-sbom) & THIRD-PARTY-NOTICES (cargo-about); attestations skipped (private repo); cargo-about --all-features; dual review APPROVED`.
- **Gate status:** actionlint 1.7.12 ✅ (both workflows) · YAML parse ✅ (`publish.needs=[build-unix,build-windows,sbom]`) · minisign sign→verify→tamper→restore round-trip ✅ (exact workflow commands) · cargo-about `--all-features` 57 crates incl. `clang-sys`, no GPL-family ✅ · cargo-sbom SPDX-2.3 ✅ · `cargo fmt --check` ✅ · `cargo clippy --all-targets --locked -D warnings` ✅ · `cargo test --locked` **391 pass / 0 fail** ✅.
