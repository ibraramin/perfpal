# Spec: js-ts

Scope: feature

# Spec: js-ts

**Scope:** feature. **Track:** B (`feat/js-ts`). **Status:** B1/B2-W0 complete; **B2-W1 committed
`90d5677` (pre-rebase `7fdfa1a`)**; **B2-W2 committed `edfc98a`** (11 `PERF-JS-*` rules + catalog landing +
schema v1.8 + fixtures/goldens); **B3 committed `9dd19bb`** (full DoD green, five-lane panel APPROVED) (TypeScript/TSX backend + `examples-ts` + `report-ts` golden).
**Owners:** Track B owns this spec, `docs/plans/js-ts.md`, the JS/TS ADR, the grammar pins, the JS/TS
fixtures/goldens, and the `src/schema` language values. Track A owns `views.rs`/`mod.rs` wiring until A step 12.

**B4 close-out (docs/contract/canary pass).** The JS/TS tier is documented as shipped: `modules.md`
§1/§3.2/§4.2/§5.2/§5.3/§7 carry the six-token matrix and the five backend directories,
`output-contract.md` publishes the language vocabulary + discovery skips, `README.md`/`roadmap.md`/
`Cargo.toml` name the JS/TS backends and the 78-rule catalog, and `docs/heuristics/js-catalog.md` is the
read-code-first 11-rule reference. Reporter label polish renders the shared family as
`import/require` / ``JavaScript/TypeScript `import`/`require` bindings`` instead of bare `js`/`ts` module
edges. The real-code canary was re-run on the pinned express/execa/gemini-cli corpora; see
`docs/evidence/js-b4/real-code-canary.md`. Spread-root attribution and the `s = f(s + "x")` STR-001
negative are published limitations/tests, not silent behaviour. No schema, rule, reporter-count or
dependency change.

**B5 close-out (real-code gate fixes).** The B4 canary's merge gate is cleared without a catalog, schema,
severity or dependency change (catalog stays **78**; 11 JS rules). `PERF-JS-CALL-001` now requires
`callee_path` to be exactly the bare function name, `!conditional_guarded`, and
`FunctionRecord::name_is_lexical_binding == true` (class/object methods and property/arrow fields are
delegation, not recursion); production hits fall 180 → 2 on the pinned corpora, both genuine recursion.
`PERF-JS-TYPE-001` skips a delete whose receiver root was bound in the same function to an object/array
literal or `new` (`UnaryEvent.operand_root_fresh`); production 14 → 2. `PERF-JS-ALLOC-003`'s spread arm
now requires the literal to be the RHS of an assignment to the attributed root (`assigned_from_literal`);
production 20 → 18. Cross-cutting: `findings[].symbol` resolves the narrowest containing record, and
`path_class` recognizes `.spec.`/`-spec.` basenames and `test-*`/`*-test(s)`/`*-spec(s)` segments; the
JS/TS minified skip list is now seven suffixes. Evidence: `docs/evidence/js-b5/real-code-canary.md`; no
schema/catalog-count change.

**B3 close-out (committed `9dd19bb`).** TypeScript and TSX are first-class backends without a second
visitor: the JS backend is parameterized by a grammar/token/flavor `Dialect` and the new
`src/scanner/backends/tree_sitter_typescript/` binds `LANGUAGE_TYPESCRIPT` (`ts`) and `LANGUAGE_TSX`
(`tsx`). Scanner wiring adds `LanguageKind::{Ts,Tsx}`, the longest-suffix map (`.d.*` still skipped), the
six-value `SUPPORTED_LANGUAGES`, and the whole family in the shared 2 MiB cap, resolver, entry declarations
and instrument arms. Type-land is never event-scanned (critical: `type_query` contains `call_expression`);
`as`/`satisfies`/`!`/`<T>expr` are transparent; TS parameters carry real annotation text; `import type` /
`export type` produce no include edge (runtime-erased). Corpora: `examples-ts/` (17 sources: 15 `.ts` + 2
`.tsx`) fires all 11 shared rules exactly once (0 hot / 5 warn / 6 info) with 18 silent controls;
`tests/examples_ts_coverage.rs`, `tests/determinism_ts.rs`, `tests/typescript_backend.rs` (17 integration
tests), 17 backend unit tests (incl. wrapper callee/receiver decomposition and comment extras), and
`tests/golden/report-ts/` (20 artifacts; `GOLDEN_TREES` 5 → 6).
No schema bump, no new rules, no dependency change. Verification: `docs/evidence/js-b3/gate.txt`.

