# Active Context

## Current Task
Session 22: **guarded-content scanner defect FIXED** (plan `guarded-scan-fix`, dual review SHIP). `extract()` in `src/scanner/backends/tree_sitter_c/mod.rs` routes both Pass 1 (file-level records) and Pass 2 (functions) through one recursive `walk_file_level` helper that descends the whole file-level tree but STOPS at `function_definition` boundaries. Guarded `#include`/`#define`/`struct`/globals AND guarded function definitions are now collected; function-local declarations/structs stay uncollected.

Verified: `/var/tmp/bxA` == `bxB` (`t.h include_counts.quoted: 1` + resolved edge) and `/var/tmp/g1` == `g2` (`PERF-C-TYPE-002` fires). Corpus caveats retired; `examples/` counts **82 → 83** (16 hot / **25** warn / 42 info) default, **84 → 85** `semantic-clang`. ARCH-001 now gains exactly **ONE** cycle finding (Warn/HIGH, path `cycle_b.h`, `coupled_with=cycle_a.h`) — the handoff's "+2" prediction was wrong (`break_cycles` drops one lowest-sorted edge per 2-node cycle). Anonymous untagged structs are skipped so the wider reach cannot emit `struct ''`. 252 tests green; fmt/clippy clean.

## Recent Decisions
- Founder-approved breadth: whole file-level tree recursion + include guarded functions (Pass 2).
- `struct_def` skips anonymous structs (no tag name ⇒ unattributable findings); NAMED typedef/nested structs ARE collected (ratified breadth, test-locked).
- Corpus caveat retirement: cycle now reports; HUB fan-out caveat retained (still non-firing — threshold counts only distinct RESOLVED targets).

## In Progress
- Nothing — task boundary reached; EOL ritual commit pending founder approval.

## Next
- Commit `guarded-scan-fix` (working tree: scanner fix + tests + corpus comments + state/docs/changelog).
- Post-MVP: LOOP-004 resolver-constness; runtime-profile ingestion; C++/Python backend; release follow-ups; optional `examples/README.md` + hub-fanout corpus extension.

## Blockers
- None.
