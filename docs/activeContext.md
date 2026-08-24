# Active Context

## Current Task
Plan `mvp-implementation` **P2 Tier 1 wave A COMPLETE** (session 12): 7 heuristics as pure IR-view functions — ALLOC-002 (free-in-loop, co-occurrence Warn→Hot, fclose deferred+documented), STR-001 (strcat accumulation), STR-002 (strlen w/ reassignment suppression), STR-003 (unbounded copy family + scanf `%s`), LOOP-001 (depth==3 exact, one finding per maximal nest entry), LOOP-003 (char-stdio primary statement), CALL-001 (direct recursion, presence-only). Heuristics split into `src/analyzer/heuristics/{alloc,str,loop,call}.rs`; REGISTRY reordered true ascending rule_id (load-bearing for first-wins dedup). Zero extraction growth. 12 parser-free fixtures + kitchen-sink all-8-rules total-order test + e2e proof of 4 new rules through the real backend. Dual review: @requirements-reviewer CHANGES_REQUESTED → both items resolved (fclose deferral docs, CALL-001 downgrade-hook proof); @reviewer SHIP-WITH-FIXES zero Critical/MAJOR → 2 MINOR test defects fixed. **89 tests green** under pinned 1.92.0. Commit pending founder approval.

## Recent Decisions
- REGISTRY declaration order = lexicographic ascending rule_id `[ALLOC-001, ALLOC-002, CALL-001, LOOP-001, LOOP-003, STR-001, STR-002, STR-003]` — first-wins dedup depends on it.
- ALLOC-002 detects `free` only; `fclose` deferred to semantic tier (heap-backedness not syntactically decidable) — rustdoc + catalog-v1.md implementation note.
- strcat-in-loop double-fires STR-001 + STR-003 by design (distinct signals).
- LOOP-001 anchors at depth==exactly-3 loops only; `max_depth` evidence covers the full chain.
- Downgrade hooks stayed central; per-family hook proofs added for str/loop/call.
- Deferred nits: loop_003 defensive depth guard; alloc_001 hardcoded Syntactic tier; str_003 scanf format-position matching; README stale wording (see handoff).

## In Progress
- Nothing — task boundary reached; awaiting commit approval.

## Next
- Founder approves commit → fresh session starts **P3 Tier 1 wave B**: ALLOC-003 (realloc linear growth), LOOP-002 (setup/I-O in loop, conditional_guarded severity lowering), LOOP-004 (variable-divisor div/mod innermost), CALL-002 (complexity ≥15 stat-promotion). Resume signposts: plan mvp-implementation, commit \<P2 hash\>.
- P3 notes: BinaryOpEvent/conditional_guarded/cyclomatic_complexity data all already extracted; consider unifying AllocCandidate resolver_tier while touching alloc family.

## Blockers
- None.
