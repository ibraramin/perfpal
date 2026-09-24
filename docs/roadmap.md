# Perfpal — Roadmap (post-v0.1.0)

> Authoritative post-MVP roadmap. Supersedes the "Post-MVP Roadmap (direction only)"
> section of `docs/specs/perfpal-core-spec.md`, which listed C++/Python next.
> Founder reshaped the roadmap in session 23: **Rust first**, C++/Python delayed.

## Status snapshot (session 34)

| Item | Value |
|---|---|
| Published | **`v0.1.0`** (tag `v0.1.0`, commit `f368133`) — the only published release |
| Pending | **`v0.2.0-beta.1`** — the release pipeline is fully staged (`beta-release` B0–B5 committed); tag/publish is gated on the founder supplying the EULA/privacy **contact address** and setting the **`MINISIGN_SECRET_KEY`**/**`MINISIGN_PASSWORD`** secrets |
| `Cargo.toml` version | `0.2.0-beta.1` (unreleased) |
| `metrics.json` schema | **1.2** |
| Languages supported | **C** + **Rust** (`.c`/`.h`/`.rs`) |
| Plans | `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` 🟡 (B0–B5 done; B6 in progress) |

## Milestone sequence

Ordered. One milestone = one or more focused sessions; one task per session.

### M1 — `rust-backend` (next implementation milestone)
Add a **Rust language backend** so Perfpal can analyze Rust — and **run on its own codebase** (dogfooding).
- New scanner backend over official **`tree-sitter-rust =0.24.2`** (MIT, ABI 15, compatible with pinned `tree-sitter 0.26.13`).
- **Full catalog mirrored** with **`PERF-RS-*`** rule ids (founder decision), not a subset.
- Schema **MINOR bump 1.1 → 1.2** (`languages` widens to `["c","rust"]`, `files[].language`); `rule_id` pattern is already open (`modules.md` §5.2).
- Dogfood report from Perfpal's own tree; `examples-rust/` corpus.
- Crate version bumps `0.1.0 → 0.2.0` (pre-beta).
- Plan: `docs/plans/rust-backend.md` (linked `perfpal-core-spec`).

### M2 — `beta-release` (official proprietary beta)
Cut **`0.2.0-beta.1`** containing **C + Rust** (Rust lands first per M1).
- Hardening (shipped in `Cargo.toml` `[profile.release]`): `opt-level=3`, `lto="fat"`, `codegen-units=1`, `strip="symbols"`, and `--remap-path-prefix` in every release build. `panic="unwind"` is kept deliberately — `panic="abort"` is **not** used because the CLI maps a caught panic to exit 3 via `std::panic::catch_unwind` (`modules.md` §8.2). `strip` + path-remapping are size/path hygiene, **not** obfuscation/anti-reversing.
- **`obfstr` NOT adopted** (closes B1's "decide obfstr for any sensitive literals"): no API keys, URLs, or secrets are compiled into Perfpal, so there are no sensitive literals to obfuscate; the hardening above is size/path hygiene only.
- Integrity: per-target `.sha256` + an aggregate **minisign-signed** `SHA256SUMS` (`SHA256SUMS.minisig`; public key `minisign.pub` committed, private key in the `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` repo secrets). GitHub **artifact attestations** are **deferred**: they require GitHub Enterprise Cloud on private repositories and the beta repository stays private (decision recorded 2026-09-24).
- **SBOM** (`cargo-sbom`, SPDX 2.3) + **THIRD-PARTY-NOTICES** (`cargo-about`) attached to the release by the tag-triggered pipeline + `cargo-deny` GPL deny-list as the standing enforcement gate.
- **EULA + privacy statement** (`EULA.md` + `PRIVACY.md`, both at repo root) + PolyForm Strict `LICENSE` (`LICENSE`) — all three attached to the release bundle. The noncommercial evaluation notice is surfaced in `--help` only (a footer); `--version` is kept byte-exact per `modules.md` §8.1. The committed `docs/release-notes-header.md` is prepended to the generated release notes. Windows Authenticode + macOS notarization deferred (HSM/cost friction) and **Azure Trusted Signing NOT wired** for beta; the unsigned-beta caveat (verify via the minisign-signed `SHA256SUMS` + `sha256sum -c`) is documented.
- Distribution: clearly-labelled GitHub **pre-release** (not GA), published with `--prerelease --latest=false` so the beta never displaces `v0.1.0` as *Latest*. The publish step is **draft-safe and idempotent** (create-as-draft → upload-while-draft → flip to published, then assert the final state): a failure before the flip leaves only a draft and re-running converges, while a re-run after publication skips the asset upload so live assets are never rewritten.
- **Tag discipline:** GitHub runs the workflow file from the commit a tag points at, so a release tag must point at the commit containing the current `.github/workflows/release.yml`; after any workflow edit, re-point (delete + re-create + push) any not-yet-validated tag so the tag, the workflow, and the built sources agree. Re-pointing an **already-published** tag additionally requires deleting or replacing the existing release/assets first — the publish step intentionally never rewrites a published release's assets.
- **Explicitly out of scope at beta:** deep obfuscation (no viable license-clean Rust tooling; obfuscation buys little) and license-key/activation (deferred to paid GA).
- Plan: `docs/plans/beta-release.md` (linked `perfpal-core-spec`).

### M3 — Website (separate repo, separate session family)
Attractive, eye-catching showcase site covering the **full scope** (shipped + planned).
- **Different repo:** `~/Perfpal-Website`; built and versioned independently; its own session family.
- Source of truth for its sessions: **`docs/website/scope-and-content.md`** (brief + sitemap + page content spec) in THIS repo.
- Not tracked by a plan in this repo (no product code here).

## Sequencing rationale
- Rust backend **before** the beta so the official beta ships **C + Rust** and the "Perfpal analyzes itself" dogfood demo is part of the product story.
- Website is orthogonal and lives in its own repo, so it can proceed in parallel without touching the product.

## Deferred (unchanged priority, not cancelled)
- **C++ / Python backends** — delayed by founder decision (Rust first).
- **LOOP-004 resolver-constness** follow-up (semantic tier).
- **Runtime-profile ingestion** (CONFIRMED-1).
- Release follow-ups: Node-24 action pins, `/Brepro` + darwin-x64 binary reproducibility, toolchain-pin drift guard.
- `stats.distributions` activation (MINOR bump on proposal acceptance).
- Housekeeping: Cargo.toml PolyForm SPDX alignment; `modules.md` §8 CLI doc drift.
- Optional: `examples/README.md`; corpus extension to fire the ARCH-001 hub-fanout sub-case.

## Versioning
- `0.1.0` released → `0.2.0` (Rust backend, unreleased) → **`0.2.0-beta.1`** (official beta, prerelease) → later betas/GA as scoped.
- `metrics.json` schema follows its own `MAJOR.MINOR` (currently **1.2** — the Rust backend schema MINOR bump has landed; see `docs/schema/metrics-json-v1.md` §5).
