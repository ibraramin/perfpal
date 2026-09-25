# ADR-0006: Shipping Scope at Beta — T0 distribution + T2 packaging; signing (T1) deferred to GA

- **Status:** Accepted (founder direction recorded session 34; formalized 2026-09-25)
- **Date:** 2026-09-25
- **Deciders:** Founder + orchestrator (evidence: @librarian Windows/macOS packaging + signing research, session 34 `lib-2`)
- **Gates:** The M5 release (plan `evidence-beta`) and the packaging legs it drives.

## Context

The beta ships **unsigned** binaries on six targets (linux musl x86_64/aarch64, linux gnu, macOS x86_64/aarch64, windows msvc) via GitHub Releases, already integrity-covered by a minisign-signed `SHA256SUMS` + SPDX SBOM + legal docs (plan `beta-release` B2/B3/B4).

Two questions remained open for the beta: **how far to go on packaging** (installing without hand-downloading an asset) and **whether to buy code signing** (authenticity + OS-warning suppression).

Research findings (session 34, @librarian):

- **Windows signing:** Azure Artifact Signing (aka Trusted Signing) is likely **ineligible** — the cheaper Individual Public Trust tier is **US/Canada only**, and Bangladesh is not a listed country. OV/EV certificates require an HSM/token and recurring cost. Critically, **signing does not silence SmartScreen** — EV certificates no longer bypass it and reputation accrues over weeks/hundreds of installs — so signing buys less than its price suggests at beta scale.
- **macOS:** notarization requires a paid Apple Developer account (**$99/yr**) **and** a `.pkg`/`.dmg` to staple — a **bare binary cannot be notarized**. Homebrew (via a tap) strips the quarantine attribute, sidestepping Gatekeeper for the common install path.
- **Packaging:** winget accepts proprietary manifests and a `portable` install (no MSI needed); **Scoop Main** has a notability bar (500★/150 forks) so a project needs its **own bucket**; **Homebrew**'s official cask has a notability bar, so a project needs its **own tap** (which can serve macOS **and** Linux); **AUR** allows a non-free `-bin` package; `cargo-binstall` serves Rust users directly. Fedora/COPR was judged unnecessary.

## Decision

**Ship T0 + T2 at beta; defer T1 (code signing/notarization) to GA.**

- **T0 — distribution:** the existing six unsigned binaries as GitHub Release assets, with the minisign-signed `SHA256SUMS` (`sha256sum -c` verification), SPDX SBOM, `THIRD-PARTY-NOTICES`, and the legal bundle (`LICENSE`, `EULA.md`, `PRIVACY.md`). The release is a clearly-labelled **pre-release**, never displacing `v0.1.0` as *Latest*.
- **T2 — packaging:** publish to **winget** (`portable`), an **own Scoop bucket**, an **own Homebrew tap** (macOS + Linux), **AUR `-bin`**, and **cargo-binstall**.
- **T1 — signing/notarization: DEFERRED to GA.** Signal the unsigned-beta caveat clearly (SmartScreen/Gatekeeper may warn; verify via the minisign-signed `SHA256SUMS`).

## Rationale

1. **Signing's cost/benefit is poor at beta.** Ineligibility for the cheap tier (geo), HSM friction, and the fact that signing does not actually silence SmartScreen mean signing would cost money and effort without removing the user-visible warning.
2. **A bare binary cannot be notarized.** Full macOS notarization demands a `.pkg`/`.dmg` **plus** $99/yr; Homebrew's tap path (T2) already avoids Gatekeeper's quarantine for most users.
3. **Packaging is where install friction actually lives.** T2 removes the manual-download step across all three desktop platforms at near-zero cost — higher leverage per unit effort than signing.
4. **Integrity ≠ authenticity, and integrity is already covered.** minisign-signed checksums give verifiable authenticity of *contents*; signing adds OS-level attribution the beta does not yet need.

## Alternatives Considered

- **Buy OV/EV Windows signing now — rejected.** Cost (certificate + HSM) with no SmartScreen benefit at beta scale; revisit at GA.
- **Skip packaging (T0 only) — rejected.** Manual asset downloads are the biggest install-friction item; T2 is cheap and high-leverage.
- **Publish to Scoop Main / Homebrew official cask — rejected.** Both have notability bars the project does not yet clear; own bucket/tap are the correct path.
- **Fedora/COPR — rejected.** Adds maintenance surface without a demonstrated audience.

## Consequences

**Positive**
- Users install via their platform's package manager (winget/Scoop/Homebrew/AUR/cargo-binstall) without hand-downloading binaries.
- No HSM/certificate purchases or geo-eligibility blockers before the first release.
- The unsigned caveat is explicit and paired with a working verification path (minisign `SHA256SUMS`).

**Negative / mitigated**
- **OS warnings persist at beta** (SmartScreen/Gatekeeper) — mitigated by clear documentation + the minisign verification path; resolved at GA via T1.
- **Own bucket/tap are publishing surfaces to maintain** — accepted; they are lightweight manifests, and deferring them would not reduce the total work, only move it.
- **AUR `-bin` carries non-free/legal visibility** — acceptable; the license is declared explicitly (`LicenseRef-Proprietary` / PolyForm Strict) and AUR permits non-free `-bin` packages.

## Reversal Triggers

1. SmartScreen/Gatekeeper friction is judged **materially harmful** during beta testing (may justify paying for a signing path earlier, e.g. Azure Trusted Signing if geo-eligibility changes).
2. A packaging channel (Scoop/Homebrew/AUR) proves unmaintainable or gains an official path (notability cleared).
3. GA is reached (T1 becomes in-scope by definition).

## References

- @librarian report `lib-2` (Windows/macOS signing + packaging, session 34) — the research detail is in the `state/session-state.json` **SESSION 34 RESEARCH** entry; the founder decision built on it is the **SESSION 34 SHIPPING SCOPE** entry.
- `docs/specs/evidence-beta-spec.md` §7 (shipping scope).
- Plan `beta-release` (B2 integrity, B3 SBOM, B4 legal, B5 signing deferral, B6 publish); `docs/roadmap.md` (shipping scope).
