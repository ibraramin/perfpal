# Active Context

## Current Task
Plan `mvp-implementation` **P3 Tier 1 wave B COMPLETE** (session 13): 4 heuristics as pure IR-view functions completing the function-local Tier 1 set — ALLOC-003 (realloc linear growth; two-part additivity proof: last arg Binary AND `'+'` span-contained in call; n*2 doubling stays silent; pointer-arith first-arg suppressed), LOOP-002 (18-name I/O family in loops; Hot→Warn when conditional_guarded), LOOP-004 (variable-divisor `/`/`%` innermost via scanner-fixed is_innermost; literal divisors skipped by design; Sizeof/Unary constant-foldable divisors documented accepted FP), CALL-002 (complexity ≥15 stat-promotion, Warn/HIGH, stat retained). AllocCandidate resolver_tier unified (alloc_001 hardcoded tier removed). REGISTRY re-sorted ascending → 12 entries. Zero extraction growth. 9 parser-free fixtures + wave_b suite (10 tests) + kitchen-sink all-12-rules total-order + e2e proof of all 4 rules through real backend (CALL-002 asserted against computed value). Dual review: @requirements-reviewer APPROVED (zero scope creep); @reviewer SHIP-WITH-FIXES zero Critical/MAJOR → both MINORs fixed per founder decisions. **99 tests green** under pinned 1.92.0. Commit pending founder approval.

## Recent Decisions
- ALLOC-003 additivity = span-contained `'+'` BinaryOpEvent + last-arg-Binary conjunct (Shape carries no operator; founder-approved both rounds).
- AllocCandidate.resolver_tier unification included in P3 (sanctioned nit; behavior-preserving).
- LOOP-004 constant-foldable divisors (Sizeof/Unary) documented as accepted FP class rather than code-excluded (founder choice; Info severity anyway).
- Wave-B tests in new `tests/wave_b_fixtures.rs`; kitchen-sink extended in place to 12 rules.
- Downgrade hooks stayed central; macro-origin inheritance proven on LOOP-002 fixture.

## In Progress
- Nothing — task boundary reached; awaiting commit approval.

## Next
- Founder approves commit → fresh session starts **P4 architecture wave**: ARCH-001 include-graph fan-out/cycles → ARCH-003 cross-file name-based call coupling (LOW confidence locked) → ARCH-002 hot-component aggregation LAST; aggregator graphs/components/stats complete with determinism tests; charts emitter (3 SVGs per ADR-0002, double-render byte-equality); reporter report.md template + Appendix A. Resume signposts: plan mvp-implementation, commit \<P3 hash\>.
- P4 notes: IncludeEdge.resolved flag ready (m6 graph-visibility decision lands here); call_graph confidence const LOW; charts follow ADR-0002 layout policy; aggregator owns all sort orders per modules.md §1.

## Blockers
- None.
