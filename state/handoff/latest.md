# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P1: vertical slice — ir types → scanner backend → analyzer registry + PERF-C-ALLOC-001 → schema serializer + embedded JSON Schema byte-copy → aggregator minimal assembly → writer atomic swap + safety guard → cli flags/exit codes 0/1/2/3 → span round-trip tests → **first end-to-end metrics.json from a sample C repo**. **DONE — dual-review passed, all MAJOR/MINOR fixes applied and verified, 71 tests green.**

## Phase
executing mvp-implementation — P1 COMPLETE; next session starts **P2 Tier 1 wave A**

## Key Decisions
- **Two fixer waves + one review-fix wave** built the slice: Wave 1 = ir/scanner/backend/analyzer+ALLOC-001; Wave 2 = schema/aggregator/writer/cli; Wave 3 = reviewer fixes.
- **Schema canonical artifact** lives at `src/schema/metrics.schema.v1.0.json` (embedded via `include_str!`); docs copy kept + byte-equality drift test (founder-approved plan addition).
- **P1 report dir = metrics.json + bundled schema only; `charts/` created empty; report.md/charts land in P4** via writer's generic `Artifact {rel_path, bytes}` map (no signature change needed).
- **Exit-code mapping:** clap parse errors → exit 1 (usage), overriding clap's default 2 — modules.md §8.2 reserves 2 for output errors. Panic → 3 via catch_unwind.
- **is_innermost semantics (review M2 resolution):** nearest enclosing loop exists AND that loop nests no other loop anywhere in its body; events outside loops always false. Matches ir-v1 §3.3 and fixture corpus.
- **statement_role (M3):** compound_statement transparent when it IS the loop's direct body child — braced `for(…){ foo(); }` is Primary; deeper compounds still bury/guard.
- **Single-file targets (M1):** rel path = file name ("main.c"); include-resolution root anchors at the file's parent directory.
- **ALLOC_WRAPPER_PATTERN_ID** shared constant lives in `src/ir/mod.rs` (the seam both backend and analyzer may import).
- **Unreadable root target dir → usage error exit 1**; nested subdir read failures stay silent-continue (partial discovery is data; documented).
- **Serializer:** `serde_json::to_string_pretty`; sorted map keys (preserve_order OFF), declaration-order structs.
- **Wave-1 deviations accepted by reviewer as sound:** IncludeEdge.to_path raw-text-then-resolver-rewrite; macro recognizer in backend (needs preproc_arg source text); wrapper-macro USE sites fire ALLOC-001 capped MEDIUM+macro_suspect; loop-header expressions count toward in_loop_depth.

## Reviews
- @requirements-reviewer: **APPROVED.** Every P1 clause + founder-approved addition PRESENT with evidence; no material scope creep (extraction emits all event types — defensible single-pass-walker choice, flagged for transparency only).
- @reviewer: **CHANGES_REQUESTED → all fixed.** MAJORs: M1 single-file scan ENOTDIR garbage; M2 is_innermost own-subtree bug; M3 statement_role Nested for braced bodies. MINORs fixed: m4 CRLF span off-by-one, m5 multi-declarator globals undercount, m8 crash-window doc, m9 unreadable root dir exit code, m10 magic-string dedup. Each fix has a regression test. Reviewer independently validated emitted metrics.json against the Draft 2020-12 schema with jsonschema — PASS.
- **Deferred with tracking:** m6 unresolved-include visibility in graphs (lands with ARCH-001/P2), m7 concurrent-run staging-dir guard + n11–n14 nits (P6 hardening).

## Risks
- cargo NOT on PATH in fresh shells (`rustup --no-modify-path`): prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"`.
- Writer rename-aside-then-swap has an inherent crash window between the two renames (previous report lost until next successful run) — documented in code; recovery-on-next-run only.
- Concurrent perfpal runs can sweep each other's staging dirs (single-process model assumed; guard deferred to P6).
- Cognitive complexity is a SonarSource-style approximation (else-if chains not specially weighted) — thresholds remain catalog-owned.
- tree-sitter-c C99 grammar vs C11/C17 ERROR-rate risk unchanged; ParseHealth downgrade hook covers it.
- clap locked at 4.6.1 (local index lagged 4.6.4+ at pin time); revisit opportunistically.

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P2 Tier 1 wave A**: ALLOC-002, STR-001, STR-002, STR-003, LOOP-001, LOOP-003, CALL-001 — each a pure IR-view function, parser-free tested against fixture IR, ParseHealth gating + downgrade hooks applied, ascending rule_id execution order. Resume with: plan mvp-implementation, commit \<P1 hash\>.
2. P2 implementer notes: LOOP-003 depends on the FIXED statement_role semantics; LOOP-004/006/008 (P3) depend on the FIXED is_innermost; LOOP-005 wants multi-declarator global capture (fixed in m5). m6 (unresolved includes visible in graphs) rides with ARCH-001.

## Uncommitted Files (pending founder-approved commit)
- Modified: `src/{ir,scanner,analyzer,aggregator,schema,writer,cli}/mod.rs`, `src/scanner/backends/tree_sitter_c/mod.rs`, `tests/common/mod.rs`
- New: `src/analyzer/finding.rs`, `src/schema/metrics.schema.v1.0.json`, `tests/{alloc001_fixtures,analyzer_registry,scanner_pipeline,pipeline_e2e}.rs`, `tests/fixtures/ir/PERF-C-ALLOC-001__*.json` (×4), `tests/fixtures/e2e/src/{main.c,util.h}`
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`
