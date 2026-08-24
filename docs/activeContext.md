# Active Context

## Current Task
Plan `mvp-implementation` **P4 architecture wave COMPLETE** (session 14): 3 ARCH heuristics as pure IR-view functions completing the 15-rule Tier 1 registry — ARCH-001 (include-graph cycles via deterministic `break_cycles` shared with charts, deep chains ≥8, hubs fan-out/in ≥12, unresolved-quoted; Warn band), ARCH-003 (cross-file name-based call coupling: file-local-first, Direct-only, smallest-path ambiguity rule, per-edge Info/LOW locked by schema), ARCH-002 LAST (hot-component aggregation: rank-1 component only via IR-only composite complexity_sum↓→loop_depth↓→alloc_density↓→prefix↑, floor 15, Hot/HIGH anchored at highest-complexity function). Aggregator completed: call_graph (LOW const, aggregator-owned re-sort), components[] (per-dir, founder-approved 4-key+prefix rank composite), m6 admission (unresolved quoted edges as data `to:null`). Charts emitter: 3 byte-deterministic SVGs per ADR-0002 (cycle-break/topo/barycenter×3/half-up, XML escape, cap N=50, emit-iff-non-empty). Reporter: report.md §5 template (8 sections, stable severity partition preserving document order, Markdown escaper, Appendix A frozen catalog, double-render modulo provenance). CLI now emits the full `perfpal-report/` layout (metrics.json + bundled schema + report.md + charts/*.svg). Review-fix wave fixed both MAJORs (total_order direction inverted-since-P1; coupling root-edge drop) + 5 MINORs (phantom caps, deep-chain/hub collision, unified HALF-UP formatter in schema, §3 language totals, docs). Dual review: @requirements-reviewer APPROVED (7/7 clauses, 5/5 founder decisions, zero scope creep); @reviewer CHANGES_REQUESTED → fixes → **re-review APPROVE** (gates independently re-run, e2e output schema-valid). **163 tests green** under pinned 1.92.0. Commit pending founder approval.

## Recent Decisions
- Founder-approved P4 gate: m6 unresolved-quoted-in-document; per-dir components + density-led rank; ARCH-002 rank-1 IR-only floor 15; ARCH-003 per-edge findings; chart cap N=50.
- break_cycles = directed reachability (union-find wrong on DAG diamonds); self-loops broken + visible.
- Scanner resolves angle includes in-repo (ADR-0002 internal-edge policy).
- Unified digit-based HALF-UP formatter lives in `schema::{round_half_up,format_fixed_point}`; charts/reporter consume it.
- total_order comparator corrected to contract direction (hot→warn→info, HIGH→MEDIUM→LOW) — was inverted since P1.

## In Progress
- Nothing — task boundary reached; awaiting commit approval.

## Next
- Founder approves commit → fresh session starts **P5 Tier 2**: MEM-002 first, then TYPE-001, TYPE-002, MEM-001, LOOP-005–008 (documented FP classes, LOW/MEDIUM confidence floors per catalog-v1 §4). Resume signposts: plan mvp-implementation, commit \<P4 hash\>.
- P5 notes: verify backend Shape/access-chain coverage for sizeof/memcpy rules BEFORE writing them; kitchen-sink extends toward 23-rule order; distributions stays reserved-for-future.

## Blockers
- None.
