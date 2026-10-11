# Handoff — Latest

## Objective
Session 44 **COMPLETE** — plan `python-backend` **W4** (task A-pf-072): the Python tier's **proof + docs** package. New `examples-python/` (a 20-file real package) fires **all 22 `PERF-PY-*` rules exactly** (29 findings: 0 hot / 12 warn / 17 info) and is locked by `tests/examples_python_coverage.rs` (22-id set-equality + 19 silent controls). 50 parser-free IR fixtures + `tests/python_negative_fixtures.rs`. The M1 precision KPI gains **6 Python rules** at 1.0/1.0. The benchmark `small` rung widens 45 → **67 rules**. A 4th golden tree `report-python` is added. Docs + `docs/evidence/dogfood-python-backend.md`. **M3 (Python backend) is now COMPLETE.**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 ✅ COMPLETE)** · **`python-backend` ✅ COMPLETE (W0–W4)**.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance heuristics, (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified metrics, so humans see bottlenecks and LLMs optimize with explicit signals instead of guessing (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; `evidence-beta` M1 ✅ · M2 ✅ · **M3 ✅ COMPLETE**; `python-backend` W0 ✅ → W1 ✅ → W2 ✅ → W3 ✅ → **W4 ✅**.
- **How this session serves the north-star:** it makes the 22 Python signals *evidence-backed* — a real corpus, a precision ratchet, a benchmark rung, and a golden lock — satisfying the north-star metric "per-rule precision/recall ratchets to 1.0/1.0 and is never lowered." Python is the third first-class language (highest AI-code share).
- **Drift flag: NONE.** W4 is exactly the queued next task. Audit counter 4 → 5 (threshold 10; not due).

## This session (44) — python-backend W4
- **Lane 0 — scoped `R-pf-093` fixes** (`src/analyzer/heuristics/python/`): removed the dead `key.0` tuple in `python_arch_002`; `python_async_001` now treats only the **builtin** `open` as blocking (module-qualified `aiofiles.open` no longer a false positive); `python_loop_007` gained a third `.index()` shape (O(n) list scan). The `R-pf-093` **list-literal** half was intentionally skipped — the backend collapses all literal containers to one shape, so a list literal is indistinguishable from a set/dict/tuple (disclosed in code, CHANGELOG, and the dogfood doc).
- **Lane 1 — `examples-python/` + coverage gate.** A 20-file real Python package (`__init__.py` + 15 numbered family modules + a `pkg/` subpackage holding the `cycle_a` ↔ `cycle_b` import-cycle pair and the cross-module helper). All 22 `PERF-PY-*` rules fire exactly; `tests/examples_python_coverage.rs` locks the 22-id set-equality, zero C/Rust leakage, `scan.languages==["python"]`, 20 files, the pinned total/severity split (29 = 0/12/17), per-rule counts, and 19 silent controls (anti-typo needle + span-overlap + non-vacuity).
- **Lane 2 — parser-free IR fixtures.** 50 `tests/fixtures/ir/PERF-PY-*__*.json` + `tests/python_negative_fixtures.rs` (not-in-loop, error-region downgrade, per-rule suppression, positive exact-count locks).
- **Lane 3 — precision KPI.** `tests/fixtures/precision/python/**` (13 files) + `labels.json` +6 rules (`ALLOC-001`, `ALLOC-003`, `STR-001`, `LOOP-007`, `ARCH-003`, `MEM-002`) — all at **1.0/1.0**.
- **Lane 4 — benchmark ladder.** `tests/benchmark/mod.rs` `active_rule_ids()` 45 → **67**; `corpus.rs` new `py_ext()` (22 templates, the six KPI fixtures lifted verbatim so `assert_small_anchors_match_m1` holds); `baseline.json` +22; the real-code rung now also scans `examples-python/`.
- **Lane 5 — goldens.** 4th `GoldenTree` (`report-python`, fixture `examples-python/`) in `tests/p6_hardening.rs`; committed golden tree byte-stable across two regens.
- **Lane 6 — docs.** README (67 heuristics + a `### Python backend (PERF-PY-*)` table + `.py`/`.pyi` + exit-code set + schema 1.3), roadmap (status snapshot, M3 COMPLETE), modules.md, ir-v1.md, metrics-json-v1.md (§5 v1.2→v1.3), python-backend.md (W2–W4 ✅), benchmark-suite.md + benchmark README (45→67), CHANGELOG, and new `docs/evidence/dogfood-python-backend.md`.

