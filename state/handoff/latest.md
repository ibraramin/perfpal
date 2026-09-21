# Handoff — Latest

## Objective
Session 23 (planning-only, zero product code): founder reshaped the post-MVP roadmap. Deliverables: two planning-toolkit plans (`rust-backend`, `beta-release`), an authoritative `docs/roadmap.md`, and a website scope brief/content spec for a separate repo.

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · **`rust-backend` + `beta-release` created (active)**.

## Version snapshot
- **Released: `v0.1.0`** (only tag, target `f368133`). `Cargo.toml` = `0.1.0`. `metrics.json` schema = **1.1**. Languages = **C only**.
- `main` unreleased: semantic resolver, `examples/` C corpus, guarded-scan fix (`5643737`).

## Key Decisions
- **New milestone order:** `rust-backend` → `beta-release`. **C++/Python delayed.**
- **Rust backend** = Rust language support (official `tree-sitter-rust =0.24.2`, MIT, ABI 15) with the **full catalog mirrored** as `PERF-RS-*`; goal is dogfooding Perfpal on its own tree. Schema **1.1 → 1.2** minor; crate `0.1.0 → 0.2.0`.
- **Beta** = official proprietary **`0.2.0-beta.1`**, **C + Rust**, pre-release (not GA). Hardening + minisign + GitHub attestations + SBOM/NOTICES + EULA/PolyForm Strict. **Deep obfuscation + license-key/activation OUT at beta**; Authenticode/notarization deferred.
- **Website** = separate repo `~/Perfpal-Website`, separate session family; this repo owns the contract `docs/website/scope-and-content.md`.

## Research (librarian, session 23)
- `tree-sitter-rust 0.24.2` — official org, MIT, LANGUAGE_VERSION 15, compatible with pinned `tree-sitter 0.26.13`; `LANGUAGE.into()` glue identical to tree-sitter-c.
- Rust `#[cfg]` = ordinary sibling `attribute_item` (C's guarded-content bug does **not** recur); `cfg_if!`/`macro_rules!` bodies are opaque `token_tree`s → documented blind spot.
- Obfuscation: **skip** (unmaintained/legally-murky rustc LLVM forks; no commercial Rust obfuscator; obfuscation ≠ protection). Activation: **skip at beta** (Keygen is the pick at GA). Signing: minisign + GitHub attestations now; Azure Trusted Signing optional; Authenticode/notarization deferred. SBOM: `cargo-sbom` (MIT, SPDX) cheap-recommended.

## Risks
- **Beta tag visibility decision (B2):** GitHub attestations free on public repos, need Enterprise Cloud on private — repo is private today.
- **Rust blind spot:** `cfg_if!`/`macro_rules!` items invisible; Rust is syntactic-tier only (libclang resolver is C-only).
- Beta defers obfuscation/activation/Authenticode/notarization (documented rationale above).
- Carried: ARCH-001 hub-fanout corpus sub-case still non-firing; release binary reproducibility (x64-darwin/msvc); Node-24 action pins; toolchain 1.92.0 mirrored in 5 places.

## Blockers
- None.

## Next Actions
1. Founder approves this planning commit → commit (see Uncommitted Files; **EXCLUDE `.opencode/**`**).
2. **Next session (`/new`) = execute plan `rust-backend` R0** (pin grammar, link-test, backend doc). Read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST, then `readPlan rust-backend`.
3. Then `beta-release`. Website proceeds independently in `~/Perfpal-Website` using `docs/website/scope-and-content.md`.

## Uncommitted Files
- **Added:** `docs/roadmap.md`, `docs/website/scope-and-content.md`, `docs/plans/rust-backend.md`, `docs/plans/beta-release.md`
- **Modified:** `CHANGELOG.md`, `docs/activeContext.md`, `state/session-state.json`, `state/handoff/latest.md`
- **EXCLUDE:** `.opencode/**` (session-tooling noise)
- Working tree otherwise clean at `5643737`.
