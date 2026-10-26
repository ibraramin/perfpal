# Track B — JS/TS backend

**Start this tab:** open `/home/shafin/Opencode/Perfpal-js-ts` (`feat/js-ts`), first message:

> `Read /home/shafin/Opencode/Perfpal-js-ts/docs/tracks/js-ts.md and execute Session B6 — merge gate (B5 is committed `1b23198`; see the handoff).`

**Next-session signpost (merge gate):** branch `feat/js-ts`, base `main @ 9a94905`; W0 `e7f5332`, W1 `90d5677`,
W2 `edfc98a`, B3 `9dd19bb`, **B4 committed `6d6bcdf` (docs/contract/canary + label polish; panel
APPROVED: R1/R2/R3 + Q1/Q2), **B5 committed `1b23198` (feat; plan DoD green, canary re-run, panel
APPROVED R1/R2/R3 + Q1/Q2, recorded in the handoff; docs `dbda7e3`).** `PERF-JS-CALL-001` production
180 → 2 and `TYPE-001` 14 → 2, both remaining sets hand-verified true positives. The merge gate `A-tb-006`
now remains blocked only on **founder approval** (+ the merge mechanics: rebase on current `main`, final DoD
on the rebased tree). No push. See `state/tracks/js-ts/handoff.md` for the full B5 record.

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
JS-only `*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js` (B5 adds `*.min.ts`/`*.min.mts`/`*.min.cts`) and
>2 MiB file guards; added the relative-specifier
resolver (`.ts/.tsx/.js/.jsx/.mjs/.cjs/.mts/.cts` + `index.*`, bare = external, internal miss = `to: null`)
and `package.json` `main`/`bin` entry declarations (`entry_declared`, IR-internal) with in-source
`require.main === module` / `import.meta.url` guards; added the aggregator JS entry ladder (`script` HIGH /
conventional `script` MEDIUM / `lib-root` MEDIUM / `unknown-entry` LOW) and the JS instrument arms (alloc
family + `throw` error path; `#private` survives normalization). Added `tests/javascript_backend.rs`
(20 tests: detection/tokens, skips, parse health, entry ladder incl. all `bin` forms + malformed manifest,
loops/cost provenance, alloc/error/idiom facts, exact include-graph edges, zero W1 findings, double-run
determinism). **No `src/reporter`/Atlas edits, no schema bump, no main state.** Verification:
`state/tracks/js-ts/handoff.md`. Next: **B2-W2** (rules + schema v1.8 + fixtures/goldens; `B-tb-003`).

## Session B2-W2 — rules + schema v1.8 — **committed `edfc98a`; full DoD green; panel APPROVED** (2026-10-24)

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
documentation fixes → applied; final confirmation pending. **Committed `edfc98a`. Merge gate
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

## Session B3 — TS/TSX backend + wiring + corpora/goldens — **committed `9dd19bb`; full DoD green; panel APPROVED** (2026-10-26)

**Goal:** TypeScript/TSX as first-class backends without a second visitor, no schema bump, no new rules.

**Close-out:** the JS backend is parameterized by `Dialect { language, token, flavor }`
(`tree_sitter_javascript::{Dialect,Flavor,extract_with}`); new
`src/scanner/backends/tree_sitter_typescript/` binds `LANGUAGE_TYPESCRIPT` (`ts` for `.ts`/`.mts`/`.cts`)
and `LANGUAGE_TSX` (`tsx` for `.tsx`). TS-specific handling: `required_parameter`/`optional_parameter`
unwrapping with real annotation text (leading `:` stripped, `this` erased, `"unknown"` when absent);
`ternary_expression`; `abstract_class_declaration`; `public_field_definition.name`;
`as`/`satisfies`/`!`/`<T>expr` transparent; **type land skipped before any event match** (critical:
`type_query` contains `call_expression`); type-only `import type`/`import typeof`/`export type` produce no
`IncludeEdge`; `declare class` produces no `StructDef`; `declare`/overloads/interfaces/type aliases produce
no records. Scanner: `LanguageKind::{Ts,Tsx}`, longest-suffix detection with `.d.*` still skipped,
one `is_js_family` predicate for the 2 MiB cap + resolver + entry declarations, `SUPPORTED_LANGUAGES` six,
and `extract_unit`/`failed_unit` tokens. Instrument `classify_alloc_call`/`detect_error_path` arms are
`"js" | "ts" | "tsx"` (idiom hash/normalization identical to JS); aggregator `entry_scoped` +
`detect_js_entries` cover the family. **No schema, catalog, rule, reporter or dependency change.**

