# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P5 Tier 2: all 8 Tier 2 heuristics (MEM-002 first, then TYPE-001/002, MEM-001, LOOP-005–008) as pure IR-view functions with documented FP classes and mandatory LOW/MEDIUM confidence floors per catalog-v1 §4; carried planning note = verify backend Shape/access-chain coverage BEFORE writing the sizeof/memcpy rules. **DONE — 23-rule catalog complete; dual-review passed (@requirements-reviewer APPROVED zero scope creep; @reviewer CHANGES_REQUESTED → founder-directed review-fix wave → re-review APPROVE incl. a REAL sizeof-extraction bug caught by the new scanner test lock), 185 tests green under pinned 1.92.0.**

## Phase
executing mvp-implementation — P5 Tier 2 COMPLETE; next session starts **P6 hardening/release** (final phase)

## Key Decisions
- **Session-start discovery:** HEAD was `93759be` (P4) but the tree held an UNRECORDED partial P5 wave from a prior session that ended without its EOL ritual — MEM-001/002 + TYPE-001/002 complete with fixtures/tests, IR additive fields (`Shape.sizeof_operand`, `StructMember.is_pointer`, both serde-backward-compatible), scanner extraction, REGISTRY 19, Appendix A pre-extended to 23 ids. Verified healthy (173 green) and ADOPTED rather than redone.
- **Carried planning note RESOLVED before rule-writing:** backend already emitted everything LOOP-005–008 need (`AssignmentEvent.scope_class` incl. Global, `CalleeKind::Indirect`, `AccessChainEvent{root,elements}`, `ComparisonEvent` shapes, `LoopNode.induction_vars` "feeds LOOP-007"); only sizeof_operand/is_pointer extraction was new (landed with the adopted wave).
- **Detection contracts (catalog §4 floors LOCKED):** LOOP-005 global-write-in-loop Info/MEDIUM dedup per (target ident, innermost enclosing loop span) earliest anchor; LOOP-006 indirect-innermost Info/LOW mandatory; LOOP-007 induced-subscript vs non-literal comparison at depth ≥ `LOOP_007_MIN_DEPTH=2` Info/LOW candidate-grade (non-literal guard excludes literal sentinels; sentinel-VARIABLE residual FP documented); LOOP-008 chain ≥ `LOOP_008_MIN_CHAIN_DEPTH=3` innermost Info/MEDIUM with vtable/ops-table FP class documented AND still firing.
- **Founder-directed review-fix wave:** MAJOR-2 resolved by AGGREGATING TYPE-001 IN CODE — one Warn/LOW finding per function, evidence `params`/`structs`/`estimated_sizes` joined in source order (per-param findings would share the function-record span and collapse under deterministic dedup; parameter spans not modeled in IR); all five MINORs + first NIT applied (single-member TYPE-002 negative; LOOP-005 non-Identifier-target defensive-gate negative honestly labeled not-backend-reachable; O(calls×binary_ops) bound comment + BTreeSet idempotence note; long-double known-type underestimate doc [8 modeled vs 16 real]; Span/ResolverTier import placement; membership_sides returns Option<(&Shape,&str)> so the index flows from the gate proof).
- **REAL BUG FOUND BY THE FIX WAVE:** MAJOR-1's predicted risk materialized immediately — new scanner unit test exposed that `sizeof(struct S)` extracted `"("` because tree-sitter-c's type-descriptor form `[sizeof, "(", type_descriptor, ")"]` puts bare paren tokens at the sizeof_expression level (expression forms arrive as `[sizeof, expr]` / `[sizeof, parenthesized_expression]`). Fixed by dispatching on `child(1).kind()=="("` → `child(2)`; locked by unit test (4 cases) AND end-to-end (e2e main.c gained struct Big/weigh/clone_row; pipeline_e2e asserts TYPE-001+MEM-001 finding_ids plus evidence-level `misuse=="pointer_param_sizeof"` through the real backend). LESSON: parser-free fixture waves must be paired with real-parse locks for new extraction.
- REGISTRY now **23 entries true lexicographic** `[ALLOC-001..003, ARCH-001..003, CALL-001..002, LOOP-001..008, MEM-001..002, STR-001..003, TYPE-001..002]` — declaration order load-bearing (first-wins dedup); LOOP-005..008 inserted between loop_004 and mem_001.
- Kitchen-sink extended to trip **ALL 23 registry rules in report §4 total order** (kitchen_units/kitchen_structs builders); framing guard covers all 13 wave-C fixtures.
- Docs: ir-v1.md additive-field note (minor-bump policy covers), catalog-v1.md §4 implementation notes ×8, CHANGELOG Added (P5 wave) + Fixed (review-fix wave incl. the sizeof bug mechanics).

