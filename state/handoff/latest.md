# Handoff — Latest

## Objective
Session 21 (second task of the session; the first — plan `semantic-resolver` — landed as commit `7ea2989`). Founder asked for a set of 10 C programs densely packed with the issues Perfpal detects, placed in an example directory, plus step-by-step usage instructions.

**Delivered:** `examples/` — 10 issue-dense C programs (`01_allocator_churn.c` … `10_cross_file_and_arch.c`) + 3 supporting headers (`perf_common.h`, `cycle_a.h`, `cycle_b.h`), 894 lines. Verified against real tool output: **all 23 catalog rules fire**; default build **82 findings (16 hot / 24 warn / 42 info)**; `semantic-clang` build with libclang **84** (+1 ALLOC-002 proven-`fclose`, +1 STR-003 resolved-format, 3 ARCH-003 edges LOW→MEDIUM). Each program's header comment lists exactly what it emits, every claim checked against output; deliberate silent controls planted (blockwise copy, `alloc_correct`/MEM-001, `print_padded`, `scan_cstr` sentinel, `fib_iterative`, `sum_direct`). No product source touched — `examples/` is the only new path.

**Building it surfaced a real scanner defect** (see Risks). Founder approved: ritual commit now; the guarded-content fix is the next session's task.

## Phase
POST-MVP — v0.1.0 released (tag `f368133`); housekeeping landed (`a84c1f0`); semantic resolver landed (`7ea2989`, plan DONE); examples corpus landing in this commit. Plans: `mvp-implementation` COMPLETE · `housekeeping-batch` COMPLETE · `semantic-resolver` DONE.

## Key Decisions
- **Corpus shape:** one program per heuristic family, dense rather than realistic-minimal; every file documents its own expected findings (the corpus doubles as living documentation of the catalog).
- **Fidelity over tidiness:** the planted include cycle is kept (guarded, realistic) even though it does not report today, and the limitation is documented *in the corpus* rather than hidden — so it doubles as the repro for the defect.
- **Review-driven corrections (12 items, all fixed):** false-silence claims, self-contradictory severities, ARCH-003/ARCH-002 mis-attribution, LOOP-004 labels on compound assignments the scanner does not model (`/=` `%=` — rewritten to binary form so they genuinely fire), size comments, and a missing prototype for `build_row_labels`.
- **Founder verdicts:** commit corpus + ritual together; next session = fix the guarded-content bug (chosen over bug-fix-plus-demo and over alternative milestones).
- **Instructions delivered in chat** (requirements-reviewer: satisfies the verbatim request; a durable `examples/README.md` is recommended, not required).

## Reviews
- **@reviewer — CHANGES-REQUESTED → all 12 items fixed → targeted re-check 11/12 CLOSED, then the last fixed** (`04_stdio_churn.c` inline `fclose … (hot)` → `(guarded -> warn)`). No open items.
- **@requirements-reviewer — APPROVED:** exactly 10 `.c` files; density verified; `examples/` plural accepted; in-chat instructions judged sufficient; no scope creep (no tracked source/doc modified).

## Risks
- **OPEN DEFECT (next session's target): guarded file-level declarations are invisible.** `src/scanner/backends/tree_sitter_c/mod.rs` Pass 1 (~97-120) walks only direct children of `translation_unit`, so `#include`, `#define`, `struct` and global declarations nested in preprocessor conditionals (`#ifndef` guards — the everyday header shape) are never collected. Impact: ARCH-001 cycles/hub/deep-chain, the macro-recognizer `macro_suspect` downgrade hooks, TYPE-001/002, and LOOP-005 scope classification all miss guarded content. Bisect proofs: `/var/tmp/bxA` (unguarded) vs `/var/tmp/bxB` (guarded) — `t.h` `include_counts.quoted` 1 vs 0; `g1` vs `g2` — 1 vs 0 TYPE-002 findings. Fix direction: recursive Pass 1 walk (stop at `function_definition` boundaries so locals are not double-collected) + regression tests incl. a guarded include cycle.
- Corpus-fidelity caveat: the two planted ARCH-001 sub-cases do not fire today — the cycle (defect above) and hub fan-out (the ≥12 threshold counts only distinct *resolved* targets; this file resolves 2). Documented in-file.
- Pre-existing/unrelated: `cargo deny` prints one unmatched-allowance warning for ISC; the examples' non-compiling constructs (`gets`, unresolved `missing_*.h`, cross-file calls) are intentional demo targets and show as clangd errors.

## Blockers
- None.

## Next Actions
1. **Next session (`/new`) = fix the guarded-content scanner bug** (founder-approved). Recursive file-level pass; regression tests for guarded include/define/struct/global + a guarded include cycle; then retire the corpus caveats (remove the "does NOT report" notes in `examples/cycle_a.h`, `examples/cycle_b.h`, `examples/10_cross_file_and_arch.c`, update their expected-findings lists — ARCH-001 gains 2 cycle findings at Warn/HIGH — and expect corpus counts to change). Plan + spec via planning-toolkit before code, dual review, then commit.
2. Optional examples follow-ups: durable `examples/README.md`; extend the corpus so the ARCH-001 hub fan-out sub-case fires (needs ≥12 distinct resolved includes).
3. Remaining post-MVP milestones: LOOP-004 resolver-constness follow-up; runtime-profile ingestion (CONFIRMED-1); C++/Python backend; release follow-ups (Node-24 pins, `/Brepro` + darwin-x64 reproducibility, toolchain drift guard); `stats.distributions` activation (MINOR bump on proposal acceptance); housekeeping follow-ups (Cargo.toml PolyForm SPDX, modules.md §8 CLI drift).
4. Session signposts for `/new`: examples corpus LANDED (this commit); plan `semantic-resolver` DONE; read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST.

## Uncommitted Files (this ritual commit)
- **Added:** `examples/` (13 files: 10 `.c` + 3 `.h`)
- **Modified:** `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`
- **EXCLUDE:** `.opencode/**` (session-tooling noise)
- Working tree is otherwise clean at `7ea2989`.