**Tests/corpora:** `tests/typescript_backend.rs` (17 integration tests) + 17 backend unit tests
(`tree_sitter_typescript/tests.rs`) + committed negatives `tests/fixtures/typescript/` (`type_only.ts`,
`type_edges.ts`, `typeof_import.ts`); `examples-ts/` 17 sources (15 `.ts` + 2 `.tsx`) with all 11 shared
rules firing **exactly once** (0 hot / 5 warn / 6 info) and 18 silent controls;
`tests/examples_ts_coverage.rs`, `tests/determinism_ts.rs`; `tests/golden/report-ts/` (20 artifacts,
`GOLDEN_TREES` 5 → 6, pinned `charts/component-01-root.svg`). The existing five golden trees were unchanged
by the regeneration.

**Verification:** full plan `js-ts` Definition of Done green (fmt, clippy, full suite, deny,
semantic-feature, double-run over `examples-js` + `examples-ts`, native check) — transcript
`docs/evidence/js-b3/gate.txt`. Panel: see `state/tracks/js-ts/handoff.md`.

**Decisions:** `D-tb-018` — B3 implemented as a dialect over the JS visitor; type-only edges erased;
TS parameter annotations recorded (IR-internal). `D-tb-019` — `R-tb-010` narrowed: both `<T,>` and `<T>`
generic arrows parse cleanly at expression position on the pinned TSX grammar.

## Session B4 — docs/contract/canary — **committed `6d6bcdf`; plan DoD green; panel APPROVED (R1/R2/R3 + Q1/Q2)** (2026-10-26)

**Goal:** close the docs/contract/label pass and re-run the real-code canary before merge, without a
schema, rule, reporter-count or dependency change.

**Close-out:** `docs/architecture/modules.md` §1/§3.2/§4.2/§5.2/§5.3/§7 now carry the six-token matrix,
the five backend directories, discovery skips and v1.8; `docs/report/output-contract.md` publishes the
language vocabulary, dependency labels and discovery policy; `README.md`/`docs/roadmap.md`/`Cargo.toml`
name the JS/TS tier and the 78-rule catalog; new `docs/heuristics/js-catalog.md` is the read-code-first
11-rule reference. Reporter `dependency_mechanisms`/`dependency_clause` render the shared family as
`import/require` / ``JavaScript/TypeScript `import`/`require` bindings`` (deduped `js`/`ts`/`tsx`); the
`report-js`/`report-ts` goldens were regenerated (the other four trees are byte-identical). Tests:
`dependency_labels_cover_the_js_family`, the real-parse STR-001 negative
(`s = f(s + "x")` silent; `s = s + "x"` fires), and the spread-root attribution pin. The B4 canary
(`docs/evidence/js-b4/real-code-canary.md`) scanned the pinned express/execa/gemini-cli corpora twice:
deterministic (charts/schema/metrics byte-identical; `report.md` provenance only), 2,987 files, 230
partial parses (the known type-query limitation), and **11/11 rules firing**. It found a **merge-gate
blocker**: `PERF-JS-CALL-001` has 150/180 production FPs from member/method-name collisions
(`this.foo()` inside `foo()`, `new X().foo()` inside `foo()`) because `receiver_root_of` returns `None`
for non-identifier receivers. `TYPE-001` has a fresh-object FP class (~10/14), `ALLOC-001` remains noisy
but gated (503 prod), `ALLOC-003` is mostly the intended `slice`-consumption shape (17/20, one
error-message spread FP), `ASYNC-002` remains test-skewed (153 prod vs 1,028 non-prod), and `ARCH-001`
remains blind in alias monorepos (0 gemini, 4 execa). Cross-cutting: `path_class` misses `.spec.` /
hyphenated test dirs, and `findings[].symbol` is `<module>` in `metrics.json` (the ranked action
list/key views resolve the narrowest function; full-detail rows still show `<module>` until `A-tb-008`).
**No rule/schema/code fix was made in B4** (docs/label scope); the CALL-001 fix is the merge-gate
follow-up (`R-tb-017`, `A-tb-007`).

