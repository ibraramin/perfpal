# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P3 Tier 1 wave B: ALLOC-003 (realloc linear growth), LOOP-002 (setup/I-O in loop w/ conditional_guarded severity lowering), LOOP-004 (variable-divisor div/mod innermost), CALL-002 (cyclomatic complexity ≥15 stat-promotion) as pure IR-view functions, parser-free fixture tests, ascending rule_id REGISTRY. **DONE — dual-review passed (requirements APPROVED zero scope creep; reviewer SHIP-WITH-FIXES zero Critical/MAJOR → both MINORs fixed per founder decisions), 99 tests green under pinned 1.92.0.**

## Phase
executing mvp-implementation — P3 Tier 1 wave B COMPLETE; next session starts **P4 architecture wave**

## Key Decisions
- **Zero extraction growth:** all required IR data pre-extracted — `BinaryOpEvent.op/left/right` (LOOP-004 + ALLOC-003 additivity), `CallEvent.conditional_guarded` (LOOP-002 lowering hook), `EventMeta.is_innermost` scanner-fixed (LOOP-004), `FunctionRecord.cyclomatic_complexity` (CALL-002 stat-promotion).
- **ALLOC-003 two-part additivity proof** (founder-approved): fires only when the realloc call's LAST arg Shape kind == Binary AND a `'+'` BinaryOpEvent is span-contained within the call span. Multiplicative doubling growth (`n*2`) stays silent (it IS the documented fix); first-arg pointer arithmetic `realloc(p+1,n)` suppressed (review MINOR fix). Hot/MEDIUM.
- **AllocCandidate resolver_tier unified** while touching the alloc family (sanctioned P2 nit): field populated from `call.meta.resolver_tier`; alloc_001 hardcoded Syntactic tier removed — behavior-preserving.
- **LOOP-002:** fixed EXPENSIVE_CALL_FAMILY const (18 names: printf-family ×8, fopen/freopen/fclose/open/close/read/write, getenv, system, regcomp — matches catalog line exactly); Hot default → **Warn when conditional_guarded**; no statement_role gate (unlike LOOP-003); MEDIUM.
- **LOOP-004:** `/` or `%` + `is_innermost` + divisor kind ≠ Literal (literal divisors skipped BY DESIGN). Constant-foldable non-literal divisors (Sizeof e.g. `sizeof(int)`, Unary e.g. `-1`) fire by design at Info severity — documented accepted FP class in catalog note (founder chose document-over-code-fix). Info/MEDIUM.
- **CALL-002:** `cyclomatic_complexity >= CALL_002_MIN_COMPLEXITY(15)` → one Warn/HIGH finding per function anchored at function span; stat RETAINED in stats.functions[] (promotion, not move); e2e asserts against actually-computed value with drift guard.
- **REGISTRY re-sorted ascending rule_id → 12 entries** `[ALLOC-001..003, CALL-001..002, LOOP-001..004, STR-001..003]` — declaration order load-bearing (first-wins dedup); reviewer verified order truly lexicographic.
- **Test layout** (founder-approved): new `tests/wave_b_fixtures.rs` (10 tests); kitchen-sink stays in wave_a file extended to all-12-rules total-order + wave-B spot-checks; `by_rule` helper moved to `tests/common/mod.rs`.
- Downgrade hooks stayed central — inherited free by all 4 rules; `PERF-C-LOOP-002__macro-origin.json` proves macro_origin path on a wave-B rule.
- Messages framed "candidate optimization" throughout; never "bug".

## Reviews
- @requirements-reviewer: **APPROVED.** All clauses (a)–(k) PRESENT with evidence; four rules match pure-heuristic pattern `fn(&[FileUnit]) -> Vec<Finding>`; zero scope creep (touched set = expected set; scanner/aggregator/ir/schema/writer/cli untouched).
- @reviewer: **SHIP-WITH-FIXES, zero Critical/MAJOR.** Independently re-ran gates (fmt/clippy/test ✓ 98→99). Verified: span semantics exact (1-based inclusive matches ir-v1 §2); compound `+=` cannot false-fire ALLOC-003 (scanner emits `'+'` only for binary_expression); per-function scoping correct (top-level function spans disjoint); alloc_site_index has no panic path; LOOP-002 family matches catalog exactly; fixtures hard-error on typos (no serde defaults); kitchen-sink assertion not vacuous; evidence/message conventions consistent with P2. Both MINORs fixed as prescribed (ALLOC-003 last-arg-Binary conjunct + fixture; LOOP-004 catalog FP documentation). NITs deferred: magic line numbers in two wave_b negative tests; kitchen `meta()` sets `is_innermost:true` at depth 0 (harmless — do not copy as scanner-contract example).

## Risks
- cargo NOT on PATH in fresh shells (`rustup --no-modify-path`): prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"`.
- Wave-B syntactic-tier limits (documented, accepted): ALLOC-003 requires literal realloc call with Binary-kind size arg (wrapper-macro realloc / Call-kind size args stay silent); LOOP-002 family is a fixed list; LOOP-004 fires on constant-foldable Sizeof/Unary divisors at Info severity.
- Prior risks unchanged: tree-sitter-c C99 grammar C11/C17 ERROR-rate (ParseHealth hook covers); STR-002 compound-op blindness; writer crash window + concurrent staging sweep (P6).

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P4 architecture wave**: ARCH-001 include-graph fan-out/cycles → ARCH-003 cross-file name-based call coupling (confidence locked LOW) → ARCH-002 hot-component aggregation LAST; complete aggregator graphs/components/stats incl. every Principle-2 sort order (+ determinism tests); charts emitter (include-graph.svg, component-coupling.svg, hot-components.svg per ADR-0002, double-render byte-equality tests); reporter report.md fixed template + Appendix A built-in catalog. Resume with: plan mvp-implementation, commit \<P3 hash\>.
2. P4 implementer notes: IncludeEdge already extracted with resolved flag (unresolvable quoted includes as data — reviewer-deferred m6 graph-visibility decision lands here); call_graph edges carry confidence const LOW per schema; charts must follow ADR-0002 layout policy (topological layering, barycenter ×3 passes, half-up rounding, deterministic cycle-breaking); aggregator owns every sort order per modules.md §1 determinism matrix.
3. Deferred nits carried forward: str_003 scanf format-position matching; loop_003 defensive in_loop_depth guard; is_strlen_in_loop duplication; rustdoc backtick nesting; README stale wording; NEW: wave_b magic line numbers; kitchen meta() depth-0 innermost.

## Uncommitted Files (pending founder-approved commit)
- Modified: `src/analyzer/{mod,finding}.rs`, `src/analyzer/heuristics/{mod,alloc,loop,call}.rs`, `docs/heuristics/catalog-v1.md`, `tests/common/mod.rs`, `tests/wave_a_fixtures.rs`, `tests/pipeline_e2e.rs`, `tests/fixtures/e2e/src/main.c`
- New: `tests/wave_b_fixtures.rs`, `tests/fixtures/ir/PERF-C-{ALLOC-003__realloc-additive-in-loop,ALLOC-003__realloc-additive-not-in-loop,ALLOC-003__realloc-multiplicative-in-loop,ALLOC-003__realloc-pointer-arith-first-arg,CALL-002__high-complexity,LOOP-002__io-in-loop,LOOP-002__macro-origin,LOOP-002__not-in-loop,LOOP-004__var-divisor-innermost}.json`
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`
- EXCLUDE from commit: `.opencode/memory/**` (session noise, ~90 untracked files)
