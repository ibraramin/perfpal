# Active Context

## Current Task
Plan `mvp-implementation` **P5 Tier 2 COMPLETE** (session 15): all 8 Tier 2 heuristics as pure IR-view functions completing the **23-rule catalog** — MEM-002 (memcpy/memmove/memset non-literal size in loop, Info/MEDIUM→Warn innermost), TYPE-001 (by-value large struct ≥32B founder-approved floor, Warn/LOW, **one aggregated finding per function** per founder-directed review fix), TYPE-002 (padding candidate ≥8B = one LP64 word, Info/LOW), MEM-001 (sizeof-misuse classes flagship `pointer_param_sizeof`, Warn/MEDIUM), LOOP-005 (global write in loop, Info/MEDIUM, dedup per target×loop earliest anchor), LOOP-006 (indirect call innermost, Info/LOW mandatory), LOOP-007 (induced-subscript vs non-literal comparison depth≥2 via `LoopNode.induction_vars`, Info/LOW candidate-grade), LOOP-008 (access chain ≥3 innermost, Info/MEDIUM, vtable FP documented AND still firing). IR additive fields `Shape.sizeof_operand` + `StructMember.is_pointer` (serde-backward-compatible); scanner extraction **unit-locked and e2e-locked after the new tests caught a REAL bug** — tree-sitter-c's type-descriptor `sizeof(struct S)` form yields bare `"("` at child(1); fixed by CST-shape dispatch. REGISTRY 23 true lexicographic; kitchen-sink trips ALL 23 rules in total order; framing guard over 13 wave-C fixtures; catalog §4 implementation notes ×8; carried planning note resolved (backend coverage verified before rule-writing). Dual review: @requirements-reviewer APPROVED (zero scope creep); @reviewer CHANGES_REQUESTED → review-fix wave → **re-review APPROVE** (gates independently re-run). **185 tests green** under pinned 1.92.0. Commit pending founder approval.

## Recent Decisions
- Founder-directed review-fix wave: TYPE-001 aggregated IN CODE (per-param findings would share the function-record span and collapse under deterministic dedup) over doc-amend; all five MINORs + first NIT applied.
- Real sizeof-extraction bug found by the new scanner test lock (MAJOR-1's predicted risk materialized): fixed via child(1)-kind dispatch on the two tree-sitter-c sizeof CST shapes; lesson recorded — parser-free fixture waves must be paired with real-parse locks for new extraction.
- Session-infrastructure fallback sanctioned: two fixer lanes failed (session error, provider endpoint unavailable) → orchestrator executed the review-fix wave directly; errored sessions are never revived (board marks them unreusable).

## In Progress
- Nothing — task boundary reached; awaiting commit approval.

## Next
- Founder approves commit → fresh session starts **P6 hardening/release** (final phase): finding_id uniqueness CI gate, golden directories, exit-code test matrix, musl static x86_64/aarch64 + macOS + MSVC builds (cargo-zigbuild), GitHub Actions SHA-pinning. Resume signposts: plan mvp-implementation, commit \<P5 hash\>.
- P6 carry-forward list in handoff §Next Actions item 2 (accumulated deferred nits incl. FindingEntry.threshold, staging-dir guard m7, stats.distributions reserved-for-future).

## Blockers
- None.
