# Active Context

## Current Task
Session 32: plan `beta-release` **B2 (minisign-signed SHA256SUMS; attestations skipped — private repo) + B3 (SPDX 2.3 SBOM + THIRD-PARTY-NOTICES)**. **Complete — uncommitted.** New tag-time `sbom` job (cargo-sbom 0.10.0 pinned prebuilt + cargo-about 0.9.2, `--all-features`) + `publish` sign/verify of an aggregate `SHA256SUMS` (minisign, passphrase via stdin). Dual review **@reviewer rev-2 APPROVE** + **@requirements-reviewer APPROVED**; gates 391 tests / 0 fail. **Commit pending founder approval (5 modified + 3 new files).**

## Recent Decisions
- **Attestations skipped** (private repo, GH Enterprise Cloud) → minisign-only; `docs/roadmap.md` M2 requalified + private visibility recorded.
- **minisign via apt (unpinned, founder decision)**; aggregate `SHA256SUMS` **kept alongside** per-target `.sha256`; passphrase via **stdin**.
- **SBOM/NOTICES generated at tag time** (`release.yml` `sbom` job, release-attached); `cargo-deny` (`ci.yml`) remains the per-push no-copyleft gate.
- **`cargo-about --all-features`** — the default closure omitted `clang-sys` et al. that the semantic builds compile (reviewer M1).
- **Beta keypair generated** (pub key id `4A06548D8A4EFF90`, committed as `minisign.pub`); secret + passphrase handed to the founder.

## In Progress
- Nothing — B2+B3 boundary reached; the tree awaits commit approval.

## Next
- **Set the two GitHub secrets** (`MINISIGN_SECRET_KEY`, `MINISIGN_PASSWORD`), **commit B2+B3** on approval (exclude `.opencode/**`), then start a fresh session for **B4+B5** (EULA + evaluation notice + no-telemetry statement + unsigned-beta caveat). **B6** publishes the pre-release.

## Blockers
- Commit approval pending; GitHub secrets not yet set (required before tagging).
- Carried: the R5 cross-language ARCH-003 substitution still needs explicit founder sign-off.
