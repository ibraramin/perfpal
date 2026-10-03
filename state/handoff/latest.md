# Handoff — Latest

## Objective
Session 42 **COMPLETE** — plan `python-backend` **W2** (task A-pf-070): the Python scanner backend. Product change = a new `src/scanner/backends/tree_sitter_python/` directory (mod/infer/scan/tests, **2332 lines**), the integration test `tests/python_backend.rs` (**380 lines**), additive IR synthesis in `src/ir/mod.rs`, scanner wiring + best-effort import resolution in `src/scanner/mod.rs`, an analyzer language gate in `src/analyzer/mod.rs`, and a design-doc amendment in `docs/architecture/python-backend.md`. No schema bump; no `PERF-PY-*` catalog (W3).

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 → W0 ✅)** · **`python-backend` ACTIVE (W0 ✅ → W1 ✅ → W2 ✅ → W3 next)**.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance heuristics, (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified metrics, so humans see bottlenecks and LLMs optimize with explicit signals instead of guessing (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; `evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 → W0 ✅); `python-backend` ACTIVE (W0 ✅ → W1 ✅ → **W2 ✅ → W3 next**).
- **How this session serves the north-star:** it turns the pinned Python grammar + W1 design into a working extraction tier — real Python IR (records/events + binding/alias inference + import graph) that the W3 catalog will score. Python is the highest AI-code share, directly serving "quantified signals instead of guessing."
- **Drift flag: NONE.** W2 is exactly the queued next task (`state/handoff/latest.md` prior: "NEXT SESSION = W2"). Periodic audit not due (`sessions_since_alignment_audit` 2 → 3; threshold 10).

## This session (42) — python-backend W2
- **Backend module.** `src/scanner/backends/tree_sitter_python/` = `mod.rs` (722; `extract` orchestration, file-level pass, synthetic `<module>` record, classes/decorators/globals, LOC/ParseHealth), `infer.rs` (378; import-alias map, single-assignment `BindingKind` lattice, receiver root, param annotations), `scan.rs` (794; `FuncScan` event engine: calls/assignments/binary/comparison/access chains/loops/comprehensions/awaits), `tests.rs` (438; **28 inline tests**). Mirrors the Rust sibling's structure/idioms.
- **W1 obligations implemented:** comprehensions as real `LoopNode{For}` (each `for_in_clause` one depth; `if_clause` = complexity only) with clause iterables visited at the clause's OUTER depth (once-evaluated fix); one synthetic `<module>` record per file (`is_module_scope`, captures top-level loops, stops at nested `def`/`class`); best-effort import resolution (relative + packages + `__init__.py` module edges; unresolved-internal → `to_path=None`, external keeps raw; `.ipynb` excluded).
- **Additive IR (serde-default, IR-internal, no schema bump):** `FunctionRecord.is_async`, `FunctionRecord.is_module_scope`, `AssignmentEvent.binding_kind` + new `enum BindingKind` (10 kinds, snake_case), `CallEvent.receiver_root` (omitted on `None`). **Founder-approved addition:** dedicated `AwaitEvent { meta }` + `FunctionUnit.awaits: Vec<AwaitEvent>` so parser-free `ASYNC-002` has an await signal (the W1 doc's "no field for await" was un-implementable across the node-kind seam).
- **Wiring:** `LanguageKind::Python`; `language_of(.py/.pyi)`; `extract_unit`/`failed_unit` arms; `.py/.pyi` in the Python module index. **`SUPPORTED_LANGUAGES` intentionally left `["c","rust"]`** (widens with the 1.3 schema bump in W3).
- **Review fixes (all founder-approved):** (1) analyzer gate — non-`c`/non-`rust` units are dropped from heuristic buckets so `.py` files no longer emit bogus `PERF-C-*`; (2) `keyword_argument` now carries `name`+`value` into the arg `Shape` (needed by `PANDAS-001`); (3) `AwaitEvent` stream; (4) `__init__.py` name-level re-export explicitly DEFERRED to T2 `python-resolver` (doc amended); (5) `.pyi` resolvable as an import target; (6) bare-relative plain-name alias.

## Verification
| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0, 0 warnings |
| Full suite | `cargo test --locked` | **438 passed; 0 failed** |
| Python inline | `cargo test --locked --lib tree_sitter_python` | **28 passed; 0 failed** |
| Python integration | `cargo test --locked --test python_backend` | **8 passed; 0 failed** |
| Analyzer gate | `cargo test --locked --test analyzer_registry` | 4 passed; 0 failed |
| Grammar link (W1) | `cargo test --locked --test python_grammar_link` | 2 passed; 0 failed |
| No copyleft | `cargo deny check licenses` | `licenses ok` |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` | exit 0 |
| Hot ≤ 8192 B | `wc -c < state/session-state.json` | **8037** |

**Dual review:** `@requirements-reviewer` round-1 **APPROVED**; round-2 re-review **APPROVED** (all W2 obligations + founder additions present; no scope creep). `@reviewer` round-1 **CHANGES_REQUESTED** (3 blocking: analyzer gate, `keyword_argument` name, missing `await` representation) → all fixed (fixes 1–7) → round-2 re-review **APPROVED** (all 3 RESOLVED; fixes non-vacuous; no new blocking issues). **No unresolved reviewer disagreement** — the two seats agreed on the final state.

## Risks / Open (append-only; hot ledger demoted the settled W1 decision)
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY contact placeholder), **`R-pf-087`/`R-pf-088`** (Python M3 design/retire conflict — discharged into the W1 doc), **`R-pf-091`/`R-pf-092`** (W0 obligations — discharged into the W1 doc), **`R-pf-089`** (no Python precision coverage until W4).
- **Blockers (hot, unchanged):** **`B-pf-001`** (real contact address for EULA/PRIVACY before tag); **`B-pf-002`** (repo secrets `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` before tag).
- **W2 carry (recorded in D-pf-192, to decide in W3/W4):** (a) the aggregator still builds `graphs.call_graph` and `stats.functions[]`/`components[]` over the full unit set, so Python IR appears in emitted `metrics.json` graph/stats sections before Python rules exist; (b) `function_count` counts the synthetic `<module>` record and `stats.functions[]` does not filter `is_module_scope`; (c) **latent bug found (not fixed, out of W2 scope):** the C/Rust backends' `loc_counts` advance `offset += len + 1` while `split_lines` uses `split_inclusive` (the newline is already in `len`) → per-line byte-offset drift that can misclassify short comments late in a file. The Python backend uses the correct `offset += len`. Fixing C/Rust changes existing LOC/goldens → defer.
- **Hygiene note:** hot `state/session-state.json` is **8037 B / 8192 B cap** (~155 B headroom); 4 settled records (D-pf-191, A-pf-067/069/070) were demoted to cold `state/archive/ledger.jsonl` this session (append-only; 485 → 489 records).

## Next Actions
1. **NEXT SESSION (`/new`) = plan `python-backend` W3:** read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST, then `readPlan python-backend` and `docs/architecture/python-backend.md`. Deliver: the `PERF-PY-*` rule-id consts + `src/analyzer/heuristics/python/**` (the ~13 mirrors + native rules as pure IR-view functions with CPython-mechanism-cited messages), `RuleLanguage::Python` + ascending registry entries + reporter catalog, and the schema MINOR `1.2 → 1.3` (canonical + embedded + golden byte-copies, drift test, CLI `VERSION_SUFFIX`, `tests/schema_1_2.rs` flip, `SUPPORTED_LANGUAGES` widen, aggregator `scan_languages`).
2. Then **W4** `examples-python/` (real package) + parser-free IR fixtures + precision KPI + benchmark ladder + goldens + docs + full gate matrix + dual review.

## Change set (this session)
- **Added:** `src/scanner/backends/tree_sitter_python/{mod,infer,scan,tests}.rs`; `tests/python_backend.rs`.
- **Modified:** `src/ir/mod.rs`; `src/scanner/backends/mod.rs`; `src/scanner/mod.rs`; `src/scanner/backends/tree_sitter_c/mod.rs`; `src/scanner/backends/tree_sitter_rust/mod.rs`; `src/analyzer/mod.rs`; `src/aggregator/mod.rs`; `src/resolver/mod.rs`; `tests/analyzer_registry.rs`; `tests/common/mod.rs`; `docs/architecture/python-backend.md`; `state/session-state.json`; `state/archive/ledger.jsonl`; `state/handoff/latest.md`; `docs/activeContext.md`; `CHANGELOG.md`; `docs/lessons-learned.md`.
- **Excluded / untracked:** `brag-output/` and `perfpal-report/` — unrelated working-tree dirs, deliberately NOT in this change set.

## Commit
1. **`6b5c8ea`** — work commit: Python backend W2 (backend dir + `tests/python_backend.rs` + IR/wiring/analyzer changes + design-doc amendment + state/docs).
2. (this EOL follow-up commit) — regenerates this handoff to embed `6b5c8ea`; marks activeContext W2 committed.

## L2 state split / L6 north-star (history)
The L2 hot/cold split landed **2026-09-29**: hot `state/session-state.json` is compact schema v2 and ≤ 8 KB, with full history in cold `state/archive/ledger.jsonl` (greppable, never auto-read). The L6 north-star (`docs/NORTH-STAR.md`, founder-owned goal / non-goals / stage / metrics) was pinned **2026-10-01** and referenced by `north_star_ref` in hot state.