**Gate:** plan `js-ts` DoD green (fmt, clippy, full suite, deny, semantic-feature, `examples-js`/
`examples-ts` double-run, native check); transcript `docs/evidence/js-b4/gate.txt`. Panel: **R1/R2/R3 +
Q1/Q2 APPROVED**, zero unresolved blocking findings; record in `state/tracks/js-ts/handoff.md`.

**Next:** merge gate `A-tb-006` — founder approval + CALL-001 gate fix + canary re-run + full DoD on the
rebased tree. No push until then.

## Session B5 — post-B4 correctness follow-up — **committed `1b23198`; full DoD green; panel APPROVED (R1/R2/R3 + Q1/Q2)** (2026-10-26)

**Goal:** close the B4 merge-gate blocker and the approved follow-ups without a schema, catalog-count,
dependency or rule-id change; re-run the pinned canary; re-baseline the goldens.

**Close-out:** `PERF-JS-CALL-001` production **180 → 2** (both verified true recursion) via exact bare
`callee_path`, `!conditional_guarded`, and additive `FunctionRecord.name_is_lexical_binding`;
`PERF-JS-TYPE-001` production **14 → 2** via additive `UnaryEvent.operand_root_fresh` (fresh-object gate);
`PERF-JS-ALLOC-003` spread arm requires the literal to be an assignment RHS (24 → 22 total; error-message
FP gone, member-target accumulators silent rather than mis-attributed); `findings[].symbol` resolves the
narrowest record (JS/TS/Python goldens re-baselined; all other trees byte-identical); `path_class` catches
`.spec.`/`test-*` (canary production denominator 1,336 → 1,178); `.min.ts`/`.min.mts`/`.min.cts` skip gap
closed; p6 compares both Atlas pages (`GOLDEN_ARTIFACTS` 17 → 19). Evidence:
`docs/evidence/js-b5/real-code-canary.md` (exact pre/post diff against the B4 binary) and
`docs/evidence/js-b5/gate.txt` (66 suites / 991 passed; semantic 994; fmt/clippy/deny/double-run/native
green). **Panel APPROVED** (R1/R2/R3 + Q1/Q2), recorded in `state/tracks/js-ts/handoff.md`; zero unresolved
blocking findings. Committed `1b23198` (feat) + `dbda7e3` (docs); no push;
merge remains `A-tb-006` + founder approval.

## Sessions B2–B5 (signposts)

- **B2 — JS backend.** Parser + language detection + rules + schema values + fixtures/goldens; full gate; panel.
- **B3 — TS/TSX backend.** Same for TS/TSX; reuse the JS path where possible; schema/ADR update only if needed.
- **B4 — docs/contract/canary.** Golden/label pass, `js-catalog.md`, README/roadmap/modules/output-contract, semver + CHANGELOG note, real-code canary; review panel. **Implemented; panel APPROVED (R1/R2/R3 + Q1/Q2).**
- **B5 — correctness follow-up.** CALL-001/TYPE-001/ALLOC-003 gates, symbol/path_class fixes, minified-TS
  skip gap, Atlas golden comparison, canary re-run, goldens. **Committed `1b23198`; merge remains
  founder-gated.**
- Merge after A step 12; B is a later release.

## Guardrails / ownership

- B owns `src/scanner` (incl. `backends/`), `src/analyzer` (rules + heuristics), `src/schema` language values, grammars, JS/TS fixtures, `Cargo.toml`/`Cargo.lock`, `deny.toml`, JS/TS spec/plan/ADR.
- A/C/D read; no edits. `views.rs`/`mod.rs` language wiring after A step 12.
- Rebase on `main` at session start; use a separate `CARGO_TARGET_DIR`; exact pins; no copyleft.