**B2-W2 close-out (committed `edfc98a`).** 11 `PERF-JS-*` rules landed in
`src/analyzer/heuristics/js/mod.rs` with 27 parser-free IR unit tests; constants in
`src/analyzer/finding.rs`; `RuleLanguage::{Js,Ts}`, the ascending registry block, and the `js|ts|tsx`
partition in `src/analyzer/mod.rs` (`Ts` is reserved and empty in v1); 11 catalog entries in
`src/reporter/mod.rs` (catalog **78** = 23 C + 11 JS + 22 PY + 22 RS). `B-tb-003` is **resolved**: the
founder approved the atomic catalog-only B edit in the B2-W2 work order (`D-tb-017`), and the entries
landed in the same change as the registry block. Schema v1.8 landed:
`src/schema/metrics.schema.v1.8.json` + byte-identical `docs/schema/metrics.schema.v1.8.json`; v1.7 is
retained; both language enums are `["c","js","python","rust","ts","tsx"]`; `tests/schema_1_7.rs` is
renamed `tests/schema_1_8.rs` and the version-pin sweep is applied. DoD (recorded): `cargo test --locked`
63 suites / 935 passed / 0 failed; `cargo deny check licenses` ok; semantic-feature and native checks
green. Panel: round 1 R2/Q1/Q2 APPROVED, R1 CHANGES_REQUESTED -> fixed; round 2 R1/R3 APPROVED, Q2 doc fixes applied; final confirmation **APPROVED**. Commit `edfc98a`.

**B2-W2 review-panel R1 close-out (implemented).** The two blocking rule-gate defects are fixed
in the same change: `PERF-JS-STR-001` is **string-gated** on the B2-W0 canary predicate (RHS must be a
string/template literal; numeric and identifier `+=` are silent) and also admits the `x = x + <literal>`
form; `PERF-JS-ALLOC-003` covers the frozen **spread** mechanism via `AllocEvent.has_spread`
(`acc = [...acc, x]`), alongside `concat`/`slice`. Carry-in nits: `PERF-JS-TYPE-001` now requires a
`MemberAccess` operand (`delete x` silent) and `PERF-JS-LOOP-001.max_depth` is scoped to loops contained
in the anchored loop's span.

**B2-W1 additive IR-internal facts (no schema/DTO change).** The backend now emits
`FunctionUnit.allocations: Vec<AllocEvent>` (`new`/`object`/`array` + `referenced_names`),
`FunctionUnit.unaries: Vec<UnaryEvent>` (`delete obj.prop`), and `FunctionRecord.entry_declared:
Option<bool>` (`package.json` `main`/`bin` or an in-source `require.main === module` / `import.meta.url`
guard). All three are serde-defaulted IR fields; they are **not** mirrored in `metrics.json`.

**B2-W2 additive IR-internal fact (no schema/DTO change).** `AllocEvent.has_spread: bool`
(`src/ir/mod.rs`, `#[serde(default)]`) is `true` only for an array literal containing a
`spread_element` child; `new` expressions and object literals report `false`. It is the frozen
ALLOC-003 spread-accumulator signal (`acc = [...acc, x]`) and is **not** mirrored in `metrics.json`.

**Disclosure — `js` preceded v1.8 (now closed).** A B2-W1 tree emitted `files[].language == "js"` and
`scan.languages == ["js"]` while the bundled schema artifact was still v1.7 (whose language enum listed
only `c`/`rust`/`python`) and the repo had no in-repo document validator. This was deliberate staging,
**not a shippable point**; the single v1.8 bump with all six tokens landed in B2-W2 before any release
(`R-tb-015` resolved).

## 1. Purpose

Add **JavaScript first, then TypeScript/TSX** as first-class Perfpal backends inside the existing Rust
core — the same shape as the C, Rust and Python tiers. Not a rewrite: a new scanner backend + IR emission
+ rule catalog + schema language values + fixtures/goldens. The analyzer consumes only the parser-agnostic
IR (ADR-0001), so no analyzer/reporter rewrite is required.

**Why now:** Perfpal's beta framing is "audits what AI wrote"; JS/TS is the dominant AI-generated-language
surface. Track B is a **later release**, non-blocking for the beta (Track A).

## 2. Non-goals

- No type-checker integration (no tsc/pyright analogue) — matches the Python tier decision (ADR-0007).
- No `node_modules`, `tsconfig` `paths`, package `exports` maps, or re-export tracing in v1.
- No framework-specific rules (React hooks, Next.js data flow, Vue reactivity) in v1.
- No cross-language linking (ADR-0004) and no CJS↔ESM interop semantics.
- No runtime profiling ingestion; static analysis stays approximate ("optimization candidates").
- No new CLI flags. No schema MAJOR bump. No change to the `semantic-clang` feature or C-only resolver.
- No edits to Atlas surfaces, `src/reporter/views.rs`, reporter `src/reporter/mod.rs` wiring, main state, or C/D track files. Scanner/analyzer `mod.rs` files are in scope.

