# Handoff — Latest

## Objective
Close the implementation-planning fork: decide implementation language per modules.md §6 criteria and set build order (Tier 1 before Tier 2). **DONE — Rust resolved (fold-in in modules.md §6), master plan `mvp-implementation` created (P0–P6), dual review passed, all 4 reviewer findings applied.**

## Phase
executing mvp-implementation — planning complete; next session starts P0 scaffold

## Key Decisions
- **LANGUAGE = RUST** (founder-approved 2026-08-23): passes all six §6 criteria. Verified facts: official org-published crates `tree-sitter` v0.26.x + `tree-sitter-c` v0.24.x (MIT); Go rejected (official go-tree-sitter is cgo-bound → CGO_ENABLED=0 impossible, per-target C cross-toolchain matrix; smacker stale >2yrs); C rejected (criterion 6 dev velocity); Python stays ruled out (criterion 1, no frozen-binary exception sought). Fold-in recorded in modules.md §6 Outcome table — no standalone ADR (founder preference). Core crates audited MIT OR Apache-2.0.
- **BUILD ORDER = master plan `mvp-implementation`** (linked perfpal-core-spec): P0 scaffold → P1 vertical slice (ALLOC-001 end-to-end + span round-trip tests) → P2 Tier 1 wave A (7 function-local C1: ALLOC-002, STR-001/002/003, LOOP-001/003, CALL-001) → P3 wave B context-dependent shapes (ALLOC-003, LOOP-002, LOOP-004, CALL-002) → P4 architecture wave (ARCH-001→ARCH-003→ARCH-002 LAST; full aggregator sort orders + determinism tests; charts; reporter) → P5 Tier 2 (MEM-002 first, then TYPE-001/002, MEM-001, LOOP-005–008) → P6 hardening/release (musl static x86_64/aarch64, macOS, MSVC). One phase per session; dual-review gate between phases.
- **Standing obligations pinned at scaffold:** serde_json `preserve_order` OFF (byte-determinism guard), dependency license audit in CI (§6 criterion 4), tree-sitter version pinning (ADR-0001 G5).

## Reviews
- @requirements-reviewer: **APPROVED** — both deliverables verbatim (language decided per §6 criteria; build order Tier 1 before Tier 2); zero implementation code; no scope creep.
- @reviewer: **SHIP-WITH-FIXES** — all 4 applied (MINORs: P3 relabeled from "cost C2" to "context-dependent shapes" since LOOP-004/CALL-002 are catalog C1; span round-trip + aggregator sort-order determinism tests explicitly scheduled P1/P4; NITs: §7 header "(language-neutral)"→"(Rust)" + Cargo.toml/Cargo.lock added to tree; preserve_order-OFF guard noted in §6 row 3).

## Risks
- tree-sitter-c C99 grammar vs C11/C17 constructs → ERROR-node monitoring carries into implementation (unchanged).
- Hand-rolled DAG layout quality unproven on real include graphs; MVP bar "legible not pretty" (unchanged).
- Cross-file coupling LOW-confidence until semantic resolver tier exists (unchanged).

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P0 scaffold** (Cargo crate per §7, CI incl. license audit, fixture-IR harness, version/feature pinning). Resume with: plan mvp-implementation, commit \<this hash\>.

## Uncommitted Files
- `docs/architecture/modules.md` (§6 fold-in RESOLVED=Rust + §7 Rust crate layout)
- `docs/plans/mvp-implementation.md` (new, toolkit-managed)
- `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md` (EOL ritual updates)
