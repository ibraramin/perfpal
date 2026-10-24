# Track B — JS/TS backend

**Start this tab:** open `/home/shafin/Opencode/Perfpal-js-ts` (`feat/js-ts`), first message:

> `Read /home/shafin/Opencode/Perfpal-js-ts/docs/tracks/js-ts.md and execute Session B3.`

**Track handoff:** `state/tracks/js-ts/handoff.md` · **Spec:** `docs/specs/js-ts.md` · **Plan:** `docs/plans/js-ts.md` · **ADR:** `docs/adr/ADR-0011-js-ts-backend.md` · **Inventory:** `docs/architecture/js-ts.md` · **Canary evidence:** `docs/evidence/js-canary.md` · **Base:** `9a94905` (rebased on `main` @ `9a94905`; B1 base `821b984`).
**Scope:** add JS, then TS/TSX, as first-class backends in the existing Rust core — like C/Rust/Python. **Not a rewrite.** A later release; not beta-blocking.

## Read first

1. `AGENTS.md` — exact pins, no copyleft, determinism, exit-code/schema invariants.
2. `docs/specs/perfpal-core-spec.md`, `docs/architecture/modules.md`, `docs/schema/`, `docs/adr/`.
3. `src/scanner/` (including `src/scanner/backends/`), `src/analyzer/` (rules + heuristics), `src/schema/`, `src/aggregator/`, `Cargo.toml`, `deny.toml`, `rust-toolchain.toml`.
4. The existing C/Rust/Python fixtures and goldens for the pattern.

## Session B1 — Phase 0 (planning only) — **DONE** (2026-10-24)

**Goal:** a spec + plan + ADR; **no production code**.

**Close-out:** delivered `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, `docs/adr/ADR-0011-js-ts-backend.md`,
`docs/architecture/js-ts.md` (Phase 0 inventory), and `state/tracks/js-ts/{session-state.json,handoff.md}`.
Decided: tree-sitter-javascript `=0.25.0` (ABI 15, MIT) + tree-sitter-typescript `=0.23.2` (ABI 14, MIT,
two parsers); tokens `js`/`ts`/`tsx` on grammar boundaries; one MINOR schema bump v1.7 -> v1.8 in B2 declaring
all three tokens; 12-rule JS candidate catalog to be frozen by a B2-W0 canary; global directory skips +
JS/TS-only file-name/size guard. Next: **B2** (JS backend).
**Panel (2026-10-24):** Q1 APPROVED; R1/R2/Q2 returned CHANGES_REQUESTED — all blocking findings fixed
(schema version-pin sweep, `GOLDEN_ARTIFACTS` filename, global-vs-file discovery split, reporter-catalog
ownership blocker, plan DoD/exit codes, named determinism suites). Final: R1, R2, Q2 and a fresh R3 lane
all **APPROVED**, zero unresolved blocking findings.

**Decide and record:** parser (tree-sitter-javascript / tree-sitter-typescript vs oxc/swc); exact pins + license/no-copyleft; Rust crates; schema language values (`js`, `ts`, `tsx`?) + version/ADR; language detection; rule scope (which heuristics/metrics); cost model; output contract; tests/goldens/fixtures; CLI behavior. **JS first, TS/TSX second.**

**Deliver:** `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, the ADR, a Phase 0 inventory, and `state/tracks/js-ts/handoff.md`. No code.

**Do not:** touch `atlas_*.rs`/`atlas_brief.rs`/Atlas specs/goldens; no main state; no C/D files; no schema change without the ADR; no new dependency before the license check; no `views.rs`/`mod.rs` language wiring until A step 12 (rebase).

**EOL:** update the track handoff; commit only with founder approval; no merge to `main`.

## Session B2-W0 — grammar pins + canary — **DONE** (2026-10-24; panel: R1–R5 + Q1/Q2 APPROVED)

**Goal:** prove the pins on the 1.92 floor and freeze the rule catalog before any backend code.

