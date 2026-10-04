# Active Context

## Current Task
Session 43 **python-backend W3** (task A-pf-071) **complete**: the full `PERF-PY-*` catalog — **22 rules** = 13 reframed C/Rust mirrors (`ALLOC-001/003`, `STR-001`, `LOOP-001/002/003/007`, `MEM-002`, `CALL-001/002`, `ARCH-001/002/003`) + 9 Python-native (`LOOP-009`, `PANDAS-001/002`, `ALLOC-004`, `N1-001`, `ASYNC-001/002`, `COMPREHENSION-001`, `EXECUTOR-001`); the 10 retired rules are NOT registered. `src/analyzer/heuristics/python/**` (mod + 11 family files) implements every rule as a pure IR-view fn with CPython-mechanism-cited messages, inference-gated at MEDIUM, never `Semantic`. `RuleLanguage::Python` + 22-entry registry, `analyze()` python bucket, reporter catalog 45→67 + completeness lock, `SUPPORTED_LANGUAGES=["c","python","rust"]`, aggregator `scan_languages` fallback, and schema MINOR **1.2 → 1.3** admitting the `"python"` token (canonical + embedded byte-identical + 3 golden copies; `tests/schema_1_2.rs` → `tests/schema_1_3.rs`).

## Recent Decisions
- **D-pf-193 (live):** W3 outcome summary — 22-rule catalog, registry/reporter/aggregator wiring, schema 1.3, Option A IR, fidelity fixes (see `state/session-state.json`). Both reviews APPROVED after round-1 fixes.
- **Option A (founder-approved):** additive Python IR completion — `ComparisonEvent.operator` (skip-if-empty → C/Rust byte-neutral) + new `TryEvent` / `FunctionUnit.try_blocks` (serde-default) so `LOOP-007`/`LOOP-009` are real, not dead. IR events never reach `metrics.json`.
- **Rule-fidelity fixes:** ALLOC-003 accumulator gate (polarity); STR-001 `+`/`+=` with accumulator gate; LOOP-007 membership + lone `seq[i]` chains (set/dict excluded); LOOP-009 off `try_blocks` at depth ≥ 1; ARCH-003 dead Semantic branch removed.
- **Registry ordering:** the 22 `PERF-PY-*` entries sit between the C and Rust blocks, global ascending rule_id (declaration order is load-bearing for first-wins dedup).
- **Schema 1.3:** MINOR additive — `scan.languages` / `files[].language` admit `"python"`; version consts + artifact name → 1.3; goldens regenerated.

## In Progress
- Nothing — W3 complete and committed as `2a80882` (this EOL follow-up commit on top).

## Next
- **NEXT SESSION (`/new`) = plan `python-backend` W4 (`A-pf-072`):** read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan python-backend`. Deliver `examples-python/` (real package) + parser-free IR fixtures + precision KPI + benchmark ladder + goldens + docs (`ir-v1.md` / `metrics-json-v1.md` / `README` / roadmap) + the full gate matrix (default **and** `semantic-clang`) + dual review before commit.
- **New carry `R-pf-093` (low):** LOOP-007 misses `x.index()` and `x in [a,b]` list-literal membership; LOOP-007 index branch fires at depth ≥ 1 (single linear `for i in range(len(seq)): use(seq[i])` gets an O(n²)-flavoured message — accepted approximation); `arch.rs:128-131` unused `key.0`; `async_rules.rs:68` `is_open` ignores receiver root.
- **W4 carry (prior):** aggregator graphs/stats include Python IR; `function_count` counts `<module>`; C/Rust `loc_counts` `len+1` double-count (latent, unfixed).
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard (`R-pf-077`); `PERF-RS-LOOP-008` deref-chain modelling (`R-pf-078`); `r#` raw-identifier receiver nit (`R-pf-080`); M2 non-blocking notes.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Hygiene:** hot state **8153 B / 8192 B**; demoted/discharged 13 records to the cold ledger this session (489 → 502 records).

## History
- **L2 state split (2026-09-29):** hot/cold split applied — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in hot state, alignment-audit counter fields added.
