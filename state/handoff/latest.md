# Handoff — Latest

## Objective
Execute `mvp-architecture` step 3: IR + metrics.json schema v1. **DONE — dual review passed, all fixes applied, mechanically verified. Commit pending founder approval.**

## Phase
executing mvp-architecture (steps 1–3/7 done; next: step 4)

## Key Decisions
- **IR-v1** (`docs/architecture/ir-v1.md`): parser-agnostic event-stream representation per ADR-0001 — file-level records (FileRecord/ParseHealth/IncludeEdge/GlobalVar/StructDef/MacroEvent) + FunctionRecord + typed events (Call/Assignment/BinaryOp/Comparison/AccessChain/LoopNode) sharing a Shape descriptor; coverage table maps **all 23 catalog heuristics** to records; spans 1-based lines/byte-columns with **INCLUSIVE ends**; confidence-downgrade hooks (macro_origin / cross-file edges / error-region overlap).
- **metrics.json schema v1.0** (`docs/schema/metrics.schema.v1.json`, Draft 2020-12): LLM-agnostic; mandatory `confidence`+`severity`+`resolver_tier`+`macro_suspect`+`error_node_overlap`+`message` per finding; deterministic `finding_id = rule:path:line:col` (parse right-to-left); sections scan/files/stats/findings/graphs(include+call)/components; downgrade caps enforced via if/then; call_graph edge confidence const LOW; C-only language consts.
- **Versioning policy**: minor = additive only (`rule_id` is an OPEN PATTERN — new heuristics/languages never break strict validation; explicit 23-ID list in description); major = closed-enum changes, required-field additions, threshold type widening, span/identity changes.
- **Dual review passed**: @requirements-reviewer APPROVED (verbatim requirements met, no scope creep); @reviewer SHIP-WITH-FIXES — **all 18 findings applied** (3 MAJOR: TYPE-001 gap → `FunctionRecord.param_types`; doc example `threshold:null` invalid → omit-if-absent; closed rule_id enum vs forward-compat tension → pattern-based rule_id).
- Mechanical verification: 8 checks green incl. negative tests (macro_suspect+HIGH rejected by if/then; call_graph non-LOW rejected).

## Risks
- tree-sitter-c C99 grammar vs C11/C17 constructs → ERROR-node monitoring carries into implementation (unchanged).
- Cross-file coupling (ARCH-003) LOW-confidence until semantic resolver tier exists (unchanged).
- Differentiation claims rest on one librarian survey pass (unchanged).

## Blockers
- None.

## Next Actions
1. Start fresh session (`/new`); resume with plan `mvp-architecture` **step 4: perfpal-report/ output contract** — exact layout (metrics.json per metrics.schema.v1.json, report.md, charts/), file naming, versioning/stability rules, human-readable report template outline.
2. Steps 5–6: ADR-0002 visualization approach → module architecture doc.

## Uncommitted Files
- `docs/architecture/ir-v1.md` (new)
- `docs/schema/metrics.schema.v1.json` (new)
- `docs/schema/metrics-json-v1.md` (new)
- `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md` (EOL ritual updates)
