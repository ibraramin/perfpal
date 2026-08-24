# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P4 architecture wave: ARCH-001 include-graph fan-out/cycles → ARCH-003 cross-file name-based call coupling (confidence locked LOW) → ARCH-002 hot-component aggregation LAST; complete aggregator graphs/components/stats incl. every ownership-table sort order (+ determinism tests); charts emitter (include-graph.svg, component-coupling.svg, hot-components.svg per ADR-0002, double-render byte-equality tests); reporter report.md fixed template + Appendix A built-in catalog. **DONE — dual-review passed (@requirements-reviewer APPROVED zero scope creep; @reviewer CHANGES_REQUESTED → fix wave → re-review APPROVE), 163 tests green under pinned 1.92.0.**

## Phase
executing mvp-implementation — P4 architecture wave COMPLETE; next session starts **P5 Tier 2**

## Key Decisions
- **Founder-approved (approval gate):** (1) m6 — unresolved QUOTED includes INCLUDED in metrics.json `graphs.include_graph.edges` (`to:null` explicit, `resolved:false`; angle-unresolved excluded; SVG resolved-internal-only per ADR-0002); (2) components = one per parent directory (root-level → "."), rank composite finding_density↓ → complexity_sum↓ → max_loop_depth↓ → alloc_site_density↓ → path_prefix↑; (3) ARCH-002 fires rank-1 component ONLY via IR-only composite (finding_density excluded — heuristic purity), floor `ARCH_002_MIN_COMPLEXITY_SUM=15`; (4) ARCH-003 = one Info/LOW finding per cross-file edge; (5) chart node cap N=50, pre-layout, degree desc/path asc, truncation rendered in-chart.
- **REGISTRY 15 entries** true lexicographic `[ALLOC-001..003, ARCH-001..003, CALL-001..002, LOOP-001..004, STR-001..003]` — declaration order load-bearing (first-wins dedup).
- **Shared `src/graph.rs`:** `break_cycles` uses DIRECTED reachability, NOT union-find (union-find provably drops DAG edges on diamonds — fixer-proven with failing test); implements ADR-0002 cycle-breaking (sort `(from,to)`, drop lowest-sorted closing edge, self-loops broken + visible as ARCH-001 findings); `topo_layers` longest-path layering. Consumed by both ARCH-001 and the chart emitter.
- **Scanner (W1):** angle-bracket includes now resolve in-repo (ADR-0002 policy — resolved angle = internal edge); stale IR doc comments corrected (angle edges carry raw text unless resolved).
- **Schema DTOs:** `call_graph.edges{from_function,to_name,from_path,to_path:string|null,confidence:"LOW"}`, `components[]{component_id,path_prefix,rank,metrics,finding_count}`, `stats.distributions[{metric,bucket,count}]`; `complexity_sum` + densities are f64 (`number` in schema); `call_graph.to_path` required-but-nullable → explicit null serialized; `IncludeEdgeDto.to` widened to `Option<String>` (artifact already allowed null).
- **ARCH-001 design:** consts `ARCH_001_MAX_INCLUDE_DEPTH=8`, `ARCH_001_HUB_FANOUT=12`; sub-case severities cycles Warn/HIGH, deep-chain Warn/MEDIUM, hub Warn/MEDIUM, unresolved-quoted Info/MEDIUM; internal edges = resolved quoted + angle-in-repo; unresolved raw target name unavailable (resolver nulls it) — accepted limitation.
- **Cross-file matcher:** file-local matches suppress (ADR-0001 tiers), Direct callee_kind only, multi-definition ambiguity → lexicographically smallest path, dedup per (caller function, callee name) keeping earliest span; `pub(crate)` reused by aggregator for call_graph assembly.
- **Review-fix wave (rev-1 CHANGES_REQUESTED → APPROVE):** MAJOR-1 findings `total_order` comparator had been INVERTED since P1 (Info-first/LOW-confidence-first; latent because the old test duplicated the reversed comparator) — flipped to ascending severity/confidence per output-contract §4 + spec-derived regression test independent of the production comparator; MAJOR-2 charts `parent_dir` ""→"." alignment restoring root-level coupling edges; MINORs: `cap_nodes` filters phantom endpoints before degree ranking, deep-chains emit explicitly once-per-file + hub anchors advance past colliding spans, rounding UNIFIED on digit-based HALF-UP `schema::{round_half_up,format_fixed_point}` (both private copies deleted; negative-zero cannot leak), §3 per-language totals moved out of §2 per contract, doc corrections (schema module docs, CHANGELOG ≥8, aggregator symbol-resolution honesty, integer complexity_sum evidence).

