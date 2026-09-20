# Handoff — Latest

## Objective
Post-MVP sessions 19–20: founder chose the **semantic resolver tier (ADR-0001)** over runtime-profile ingestion / C++-Python backends / release follow-ups. DONE: plan `semantic-resolver` COMPLETE (status done) — libclang enrichment via default-OFF cargo feature `semantic-clang` (clang-sys 1.9.1 runtime/dlopen); ARCH-003 true-callee resolution (Info/MEDIUM/semantic, same-name FP class retired); ALLOC-002 proven-`fclose` (Warn/MEDIUM, no escalation — founder verdict over B2's mirror-free HIGH); STR-003 resolved-format identifiers; LOOP-004 constness DEFERRED to a follow-up (recorded, zero partial code); schema v1.0→v1.1 MINOR bump (additive only); 6-target release matrix (musl OFF byte-identical, gnu NEW, darwin x2 + win ON); gates green first-hand (238 default / 243 feature, fmt + clippy both configs + deny ok, goldens idempotent). Dual review + re-review APPROVE; awaiting founder ritual-commit approval.

## Phase
POST-MVP — v0.1.0 released (tag `f368133`); housekeeping batch landed (`a84c1f0`); semantic-resolver milestone landed on main as uncommitted working tree (pending founder approval). Plan `semantic-resolver` DONE; `mvp-implementation` + `housekeeping-batch` remain COMPLETE.

## Key Decisions
- **Milestone + distribution + scope (founder, question tool):** semantic resolver; darwin x2 + windows + NEW gnu carry the feature, musl stays syntactic-only BY DESIGN (musl dlopen impossible — librarian-verified, bindgen#2360 precedent); 3 named retirements + LOOP-004 stretch-or-defer; no feature spec.
- **Resolver rules:** conservative unambiguous-only enrichment; USRs internal-only (never in identity/output — reviewer-verified); macro_origin + error-region gating; `Some(CallSemantic)`-only-when-proven; explicit-root signature (scan global removed in review-fix).
- **Confidence verdicts:** resolved ARCH-003 = MEDIUM (no compile db, partial TUs); proven-fclose = Warn/MEDIUM no escalation (reviewer taxonomy argument: documented FP class ⇒ MEDIUM; balanced open→close not hoistable) — founder-approved over B2's mirror-free HIGH.
- **Gate design:** `apply_upgrade_hooks` = kind-matched (rule↔fact) + start-point containment + default-deny; record-tier disjunct dropped as pure liability (reviewer MAJOR → accepted).
- **Rejections with rationale (documented):** `total_order`-reuse + message-constructor NITs (fail loudly, test-only); downgrade/error-region `intersects()` untouched (conservative direction correct).
- **EOL tail (orchestrator-direct):** proved feature-without-libclang e2e failure → `libclang_loadable()` probe; parser-free resolved-edge + file-local ARCH-003 fixtures (guardrail-2 completeness); README `--version` 1.0→1.1.
- **Session-infra lesson:** a revived specialist session vanished from the tracker with work landed but broken — orchestrator verified gates directly and rescued the tail; always re-verify landed-but-unreported work.

## Reviews
- @reviewer: **CHANGES_REQUESTED** (gate kind-agnosticism, fclose Hot/HIGH, intersects touching, scan_root, strip terminator; release/CI verified no-action; USR rule holds; schema additive-only; unsafe confined to FFI boundary) → fix wave → **re-review APPROVE, all 5 CLOSED** (negatives genuine; free path untouched; 10/10 resolver_health RUN with libclang).
- @requirements-reviewer: **CHANGES_REQUESTED** (README x-gaps only — 6-target table, v1.1 refs, tier wordings; all clauses a–w PRESENT, no scope creep) → closed (verified: table, refs, `--version` 1.1, qualifications).

## Risks
- release.yml 6-target matrix + feature builds NEVER executed on GitHub — first v* tag validates (gnu job, darwin/win flags, musl byte-identity, publish merge).
- libclang version diversity untested (local 22.1 + CI apt only); darwin/Windows discovery paths never exercised (graceful Absent fallback).
- libclang-22-shaped visitor heuristics (FILE spelling, single-token literals); conservative-silent direction bounds risk.

## Blockers
- None.

## Next Actions
1. Founder approves the ritual commit. Suggested message: `feat: semantic resolver tier (ADR-0001) — libclang enrichment via default-OFF semantic-clang; ARCH-003 resolved edges, ALLOC-002 proven-fclose Warn/MEDIUM, STR-003 resolved-formats; schema v1.1 minor; 6-target release matrix; LOOP-004 deferred (238/243 tests)`.
2. Commit files (see §Uncommitted Files; EXCLUDE `.opencode/**` session noise).
3. Remaining milestones (founder picks): LOOP-004 constness follow-up, runtime-profile ingestion (CONFIRMED-1), C++/Python backend, release follow-ups, stats.distributions activation, housekeeping follow-ups (Cargo SPDX, modules §8).
4. Fresh session (`/new`) signposts: **semantic-resolver LANDED** (dual review + re-review APPROVE, gates 238/243); plan `semantic-resolver` DONE; read state files FIRST.

## Uncommitted Files (pending founder-approved ritual commit)
- Added: `src/resolver/`, `src/schema/metrics.schema.v1.1.json`, `docs/schema/metrics.schema.v1.1.json`, `tests/resolver_health.rs`, `tests/semantic_b2.rs`, `tests/golden/report/metrics.schema.v1.1.json`, `tests/fixtures/ir/PERF-C-ALLOC-002__fclose-in-loop.json`, `PERF-C-ALLOC-002__fclose-not-proven.json`, `PERF-C-STR-003__resolved-format{,-negative}.json`, `PERF-C-ARCH-003__resolved-edge.json`, `PERF-C-ARCH-003__resolved-file-local.json`, `docs/plans/semantic-resolver.md`
- Modified: `Cargo.toml`, `Cargo.lock`, `src/{lib,ir,scanner,scanner/backends/tree_sitter_c,analyzer,analyzer/heuristics/{mod,arch,alloc,str},aggregator,schema,reporter,cli}/mod.rs` (aggregator unit-test helper + schema/DTO lifts; heuristics = arch/alloc/str only), `docs/{architecture/{modules,ir-v1},heuristics/catalog-v1,schema/metrics-json-v1}.md`, `docs/plans/housekeeping-batch.md` (status active→done), `tests/{common/mod,arch_fixtures,pipeline_e2e,p6_hardening,analyzer_registry,reporter_render}.rs`, `tests/golden/report/{metrics.json,report.md}` (v1.1 regen), `.github/workflows/{ci,release}.yml`, `README.md`, `CHANGELOG.md`, `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`
- Deleted: `tests/golden/report/metrics.schema.v1.0.json` (replaced by v1.1 byte-copy)
- EXCLUDE from commit: `.opencode/**` (session-tooling noise)
