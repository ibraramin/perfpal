# Active Context

## Current Task
Session 31: plan `beta-release` **B0 (version freeze) + B1 (release hardening)**. **Complete — uncommitted.** Crate `0.2.0 → 0.2.0-beta.1`; hardened `[profile.release]` (`panic="unwind"` kept — `abort` rejected); `--remap-path-prefix` in both release build steps; README beta banner; CHANGELOG `## [0.2.0-beta.1] - 2026-09-24`; roadmap obfstr/unwind decisions; 3 golden trees regenerated (version-only). 391 tests / 0 fail; hardened release build + smoke green. Dual review rev-2: **@reviewer APPROVE** + **@requirements-reviewer APPROVED**. **Commit pending founder approval (17 files).**

## Recent Decisions
- **Founder (question tool):** B0+B1 only this session; `panic="abort"` **rejected** (keep unwinding → preserve exit-code-3 `catch_unwind` contract); repo **private** → GitHub attestations **deferred** (minisign-only at B2/B6).
- **`obfstr` NOT adopted** (no sensitive literals; strip/remap = size/path hygiene, not obfuscation) — recorded in `docs/roadmap.md` M2 + CHANGELOG.
- **Goldens regenerated** (not masked) on the version bump; verified version-only across all 3 trees.

## In Progress
- Nothing — B0+B1 boundary reached; the 17-file tree awaits commit approval.

## Next
- **Commit B0+B1** on approval (exclude `.opencode/memory/**`), then start a fresh session for **B2+B3** (minisign + SBOM/NOTICES; **skip attestations** — private repo). B4+B5 (EULA + signing) and B6 (pre-release publish) follow.

## Blockers
- Commit approval pending.
- Carried: the R5 cross-language ARCH-003 substitution still needs explicit founder sign-off.
