# Handoff — Latest

## Objective
Execute `mvp-architecture` step 2: heuristic catalog v1 for C. **DONE — founder-signed-off and committed.**

## Phase
executing mvp-architecture (steps 1–2/7 done; next: step 3)

## Key Decisions
- **CATALOG-v1 SIGNED OFF** (founder, "All approved"): Tier 1 (15 heuristics) = MVP set; Tier 2 (8 heuristics) included in MVP scope (build-ordered after Tier 1); §5 deferred/excluded accepted as out-of-scope.
- Catalog at `docs/heuristics/catalog-v1.md`: ranking model Signal S1–S3 × Cost C1–C3; confidence HIGH/MEDIUM/LOW with macro-suspect/ERROR-node downgrades; every entry `(syntactic)`-tagged with default severity band (hot/warn/info).
- Differentiation space confirmed: alloc-in-loop, strcat-in-loop, strlen-in-loop, setup/I-O-in-loop not flagged by surveyed mainstream tools.
- Dual review passed before sign-off: @requirements-reviewer APPROVED; @reviewer SHIP-WITH-FIXES (all 7 findings applied).

## Risks
- tree-sitter-c C99 grammar vs C11/C17 constructs → ERROR-node monitoring carries into implementation.
- Cross-file coupling (ARCH-003) LOW-confidence until semantic resolver tier exists.
- Differentiation claims rest on one librarian survey pass; harden before external messaging.

## Blockers
- None.

## Next Actions
1. Start fresh session (`/new`); resume with plan `mvp-architecture` step 3: IR + metrics.json schema v1 — consume stable PERF-C-* IDs, confidence/severity fields, LLM-agnostic versioned schema.
2. Steps 4–6: perfpal-report/ output contract → ADR-0002 visualization → module architecture doc.

## Uncommitted Files
- None expected — session ended with approved commit (see git log for hash).
