# Active Context

## Current Task
Session 42 **python-backend W2** (task A-pf-070) **complete**: the Python scanner backend `src/scanner/backends/tree_sitter_python/` (mod/infer/scan + 28 inline tests) implements the shared `Extraction` contract (FileRecord/ParseHealth parity, imports, globals, classes, decorators, functions, typed events), with minimal binding/alias inference, best-effort import resolution, 4 additive IR fields + an additive `AwaitEvent` stream, and `LanguageKind::Python` / `language_of(.py/.pyi)` wiring. `tests/python_backend.rs` (8 integration tests) locks discovery, the synthetic `<module>` record, comprehensions, import resolution (incl. `.pyi`), and parse-health parity.

## Recent Decisions
- **D-pf-192 (live):** W2 outcome + carry (see `state/session-state.json`). Both reviews APPROVED.
- **Analyzer gate:** non-`c`/non-`rust` units are dropped from heuristic buckets until W3's `RuleLanguage::Python` — `.py` files must not emit C/Rust findings.
- **`AwaitEvent` (founder-approved):** a dedicated additive `FunctionUnit.awaits: Vec<AwaitEvent>` carries `await` (meta: span/loop depth/innermost) so parser-free `ASYNC-002` works; the W1 doc's "no field for await" was un-implementable across the seam (`python-backend.md` §3.9/§4/§6 amended).
- **Name-level `__init__.py` re-export: DEFERRED** to T2 `python-resolver` (`python-backend.md` §5 amended); W2 keeps module-level edges.
- **`SUPPORTED_LANGUAGES` stays `["c","rust"]`**; widens with the 1.3 schema bump in W3.
- **`.ipynb` excluded** (unchanged); `.py`/`.pyi` dispatch.

## In Progress
- Nothing — W2 code complete and gated; EOL docs updated; commit pending founder approval.

## Next
- **NEXT SESSION (`/new`) = plan `python-backend` W3:** read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan python-backend` + `docs/architecture/python-backend.md`. Deliver the `PERF-PY-*` catalog (consts + `src/analyzer/heuristics/python/**` as pure IR views), `RuleLanguage::Python` + registry/reporter, and schema MINOR `1.2 → 1.3` (canonical/embedded/golden copies, drift test, CLI `VERSION_SUFFIX`, `tests/schema_1_2.rs` flip, `SUPPORTED_LANGUAGES` widen, aggregator `scan_languages`).
- **W2 carry (decide in W3/W4):** aggregator graphs/stats include Python IR pre-W3; `function_count` counts `<module>`; C/Rust `loc_counts` `len+1` double-count (latent, unfixed).
- **Carry (from M1/M2):** semantic-tier USR-ambiguity guard; `PERF-RS-LOOP-008` deref-chain modelling; `r#` raw-identifier receiver nit; M2 non-blocking notes.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (unchanged).
- **Hygiene:** hot state **8037 B / 8192 B** (~155 B headroom); demoted 4 settled records to the cold ledger this session.

## History
- **L2 state split (2026-09-29):** hot/cold split applied — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in hot state, alignment-audit counter fields added.