**Close-out:** rebased onto `main` @ `189df91`; added `tree-sitter-javascript =0.25.0` +
`tree-sitter-typescript =0.23.2` (MIT, C11 only; `cargo deny` green; both compile on `rustc 1.92.0`);
added `tests/javascript_grammar_link.rs` + `tests/typescript_grammar_link.rs` (6 tests: ABI window,
link, clean representative parse, node-kind locks, malformed non-vacuity; TSX `<T,>` parses cleanly);
ran the pinned-repo canary (`tools/js_canary.rs` + `docs/evidence/js-canary/run.sh`) over express
`v5.1.0`, execa `v9.6.1`, gemini-cli `v0.65.0` — 1,525 prod files / 321.1 kLOC, 230 parse failures
(220 = the pinned TS grammar's `f<typeof import("x")>()` limitation, `R-tb-014`). Froze **11 ship / 1
cut**: `PERF-JS-LOOP-003` cut (zero array-literal RHS), `ALLOC-001` reframed behind a loop-invariance
gate, `CALL-001` guard-gated, `ASYNC-002` async-gated. Evidence `docs/evidence/js-canary.md`;
dispositions `docs/architecture/js-ts.md` §8. Next: **B2-W1** (JS backend). Blockers `B-tb-003`
(reporter catalog, W2) and `B-tb-001` (views wiring, A step 12) remain open and were not touched.

## Session B2-W1 — JS backend + wiring — **committed `90d5677` (pre-rebase `7fdfa1a`); full DoD green; panel APPROVED** (2026-10-24)

**Goal:** implement the JavaScript tree-sitter backend and every wiring point from the plan, with no rules
or schema bump yet.

**Close-out:** added `src/scanner/backends/tree_sitter_javascript/` (extraction, module record, imports,
loops/trips, alloc/unary/await/try events, entry guards); wired `LanguageKind::Js` + longest-suffix
`language_of` + `extract_unit`/`failed_unit`/`SUPPORTED_LANGUAGES` (`["c","js","python","rust"]`); implemented
global directory skips (`node_modules`, `bower_components`, `dist`, `build`, `out`, `coverage`) plus the
JS-only `*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js` and >2 MiB file guards; added the relative-specifier
resolver (`.ts/.tsx/.js/.jsx/.mjs/.cjs/.mts/.cts` + `index.*`, bare = external, internal miss = `to: null`)
and `package.json` `main`/`bin` entry declarations (`entry_declared`, IR-internal) with in-source
`require.main === module` / `import.meta.url` guards; added the aggregator JS entry ladder (`script` HIGH /
conventional `script` MEDIUM / `lib-root` MEDIUM / `unknown-entry` LOW) and the JS instrument arms (alloc
family + `throw` error path; `#private` survives normalization). Added `tests/javascript_backend.rs`
(20 tests: detection/tokens, skips, parse health, entry ladder incl. all `bin` forms + malformed manifest,
loops/cost provenance, alloc/error/idiom facts, exact include-graph edges, zero W1 findings, double-run
determinism). **No `src/reporter`/Atlas edits, no schema bump, no main state.** Verification:
`state/tracks/js-ts/handoff.md`. Next: **B2-W2** (rules + schema v1.8 + fixtures/goldens; `B-tb-003`).

## Session B2-W2 — rules + schema v1.8 — **implemented (uncommitted)** (2026-10-24)

**Goal:** ship the frozen 11 `PERF-JS-*` rules, land the catalog atomically, bump the schema to v1.8, and
add the corpora/fixtures/goldens. Branch `feat/js-ts` rebased on `main` @ `9a94905`.

**Deliverables:** `src/analyzer/heuristics/js/{mod.rs,tests.rs}` (11 rules + helpers; 27 IR unit tests);
`JS_*_RULE_ID` constants in `src/analyzer/finding.rs`; `RuleLanguage::{Js,Ts}`, the ascending
`PERF-JS-*` registry block, and the `js|ts|tsx` partition in `src/analyzer/mod.rs` (`Ts` reserved and empty
in v1); 11 `HEURISTIC_CATALOG_V1` entries + count locks 67 → 78 in `src/reporter/mod.rs`;
`src/schema/metrics.schema.v1.8.json` + byte-identical `docs/schema/metrics.schema.v1.8.json` (v1.7 kept);
`SCHEMA_VERSION`/`SCHEMA_ARTIFACT_NAME`/`EMBEDDED_SCHEMA`/CLI `VERSION_SUFFIX` = 1.8; `tests/schema_1_7.rs`
→ `tests/schema_1_8.rs` + version-pin sweep; `examples-js/` (15 scanned files: 14 `.js` + 1 `.cjs`, plus
`package.json` `main`/script HIGH), `tests/examples_js_coverage.rs`, 22 parser-free IR fixtures +
`tests/js_ir_fixtures.rs`, `tests/determinism_js.rs`, `tests/fixtures/precision/js/` (25 files) + labels,
the `report-js` golden tree (20 files; `GOLDEN_TREES` 4 → 5), and the benchmark `small` rung (142 files /
1,655 LOC / 96 findings; 78 active rules).

**Rule table (shipped):**

| id | severity | gate (implemented) |
|---|---|---|
| `PERF-JS-ALLOC-001` | info | object/array literal in a loop; silent when a referenced name is the loop induction variable or is assigned in that loop, or no enclosing loop |
| `PERF-JS-ALLOC-003` | info | `concat`/`slice` calls **or** array literals with spread (`acc = [...acc, x]`) whose accumulator root the same enclosing loop reassigns; snapshot roots stay silent |
| `PERF-JS-STR-001` | warn | string/template-literal RHS on `+=`, or `x = x + <string\|template literal>` where the binary is the whole RHS, in a loop; numeric/identifier/subscript/call RHS silent; canonical trip count < 8 silent |
| `PERF-JS-LOOP-001` | info | one finding per function at the earliest loop with depth ≥ 2; `max_depth` scoped to that loop's span |
| `PERF-JS-LOOP-002` | warn | known sync Node API (23 names) at loop depth ≥ 1 |
| `PERF-JS-LOOP-007` | warn | membership callee in a loop with non-empty receiver; silent on canonical trip count < 16; unknown trip fires |
| `PERF-JS-ASYNC-001` | warn | `await` in a loop, only inside an `is_async` function |
| `PERF-JS-ASYNC-002` | warn | known sync Node API anywhere in an `is_async` function; no loop requirement |
| `PERF-JS-CALL-001` | info | bare direct self-call; fires only when no comparison starts before the call span |
| `PERF-JS-ARCH-001` | info | resolved relative-import cycle; one finding per `break_cycles` edge at its earliest span |
| `PERF-JS-TYPE-001` | info | `delete obj.prop` with a `MemberAccess` operand; subscript form and bare identifier excluded; the property name is carried in evidence |

**Verification (recorded):** `cargo test --locked` → 63 suites, **935 passed**, 0 failed;
`cargo deny check licenses` → ok; semantic-feature clippy + `PERFPAL_SEMANTIC_TESTS=1` tests green; native
check green. `examples-js` emits exactly 11 findings (0 hot / 5 warn / 6 info), each rule once, and all 18
silent controls stay silent. Precision corpus: all 11 rules 1.0 / 1.0 (25 JS fixture files). `examples-js`
double-run is byte-identical modulo `scan.started_at`/`scan.duration_ms`. Gate transcript:
`docs/evidence/js-w2/gate.txt` (recorded in the tree). **Panel:** round 1 R2/Q1/Q2 APPROVED, R1
CHANGES_REQUESTED (two rule-gate defects) → fixed; round 2 R1 and a fresh R3 APPROVED, Q2 asked for
documentation fixes → applied; final confirmation pending. **Commit: pending founder approval. Merge gate
`A-tb-006` still requires founder approval; `B-tb-002` stays open.**

**Decisions:** `D-tb-017` — `B-tb-003` resolved: the founder approved the atomic catalog-only B edit in the
B2-W2 work order; the 11 entries landed in the same change as the registry block.

**Risks:** `R-tb-015` (schema staging) **resolved** — v1.8 landed in both copies before any release; v1.7
retained. New `R-tb-016` (lexical rule gates) — `ALLOC-001` models loop-invariance rather than a separate
escape analysis, `CALL-001` treats any earlier comparison as a visible base case, `LOOP-001` cannot test
for early exits (no break/return events in IR), and `LOOP-007` proxies the frozen "small collection" FP
class with loop trip count. `ALLOC-003` misses member/`this` accumulators (`assigned_within` matches plain
identifier targets only) and the nested-function identifier walk can suppress an `ALLOC-001` hit. The
1.0/1.0 precision result is on the synthetic precision corpus only; a
real-code canary re-run is owed before merge.

**Next:** **B3** — TS/TSX backend (`.ts`/`.mts`/`.cts` via `LANGUAGE_TYPESCRIPT`, `.tsx` via
`LANGUAGE_TSX`), `SUPPORTED_LANGUAGES` to six values, `examples-ts/`, `report-ts` golden (`GOLDEN_TREES`
5 → 6), TS type-only negative fixtures incl. `<T,>`, and TS coverage + determinism suites; no schema bump
and no duplicated rule implementations.

## Sessions B2–B4 (signposts)

- **B2 — JS backend.** Parser + language detection + rules + schema values + fixtures/goldens; full gate; panel.
- **B3 — TS/TSX backend.** Same for TS/TSX; reuse the JS path where possible; schema/ADR update only if needed.
- **B4 — tests/goldens/docs.** New JS/TS golden trees; report language labels; output contract; semver; review panel.
- Merge after A step 12; B is a later release.

## Guardrails / ownership

- B owns `src/scanner` (incl. `backends/`), `src/analyzer` (rules + heuristics), `src/schema` language values, grammars, JS/TS fixtures, `Cargo.toml`/`Cargo.lock`, `deny.toml`, JS/TS spec/plan/ADR.
- A/C/D read; no edits. `views.rs`/`mod.rs` language wiring after A step 12.
- Rebase on `main` at session start; use a separate `CARGO_TARGET_DIR`; exact pins; no copyleft.