## 3. Tier model and parser

**Tier:** `syntactic` throughout. `resolver_tier` is never claimed as `semantic`; no libclang involvement.

**Parser: tree-sitter, exact-pinned (ADR-0001 G5).**

| Crate | Pin | ABI | License | Role |
|---|---|---|---|---|
| `tree-sitter` | `=0.26.13` | accepts 13–15 | MIT | existing runtime (unchanged) |
| `tree-sitter-javascript` | `=0.25.0` | 15 | MIT | `.js`, `.jsx`, `.mjs`, `.cjs` |
| `tree-sitter-typescript` | `=0.23.2` | 14 | MIT | `.ts`, `.mts`, `.cts` (`LANGUAGE_TYPESCRIPT`) and `.tsx` (`LANGUAGE_TSX`) |

Both grammar crates are official `tree-sitter` org repos, compile **C11 only** (verified in each
`bindings/rust/build.rs` — no C++ requirement), and add no copyleft. `deny.toml` already allows MIT.
No other direct dependency is added: `tree-sitter-language` is the shared `LanguageFn` glue crate already in the lock, and `cc` is build-only; the exact closure is whatever `Cargo.lock` pins.

**Rejected alternatives.** `oxc_parser` (latest requires Rust **1.97** > the 1.92 floor; arena AST; weekly
version churn; dozens of crates) and `swc_ecma_parser` (unverified MSRV; major-version churn; heavy
`SourceMap`/`Handler` scaffolding). Both would break the deliberate-upgrade discipline of ADR-0001 G5.

## 4. Language identity and detection

Schema tokens: **`js`, `ts`, `tsx`**. Tokens follow the *grammar* boundary, not cosmetics: `.jsx` parses
with the JS grammar (which embeds JSX), so it stays `js`; `.ts` and `.tsx` use two distinct grammars, so
they are two tokens.

Extension map (longest suffix first; unknown extensions still dropped silently as today):

| Suffix | Token |
|---|---|
| `.d.ts`, `.d.mts`, `.d.cts` | **skipped** (declaration-only; no runtime code, all rule output would be FP) |
| `.tsx` | `tsx` |
| `.ts`, `.mts`, `.cts` | `ts` |
| `.js`, `.jsx`, `.mjs`, `.cjs` | `js` |

**Discovery policy.** Directory skips are **global** — discovery is language-blind at directory level
(`src/scanner/mod.rs:314-354` walks directories, then filters file names). Skip directories named
`node_modules`, `bower_components`, `dist`, `build`, `out`, `coverage`. Dot-directories are already skipped
today, so `.next`, `.nuxt`, `.svelte-kit`, `.cache` need no entries. File-level skips apply to **JS/TS files
only**: `*.min.js`, `*.min.mjs`, `*.min.cjs`, `*.bundle.js`, `*.min.ts`, `*.min.mts`, `*.min.cts`, and
files larger than **2 MiB** (minified
bundles). All skips are silent and deterministic (path/metadata-based, no content sniffing). Skip counters
are **not** added to the schema in v1.8.

**Consequence (recorded honestly):** C/Rust/Python files beneath those directory names are no longer
scanned. The existing corpora contain none of them, so their finding sets and goldens do not move; the
discovery change is documented in the output contract and covered by acceptance criterion 6.

**Determinism:** discovery stays lexicographic (existing sort); all new maps are `BTreeMap`; no `HashMap`
iteration may reach output; `serde_json` stays default-features (no `preserve_order`).

## 5. Schema and output contract

**One MINOR bump v1.7 → v1.8**, **landed in B2-W2** and declaring **all three** tokens up front (B3 then
emits `ts`/`tsx` without a second bump — this is the approved trade: one artifact re-emission and one
golden regeneration instead of two).

- `scan.languages.items.enum` → `["c", "js", "python", "rust", "ts", "tsx"]`, still derived from the
  sorted-unique set of `files[].language`; zero-file scans fall back to the supported set.
- `files[].language.enum` → same six values.
- No required list grows; no field is removed or retyped; every v1.7 document still validates once re-marked.
- Artifact re-emitted as `src/schema/metrics.schema.v1.8.json` + byte-identical
  `docs/schema/metrics.schema.v1.8.json`; v1.7 retained as history; drift test updated.
- `$id`, `title`, `description`, `schema_version.const` (`src/schema/metrics.schema.v1.7.json:12`), and
  **both** enum descriptions are rewritten to v1.8 (the v1.7 text still names the old set at
  `:3-5,35-37,71`). The `scan.languages` description must describe the fallback **generically** ("the
  binary's supported set"), not hardcode six, so it tracks the staged `SUPPORTED_LANGUAGES` (four in B2,
  six after B3; D-tb-010).