## Reviews
- @requirements-reviewer: **APPROVED.** Clauses (a)–(j) all PRESENT with file:line evidence: pure IR-view fns ×8, floors exact per catalog table, FP classes in rustdoc AND catalog notes, backend verification before rule-writing (no LOOP-rule extraction growth), fixtures incl. negatives + both downgrade-hook proofs on new rules, kitchen-sink all-23 total order, REGISTRY lexicographic 23, docs landed, zero scope creep (Cargo.toml byte-stable). Caveats environmental only (no shell in its sandbox → gates delegated to orchestrator, who ran them live).
- @reviewer: **CHANGES_REQUESTED** round 1 (0 Critical / 2 MAJOR / 5 MINOR / 3 NIT) → fix wave → **re-review APPROVE**: all 11 fix sites confirmed line-by-line, gates independently re-run (fmt/clippy/test 185 passed), requirements 1–10 spot-checked post-fix, zero new Critical/MAJOR/MINOR; residual NIT (framing-guard fixture list) applied post-verdict.

## Risks
- cargo NOT on PATH in fresh shells: prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"`.
- P5 accepted limits (documented): TYPE family approximate LP64 model (typedef'd params invisible, nested structs unmodeled w/ conservative unknown=8B, long double 8-vs-16 known underestimate); MEM-001 O(calls×binary_ops) bound accepted w/ named upgrade path; MEM-002 literal-size silence by design; LOOP-005 accumulator FP stays flagged + non-Identifier gate defensive-only; LOOP-006 dispatch-table FP (LOW mandatory); LOOP-007 candidate-grade noise (sentinel-variable indistinguishable); LOOP-008 vtable dispatch never suppressed.
- Session-infrastructure risk (observed): specialist lanes can fail mid-session (fixer session error + provider endpoint unavailable; reviewer timeout once) — orchestrator direct-execution is the sanctioned fallback when the fix list is fully specified; NEVER revive an errored session (board marks it unreusable).
- Prior risks unchanged: tree-sitter-c C99 grammar C11/C17 ERROR-rate (ParseHealth hook covers); STR-002 compound-op blindness; writer crash window + concurrent staging sweep (P6).

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P6 hardening/release** (final phase): finding_id uniqueness CI gate, golden directories, exit-code test matrix, musl static x86_64/aarch64 + macOS + MSVC release builds (cargo-zigbuild), GitHub Actions SHA-pinning. Resume with: plan mvp-implementation, commit \<P5 hash\>.
2. P6 carry-forward list (accumulated deferred nits): FindingEntry.threshold never populated despite LOOP-001/CALL-002 consts; concurrent-run staging-dir guard (m7); str_003 scanf format-position; loop_003 defensive guard; is_strlen_in_loop duplication; rustdoc backtick nesting alloc.rs; fixtures README stale wording; wave_b magic line numbers; kitchen-sink meta() is_innermost:true at depth 0 (do NOT copy as scanner-contract example); stats.distributions reserved-for-future (population needs founder-visible proposal).
3. Post-MVP upgrade paths recorded: semantic resolver tier retires ARCH-003 name-based FPs + LOOP-005/006 confidence floors; runtime-profile ingestion adapters (CONFIRMED-1).

## Uncommitted Files (pending founder-approved commit)
- New: `src/analyzer/heuristics/{mem,type}.rs`, `tests/wave_c_fixtures.rs`, `tests/fixtures/ir/PERF-C-{MEM×3,TYPE×4,LOOP-005×2,LOOP-006×2,LOOP-007×1,LOOP-008×1}__*.json` (13)
- Modified: `src/analyzer/{mod,finding}.rs`, `src/analyzer/heuristics/{mod,loop}.rs`, `src/ir/mod.rs`, `src/scanner/backends/tree_sitter_c/mod.rs`, `src/reporter/mod.rs`, `tests/fixtures/e2e/src/main.c`, `tests/pipeline_e2e.rs`, `tests/wave_a_fixtures.rs`, `docs/architecture/ir-v1.md`, `docs/heuristics/catalog-v1.md`, `CHANGELOG.md`
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`
- EXCLUDE from commit: `.opencode/memory/**` (session noise)
