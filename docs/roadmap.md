# Perfpal — Roadmap (post-v0.1.0)

> Authoritative post-MVP roadmap. Supersedes the "Post-MVP Roadmap (direction only)"
> section of `docs/specs/perfpal-core-spec.md`, which listed C++/Python next.
> Founder reshaped the roadmap in session 23: **Rust first**, C++/Python delayed.

## Status snapshot (session 23)

| Item | Value |
|---|---|
| Released | **`v0.1.0`** (tag `v0.1.0`, commit `f368133`) — the only release tag |
| Unreleased on `main` | semantic resolver tier, `examples/` C corpus, guarded-scan fix (commit `5643737`) |
| `Cargo.toml` version | `0.1.0` |
| `metrics.json` schema | **1.1** |
| Languages supported | **C** only (`.c`/`.h`) |
| Plans | `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ |

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
- Integrity: SHA256SUMS + **minisign** signature + GitHub **artifact attestations** (SLSA/Sigstore).
- **SBOM** (`cargo-sbom`, SPDX) + **THIRD-PARTY-NOTICES** (`cargo-about`) + `cargo-deny` GPL deny-list.
- **EULA** + PolyForm Strict `LICENSE`, non-commercial evaluation notice in `--version`/`--help` and the bundle.
- Distribution: clearly-labelled GitHub **pre-release** (not GA).
- **Explicitly out of scope at beta:** deep obfuscation (no viable license-clean Rust tooling; obfuscation buys little) and license-key/activation (deferred to paid GA). Windows Authenticode + macOS notarization deferred (cost); optional Azure Trusted Signing.
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
