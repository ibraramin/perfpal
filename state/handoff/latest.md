# Handoff — Latest

## Objective
Session 41 **COMPLETE** — plan `python-backend` **W1** (task A-pf-069): pin the Python tree-sitter grammar (exact pin, ADR-0001 guardrail G5) + the ABI link test + the W1 backend design doc. Product change = **4 files** (`Cargo.toml`, `Cargo.lock`, `tests/python_grammar_link.rs`, `docs/architecture/python-backend.md`); **no backend/catalog/schema code** (W2–W4 remain).

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 → W0 ✅)** · **`python-backend` ACTIVE (W0 ✅ → W1 ✅ → W2 next)**.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance heuristics, (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified metrics, so humans see bottlenecks and LLMs optimize with explicit signals instead of guessing (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; `evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 → W0 ✅); `python-backend` ACTIVE (W0 ✅ → **W1 ✅ → W2 next**).
- **How this session serves the north-star:** it lands the foundation of the third language tier (Python — the highest AI-code share) with a pinned, ABI-guarded grammar and a durable, evidence-backed rule disposition table that keeps the catalog honest (precision over breadth), directly serving "quantified signals instead of guessing."
- **Drift flag: NONE.** W1 is exactly the queued next task (`state/handoff/latest.md` prior: "NEXT SESSION = W1"). Periodic audit not due (`sessions_since_alignment_audit` 1 → 2; threshold 10).

## This session (41) — python-backend W1
- **Grammar pin.** `Cargo.toml`: `tree-sitter-python = "=0.25.0"` (MIT, official tree-sitter org, **ABI 15** — inside the pinned runtime `tree-sitter 0.26.13` window `13..=15`). `Cargo.lock` resolves exactly `0.25.0` (checksum `6bf85fd39652e740bf60f46f4cda9492c3a9ad75880575bf14960f775cb74a1c`).
- **Link test.** `tests/python_grammar_link.rs` mirrors `rust_grammar_link.rs`: ABI-window assert + `set_language` + a representative Python module (docstring, aliased imports, decorator, typed fn, class+base+method, `for`/`while`, list/set/dict comprehensions, generator expression, f-string, `async def`+`await`, `try`/`except`/`finally`, `with`, lambda, `not`, chained comparison) parsed with **zero ERROR/missing** + a recursive **15-kind presence lock** + a malformed-source **non-vacuity** case. **2/2 pass.**
- **Design doc.** `docs/architecture/python-backend.md` (282 lines): node-kind→IR mapping read from the pinned `node-types.json` (217 entries / 128 named), the comprehension-as-`LoopNode` / synthetic module-scope `FunctionRecord` / `async` / `.ipynb`-excluded decisions, the 4 additive serde-default IR fields (`is_async`, `is_module_scope`, `binding_kind`, `receiver_root`), and the **durable disposition table** (10 retire / 13 mirror / 8 native add / 3 cut) resolving the "mirror full catalog vs retire-10" conflict (R-pf-087/R-pf-088) and the W0 obligations (R-pf-091: `ALLOC-001` loop-invariance, `LOOP-002` print/input family, `LOOP-007` loop-scoped+kind-gated, `ARCH-*` thresholds deferred to W2/W4).

## Verification
| Claim | Command | Result |
|---|---|---|
| Exact pin | `grep -q 'tree-sitter-python = "=0.25.0"' Cargo.toml` | exit 0 |
| Lock resolves 0.25.0 | `grep -A1 'name = "tree-sitter-python"' Cargo.lock` | `v0.25.0`, checksum `6bf85fd3…` |
| ABI link test | `cargo test --locked --test python_grammar_link` | **2 passed; 0 failed** |
| Full suite | `cargo test --locked` | exit 0 (all test binaries ok) |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| No copyleft | `cargo deny check licenses` | `licenses ok` |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` | exit 0 |
| Hot ≤ 8192 B | `wc -c < state/session-state.json` | **8076** |
| Hot schema v2 + status vocabulary | `jq -e '...'` (session-40 gate) | exit 0 |
| Hot ids unique | `jq -e '(...)|length==(...|unique|length)'` | exit 0 |

**Dual review:** `@reviewer` round-1 **CHANGES_REQUESTED** (6 findings: 5 grammar-fact errors — `await`/`lambda` named-node, `comparison_operator` has no `operands`, no `positional_argument`, `not` is `not_operator` — plus a missing `list()/dict()` materialisation-churn disposition) → **all fixed and independently re-verified against `node-types.json`** → round-2 **APPROVED** (1 non-blocking wording nit at `python-backend.md:188` → fixed). `@requirements-reviewer` round-1 **APPROVED** (no missing/partial W1 requirement). **No unresolved reviewer disagreement.**

## Risks / Open (append-only)
- **Open risks (hot, unchanged key ones):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY contact placeholder), **`R-pf-087`** (high; Python M3 design obligations → **now discharged into the W1 doc, W2 implementation remains**), **`R-pf-091`** (high; W1 obligations → discharged into the W1 doc).
- **Blockers (hot, unchanged):** **`B-pf-001`** (real contact address for EULA/PRIVACY before tag); **`B-pf-002`** (repo secrets `MINISIGN_SECRET_KEY` + `MINISIGN_PASSWORD` before tag).
- **Hygiene note:** hot `state/session-state.json` is **8076 B / 8192 B cap (~116 B headroom)** — the next session should expect to demote settled decisions before appending much.
- **W2 obligations (carry):** implement the doc's decisions (comprehension-as-`LoopNode` + once-evaluated iterables; synthetic module record; 4 additive IR fields; best-effort import resolution with `__init__.py` one-level rule); `.ipynb` stays excluded; `ARCH-*` thresholds unmeasured until W4.

## Next Actions
1. **NEXT SESSION (`/new`) = plan `python-backend` W2:** read `state/session-state.json` + `state/handoff/latest.md` + `docs/activeContext.md` FIRST, then `readPlan python-backend` and `docs/architecture/python-backend.md`. Deliver: `src/scanner/backends/tree_sitter_python/mod.rs` (shared `Extraction` contract + ParseHealth parity), the minimal binding/alias inference pass + additive IR fields, the best-effort import-resolution pass, and wiring `LanguageKind::Python` + `language_of(.py/.pyi)` + `extract_unit`/`failed_unit`/`SUPPORTED_LANGUAGES` in `src/scanner/mod.rs`.
2. Then **W3** catalog (`PERF-PY-*`) + registry/reporter + schema `1.3` → **W4** `examples-python/` / precision / benchmark / goldens / docs + dual review.

## Change set (this session)
- **Added:** `tests/python_grammar_link.rs`, `docs/architecture/python-backend.md`.
- **Modified:** `Cargo.toml`, `Cargo.lock`, `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`.
- **Excluded / untracked:** `brag-output/` and `perfpal-report/` — unrelated working-tree dirs, deliberately NOT in this change set.

## Commit
1. **`2b897b4`** — work commit: W1 grammar pin + link test + backend doc + state/handoff/activeContext/CHANGELOG.
2. (this EOL follow-up commit) — regenerates this handoff to embed `2b897b4`.

## L2 state split / L6 north-star (history)
The L2 hot/cold split landed **2026-09-29**: hot `state/session-state.json` is compact schema v2 and ≤ 8 KB, with full history in cold `state/archive/ledger.jsonl` (greppable, never auto-read). The L6 north-star (`docs/NORTH-STAR.md`, founder-owned goal / non-goals / stage / metrics) was pinned **2026-10-01** and referenced by `north_star_ref` in hot state.