- `SCHEMA_VERSION`, `SCHEMA_ARTIFACT_NAME`, `EMBEDDED_SCHEMA`, and the CLI `VERSION_SUFFIX` move together;
  the CLI's `"(metrics schema 1.7)"` assertion (`src/cli/mod.rs:396-410`) flags with them.
- **`SUPPORTED_LANGUAGES` (staged, now landed):** B2 → `["c", "js", "python", "rust"]` (no TS backend
  yet); **B3 → `["c", "js", "python", "rust", "ts", "tsx"]` (implemented)**. The schema enum already admits
  all six; the zero-file fallback reports what the scanner can actually detect. Pinned tests updated in B2
  and B3 respectively (`tests/schema_1_4.rs`, `tests/exit_code_matrix.rs`, aggregator unit tests).
- All four existing golden trees regenerate: the 17 artifacts are unchanged **except one filename** —
  `GOLDEN_ARTIFACTS` (`tests/p6_hardening.rs:113`) and the regenerator's `provenance_free` list (`:465`)
  replace `metrics.schema.v1.7.json` with `metrics.schema.v1.8.json` (same 17 entries).
- `rule_id` is an open pattern: new `PERF-JS-*`/`PERF-TS-*` ids need no schema change.
- No new metric fields. `function_cost_v2`, `trip_count_source`, `known_trip_value`, `path_class` are reused
  as-is. `docs/schema/metrics-json-v1.md` gains the v1.7→v1.8 delta entry.
- **Semver position:** the JS/TS track is a **later release** (non-blocking for the beta). Its schema change
  is an **additive MINOR** (v1.7 → v1.8): no required list grows, no field is removed or retyped, and a
  re-marked v1.7 document validates. The crate version stays `0.2.0-beta.1` until the release owner cuts the
  next version; B4 does not bump it.

## 6. Rule catalog

**Shared family rule.** JS and TS/TSX share one analyzer family, `PERF-JS-*`, because the runtime
mechanism is identical (V8 / Node). `RuleLanguage::Js` matches `js|ts|tsx`. `PERF-TS-*` is reserved for
constructs that exist only in TypeScript; the B2 canary proved none, so v1 ships **zero** `PERF-TS-*`
rules. This avoids duplicating ~12 implementations and two diverging catalogs. `RuleLanguage` therefore gains
`Js` and `Ts`; the analyzer partition maps `js`/`ts`/`tsx` to `Js` (+ `Ts` for `ts`/`tsx`). Unknown
languages stay dropped by the analyzer's `_ => {}` arm.

**Catalog discipline (frozen by the B2-W0 canary).** The 12-rule candidate set below was frozen by a
**W0 canary** over three pinned repos — express `v5.1.0`, execa `v9.6.1`, gemini-cli `v0.65.0`
(1,525 production files / 321.1 kLOC) — using frequency × plausibility, explicit silent controls, and a
Perflint/Ruff-analogue baseline; evidence: `docs/evidence/js-canary.md` and
`docs/evidence/js-canary/canary_results.json`, reproducible via `docs/evidence/js-canary/run.sh`.
**11 rules ship; `PERF-JS-LOOP-003` is cut for v1** — all 24 production `for...in` hits iterate
identifiers, with zero array-literal RHS, so the candidate is not statically identifiable without type
information. `ALLOC-001` ships **reframed** behind a mandatory loop-invariance gate (Info-first),
`CALL-001` is guard-gated (only 6/108 production self-calls are unguarded), and `ASYNC-002` is
async-function-gated and test-skewed. Every rule message must cite the engine mechanism (V8/Node), the
fix, and its accepted FP class. The M1 lesson governs: 5 rules once produced 89% of self-scan noise, so
precision beats breadth.

