# Spec: js-ts

Scope: feature

# Spec: js-ts

**Scope:** feature. **Track:** B (`feat/js-ts`). **Status:** B2-W0 complete (pins, link tests, canary freeze); no backend/analyzer code yet.
**Owners:** Track B owns this spec, `docs/plans/js-ts.md`, the JS/TS ADR, the grammar pins, the JS/TS
fixtures/goldens, and the `src/schema` language values. Track A owns `views.rs`/`mod.rs` wiring until A step 12.

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
(`src/scanner/mod.rs:221-259` walks directories, then filters file names). Skip directories named
`node_modules`, `bower_components`, `dist`, `build`, `out`, `coverage`. Dot-directories are already skipped
today, so `.next`, `.nuxt`, `.svelte-kit`, `.cache` need no entries. File-level skips apply to **JS/TS files
only**: `*.min.js`, `*.min.mjs`, `*.min.cjs`, `*.bundle.js`, and files larger than **2 MiB** (minified
bundles). All skips are silent and deterministic (path/metadata-based, no content sniffing). Skip counters
are **not** added to the schema in v1.8.

**Consequence (recorded honestly):** C/Rust/Python files beneath those directory names are no longer
scanned. The existing corpora contain none of them, so their finding sets and goldens do not move; the
discovery change is documented in the output contract and covered by acceptance criterion 6.

**Determinism:** discovery stays lexicographic (existing sort); all new maps are `BTreeMap`; no `HashMap`
iteration may reach output; `serde_json` stays default-features (no `preserve_order`).

## 5. Schema and output contract

**One MINOR bump v1.7 → v1.8**, landing in B2, declaring **all three** tokens up front (B3 then emits
`ts`/`tsx` without a second bump — this is the approved trade: one artifact re-emission and one golden
regeneration instead of two).

- `scan.languages.items.enum` → `["c", "js", "python", "rust", "ts", "tsx"]`, still derived from the
  sorted-unique set of `files[].language`; zero-file scans fall back to the supported set.
- `files[].language.enum` → same six values.
- No required list grows; no field is removed or retyped; every v1.7 document still validates once re-marked.
- Artifact re-emitted as `src/schema/metrics.schema.v1.8.json` + byte-identical
  `docs/schema/metrics.schema.v1.8.json`; v1.7 retained as history; drift test updated.
