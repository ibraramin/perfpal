# Active Context

## Current Task
`mvp-architecture` step 4 CLOSED: `perfpal-report/` output contract (`docs/report/output-contract.md`) written, dual-reviewed (@requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES — all 18 findings applied). Session boundary reached; commit pending founder approval.

## Recent Decisions
- Output contract v1: layout = metrics.json + bundled schema byte-copy (`metrics.schema.v<MAJOR>.<MINOR>.json`) + report.md + charts/; atomic regeneration (rename-aside-then-swap) + safety guard against replacing non-report directories.
- Single version marker: metrics.json `schema_version` governs the whole report dir; chart naming fixed (kebab-case), chart set deferred to ADR-0002.
- report.md template: deterministic sections, total finding sort order (severity→confidence→rule_id→path→start_line→start_col), exact `finding_id` cross-referencing, optional-field fallbacks.

## In Progress
- Nothing — task boundary reached.

## Next
- Fresh session (`/new`) → plan `mvp-architecture` step 5: ADR-0002 visualization approach (no-copyleft static chart tech; finalize chart set per output-contract §3).
- Step 6: module architecture doc (scanner → analyzer → reporter pipeline; CLI surface).

## Blockers
- None.
