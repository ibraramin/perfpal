# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- ADR-0002 visualization approach (`docs/adr/ADR-0002-visualization.md`, plan step 5): hand-rolled zero-dependency deterministic SVG emitter chosen over Graphviz-invoke/cgraph-link/JS-renderers/matplotlib-gnuplot (license audit verified against primary sources; byte-determinism by construction; self-contained CLI preserved). Chart set v1 frozen: `include-graph.svg` (file-level DAG), `component-coupling.svg` (matrix heatmap with LOW-confidence call edges folded in), `hot-components.svg` (finding-density bars). Deterministic layered layout + barycenter policy, cycle-breaking rule, emission/determinism guardrails, post-MVP `.dot` export as reversal trigger.
- Dual review on step 5: @requirements-reviewer APPROVED (all verbatim requirements met, no scope creep); @reviewer SHIP-WITH-FIXES — all 11 findings applied (MAJOR: output-contract tree-diagram regression fixed, deterministic cycle-breaking rule added; MINORs/NITs: component sort order, barycenter pass-count+rounding pinned, license-date hedge, reference URL fix, empty-input emission rule, diffability claim scoped to charts).
- `perfpal-report/` output contract (`docs/report/output-contract.md`, plan step 4): exact directory layout (`metrics.json` + bundled schema byte-copy + `report.md` + `charts/`), atomic regeneration with rename-aside-then-swap + non-report-directory safety guard, file-naming/stability rules (kebab-case chart names, full-version schema-copy filename), single version marker (= metrics.json `schema_version`), deterministic `report.md` template outline with total finding sort order and `finding_id` cross-referencing. Chart set deferred to ADR-0002; CLI surface deferred to step 6.
- Dual review on step 4: @requirements-reviewer APPROVED (all verbatim requirements + founder additions met, no scope creep); @reviewer SHIP-WITH-FIXES — all 18 findings applied (MAJOR: findings sort order made total via `span.start_col` tie-break; MINOR: built-in-catalog caveat, optional-field fallbacks, top-N pinning, `root_path` provenance-only carve-out, markdown-injection escaping, schema-discovery rule).
- Lessons Learned Log (`docs/lessons-learned.md`): append-only mistake/issue register with fallback guidance per entry; seeded with 6 entries from sessions 1–5 (edit-anchor misplacement, standalone `$def` validation, compressed-notation greppability, closed-enum vs forward-compat tension, bootstrap timeout seams, tree-sitter C11/C17 watch item).
- IR v1 spec (`docs/architecture/ir-v1.md`): parser-agnostic event-stream representation keyed `(path, name, span)` per ADR-0001 — FileRecord/ParseHealth/IncludeEdge/GlobalVar/StructDef/MacroEvent + FunctionRecord + typed events (Call/Assignment/BinaryOp/Comparison/AccessChain/LoopNode) with shared Shape descriptor; coverage table maps all 23 catalog heuristics to records; confidence-downgrade hooks; extension seams for semantic/dynamic tiers.
- metrics.json schema v1.0 (`docs/schema/metrics.schema.v1.json`, JSON Schema Draft 2020-12): LLM-agnostic, versioned output contract — mandatory `confidence`+`severity` per finding, resolver-tier tags, macro/error downgrade flags enforced via if/then, deterministic `finding_id` (`rule:path:line:col`, right-split), include/call graphs, hot-component aggregation.
- metrics.json v1 design & stability doc (`docs/schema/metrics-json-v1.md`): versioning policy (minor = additive incl. new rule_ids via open pattern; major = closed-enum changes, required-field additions, span/identity changes), enum semantics, worked example, LLM-consumption guidance.
- Dual review on step 3: @requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES — all 18 findings applied (incl. TYPE-001 `param_types` gap, pattern-based `rule_id`, span end-inclusive convention) and mechanically re-verified (8 checks incl. negative tests).

### Changed
- Founder SIGNED OFF heuristic catalog v1 ("All approved"): Tier 1 (15 heuristics) confirmed as MVP set, Tier 2 (8) included in MVP scope, deferred/excluded list accepted as out-of-scope. Catalog IDs (`PERF-C-*`) stable for schema work.
- Updated session-state.json, handoff/latest.md, and activeContext.md to reflect retry status and research findings.
- Expanded .opencode/spec.md with full founder answers: CLI-only, architecture-level visualization, dual LLM output, source-available license, no copyleft deps.
- Finalized Phase 0: all 5 blocking questions answered, assumptions confirmed, formal planning objects created (spec: perfpal-core-spec, plan: mvp-architecture).
- Founder formally APPROVED specs (perfpal-core-spec + mvp-architecture) — design gate open. Initial skeleton committed as 6bbfafd.