- `$id`, `title`, `description`, `schema_version.const` (`src/schema/metrics.schema.v1.7.json:12`), and
  **both** enum descriptions are rewritten to v1.8 (the v1.7 text still names the old set at
  `:3-5,35-37,71`). The `scan.languages` description must describe the fallback **generically** ("the
  binary's supported set"), not hardcode six, because B2 emits four values on a zero-file scan (D-tb-010).
- `SCHEMA_VERSION`, `SCHEMA_ARTIFACT_NAME`, `EMBEDDED_SCHEMA`, and the CLI `VERSION_SUFFIX` move together;
  the CLI's `"(metrics schema 1.7)"` assertion (`src/cli/mod.rs:396-410`) flags with them.
- **`SUPPORTED_LANGUAGES` is staged:** B2 → `["c", "js", "python", "rust"]` (no TS backend yet);
  B3 → `["c", "js", "python", "rust", "ts", "tsx"]`. The schema enum already admits all six; the zero-file
  fallback reports what the scanner can actually detect. Pinned tests update in B2 and B3 respectively.
- All four existing golden trees regenerate: the 17 artifacts are unchanged **except one filename** —
  `GOLDEN_ARTIFACTS` (`tests/p6_hardening.rs:113`) and the regenerator's `provenance_free` list (`:465`)
  replace `metrics.schema.v1.7.json` with `metrics.schema.v1.8.json` (same 17 entries).
- `rule_id` is an open pattern: new `PERF-JS-*`/`PERF-TS-*` ids need no schema change.
- No new metric fields. `function_cost_v2`, `trip_count_source`, `known_trip_value`, `path_class` are reused
  as-is. `docs/schema/metrics-json-v1.md` gains the v1.7→v1.8 delta entry.

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

**Catalog wiring is A-owned until A step 12.** `src/reporter/mod.rs` holds `HEURISTIC_CATALOG_V1` (`:82`),
the 67-entry/count/equality assertions (`:2980-3002`), and panics on any uncatalogued `rule_id` at
`:1456` and `:2381` (line numbers re-read on the rebased tree, 2026-10-24; they move with A's edits).
B2 cannot ship a rule without a catalog entry. Track B must either receive
the entries from Track A before B2, or obtain explicit founder approval to edit **only** the catalog block
in that file. Recorded as blocker `B-tb-003`; the count assertions move by exactly the number of shipped
JS rules. **Atomic landing:** because `catalog == registry` is asserted (`src/reporter/mod.rs:2999-3004`), a
catalog-only landing ahead of B2 fails the count/equality tests immediately — A's entries and B2's registry
block must land in the same change, or the founder-approved catalog-only B edit happens inside B2-W2.

| Candidate id | Mechanism (why it costs) | Frozen disposition (B2-W0) |
|---|---|---|
| `PERF-JS-ALLOC-001` | Object/array literal constructed per iteration inside a hot loop → young-gen churn; escape analysis rarely removes it | **KEEP — reframed** (loop-invariance gate mandatory, Info-first) |
| `PERF-JS-ALLOC-003` | `concat`/spread/`slice` accumulation in a loop copies n elements per iteration → quadratic | **KEEP (Info)** |
| `PERF-JS-STR-001` | `+=` string accumulation in a loop → cons-string flattening cost at first use (MEDIUM; V8 ropes mitigate) | **KEEP (MEDIUM)** — honest V8-rope caveat |
| `PERF-JS-LOOP-001` | Nested loop depth ≥ 2 → superlinear iteration with no early exit | **KEEP (Info)** |
| `PERF-JS-LOOP-002` | Synchronous blocking I/O in a loop (`readFileSync`, `execSync`, `spawnSync`, sync `crypto`) | **KEEP (MEDIUM)** |
| `PERF-JS-LOOP-003` | `for...in` over an array → enumerates enumerable string keys incl. the prototype chain | **CUT (v1)** — zero array-literal RHS; identifier RHS is unidentifiable without types |
| `PERF-JS-LOOP-007` | Linear membership/index scan (`indexOf`/`includes`/`find`) inside a loop → O(n²) | **KEEP — flagship** |
| `PERF-JS-ASYNC-001` | Sequential `await` inside a loop over independent work → latency serialization; fix = `Promise.all` | **KEEP — flagship** |
| `PERF-JS-ASYNC-002` | Synchronous blocking call inside an `async` function → event-loop stall | **KEEP — flagship** (async-function gate mandatory; test-skewed) |
| `PERF-JS-CALL-001` | Direct recursion (self-call) with no visible base-case guard | **KEEP (Info) — guard-gated** |
| `PERF-JS-ARCH-001` | Import cycle between modules | **KEEP (Info)** — relative-only graph limitation published |
| `PERF-JS-TYPE-001` | `delete obj.prop` → hidden-class/dictionary-mode transition | **KEEP (Info)** |

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

- **Grammar link tests:** `tests/javascript_grammar_link.rs` and `tests/typescript_grammar_link.rs` —
  assert the grammar ABI is inside the runtime's accepted window, `set_language` succeeds, a representative
  source parses with zero `ERROR`/missing nodes, and a malformed source is non-vacuously degraded.
- **Corpora:** `examples-js/` (Node-style ESM + CJS mix, one module per rule family, explicit silent
  controls) and `examples-ts/` (`.ts` + `.tsx`, type-only constructs that must NOT fire).
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
- **Version-pin sweep (mechanical, explicit in B2):** `tests/schema_1_7.rs` becomes the v1.8 lock (including
  its required-list comparison against v1.7, the executable proof that no required list grew);
  `tests/schema_1_4.rs:88-119,247-291`; `tests/pipeline_e2e.rs:77,85`; `tests/resolver_health.rs:217`;
  `tests/exit_code_matrix.rs:75,90-93`; `tests/i1a_entry_c.rs:406`; `tests/i1b_entry_rust.rs:498`;
  `tests/i1c_entry_python.rs:582`; `tests/atlas_fixtures/mod.rs:155`; `tests/key_views.rs:203`.
  `GOLDEN_ARTIFACTS` and the regenerator's `provenance_free` list take the v1.8 filename (same 17 entries).
- **Reporter catalog:** `HEURISTIC_CATALOG_V1` gains one entry per shipped JS rule — **11 entries** after
  the W0 cut — and the count, catalog==registry and Appendix-A panic tests move by exactly 11 (see §6 and
  blocker `B-tb-003`).
- **Determinism:** masked double-run byte equality on the JS and TS corpora via dedicated suites —
  `tests/determinism_js.rs` (B2) and `tests/determinism_ts.rs` (B3), both reusing the repo's provenance
  maskers. Each suite compares the full artifact set: `metrics.json` (provenance masked), `report.md`,
  `report.html`, `findings-detail.md/.html`, `profile-queue.md/.html`, `charts/*`, and the bundled schema
  artifact. `.length`/ordering checks on every new visitor; a golden tree alone is not a double-run.

## 10. CLI behavior

- No new flags. Auto-detection by extension is the only entry path; a single-file target with a JS/TS
  extension is scanned normally.
- The single-file "unknown extension defaults to C" fallback stays as-is (only reachable for unknown
  extensions).
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
5. The v1.8 artifact is byte-identical between `src/schema/` and `docs/schema/`; all six golden trees carry
   it; the v1.7 artifact is retained as history.
6. No change to Atlas behavior and no schema field additions. C/Rust/Python finding sets are unchanged on the
   existing corpora (none contains a global-skip directory); the only mechanical delta is the v1.8 artifact
   name in every golden tree. The global directory skip is documented as an intentional discovery change.
7. Parallel review panel: >= 2 reviewer + >= 2 requirements-reviewer lanes, >= 1 APPROVE per family, zero
   unresolved blocking findings.

## 12. Accepted limitations (must be published in the report/docs)

No type information; no path aliases or `node_modules` resolution; declaration files and minified bundles
skipped; no framework semantics; JSX is parsed structurally but not semantically; dynamic `require`/`import()`
with computed specifiers is invisible; static analysis is approximate and profile-to-confirm. Additional
language-specific limits: **CJS/ESM module kind is not inferred** (no `package.json` `"type"` analysis);
**TSX vs generic-arrow ambiguity** can degrade parse health for `<T,>` constructs; **`.d.ts` skipping** makes
type-only modules invisible, so imports that only resolve through them become unresolved-internal data-only;
a **minified file under 2 MiB** that is not named `*.min.*`/`*.bundle.js` is still parsed and can fire rules;
**extension matching is case-sensitive**, so `.JS`/`.TS` are silently dropped.