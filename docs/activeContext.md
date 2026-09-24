# Active Context

## Current Task
Session 34: plan `beta-release` **B6 — unblocked half COMPLETE**. The GitHub pre-release publish is now **draft-safe** (`create-as-draft → upload-while-draft → flip --draft=false --prerelease --latest=false → fail-closed assert`), tag discipline is recorded, the **19-asset inventory** is handed to the website milestone, and the `0.2.0-beta.1` musl binary is pre-flight smoke-tested (`examples/` 83, `examples-rust/` 41, exit 0). **Not yet committed** (founder approval pending). Founder-only half (contact address + secrets + tag/publish) deferred.

## Recent Decisions
- **Scope split:** only the unblocked half this session; contact address and the `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` secrets are founder-only, so the tag/publish waits.
- **Publish pattern:** draft-then-flip (recoverable) over one-shot create — gh semantics verified by @librarian.
- **Contact placeholder deliberately deferred** (founder: will buy an address later); governing law = Bangladesh.

## In Progress
- Nothing — B6 unblocked half done; gates green; dual review closed (rev-2 APPROVE); awaiting the founder's commit approval.

## Next
- **Founder:** approve the B6 ritual commit; supply the contact address; set the two secrets.
- **Next session (`/new`)** = B6 finish: replace the contact placeholder → confirm secrets → commit → tag `v0.2.0-beta.1` at the `release.yml`-bearing commit → publish → smoke-test the **PUBLISHED** musl binary vs `examples/` + `examples-rust/` → finalize `docs/website/scope-and-content.md` §8.1 with the real release URL.

## Blockers
- EULA/PRIVACY contact placeholder still unfilled (pre-tag; deliberate deferral).
- GitHub secrets `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` not yet set (required before tagging).
- Carried: R5 cross-language ARCH-003 substitution still needs explicit founder sign-off.
