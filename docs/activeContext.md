# Active Context

## Current Task
Session 21 (second task): **`examples/` demo corpus delivered, reviewed, and landing in this commit.** 10 issue-dense C programs + 3 supporting headers (894 lines) exercising the whole catalog — all 23 rules fire (82 findings default: 16 hot / 24 warn / 42 info; 84 with `semantic-clang` + libclang). Every program documents its own expected findings, verified against real output; deliberate silent controls confirm the false-positive guards. Step-by-step usage instructions delivered in chat.

Building it found a **real scanner defect**: file-level declarations nested in preprocessor conditionals are invisible (`src/scanner/backends/tree_sitter_c/mod.rs` Pass 1 walks only direct children of `translation_unit`), so guarded `#include`/`#define`/`struct`/globals are dropped — affecting ARCH-001 cycles/hub/deep-chain, `macro_suspect` hooks, TYPE-001/002, LOOP-005.

## Recent Decisions
- Corpus shape: one program per family, dense, self-documenting; fidelity over tidiness (the non-reporting include cycle is kept and documented as the defect's repro).
- 12 reviewer items fixed (false-silence claims, severity contradictions, ARCH attribution, LOOP-004 labels on unmodelled compound assignments → rewritten to binary form, size comments, missing prototype).
- Founder: commit corpus + ritual together; **next session = fix the guarded-content bug**.
- Instructions stay in chat (requirements-reviewer: sufficient; durable README recommended-not-required).

## In Progress
- Nothing — task boundary reached; ritual commit pending approval to execute (already approved by founder: `Yes — ritual + commit`).

## Next
- **Next session (`/new`): fix the guarded-content scanner bug** — recursive Pass 1 walk + regression tests (guarded include/define/struct/global + guarded cycle); then retire the corpus caveats and update expected-findings lists (ARCH-001 gains 2 cycle findings).
- Optional: `examples/README.md`; corpus extension to make ARCH-001 hub fan-out fire.

## Blockers
- None.
