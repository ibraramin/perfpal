---
plan name: housekeeping-batch
plan description: License, docs, catalog, schema proposal
plan status: done
---

## Idea
Post-release housekeeping batch for the published (private) repo github.com/ibraramin/perfpal at v0.1.0 (tag f368133, ritual e0928b8). Founder-approved scope: (1) LICENSE = PolyForm Strict v1.0.0 verbatim (matches F12: free to use; modification/redistribution require explicit permission; no copyleft deps — already audited); repo stays private for now. (2) README.md via docs-writer read-code-first — what Perfpal is, 23-heuristic catalog summary, install from GitHub release assets, CLI flags/exit codes per modules.md §8, output layout per output-contract v1, determinism scope (charts/metrics only), license note. (3) STR-003 residual FP classes documented durably in catalog-v1.md §4 (%%s escaped-percent substring can false-fire contains('%s'); non-literal concatenated format args stay silent — conservative direction). (4) stats.distributions v1 population proposal doc (complexity histogram, loop-depth histogram) — stays reserved-for-future in schema; activating later = minor bump per schema-versioning policy. Zero src/ changes; 206-test suite is the regression net.

## Implementation
- LICENSE: fetch PolyForm Strict v1.0.0 canonical text (librarian-verified verbatim against polyformproject.org), write LICENSE at repo root with copyright line 'Copyright (c) 2026 ibraramin' (founder may amend holder name at review).
- README.md via @docs-writer (read-code-first, never invent): overview + heuristics catalog summary (23 rules, tiers), install via release binaries, quickstart (perfpal [path] -o perfpal-report), flags + exit-code matrix from modules.md §8, perfpal-report/ layout per output-contract v1, determinism scope statement, PolyForm Strict license section.
- STR-003 note: append accepted-FP-class implementation note to docs/heuristics/catalog-v1.md §4 — '%%s' escaped-percent substring + non-literal concatenated format args (silent, conservative) — wording aligned with session-state open_risks entry.
- stats.distributions proposal: docs/schema/stats-distributions-proposal.md defining v1 populations (cyclomatic-complexity histogram, max-loop-depth histogram) with bucket rules matching existing deterministic rounding conventions; explicit statement that schema stays reserved-for-future until activated (minor bump path per schema-versioning policy).
- Validation + ritual: cargo fmt --check / clippy -D warnings / test --locked all green (no src/ changes expected); CHANGELOG [Unreleased] Added entries; state/session-state.json append-only + handoff + activeContext regen; dual review (@reviewer + @requirements-reviewer); founder-approved commit.

## Required Specs
<!-- SPECS_START -->
- perfpal-core-spec
<!-- SPECS_END -->