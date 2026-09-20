# Active Context

## Current Task
Sessions 19–20 (post-MVP): **semantic-resolver milestone COMPLETE and reviewed.** Deliverables: `src/resolver/` (clang-sys 1.9.1 runtime/dlopen behind default-OFF `semantic-clang`; USR maps + fopen/literal visitor; explicit-root signature), tier mutations (`CallEvent.semantic`, `FunctionRecord.resolver_tier`, `ParseHealth.resolver`), ARCH-003 resolved edges (Info/MEDIUM/semantic), ALLOC-002 proven-fclose (Warn/MEDIUM, no escalation — founder verdict), STR-003 resolved formats, kind-matched `apply_upgrade_hooks` gate, schema v1.1 minor, CI feature job, 6-target release matrix (musl syntactic-only BY DESIGN), LOOP-004 deferred with record. Gates green first-hand: **238 default / 243 feature**, fmt + clippy both configs, deny ok, goldens idempotent. Dual review + re-review APPROVE.

## Recent Decisions
- Milestone/distribution/scope (founder): semantic resolver; darwin+win+gnu ON, musl OFF; 3 named + LOOP-004 stretch-or-defer; no feature spec; execution + fix-wave approvals via question tool.
- fclose = Warn/MEDIUM no escalation (reviewer taxonomy argument beat B2 mirror-free HIGH; founder-approved).
- Gate = kind-matched + containment + default-deny; record-tier disjunct dropped; `total_order`/message NITs rejected (fail loudly, test-only).
- EOL tail: libclang-absence probe fix (proven red), parser-free resolved ARCH-003 fixtures, README `--version` 1.1.

## In Progress
- Nothing — task boundary reached; awaiting founder ritual-commit approval (message + file list in `state/handoff/latest.md`).

## Next
- Founder picks next milestone: LOOP-004 constness follow-up, runtime-profile ingestion (CONFIRMED-1), C++/Python backend, release follow-ups, stats.distributions activation, housekeeping follow-ups.
- Fresh session (`/new`): semantic-resolver LANDED, plan DONE; read state files FIRST.

## Blockers
- None.
