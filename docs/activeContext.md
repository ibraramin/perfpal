# Active Context

## Current Task
`mvp-architecture` step 3 CLOSED: IR v1 spec + metrics.json schema v1.0 + design/stability doc written, dual-reviewed (@requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES — all 18 findings applied), mechanically verified (8 checks incl. negative tests). Session boundary reached; commit pending founder approval.

## Recent Decisions
- IR-v1: parser-agnostic event-stream model keyed `(path, name, span)`; spans 1-based with INCLUSIVE ends; coverage table proves all 23 catalog heuristics implementable from records (TYPE-001 via new `param_types`; LOOP-003 via `statement_role`).
- metrics.json v1.0: mandatory confidence/severity/resolver_tier per finding; downgrade caps enforced by if/then in schema; deterministic `finding_id = rule:path:line:col`; versioning = minor additive (open rule_id pattern) / major breaking.

## In Progress
- Nothing — task boundary reached.

## Next
- Fresh session (`/new`) → plan `mvp-architecture` step 4: perfpal-report/ output contract (metrics.json + report.md + charts/, naming, stability rules, report template outline).
- Steps 5–6: ADR-0002 visualization approach → module architecture doc.

## Blockers
- None.
