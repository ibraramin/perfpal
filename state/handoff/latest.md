# Handoff — Latest

## Objective
Session 31: plan `beta-release` **B0 (version freeze) + B1 (release hardening)**. **Complete** — 391 tests / 0 fail; hardened release build + smoke test green; dual review rev-2 **@reviewer APPROVE** + **@requirements-reviewer APPROVED**. **Awaiting founder commit approval (17 files).**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ (R0–R6) · **`beta-release` — B0+B1 ✅ (uncommitted); B2–B6 remaining.**

## Version snapshot
- **Released: `v0.1.0`** (tag, target `f368133`).
- **`Cargo.toml` = `0.2.0-beta.1`** (B0 prerelease freeze; was `0.2.0`). `examples-rust/Cargo.toml` deliberately stays `0.1.0`.
- **`metrics.json` schema = 1.2 — UNCHANGED.**
- `main` unreleased: semantic resolver + both example corpora + guarded-scan fix + R0–R6 + **B0+B1 (uncommitted, 17 files)**.

## Key Decisions
- **Scope (founder, question tool):** this session = **B0+B1 only**; B2–B6 deferred to later sessions.
- **`panic = "abort"` REJECTED** — keep `panic = "unwind"` to preserve the documented **exit-code-3** contract (`std::panic::catch_unwind`, modules.md §8.2). (The plan's B1 text still says `abort` — overridden by founder decision.)
- **Repo stays PRIVATE** — GitHub artifact attestations **deferred** (private repos need GH Enterprise Cloud); **minisign-only** path for B2/B6. The plan's B2 "attestations" line must be requalified when B2 lands.
- **B0:** `0.2.0 → 0.2.0-beta.1`; `--version` auto-tracks → `perfpal 0.2.0-beta.1 (metrics schema 1.2)`; README beta banner + example; CHANGELOG `[Unreleased]` → `## [0.2.0-beta.1] - 2026-09-24`; **goldens regenerated** (version-only).
- **B1:** hardened `[profile.release]` (`opt-level=3`, `lto="fat"`, `codegen-units=1`, `strip="symbols"`, `panic="unwind"`); `--remap-path-prefix` in **both** release build steps. **`obfstr` NOT adopted** (no sensitive literals; strip+remap are size/path hygiene, **not** obfuscation) — recorded in `docs/roadmap.md` M2 + CHANGELOG.

## Risks
- **B2 forward:** attestations unavailable on the private repo → requalify to minisign-only; `docs/roadmap.md` M2 "Integrity" still lists attestations.
- **B6 forward:** `release.yml` `gh release create` lacks `--prerelease --latest=false` — must add, or the beta tag would publish as the repo's LATEST GA release.
- **Unverified locally:** Windows `pwsh` leg (no pwsh on this host) + musl/darwin/gnu cross-builds (CI-only).
- **Known NIT:** 7 residual `/home/shafin/.cargo/…` strings are third-party tree-sitter **C** `__FILE__` literals (not rustc-remappable; not panic strings) — out of B1's scope.
- Carried: R5 cross-language ARCH-003 substitution **still needs founder sign-off**; R5 deferred MINOR test items open; `error_node_overlap` FORCES `Medium` (founder-level design question).

## Blockers
- **Commit approval pending** (B0+B1, 17 files). Suggested message: `feat(beta-release): B0 version freeze 0.2.0-beta.1 + B1 release hardening (opt-level=3, fat LTO, cgu=1, strip, unwind kept, remap-path-prefix); goldens regen; roadmap/CHANGELOG decisions`.
- Carried: R5 cross-language ARCH-003 substitution awaits explicit founder sign-off.

## Next Actions
1. **Commit B0+B1** on approval (EXCLUDE `.opencode/memory/**`).
2. **Next session (`/new`) = plan `beta-release` B2 + B3** — minisign-sign SHA256SUMS + SBOM/NOTICES (**skip attestations**: private repo). Read state files + `readPlan beta-release` FIRST.
3. Then **B4+B5** (EULA + signing deferral), then **B6** (pre-release publish + website handoff).

## B0+B1 Change Set (uncommitted, 17 files)
- **Config:** `Cargo.toml` (version + hardened `[profile.release]`), `Cargo.lock` (perfpal `0.2.0-beta.1`), `.github/workflows/release.yml` (remap in both build steps + header note).
- **Docs:** `README.md` (banner + `--version` example), `CHANGELOG.md` (new `## [0.2.0-beta.1]` section + beta notice + B0/B1 Changed bullets), `docs/roadmap.md` (M2 unwind correction + obfstr decision bullet).
- **Test-only:** `src/aggregator/mod.rs`, `src/schema/mod.rs`, `tests/charts_render.rs`, `tests/reporter_render.rs`, `tests/reporter_rust.rs` (version literals → crate version).
- **Goldens (version-only regen):** `tests/golden/{report,report-mixed,report-rust}/{metrics.json,report.md}` (6 files).
- **State (EOL):** `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Gate status:** `cargo fmt --check` ✅ · `cargo clippy --all-targets --locked -- -D warnings` ✅ · `cargo test --locked` **391 pass / 0 fail**; hardened release build (with the CI remap RUSTFLAGS) ✅ 1m12s; smoke ✅ (`--version` exit 0; `examples/` 83 findings; `examples-rust/` 41 findings; full report trees).