**Catalog wiring was A-owned until A step 12.** `src/reporter/mod.rs` holds `HEURISTIC_CATALOG_V1` (`:82`),
the catalog count/equality assertions (`:3010-3039`), and panics on any uncatalogued `rule_id` at
`:1456` and `:2381` (line numbers re-read on the rebased tree, 2026-10-24; they move with A's edits).
B2 could not ship a rule without a catalog entry. Track B had to either receive
the entries from Track A before B2, or obtain explicit founder approval to edit **only** the catalog block
in that file. Recorded as `B-tb-003` (now resolved); the count assertions move by exactly the number of shipped
JS rules. **Atomic landing:** because `catalog == registry` is asserted (`src/reporter/mod.rs:2999-3004`), a
catalog-only landing ahead of B2 fails the count/equality tests immediately — A's entries and B2's registry
block must land in the same change, or the founder-approved catalog-only B edit happens inside B2-W2.
**Resolved (B2-W2, `D-tb-017`):** the founder approved the atomic catalog-only B edit; the 11 entries
landed with the registry block, and the count/equality/Appendix-A locks moved by exactly 11 (catalog 78).

| Candidate id | Mechanism (why it costs) | Frozen disposition (B2-W0) |
|---|---|---|
| `PERF-JS-ALLOC-001` | Object/array literal constructed per iteration inside a hot loop → young-gen churn; escape analysis rarely removes it | **KEEP — reframed** (loop-invariance gate mandatory, Info-first) |
| `PERF-JS-ALLOC-003` | `concat`/spread/`slice` accumulation in a loop copies n elements per iteration → quadratic | **KEEP (Info)** — spread arm gated on the literal being an assignment RHS to the accumulator (`assigned_from_literal`, B5) |
| `PERF-JS-STR-001` | `+=` string accumulation in a loop → cons-string flattening cost at first use (MEDIUM; V8 ropes mitigate) | **KEEP (MEDIUM)** — honest V8-rope caveat |
| `PERF-JS-LOOP-001` | Nested loop depth ≥ 2 → superlinear iteration with no early exit | **KEEP (Info)** |
| `PERF-JS-LOOP-002` | Synchronous blocking I/O in a loop (`readFileSync`, `execSync`, `spawnSync`, sync `crypto`) | **KEEP (MEDIUM)** |
| `PERF-JS-LOOP-003` | `for...in` over an array → enumerates enumerable string keys incl. the prototype chain | **CUT (v1)** — zero array-literal RHS; identifier RHS is unidentifiable without types |
| `PERF-JS-LOOP-007` | Linear membership/index scan (`indexOf`/`includes`/`find`) inside a loop → O(n²) | **KEEP — flagship** |
| `PERF-JS-ASYNC-001` | Sequential `await` inside a loop over independent work → latency serialization; fix = `Promise.all` | **KEEP — flagship** |
| `PERF-JS-ASYNC-002` | Synchronous blocking call inside an `async` function → event-loop stall | **KEEP — flagship** (async-function gate mandatory; test-skewed) |
| `PERF-JS-CALL-001` | Direct recursion (self-call) with no visible base-case guard | **KEEP (Info) — guard-gated**; B5: bare `callee_path == function name`, `!conditional_guarded`, lexical-binding record — production 180 → 2 |
| `PERF-JS-ARCH-001` | Import cycle between modules | **KEEP (Info)** — relative-only graph limitation published |
| `PERF-JS-TYPE-001` | `delete obj.prop` → hidden-class/dictionary-mode transition | **KEEP (Info)**; B5 fresh-object gate (`operand_root_fresh`) — production 14 → 2 |

**Explicitly cut or deferred (with reason):** `arr.length` hoisting (V8 does not re-read it cost-wise —
folklore); `new RegExp(literal)` in a loop (V8 keeps a regexp compilation cache — the Python `re.compile`
lesson); `for`-loop index vs `map` micro-benchmarks; `JSON.parse(JSON.stringify())` clone (ALLOC covers the
allocation; no separate rule); microtask-ordering rules; anything requiring types.

## 7. Cost model

Reuse `function_cost_v2` exactly; only the loop/trip extraction is language-specific.

- Loop nodes: C-style `for`, `for...of`, `for...in`, `while`, `do...while`. `await` inside a loop does not
  change loop shape (ASYNC-001 carries the cost signal).
- **Literal trip counts** v1: only canonical counted `for (let i = 0; i < N; i++)` with an integer literal
  `N` (and the `<=` variant, clamped to the existing 1_000_000 ceiling). `for...of` over an array literal is
  **not** literal in v1 (deferred; recorded as a limitation). Everything else is `unknown`.
- `trip_count_source` = `literal` when every loop has a literal trip, `partial` when some do, `unknown`
  when the function has loops but none literal; omitted when there are no loops.
- `alloc_site_count` counts allocation events in the function: `new` expressions plus object and array
  literals. Arithmetic is saturating u64, identical to the existing implementation.

## 8. Import graph and resolution (v1, honest)

- Resolve **relative specifiers only** (`./`, `../`), probing `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`,
  `.mts`, `.cts`, then `<dir>/index.<ext>`.
- The nearest `package.json` (walking to the scan root) defines the package root for reporting, not for
  resolution.
- Bare specifiers (`react`, `lodash`), absolute `/...`, URL/`node:` imports → **external** (unresolved) and
  excluded from internal coupling, exactly like angle-bracket C includes.
- Unresolved *internal* imports stay **data-only**: no finding. This mirrors the Rust N1 decision that
  killed the unresolved-edge FP stream.
- No `tsconfig` `paths`, no `node_modules`, no `exports` map, no re-export tracing, no CJS `require`
  dynamic-pattern resolution (static string literals only).
- A malformed or unreadable `package.json` is **per-file data**: it is ignored, the package root falls back
  to the scan root, and the scan continues. It must never panic or produce exit 3.
- Entry detection mirrors Python: `package.json` `main`/`bin` when available, otherwise conventional
  `index.*`/`main.*`/`server.*`/`app.*` roots and `has_main_guard`-style script detection (`require.main ===
  module`, `import.meta.url` guard) → else the `unknown-entry` stub. Confidence follows the existing ladder
  (HIGH/MEDIUM/MEDIUM/LOW). Candidates are **sorted, deduped, first-match**; `bin` may be a string, object
  or array and all three forms are normalised deterministically.
- Reachability stays resolved-edge-only (no name-based edges), same as C/Rust/Python.

## 9. Tests, fixtures, goldens

- **W1 test inventory:** `tests/javascript_backend.rs` (20 integration tests over the real grammar,
  scanner, aggregator and CLI) + the backend unit tests in
  `src/scanner/backends/tree_sitter_javascript/tests.rs` (grammar node kinds, loop context, params, events).
- **B3 test inventory:** `tests/typescript_backend.rs` (17 integration tests: detection/tokens, mixed trees,
  discovery skips, parse health, entry ladder, TS/TSX import resolution, type-only edge erasure, single-file
  policy, parameter types, TSX generic arrows, non-vacuous cross-language scoring, IR leak guard, double-run) + 17
  unit tests in `src/scanner/backends/tree_sitter_typescript/tests.rs` (node-kind locks, TS parameters,
  type-land silence, wrappers incl. callee/receiver decomposition, type-position silence and comment extras, abstract
  classes, enums, JSX, ternary, commented type-import erasure) + committed negatives in
  `tests/fixtures/typescript/` (`type_only.ts`, `type_edges.ts`, `typeof_import.ts`).
- **Grammar link tests:** `tests/javascript_grammar_link.rs` and `tests/typescript_grammar_link.rs` —
  assert the grammar ABI is inside the runtime's accepted window, `set_language` succeeds, a representative
  source parses with zero `ERROR`/missing nodes, and a malformed source is non-vacuously degraded.
- **Corpora:** `examples-js/` (15 scanned sources: 14 `.js` + 1 `.cjs`, one module per rule family,
  explicit silent controls) and **`examples-ts/` (17 scanned sources: 15 `.ts` + 2 `.tsx`, typed
  equivalents of the same rule mechanisms plus a type-only module, a JSX component and generic arrows;
  every one of the 11 shared rules fires exactly once, 18 silent controls)**.
- **Goldens:** `tests/golden/report-js/` and `tests/golden/report-ts/`; `GOLDEN_TREES` grows 4 → 5 in B2
  (`report-js`) and 5 → 6 in B3 (`report-ts`); the
  pinned detail-page set is extended per tree; regen via `PERFPAL_REGEN_GOLDEN=1`.
- **Parser-free IR fixtures:** `tests/fixtures/ir/PERF-JS-*__*.json`, one positive + one control per rule,
  run with no parser bound (ADR-0001 guardrail 2).
- **Negative/suppression fixtures:** minified files, `.d.ts`, `node_modules`, oversized files, JSX-heavy
  files, and TS type-only code must produce zero findings.
- **Coverage locks:** `tests/examples_js_coverage.rs` / `tests/examples_ts_coverage.rs` mirroring the Python
  coverage test (exact rule-id set, zero cross-language ids, `scan.languages`, file count, per-rule counts,
  anti-typo needles).
- **Precision KPI:** `tests/fixtures/precision/js/` + `labels.json` entry; every shipped rule must reach and
  hold the repo ratchet (**precision 1.0 / recall 1.0**), matching the roadmap ratchet.
- **Benchmark ladder:** extend `tests/benchmark/corpus.rs` + `baseline.json` + active-catalog count so JS is
  measured, not asserted.
- **Version-pin sweep (mechanical, landed in B2-W2):** `tests/schema_1_7.rs` became the v1.8 lock and was
  renamed `tests/schema_1_8.rs` (including its required-list comparison against v1.7, the executable proof
  that no required list grew);
  `tests/schema_1_4.rs:88-119,247-291`; `tests/pipeline_e2e.rs:77,85`; `tests/resolver_health.rs:217`;
  `tests/exit_code_matrix.rs:75,90-93`; `tests/i1a_entry_c.rs:406`; `tests/i1b_entry_rust.rs:498`;
  `tests/i1c_entry_python.rs:582`; `tests/atlas_fixtures/mod.rs:155`; `tests/key_views.rs:203`.
  `GOLDEN_ARTIFACTS` and the regenerator's `provenance_free` list take the v1.8 filename (same 17 entries).
- **Reporter catalog:** `HEURISTIC_CATALOG_V1` gained one entry per shipped JS rule — **11 entries** after
  the W0 cut — and the count, catalog==registry and Appendix-A panic tests moved by exactly 11 (landed in
  B2-W2; see §6).
- **Determinism:** masked double-run byte equality on the JS and TS corpora via dedicated suites —
  `tests/determinism_js.rs` (B2) and `tests/determinism_ts.rs` (B3), both reusing the repo's provenance
  maskers. Each suite compares the full artifact set: `metrics.json` (provenance masked), `report.md`,
  `report.html`, `findings-detail.html`, `profile-queue.md/.html`, `charts/*`, and the bundled schema
  artifact. `.length`/ordering checks on every new visitor; a golden tree alone is not a double-run.
- **B4 locks:** `tests/javascript_backend.rs` gains the real-parse negative
  `str_001_real_parse_ignores_a_string_plus_nested_in_a_call` (the `s = f(s + "x")` assignment arm stays
  silent while `s = s + "x"` fires); `src/analyzer/heuristics/js/tests.rs` pins the documented
  spread-root attribution order; the reporter unit test `dependency_labels_cover_the_js_family` locks the
  `import/require` caption/family prose and the unknown-language fallback. Golden trees `report-js` and
  `report-ts` are regenerated for the label change; the other four trees stay byte-identical.

## 10. CLI behavior

- No new flags. Auto-detection by extension is the only entry path. A single-file target with a JS suffix
  is scanned normally **except** the JS skip names (`*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js`/
  `*.min.ts`/`*.min.mts`/`*.min.cts`), which are skipped silently. A single-file target with a
  **declaration-only** TypeScript suffix
  (`.d.ts`, `.d.mts`, `.d.cts`) is skipped silently — never parsed as C; `.ts`/`.mts`/`.cts`/`.tsx` are
  scanned with their `ts`/`tsx` tokens (B3). A genuinely unknown extension keeps the documented C fallback.
- `--version` suffix moves to `(metrics schema 1.8)` with the schema bump; the `about` line widens from
  "C/Rust codebase" to name JS/TS. These string changes are forced by the schema bump (precedent:
  `python-backend` W3).
- Exit-code contract unchanged (0/1/2/3); an unreadable JS file is per-file `ParseStatus::Failed` data, not
  a scan abort.

## 11. Acceptance criteria

1. `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test
   --locked`, `cargo deny check licenses`, and the `semantic-clang` feature gate all pass on the final tree.
2. A scan of `examples-js` and `examples-ts` emits only `js`/`ts`/`tsx` language tokens and every shipped
   `PERF-JS-*` rule fires at least once, with silent controls emitting zero findings.
3. Two consecutive scans of the same corpus are byte-identical **modulo the contract-permitted provenance**
   (`scan.started_at`, `scan.duration_ms`; `docs/report/output-contract.md:15,198`), which the double-run
   tests mask before comparing.
4. No `node_modules`, `dist`, `build`, `.d.ts`, `*.min.js`, or >2 MiB JS/TS file contributes a finding.
5. The v1.8 artifact is byte-identical between `src/schema/` and `docs/schema/`; all five golden trees (six after B3) carry
   it; the v1.7 artifact is retained as history.
6. No change to Atlas behavior and no schema field additions. C/Rust/Python finding sets are unchanged on the
   existing corpora (none contains a global-skip directory); the only mechanical delta is the v1.8 artifact
   name in every golden tree. The global directory skip is documented as an intentional discovery change.
7. Parallel review panel: >= 2 reviewer + >= 2 requirements-reviewer lanes, >= 1 APPROVE per family, zero
   unresolved blocking findings.
8. B4/B5 evidence: `docs/heuristics/js-catalog.md` exists and matches the implemented 11 rules;
   `docs/evidence/js-b5/real-code-canary.md` records the gate-fixed pinned-corpus re-run (the B4 baseline
   remains at `docs/evidence/js-b4/real-code-canary.md`); the reporter label tests and the real-parse
   STR-001 negative pass; README/roadmap/modules/output-contract/Cargo.toml name the JS/TS tier and the six
   tokens.

## 12. Accepted limitations (must be published in the report/docs)

No type information; no path aliases or `node_modules` resolution; declaration files and minified bundles
skipped; no framework semantics; JSX is parsed structurally but not semantically; dynamic `require`/`import()`
with computed specifiers is invisible; static analysis is approximate and profile-to-confirm. Additional
language-specific limits: **CJS/ESM module kind is not inferred** (no `package.json` `"type"` analysis);
**type-only imports are erased** — `import type`/`import typeof`/`export type` produce no include edge, so
`PERF-JS-ARCH-001` cannot report a type-only cycle (inline `{ type X, y }` specifiers keep the value edge);
**type land is never scored** — interfaces, type aliases, ambient `declare` blocks, overload signatures and
`typeof import("…")` type positions emit no events/records, and a file may be `Partial` because of such a
type position while still emitting no finding from it; **`declare class`** emits no `StructDef`;
**TS parameter types are IR-internal** (`param_types`, raw annotation text, `this` erased, `"unknown"`
when absent — not mirrored in `metrics.json`); **TSX generic arrows** — the pinned TSX grammar resolves both
`<T,>(x: T) => x` and `<T>(x: T) => x` at expression position, with `<T,>` the portable form (the limitation
is re-checked by `tests/typescript_backend.rs` on any grammar upgrade); **`.d.ts` skipping** makes
type-only modules invisible, so imports that only resolve through them become unresolved-internal data-only;
a **minified file under 2 MiB** that is not named `*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js`/
`*.min.ts`/`*.min.mts`/`*.min.cts` is still parsed and can fire rules;
**extension matching is case-sensitive**, so `.JS`/`.TS` are silently dropped. **Single-file targets follow
the discovery skip policy:** the seven JS min/bundle names and declaration-only `.d.ts`/`.d.mts`/`.d.cts`
are skipped silently, while `.ts`/`.mts`/`.cts`/`.tsx` are scanned and a genuinely unknown extension keeps
the C fallback. **Class bodies are not event owners:**
methods and arrow-function fields are their own records, but non-arrow field-initializer expressions and
`class_static_block` bodies are not event-scanned at all.

**B4-published limitations and locks (B5 supersedes the fixed gates):**

- **Spread-arm attribution (`ALLOC-003`, B5):** the spread literal must be (part of) the RHS of an
  assignment to the attributed root (`assigned_from_literal`); a spread in an arbitrary expression is
  silent, and a member-target accumulator (`currentTurn.content.parts = [...]`) is silent rather than
  mis-attributed. When several loop-reassigned referenced names also satisfy the gate, the first in the
  IR's sorted `referenced_names` is reported, not necessarily the spread operand; member/`this`
  accumulators remain undetected (a member-root attribution field is a recorded follow-up).
- **STR-001 assignment arm:** the `x = x + "…"` form fires only when the binary is the **whole** RHS.
  `s = f(s + "x")` is silent by design (the `+` is nested in a call), locked by a real-parse integration
  test. `s += "x"` remains the `+=` arm.
- **Reporter labels:** JS/TS/TSX render as the shared `import/require` mechanism and
  ``JavaScript/TypeScript `import`/`require` bindings``; `PERF-TS-*` remains reserved/empty, so no
  TypeScript-only label is claimed.
- **Real-code canary:** the B5 re-run measures hits and a hand-verified sample on three pinned
  repositories; it is not a corpus-wide precision claim, and the synthetic 1.0/1.0 KPI remains
  synthetic-only (`R-tb-016`). The canary does not change the rule set.
- **B5 gate close-out (no schema/catalog change):** `PERF-JS-CALL-001` is **merge-ready** — production
  180 → 2, both genuine recursion; the B4 member/method-name collision class is removed by requiring
  `callee_path` to equal the bare function name, `!conditional_guarded`, and a lexical-binding record.
  The accepted false-negative class is recursion guarded only by an `if`/`catch` ancestor (11 observed:
  10 gemini-cli production + 1 execa catch-retry). `PERF-JS-TYPE-001` adds the fresh-object gate
  (`operand_root_fresh`), production 14 → 2; `PERF-JS-ALLOC-003` tightens the spread arm, production
  20 → 18. `PERF-JS-ALLOC-001` remains noisy but gated (485 production hits); `PERF-JS-ARCH-001` remains
  structurally blind in alias monorepos (0 gemini-cli, 4 execa cycles); `PERF-JS-ASYNC-002` remains
  test-skewed (134 production vs 1,047 non-production). Cross-cutting: `path_class` now recognizes
  `.spec.`/`-spec.` basenames and `test-*`/`*-test(s)`/`*-spec(s)` segments (production total 1,336 →
  1,178; `src/test-data` stays `fixture`), and `findings[].symbol` resolves the narrowest containing
  record. Evidence: `docs/evidence/js-b5/real-code-canary.md`.
- **`metrics.json` `findings[].symbol` for JS/TS (B5 fixed):** the aggregator chooses the **narrowest
  containing** function record (line containment, width, then id; `src/aggregator/mod.rs::findings_with_symbols`)
  instead of first-hit. In the JS/TS goldens 10/11 findings resolve to a real function and the single
  `<module>` is the module-level `ARCH-001` cycle anchor; Python goldens improved the same way. The ranked
  action list/key views and `metrics.json` therefore agree.