## Verification
| Claim | Command | Result |
|---|---|---|
| Full suite (default) | `cargo test --locked` | **526 passed; 0 failed** (was 473 at W3) |
| Full suite (semantic-clang) | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | **531 passed; 0 failed** |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint (default) | `cargo clippy --locked --all-targets -- -D warnings` | exit 0, 0 warnings |
| Lint (semantic-clang) | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| No copyleft | `cargo deny check licenses` | `licenses ok` |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` | exit 0 |
| Coverage gate | `cargo test --locked --test examples_python_coverage` | 1 passed (22/22, 29 findings) |
| Precision KPI | `cargo test --locked --test precision_kpi` | 16 rules at 1.0/1.0 |
| Benchmark | `cargo test --locked --test benchmark` | green (small = 68 scoped) |
| Goldens | `PERFPAL_GEN…` regen → `git diff tests/golden` | empty (idempotent) |

**Dual review:** `@requirements-reviewer` **APPROVED** (all 8 W4 items present). `@reviewer` **UNAVAILABLE** — every code-review subagent attempt (5) was blocked by the provider content filter (a tool outage that triggers when a subagent opens `.py` source). A disclosed self-review covered the reviewer's checklist: `active_rule_ids()==67` and "every baseline key planted" are asserted by passing benchmark guards; the coverage gate's vacuity risks are closed by set-equality + exact counts + direct control checks; C/Rust byte-neutrality is proven by the empty `git diff` on the existing goldens. **No unresolved reviewer disagreement exists (the reviewer could not return one).**

## Post-W4 follow-up — cross-platform (Windows) hardening (commit `cccd91c`)
A background cross-platform audit + a real mingw cross-compile found and fixed Windows issues:
- **M1 (fixed):** `tests/scanner_pipeline.rs` used `std::os::unix` in an **ungated** test → `cargo test` could not compile on Windows; now `#[cfg(unix)]`-gated (matching the sibling at `:229`).
- **M2 (fixed):** the semantic-resolver re-read joined a POSIX repo-relative path onto the (Windows `\\?\`-prefixed) canonicalized root; now translates `/` → `MAIN_SEPARATOR` first (mirrors `scanner::resolve_c_edges`). The tier was non-functional on Windows.
- **M4 (fixed):** `discover` gained a canonical-path visited-set so a Windows directory junction (which reports `is_dir()` and is **not** `is_symlink()`) cannot recurse without bound.
- **CI (added):** a `windows-latest` `cargo check --all-targets` job — **no Windows CI job existed**, which is why M1/M2 slipped through.
- **M3 (fixed):** the MSVC release step now builds `-C target-feature=+crt-static` so the `.exe` is self-contained (no `VCRUNTIME140.dll`).

**Verification:** `cargo build --release --locked --target x86_64-pc-windows-gnu` → `perfpal.exe` (PE32+ x86-64); `cargo check --all-targets` for windows-gnu is **green after M1** (was 3 errors); Linux suite **526 passed / 0 failed**; goldens **byte-neutral** (`git diff tests/golden` empty). Residuals (M5 legacy-console non-ASCII, M6 `root_path` OS separators, real-Windows runtime paths) tracked as **`R-pf-095`**. Binaries staged: `/home/shafin/Opencode/perfpal-linux-gnu-x86_64` (CachyOS) + `/home/shafin/Opencode/perfpal-windows-gnu-x86_64.exe`.

## Risks / Open (append-only; hot ledger demoted the settled records)
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY contact placeholder), **`R-pf-082`** (medium; M2 corpora synthetic — real-repo credibility is M4), **`R-pf-007`** (medium; differentiation claims rest on one survey pass), **`R-pf-094`** (low; NEW — root-package relative imports don't resolve + the residual `R-pf-093` LOOP-007 depth≥1 note).
- **Blockers (hot, unchanged):** **`B-pf-001`** (real contact address before tag); **`B-pf-002`** (repo secrets before tag).
- **Discharged this session:** `R-pf-089` (Python precision coverage — now 6 rules at 1.0/1.0) and `R-pf-093` (scoped fixes applied; residual carried as `R-pf-094`).
- **Unchanged carries (cold):** C/Rust `loc_counts` `len+1` double-count; `function_count` counts the synthetic `<module>` record; aggregator graphs/stats include Python IR; M5 release-pipeline items.
- **Hot-state hygiene:** `state/session-state.json` = **6247 B / 8192 B**; this session demoted 30 stale/disabled risk records + settled `A-pf-072` to cold `state/archive/ledger.jsonl` (append-only; 502 → 533 records).

## Next Actions
1. **NEXT SESSION (`/new`) = `evidence-beta` M4** (AI-code credibility showcase): scan pinned Aider (Apache-2.0) + chunkhound (MIT) at pinned commits, hand-verify true positives, produce a repro packet per repo + an accepted-limitations page. (Read the state files + this handoff first; then `readPlan evidence-beta`.)
2. Carry **`R-pf-094`** (low): root-package relative-import resolution + the LOOP-007 depth≥1 message note.
3. Re-run `@reviewer` on commit `67fb682` in a fresh session to close the dual-review gap (the tool outage may be transient).

## Change set (this session)
- **Added:** `examples-python/**` (20 files); `tests/examples_python_coverage.rs`; `tests/fixtures/ir/PERF-PY-*.json` (50); `tests/python_negative_fixtures.rs`; `tests/fixtures/precision/python/**` (13); `tests/golden/report-python/**` (6); `docs/evidence/dogfood-python-backend.md`.
- **Modified:** `src/analyzer/heuristics/python/{arch,async_rules,loop}.rs`; `tests/benchmark/{corpus.rs,mod.rs}`; `tests/fixtures/benchmark/{baseline.json,README.md}`; `tests/fixtures/precision/labels.json`; `tests/p6_hardening.rs`; `README.md`; `docs/architecture/{modules.md,ir-v1.md,python-backend.md}`; `docs/schema/metrics-json-v1.md`; `docs/roadmap.md`; `docs/evidence/benchmark-suite.md`; `CHANGELOG.md`; `state/session-state.json`; `state/archive/ledger.jsonl`; `state/handoff/latest.md`; `docs/activeContext.md`.
- **Excluded / untracked:** `brag-output/`, `perfpal-report/` (unrelated working dirs) and `docs/research/ai-code-trends.md` (founder-authored positioning note, not part of W4).

## Commit
1. **`67fb682`** — work commit: python-backend W4 (`examples-python` corpus + 50 IR fixtures + precision KPI + 67-rule benchmark + report-python golden + docs).
2. **`cda4315`** — EOL state: mark W4/M3 done, demote 30 stale risks to the cold ledger.
3. **`cccd91c`** — post-W4 cross-platform (Windows) hardening (M1/M2/M3/M4 + Windows CI job).
4. (this EOL follow-up) — records the cross-platform follow-up in `state/session-state.json` + `docs/activeContext.md`, marks the `python-backend` plan done.

## L2 state split / L6 north-star (history)
The L2 hot/cold split landed **2026-09-29**: hot `state/session-state.json` is compact schema v2 and ≤ 8 KB, with full history in cold `state/archive/ledger.jsonl` (greppable, never auto-read). The L6 north-star (`docs/NORTH-STAR.md`, founder-owned goal / non-goals / stage / metrics) was pinned **2026-10-01** and referenced by `north_star_ref` in hot state.
