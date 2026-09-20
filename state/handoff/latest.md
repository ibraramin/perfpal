# Handoff — Latest

## Objective
Session 22: fix the **guarded-content scanner defect** found while building `examples/` (founder-approved as session 21's next task). Delivered as plan `guarded-scan-fix`.

**Defect:** `extract()` in `src/scanner/backends/tree_sitter_c/mod.rs` walked only DIRECT children of `translation_unit`, so `#include`/`#define`/`struct`/globals — and function definitions — nested in preprocessor conditionals (`#ifndef` guards, the everyday header shape) were invisible.

**Fix:** one recursive `walk_file_level(node, visit)` helper that visits every node but STOPS descending at `function_definition`; both Pass 1 (file-level records) and Pass 2 (functions) use it. Founder-approved breadth = whole file-level tree + guarded functions. `struct_def` now skips anonymous (untagged) structs so the wider reach cannot emit unattributable `struct ''` findings; NAMED typedef/nested structs are collected (ratified, test-locked).

## Phase
POST-MVP. Plans: `mvp-implementation` COMPLETE · `housekeeping-batch` COMPLETE · `semantic-resolver` DONE · `guarded-scan-fix` DONE (this commit).

## Key Decisions
- **Breadth (founder, 2 questions):** include guarded functions (Pass 2); recurse the whole file-level tree, stopping at `function_definition`.
- **Anonymous-struct skip** in `struct_def` (added in the review-fix wave): the whole-tree reach would otherwise collect `typedef struct {…} X;` and inline struct members, producing nameless `struct ''` findings. Named structs stay collected.
- **Measured, not assumed:** ARCH-001 gains exactly **ONE** cycle finding (Warn/HIGH, path `cycle_b.h`, `coupled_with=cycle_a.h`) — the handoff's "+2" was wrong (`break_cycles` drops one lowest-sorted edge per 2-node cycle).

## Evidence
- **Bisect parity restored:** `/var/tmp/bxA` == `bxB` (`t.h include_counts.quoted: 1` + resolved quoted edge); `/var/tmp/g1` == `g2` (one `PERF-C-TYPE-002`).
- **Corpus:** `examples/` **82 → 83** (16 hot / **25** warn / 42 info) default; **84 → 85** `semantic-clang` (ARCH-003 3×MEDIUM, +1 ALLOC-002, +1 STR-003 as before). Only ARCH-001 moved; no other rule count changed.
- **Gates:** `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo test` **252 passed / 0 failed** (golden report comparator clean).
- **Anonymous probes:** `typedef struct {…} TAnon;` → 0 findings; `struct Bag { struct {…} data; int n; };` → 0 findings (was `struct ''` each).

## Reviews
- **@reviewer** — rev-1 CHANGES-REQUESTED (1 MAJOR: anonymous `struct ''` findings from the new breadth; MINORs: false doc, untested scope, thin cycle assertion, hub wording) → fix wave → **rev-2 SHIP** (all closed, gates re-run 252/252, probes clean).
- **@requirements-reviewer** — **CHANGES_REQUESTED** with close-out-only blockers (CHANGELOG known-issue retirement, state/activeContext updates, plan-done, lessons-learned) + runtime repro verification (orchestrator confirmed bxA==bxB, g1==g2) → **all resolved in this ritual commit**.
- **Zero scope creep** confirmed by both.

## Risks
- No new risks. The prior **OPEN DEFECT (guarded-content)** risk is **RESOLVED** (append-only history retained in `session-state.json`).
- Carried (unchanged): corpus HUB fan-out sub-case still deliberately non-firing (threshold counts distinct RESOLVED targets only; this file resolves 2) — documented in `examples/10_cross_file_and_arch.c`.
- Carried (unchanged): release x64-darwin / windows-msvc binaries not byte-reproducible; Node-24 action pins; toolchain 1.92.0 mirrored in 5 places; `cargo deny` unmatched-allowance warning for ISC; examples' non-compiling constructs (`gets`, `missing_*.h`) are intentional demo targets.

## Blockers
- None.

## Next Actions
1. Founder approves the ritual commit → commit the working tree (see Uncommitted Files; **EXCLUDE `.opencode/**`**).
2. Post-MVP milestones (founder picks next session): LOOP-004 resolver-constness follow-up; runtime-profile ingestion (CONFIRMED-1); C++/Python backend; release follow-ups (Node-24 pins, `/Brepro` + darwin-x64 reproducibility, toolchain drift guard); `stats.distributions` activation (MINOR bump on proposal acceptance); housekeeping follow-ups (Cargo.toml PolyForm SPDX, modules.md §8 CLI drift).
3. Optional: `examples/README.md`; extend the corpus so the ARCH-001 hub fan-out sub-case fires (needs ≥12 distinct RESOLVED includes).
4. Session signposts for `/new`: plan `guarded-scan-fix` DONE; read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST.

## Uncommitted Files (this ritual commit)
- **Modified (code):** `src/scanner/backends/tree_sitter_c/mod.rs` (recursive `walk_file_level`, Pass 1/2 rewired, `struct_def` anonymous skip, 10 unit tests)
- **Added (tests):** `tests/guarded_content.rs` (4 end-to-end bisect-parity tests)
- **Modified (corpus):** `examples/cycle_a.h`, `examples/cycle_b.h`, `examples/10_cross_file_and_arch.c` (caveats retired; counts/anchors corrected)
- **Modified (docs/state):** `CHANGELOG.md`, `docs/activeContext.md`, `docs/lessons-learned.md`, `docs/plans/guarded-scan-fix.md`, `state/session-state.json`, `state/handoff/latest.md`
- **EXCLUDE:** `.opencode/**` (session-tooling noise, incl. mode-only `rules.md`/`spec.md` changes)
- Working tree otherwise clean at `280a20a`.