## Reviews
- @requirements-reviewer: **APPROVED.** 7/7 requirement clauses PRESENT with evidence; 5/5 founder decisions faithful; zero scope creep (every changed file accounted for, no new dependencies, Cargo.toml byte-matches P0 pins). Caveats were environmental only (sandbox had no shell — gates covered by orchestrator + rev-1 runs).
- @reviewer: **CHANGES_REQUESTED → fix wave → re-review APPROVE.** Round 1: zero Criticals, 2 MAJORs + 5 MINORs + 4 nits. Re-review verified all seven fix sites line-by-line, independently re-ran gates (fmt/clippy/test ✓ 163 passed), validated fresh e2e output against metrics.schema.v1.0.json (PASS), confirmed report §4/document order match. Residual non-blocking: CHANGELOG count clarified (157 at wave close → 163 after fix wave); hub anchor-advance fallback sound + safe.

## Risks
- cargo NOT on PATH in fresh shells (`rustup --no-modify-path`): prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"`.
- ARCH-002 asymmetry (founder-approved, documented): heuristic's IR-only pick may differ from `components[].rank`-1 (which uses finding_density).
- ARCH-003 documented FP classes stand until semantic resolver: same-name static functions across TUs, duplicate names across vendored deps.
- Chart SVG layout performance on very large graphs is reasoned-only (node cap N=50 bounds worst case); hub anchor-advance fallback unreachable while hubs have ≥12 distinct directive spans but degrades safely.
- Prior risks unchanged: tree-sitter-c C99 grammar C11/C17 ERROR-rate (ParseHealth hook covers); STR-002 compound-op blindness; writer crash window + concurrent staging sweep (P6).

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P5 Tier 2**: MEM-002 first (cost C1), then TYPE-001, TYPE-002, MEM-001, LOOP-005, LOOP-006, LOOP-007, LOOP-008 — each as pure IR-view function with documented FP classes and mandatory LOW/MEDIUM confidence floors per catalog-v1 §4. Resume with: plan mvp-implementation, commit \<P4 hash\>.
2. P5 implementer notes: check backend extraction coverage for sizeof/memcpy-family shapes BEFORE writing rules (TYPE/MEM need Shape kinds + access chains — verify what P1 emitted); extend kitchen-sink toward the 23-rule total order; follow tests/arch_fixtures.rs + wave-suite patterns; `stats.distributions` stays reserved-for-future (comparator implemented + tested, population needs a founder-visible proposal).
3. Deferred nits carried forward: str_003 scanf scans all args for %s instead of format position; loop_003 defensive in_loop_depth guard; is_strlen_in_loop duplication; rustdoc backtick nesting; fixtures README stale wording; wave_b magic line numbers; NEW: `FindingEntry.threshold` never populated despite LOOP-001/CALL-002 consts (P6 candidate); golden dirs (P6); GitHub Actions SHA-pinning (P6).

## Uncommitted Files (pending founder-approved commit)
- New: `src/graph.rs`, `src/analyzer/heuristics/arch.rs`, `src/charts/mod.rs` (filled from stub), `src/reporter/mod.rs` (filled from stub), `tests/{arch_fixtures,charts_render,reporter_render,p4_determinism}.rs`, `tests/fixtures/e2e/src/util.c`, `tests/fixtures/ir/PERF-C-ARCH-*__*.json` (×9)
- Modified: `src/lib.rs`, `src/ir/mod.rs`, `src/scanner/mod.rs`, `src/schema/mod.rs` (+ embedded-schema DTO alignment), `src/aggregator/mod.rs`, `src/analyzer/{mod,finding}.rs`, `src/analyzer/heuristics/mod.rs`, `src/cli/mod.rs`, `src/writer/mod.rs`, `tests/{common/mod,wave_a_fixtures,pipeline_e2e,scanner_pipeline}.rs`, `docs/heuristics/catalog-v1.md`, `docs/architecture/modules.md`, `CHANGELOG.md`
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`
- EXCLUDE from commit: `.opencode/memory/**` (session noise)
