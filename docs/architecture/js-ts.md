# Perfpal — JS/TS backend (Track B — inventory and freeze record)

**Status:** B1 planning complete; **B2-W0 complete** (exact pins, grammar-link tests, canary freeze);
**B2-W1 committed `90d5677` (pre-rebase `7fdfa1a`)**; **B2-W2 committed `edfc98a`** (11 `PERF-JS-*` rules,
catalog 78, schema v1.8, `examples-js`, `report-js` golden); **B3 implemented — uncommitted, full DoD green,
panel recorded; commit pending founder approval.** B3 adds the TypeScript/TSX backend (dialect over the JS
visitor), the six-value `SUPPORTED_LANGUAGES`, `examples-ts/` and the `report-ts` golden (§16). The merge
gate `A-tb-006` still requires founder approval. Execution plan: `docs/adr/ADR-0011-js-ts-backend.md`,
`docs/specs/js-ts.md`, `docs/plans/js-ts.md`.
**Branch:** `feat/js-ts`, rebased on `main` @ `9a94905`; B1 commits `3adf115` + `3e414e3`; B2 base `821b984`.

## 1. Purpose

Record the Phase 0 inventory for adding JavaScript (B2) then TypeScript/TSX (B3) as first-class backends:
the exact extension points in the current tree, the node-kind→IR mapping to build against, the rule
disposition table, the cost model, and the accepted limitations. This is the design record the B2/B3
sessions execute against; it is not itself code.

## 2. Corrections to the track brief

The brief (`docs/tracks/js-ts.md`) names `src/parser/` and `src/rules/` as read-first targets. **Neither
directory exists.** The real seams are:

- parsing + extraction: `src/scanner/backends/tree_sitter_<lang>/` (there is no separate parser layer);
- rules: `src/analyzer/mod.rs` (registry) + `src/analyzer/heuristics/` (implementations).

The ownership matrix and the "do not touch" list are still accurate.

## 3. Pinned pair (ABI compatibility)

| Crate | Pin | Parses | ABI | Runtime window | Licence |
|---|---|---|---|---|---|
| `tree-sitter` | `=0.26.13` (unchanged) | — | accepts 13–15 | — | MIT |
| `tree-sitter-javascript` | `=0.25.0` | `.js`/`.jsx`/`.mjs`/`.cjs` | 15 | OK | MIT |
| `tree-sitter-typescript` | `=0.23.2` | `.ts`/`.mts`/`.cts` via `LANGUAGE_TYPESCRIPT`; `.tsx` via `LANGUAGE_TSX` | 14 | OK | MIT |

- Both crates compile pre-generated C parsers with **C11 only** (`bindings/rust/build.rs`); no C++ stdlib.
- Neither crate declares `rust-version`. **B2-W0 compiled both on 1.92** (`rustc 1.92.0`); the
  documented fallback `tree-sitter-javascript = "=0.23.1"` was not needed.
- `tree-sitter-language` and build-only `cc` are the only transitives; `deny.toml` already allows `MIT`.

## 4. Phase 0 inventory — every touchpoint, in dependency order

Line numbers are current-tree (`821b984`) and are evidence; re-read before editing.

### 4.1 Scanner registration and detection

| # | File:line | Current | B2/B3 change |
|---|---|---|---|
| 1 | `src/scanner/mod.rs:111-116` | `enum LanguageKind { C, Rust, Python }` | add `Js`, `Ts`, `Tsx` |
| 2 | `src/scanner/mod.rs:263-274` | `language_of`: `.rs`/`.py`/`.pyi`/`.c`/`.h`; else `None` | add longest-suffix-first JS/TS map; `.d.*` → `None` |
| 3 | `src/scanner/mod.rs:287-292` | `extract_unit` dispatch match | add three arms |
| 4 | `src/scanner/mod.rs:307-311` | `instrument::populate` language token | add `"js"`/`"ts"`/`"tsx"` |
| 5 | `src/scanner/mod.rs:322-330` | `failed_unit` language token arm | add three arms |
| 6 | `src/scanner/mod.rs:107` | `SUPPORTED_LANGUAGES = ["c","python","rust"]` | B2 `["c","js","python","rust"]`; **B3 landed six** `["c","js","python","rust","ts","tsx"]` (staged — never advertise a backend that does not exist) |
| 7 | `src/scanner/mod.rs:188-213` | per-language whole-scan passes | add JS/TS import-resolution pass |
| 8 | `src/scanner/mod.rs:166` | single-file unknown extension → `C` | unknown extensions keep `C`; single-file JS skip names and declaration-only TS files (`.d.ts`/`.d.mts`/`.d.cts`) are skipped silently — `.ts`/`.mts`/`.cts`/`.tsx` are scanned (B3) |
| 9 | `src/scanner/backends/mod.rs:16-18` | three `pub mod` declarations | add JS + TS modules |

### 4.2 Backend contract

- `src/scanner/backends/mod.rs:28-35` — `struct Extraction { file, parse_health, includes, global_vars, structs, macros, functions }` is the **only** shared backend interface. There is no trait, no registry, and no per-backend extension metadata.
- Per-language entry point: `pub fn extract(path: &str, source: &[u8]) -> Extraction` + `pub fn language() -> Language`.
- Grammar binding pattern: `tree_sitter_python::LANGUAGE.into()` (`tree_sitter_python/mod.rs:156-158`).
- Parser construction + failure handling pattern: `Parser::new()` → `set_language` → `parse`; any failure yields `ParseStatus::Failed`, never a panic (`tree_sitter_python/mod.rs:54-65`).
- **Implemented (B2-W1):** `src/scanner/backends/tree_sitter_javascript/{mod.rs,scan.rs,infer.rs,tests.rs}`
  exists and mirrors the Python layout; `extract` returns the shared `Extraction` and `language()` returns
  the pinned `tree_sitter_javascript::LANGUAGE`. There is still no trait/registry: the dispatch is the
  `LanguageKind::Js` arm in `src/scanner/mod.rs`.

### 4.3 Discovery / ignore policy (new risk found in B1)

- `src/scanner/mod.rs:314-354` — discovery admits only known extensions, skips dot-directories, does not follow symlinks, has a canonical-path cycle guard.
- **There is no `.gitignore` support and no binary guard.** The W1 JS/TS policy adds the global directory
  skip set and a JS-only file-size cap (§7); `discover_packages` skips `node_modules` only for **Cargo**
  package discovery, not for source scanning.
- Without a JS/TS policy, `node_modules/**/*.js`, `dist/`, and minified bundles would be scanned and would
  dominate findings. **Implemented in B2-W1:** the global directory skip set plus the JS/TS-only
  file-name/size guard (§7). C/Rust/Python discovery changes only beneath the skipped directory names (the
  existing corpora contain none).

### 4.4 Resolver

- Python: real whole-scan import pass `resolve_python_edges` (`src/scanner/mod.rs:537`), with the
  dotted-module helper `python_dotted_module` (`:617`) and the same normalization/external-bucket split.
- Rust: Cargo-aware module graph (`discover_packages:980`, `RustModuleMap::build:1037`,
  `resolve_rust_mod_edges:1177`, `resolve_rust_use_edges:1218`).
- JS/TS v1 mirrors the Python shape at lower ambition: relative specifiers only (§9).
  **Implemented (B2-W1):** the backend emits raw specifier text with `Quoted` for `./`/`../` and `Angle`
  for bare/absolute/URL/`node:` specifiers; `resolve_js_edges` probes the scanned-path set (exact path,
  then `.ts/.tsx/.js/.jsx/.mjs/.cjs/.mts/.cts`, then `<dir>/index.<ext>`) and normalizes an internal miss
  to `to_path: null`. `mark_js_entry_declarations` reads `package.json` `main`/`bin` into the IR-internal
  `entry_declared` fact.
- Reachability: resolved edges only; name-based edges are LOW confidence and excluded (the exclusion is
  documented on `resolved_c_edges` / `rust_reachable` in `src/aggregator/mod.rs`). Python's reachable set
  is honestly empty (`python_reachable:1150`).
  **JS (B2-W1):** `resolved_js_edges` (`:1258`) is also honestly empty (the backend emits file-level
  imports, not function call facts), so JS reachability is entries-only by construction.

### 4.5 Analyzer / rules

| # | File:line | Current | B2/B3 change |
|---|---|---|---|
| 1 | `src/analyzer/mod.rs:51-56` | `enum RuleLanguage { C, Python, Rust }` | add `Js`, `Ts` |
| 2 | `src/analyzer/mod.rs:87-424` | `REGISTRY`, globally ascending `rule_id`; declaration order load-bearing (first-wins dedupe) | insert `PERF-JS-*` between `PERF-C-*` and `PERF-PY-*`; `PERF-TS-*` after `PERF-RS-*` |
| 3 | `src/analyzer/mod.rs:462-472` | partition by language; `_ => {}` silently drops unknown | add `js`/`ts`/`tsx` → `Js` (+ `Ts`), else drop |
| 4 | `src/analyzer/mod.rs:477-487` | exhaustive slice dispatch | add arm |
| 5 | `src/analyzer/mod.rs:549-585` | semantic-claim allowlist by rule id | unchanged (JS/TS stay syntactic) |
| 6 | `src/analyzer/finding.rs:17-107` | `*_RULE_ID` constants | add `JS_*`/`TS_*` constants |
| 7 | `src/analyzer/heuristics/mod.rs:15-70` | family modules + re-exports | add `js/` family |
| 8 | `src/scanner/instrument.rs:93-168` | alloc-family classification per language | add JS alloc/idiom arm |
| 9 | `src/scanner/instrument.rs:213-238` | `normalize_idiom` strips `//` and `/* */` for **every** language; only `#` is Python-gated (`strip_hash`, `:214-215`) | **no change expected** — JS `#` (private fields) must stay unstripped; verify C/Rust/Python idiom hashes are unchanged |
| 10 | `src/scanner/instrument.rs:289-325` | `detect_error_path` per language | add `throw`/`try`/`catch` arm |

Rule counts after B2-W2: **C 23, JS 11, Python 22, Rust 22 = 78**. Naming:
`PERF-<LANG>-<FAMILY>-<NNN>`.

**Implemented in B2-W2:** `JS_*_RULE_ID` constants in `src/analyzer/finding.rs`;
`RuleLanguage::{Js,Ts}` and the ascending `PERF-JS-*` block between C and Python in
`src/analyzer/mod.rs`; the `js|ts|tsx` partition (`Ts` is a reserved arm that runs over `&[]` in v1); and the
family module `src/analyzer/heuristics/js/mod.rs`. Detail in §13.

### 4.6 Aggregator / IR

| # | File:line | Current | Change |
|---|---|---|---|
| 1 | `src/ir/mod.rs:164-174` | `FileRecord.language: String` (free-form) | no shape change; emit `js`/`ts`/`tsx` |
| 2 | `src/ir/mod.rs:583-599` | `FunctionRecord` entry/reachability fields | no change |
| 3 | `src/aggregator/mod.rs:130-145` | three language lanes in `assemble` | **implemented (B2-W1):** JS lane + `js_reachable` over the honestly empty resolved-edge set |
| 4 | `src/aggregator/mod.rs:229-238` | stats gate `c|rust|python` | widened to include `js` in B2-W1 |
| 5 | `src/aggregator/mod.rs:1174-1270` | `detect_*_entries` / `resolved_*_edges` / `*_reachable` (JS: `detect_js_entries:1174`, `resolved_js_edges:1258`, `js_reachable:1265`; Python: `:999`, `:1088`, `:1150`) | **implemented (B2-W1):** `detect_js_entries`, `resolved_js_edges`, `js_reachable` |
| 6 | `src/aggregator/mod.rs:205-216` | `scan_languages` sorted-unique + fallback | inherits the new const |

### 4.7 Schema / output

| # | File:line | Current | Change |
|---|---|---|---|
| 1 | `src/schema/mod.rs:96-109` | `SCHEMA_VERSION = "1.7"`, artifact name, `EMBEDDED_SCHEMA` | `1.8` + `metrics.schema.v1.8.json` |
| 2 | `src/schema/metrics.schema.v1.7.json:3-5,12,35-37,71` | `$id`/`title`/`description` name v1.7 + the old language set; `schema_version.const "1.7"`; `enum ["c","rust","python"]` ×2 | v1.8 file with `$id`, `title`, `description`, `schema_version.const` and both enums moved; the `files[].language` and `scan.languages` descriptions name the six values, and the **fallback clause stays generic** ("the supported set") so it tracks `SUPPORTED_LANGUAGES` (four in B2, six after B3; D-tb-010) |
| 3 | `docs/schema/metrics.schema.v1.7.json` | byte-identical doc copy | v1.8 copy, byte-identical |
| 4 | `src/schema/mod.rs:667-686` | drift + artifact-name locks | unchanged logic, new names |
| 5 | `src/cli/mod.rs:39` | `VERSION_SUFFIX` hardcodes `1.7` | `1.8` (locked by `src/cli/mod.rs:396-410`, which also hardcodes `"(metrics schema 1.7)"`) |
| 6 | `docs/schema/metrics-json-v1.md` | policy doc | add v1.7→v1.8 delta |
| 7 | `src/reporter/mod.rs:2700-2718, 2762-2788` | mechanism/caption maps | **A-owned until A step 12**; after step 12, B4 may add label/caption strings only (no Atlas/view changes) |
| 8 | `src/reporter/mod.rs:1077-1085, 1646-1655` | language list rendering | label-only; verify no exhaustive match breaks |
| 9 | `src/reporter/mod.rs:82`, `:1456`, `:2381`, `:3010-3039` | `HEURISTIC_CATALOG_V1` + count/equality assertions + Appendix-A panics | **landed (B2-W2, `D-tb-017`):** the founder approved the atomic catalog-only B edit in the B2-W2 work order; the 11 entries landed **with** the registry block. Catalog is now **78** (23 C + 11 JS + 22 PY + 22 RS); the count lock, `catalog == registry`, and Appendix-A panics moved by exactly 11. `B-tb-003` resolved |
| 10 | `docs/architecture/modules.md:22` | determinism table still records `["c","rust"]` | B4 updates the supported-set row alongside §4.2/§5.2 |

`rule_id` is an **open pattern** in the schema: new rule ids need no schema change. Only the language
token required the minor bump.

**B2-W2 additive IR fact (IR-internal, no schema/DTO change).** `AllocEvent.has_spread: bool`
(`src/ir/mod.rs`, `#[serde(default)]`) is `true` only for an array literal containing at least one
`spread_element` child; `new` expressions and object literals always report `false`. The JS backend sets
it in `tree_sitter_javascript/scan.rs`, and `PERF-JS-ALLOC-003` uses it to recognize the
`acc = [...acc, x]` accumulator without re-opening the engine seam. `metrics.json`/schema DTOs are
unchanged.

**B2-W2 schema landing:** `SCHEMA_VERSION`, `SCHEMA_ARTIFACT_NAME`, `EMBEDDED_SCHEMA`, and the CLI
`VERSION_SUFFIX` are `1.8`; `src/schema/metrics.schema.v1.8.json` and
`docs/schema/metrics.schema.v1.8.json` are byte-identical; v1.7 is retained; `tests/schema_1_7.rs` is
renamed `tests/schema_1_8.rs` and the version-pin sweep is applied. `docs/schema/metrics-json-v1.md`
already carries the v1.7 → v1.8 delta entry.

### 4.8 CLI

- `src/cli/mod.rs:54-79` — `Args { path, output, force }`; **no language flag**. B2 adds none.
- Exit codes: usage 1, output 2, internal 3 (`src/cli/mod.rs:81-108`); unchanged.
- ``about`/help text now names the current contract: C/Rust/Python/JS/TS (B2-W2).

### 4.9 Tests / goldens / CI

| # | File:line | Current | Change |
|---|---|---|---|
| 1 | `tests/p6_hardening.rs:57-82` | four `GOLDEN_TREES` | B2: `[GoldenTree; 5]` with `report-js`; B3: `[GoldenTree; 6]` with `report-ts` |
| 2 | `tests/p6_hardening.rs:111-129` | 17 pinned artifacts incl. `metrics.schema.v1.7.json` | 17 entries; the schema entry becomes `metrics.schema.v1.8.json` in `GOLDEN_ARTIFACTS` (`:113`) **and** `provenance_free` (`:465`) |
| 3 | `tests/p6_hardening.rs:373-546` | `PERFPAL_REGEN_GOLDEN=1` regenerator | used for all six trees |
| 4 | `tests/p6_hardening.rs:165-176` | pinned detail-page set | extend per tree |
| 5 | `tests/schema_1_4.rs:88-119, 247-291` | pins the supported set, `SCHEMA_VERSION`, artifact name, doc copy, emitted version | update to the staged fallback (B2/B3) and v1.8 |
| 6 | `tests/schema_1_7.rs:36-63, 52-58, 121-163, 222` | v1.7 version/artifact/`$id`/doc-copy lock + required-list comparison against v1.6 | became the **v1.8** lock, renamed `tests/schema_1_8.rs`, keeping the required-list comparison against v1.7 (landed) |
| 7 | `tests/pipeline_e2e.rs:77,85` | compares bundled artifact to docs v1.7 + emitted version | v1.8 |
| 8 | `tests/resolver_health.rs:217`, `tests/i1a_entry_c.rs:406`, `tests/i1b_entry_rust.rs:498`, `tests/i1c_entry_python.rs:582` | emitted `schema_version == "1.7"` | v1.8 |
| 9 | `tests/atlas_fixtures/mod.rs:155`, `tests/key_views.rs:203` | synthetic docs pinned to `"1.7"` | v1.8 (or version-agnostic; verify the assertion) |
| 10 | `tests/exit_code_matrix.rs:75, 90-93` | empty-scan fallback lock | staged fallback (B2/B3) |
| 11 | `tests/benchmark/mod.rs:349, 393-405, 448-453` | active catalog = 67 + baseline locks | extend with shipped JS rules and corpus entries |
| 12 | `tests/mixed_language_e2e.rs:226-227` | panics on non `.c/.h/.rs` paths | extend only if a JS fixture joins that tree |
| 13 | `.github/workflows/ci.yml:17-48` | `check` job | no edit (Cargo compiles grammars) |
| 14 | `.github/workflows/ci.yml:85-105` | `license-audit` | no edit (MIT already allowed) |
| 15 | `.github/workflows/release.yml:81-240` | build + SBOM | no edit; new crates appear automatically |
| 16 | `tests/determinism_js.rs`, `tests/determinism_ts.rs` (new) | no JS/TS double-run suite today | B2/B3 add masked double-run suites covering `metrics.json` (masked), `report.md`, `report.html`, `findings-detail.*`, `profile-queue.*`, `charts/*`, and the bundled schema artifact |

Reference harnesses to copy: `tests/python_grammar_link.rs`, `tests/examples_python_coverage.rs`,
`tests/python_negative_fixtures.rs`, `tests/fixtures/precision/python/`, `tests/benchmark/corpus.rs`.

## 5. Language detection (locked)

Longest suffix first; unknown extensions stay dropped (existing behaviour).

| Suffix | Token | Note |
|---|---|---|
| `.d.ts`, `.d.mts`, `.d.cts` | — | **skipped**: declaration-only, zero runtime code |
| `.tsx` | `tsx` | `LANGUAGE_TSX` |
| `.ts`, `.mts`, `.cts` | `ts` | `LANGUAGE_TYPESCRIPT` |
| `.js`, `.jsx`, `.mjs`, `.cjs` | `js` | JS grammar embeds JSX |

## 6. Node-kind → IR mapping (confirmed against the pinned grammars in B2-W0)

| IR target | JS/TS node kinds |
|---|---|
| function | `function_declaration`, `function_expression`, `arrow_function`, `method_definition`, `generator_function*`, `class_declaration` (methods) |
| class → `StructDef` | `class_declaration`, `class` (class expression) |
| import → `IncludeEdge` | `import_statement` (`source`), `export_statement` with `source`, `call_expression` with `require("literal")` |
| global → `GlobalVar` | top-level `variable_declaration`, `lexical_declaration` |
| call | `call_expression`, `new_expression` (alloc event) |
| loop → `LoopNode` | `for_statement`, `for_in_statement`, `while_statement`, `do_statement` |
| allocation event | `new_expression`, `object`, `array` |
| async | `await_expression` inside `for_statement`/`for_in_statement`/`while_statement`; `function_declaration`, `method_definition`, or `arrow_function` with an `async` token |
| error path | `throw_statement`, `try_statement`, `catch_clause` |
| string build | `binary_expression` with `+`, `template_string`, `augmented_assignment_expression` with `+=` |
| TS-only (non-runtime) | `interface_declaration`, `interface_body`, `type_alias_declaration`, `ambient_declaration`, `type_annotation*`, `type_parameters`, `type_arguments`, `implements_clause`, `function_signature`, `method_signature`, `abstract_method_signature`, `index_signature`, `call_signature`, `construct_signature`, `property_signature`, `object_type`, `function_type`, `constructor_type`, `type_query`, `conditional_type`, `infer_type`, `mapped_type_clause`, `generic_type`, `union_type`, `intersection_type`, `array_type`, `tuple_type`, `optional_type`, `rest_type`, `parenthesized_type`, `literal_type`, `type_identifier`, `predefined_type`, `this_type`, `lookup_type`, `index_type_query`, `template_literal_type`, `nested_type_identifier`, `readonly_type` → **skipped before any event match** (B3) |
| TS expression wrappers | `as_expression`, `satisfies_expression`, `non_null_expression`, `type_assertion` → **transparent**: the value child is visited/shaped, the type child is skipped (B3) |
| TS parameters | `required_parameter` / `optional_parameter` with `pattern` (or `name`), `type`; a `this` parameter is erased; the `unknown` sentinel is kept when unannotated (B3) |
| TS branches | `ternary_expression` (the TS grammar names ternaries this; `conditional_expression` is JS-only) |
| TS class layout | `abstract_class_declaration` → `StructDef`; members read `public_field_definition.name` (JS uses `field_definition.property`); `declare class` under `ambient_declaration` → no `StructDef` |
| type-only imports | `import type` / `import typeof` / `export type` → **no `IncludeEdge`** (runtime-erased; structural child-1 `type`/`typeof` token) |
| JSX | `jsx_element`, `jsx_self_closing_element`, `jsx_attribute`, `jsx_expression` |
| parse health | `ERROR` / `MISSING` nodes → `ParseStatus::{Partial,Failed}` exactly like the three existing backends |

**B2-W0 confirmations (pinned grammars).** The link tests lock the subset they exercise (function/class,
all loop forms, async, JSX, imports, and the TS type kinds) in `tree-sitter-javascript =0.25.0` and
`tree-sitter-typescript =0.23.2`; the remaining kinds in the table come from the pinned
`node-types.json` and are re-checked before use. Three clarifications for the backend: `for...of` and
`for...in` are both `for_in_statement` (distinguish by the `operator` token `of`/`in`); a method
self-call `this.f()` is a `member_expression` whose object kind is `this`; `delete obj.prop` is a
`unary_expression` (the `delete obj["k"]` subscript form is a different argument kind and is not the
candidate construct). The pinned TS grammar cannot parse `f<typeof import("x")>()` (220 of
the canary's 229 gemini-cli parse failures); this is accepted degradation under `R-tb-014`,
not a reason to bump the pin without an ADR-0001 G5 review.

**B3 confirmations (pinned `tree-sitter-typescript =0.23.2`).** TypeScript/TSX reuse the JS visitor through
a `Dialect { language, token, flavor }`; the TS module owns only the two grammar bindings and the `ts`/`tsx`
tokens. The differences that are handled explicitly: parameters are `required_parameter`/`optional_parameter`
wrappers (`pattern`/`name` + `type`); ternaries are `ternary_expression`; class fields expose `name` (not
`property`); abstract classes are `abstract_class_declaration`; and **type land is never event-scanned** —
this is not cosmetic: `type_query`'s children include `call_expression`, and `interface_body`/`object_type`
contain `method_signature` → `formal_parameters` → `identifier`, so a naive walk manufactures events from
type positions. `as`/`satisfies`/`!`/`<T>expr` stay transparent (value child only). Observed on the pinned
TSX grammar: both `<T,>(x: T) => x` and `<T>(x: T) => x` parse cleanly at expression position; `<T,>` stays
the portable form and the limitation is recorded rather than assumed (`R-tb-010`).

**Additive IR-internal facts emitted in B2-W1 (no schema/DTO change).** The backend adds three
serde-defaulted IR fields that carry the JS evidence the analyzer consumes in B2-W2; none is mirrored in
`metrics.json`. The wire schema bump to v1.8 landed in B2-W2 (§13), and these facts still do not appear on
the wire:

- `FunctionUnit.allocations: Vec<AllocEvent>` — `kind` is `new` | `object` | `array`; each event carries
  the shared `EventMeta` (span, loop depth, innermost) and sorted/deduped `referenced_names` for the
  future loop-invariance gate.
- `FunctionUnit.unaries: Vec<UnaryEvent>` — v1's only emitter is `delete obj.prop` (`op == "delete"`);
  the operand is a `Shape`.
- `FunctionRecord.entry_declared: Option<bool>` — `Some(true)` on a synthetic `<module>` record proven to
  be an entry by a `package.json` `main`/`bin` target or an in-source `require.main === module` /
  `import.meta.url` guard; `Some(false)` on other JS module records; `None` on real functions and every
  non-JS backend.

The public wire surface for this evidence remains the existing `stats.functions[]` entry/reachability,
`alloc_site_count`, `alloc_family`, `error_path`, `idiom_hash`, and `function_cost_v2` fields.

## 7. Discovery policy

Discovery is language-blind at directory level (`src/scanner/mod.rs:314-354`), so the two scopes are
different. **Implemented in B2-W1:**

- **Directory skips are global** (`SKIP_DIRS`): `node_modules`, `bower_components`, `dist`, `build`, `out`,
  `coverage`. Dot-directories (`.next`, `.nuxt`, `.svelte-kit`, `.cache`, …) were already skipped by the
  existing rule, so they have no entries.
- **File-level skips are JS/TS-only** (`JS_SKIP_FILE_SUFFIXES` + `JS_MAX_FILE_BYTES`): `*.min.js`,
  `*.min.mjs`, `*.min.cjs`, `*.bundle.js`, and JS/TS files larger than **2 MiB** (the cap is checked with
  `fs::metadata` before reading, so oversized bundles are never parsed). The bounded `package.json` walk
  (`src/scanner/mod.rs:847-884`) filters **directories only** (dot-dirs, the global skip set, symlinks); it
  does not apply the file-name/size filter.
- All skips are path/metadata-based and deterministic; no content sniffing; no skip counters in v1.8.
- Existing symlink policy and canonical-path cycle guard are reused.
- **Consequence:** C/Rust/Python files beneath a global-skip directory are no longer scanned. `vendor` is
  deliberately **not** skipped (too generic; `path_class` already labels it). The existing corpora contain
  none of these directories, so their finding sets and goldens do not move. The B2-W1 integration suite
  locks the surviving set (a `vendor/` JS file is scanned; the six skipped directories are not; oversized
  JS is dropped while oversized C still scans).

## 8. Rule disposition table (frozen by the B2-W0 canary; implemented in B2-W2)

**Freeze record (B2-W0, 2026-10-24).** Evidence: [`docs/evidence/js-canary.md`](../evidence/js-canary.md),
the raw [`canary_results.json`](../evidence/js-canary/canary_results.json), and the counter
`tools/js_canary.rs`, run over three pinned repos — express `v5.1.0` (CJS), execa `v9.6.1`
(ESM/TS), gemini-cli `v0.65.0` (AI-assisted TS): 1,525 production files / 321.1 kLOC and 1,413 test
files, with 230 parse failures (220 are one pinned-TS-grammar limitation, `f<typeof import("x")>()`).
**Net: 11 rules ship, 1 is cut, 0 deferred.** The cut is an *identifiability* failure, not a
frequency failure — no candidate scored zero corpus-wide.

**Implementation status (B2-W2).** All 11 KEEP rules are implemented in
`src/analyzer/heuristics/js/mod.rs` (`js_alloc_001` … `js_type_001`), registered with
`RuleLanguage::Js`, and catalogued in `src/reporter/mod.rs`. `PERF-JS-LOOP-003` remains cut: it has no
function, no `finding.rs` constant, and no catalog entry.

Every rule message must cite **mechanism → fix → accepted FP class**.

| Candidate | Construct | Mechanism | Fix | Accepted FP class | B2-W0 verdict | B2-W2 |
|---|---|---|---|---|---|---|
| `PERF-JS-ALLOC-001` | object/array literal per iteration in a hot loop | young-gen allocation churn; escape analysis rarely elides | hoist the invariant construction | object created once for a different reason but mutated per iteration | **KEEP — reframed**: loop-invariance/escape gate mandatory, Info-first (1,021 prod vs 21,094 outside loops; the raw "literal in loop" shape is the corpus noise cannon) | implemented |
| `PERF-JS-ALLOC-003` | `concat`/spread/`slice` accumulation in a loop | copies n elements per iteration → O(n²) | `push` into one array, or a single `concat` | deliberate immutable snapshot per iteration | **KEEP (Info)** — 16 prod / 32 outside-loop controls | implemented as: `concat`/`slice` calls + `has_spread` array literals, both gated on a loop-reassigned accumulator root |
| `PERF-JS-STR-001` | `+=` string accumulation in a loop | cons-string flattening at first use (MEDIUM; V8 ropes mitigate) | collect parts + `join("")` | tiny bounded loop (< 8 iterations) | **KEEP (MEDIUM)** — 73 prod / 159 outside-loop controls; honest V8-rope caveat | implemented as: string/template-literal RHS gate on `+=`, plus the `x = x + <literal>` form **only when the binary is the whole RHS**; numeric `+=` stays silent |
| `PERF-JS-LOOP-001` | nested loop depth ≥ 2 | superlinear iteration with no early exit | index map / early exit / hoist | matrix work where n² is inherent | **KEEP (Info)** — 165 prod / 900 depth-1 controls | implemented |
| `PERF-JS-LOOP-002` | sync blocking I/O in a loop | one syscall/serialization per iteration, blocks the loop | batch, async API, or a single call outside the loop | startup-only loop | **KEEP (MEDIUM)** — 112 prod / 759 outside-loop controls | implemented |
| `PERF-JS-LOOP-003` | `for...in` over an array | enumerates enumerable string keys incl. the prototype chain | `for...of` / indexed `for` | intentionally enumerating an object's keys | **CUT (v1)** — 0 array-literal RHS; all 24 prod hits are identifier RHS, statically indistinguishable from legitimate object enumeration without types | cut (no code) |
| `PERF-JS-LOOP-007` | `indexOf`/`includes`/`find` inside a loop | linear scan per iteration → O(n²) | `Set`/`Map` lookup | small (< 16) collections | **KEEP — flagship** — 177 prod / 857 outside-loop controls | implemented |
| `PERF-JS-ASYNC-001` | `await` in a loop over independent work | serializes latency; one microtask round-trip per item | `Promise.all(items.map(...))` | deliberately sequential (rate limit, ordering) | **KEEP — flagship** — 395 prod (278 in `for...of`) / 4,363 await controls outside loops | implemented |
| `PERF-JS-ASYNC-002` | sync blocking call inside `async` fn | stalls the event loop for every concurrent task | async API (`fs/promises`, `exec`) | CLI single-shot path | **KEEP — flagship** — 270 prod but 882 test hits (test-skewed); the async-function gate is mandatory; 462 non-async controls | implemented |
| `PERF-JS-CALL-001` | direct recursion | stack growth + repeated frame cost | loop / accumulator | recursive tree walk with small depth | **KEEP (Info) — guard-gated** — 102/108 prod self-calls are guarded; only 6 unguarded, so the visible-base-case predicate is mandatory | implemented |
| `PERF-JS-ARCH-001` | import cycle | module-init ordering + coupling; bundlers cannot tree-shake cleanly | break the cycle via a third module | type-only import cycles (TS erases them) | **KEEP (Info)** — 6 cyclic modules (execa); the relative-only graph is blind to alias monorepos (gemini-cli: 9 resolved edges over 1,255 modules) | implemented |
| `PERF-JS-TYPE-001` | `delete obj.prop` | hidden-class → dictionary-mode transition | set to `undefined` or rebuild the object | intentional cache eviction (rare) | **KEEP (Info)** — 31 prod / 182 `.delete(...)` controls | implemented |

**Cut / deferred with reasons:** `PERF-JS-LOOP-003` is **cut for v1** (see the verdict row). The
pre-canary cuts remain: `arr.length` hoisting (V8 does not pay a re-read — folklore);
`new RegExp(literal)` in a loop (V8 keeps a regexp compilation cache — the Python `re.compile`
lesson); `JSON.parse(JSON.stringify())` clone (already covered by allocation rules); `map` vs `for`
micro-benchmarks; microtask-ordering; anything requiring types. **`PERF-TS-*`: the canary produced no
TS-only candidate, so v1 ships none** — the shared `PERF-JS-*` family covers TS/TSX.

**Rerun:** `docs/evidence/js-canary/run.sh` (clones the pinned tags, verifies their SHAs, runs the
counter twice and `cmp`s the output).

## 9. Import resolution and entry detection (v1)

- Resolve **relative specifiers only** (`./`, `../`): probe `.ts/.tsx/.js/.jsx/.mjs/.cjs/.mts/.cts`, then
  `<dir>/index.<ext>`. Bare specifiers, absolute `/…`, `node:` and URLs → **external** (unresolved).
  **Implemented in B2-W1** (`resolve_js_edges` / `resolve_js_specifier`); root-escaping `../` is treated
  as unresolved-internal.
- `package.json` is located by a bounded, deterministic walk to the scan root (reporting/package root
  only). The scanner reads `main` (string) and `bin` (string | object | array) and marks the matching
  module record with `entry_declared = Some(true)`. Malformed/unreadable manifests are per-file data:
  ignored, scan continues, never exit 3.
- Unresolved internal imports are **data-only** (no finding), mirroring the Rust N1 decision.
- Out of scope: `tsconfig` `paths`, `node_modules`, `exports` maps, re-export tracing, dynamic `require`,
  and `package.json` `"type"` analysis (CJS/ESM kind is **not** inferred).
- **Entry ladder (implemented in the aggregator, `detect_js_entries`):**
  1. declared module entry (`entry_declared == Some(true)` — a `package.json` target or an in-source
     `require.main === module` / `import.meta.url` guard) → `script` / **HIGH**;
  2. conventional module root (`index.*`, `main.*`, `server.*`, `app.*`) → `script` / **MEDIUM**;
  3. real non-underscore function in a file with no module entry → `lib-root` / **MEDIUM**;
  4. no candidate anywhere → `unknown-entry` / **LOW** on every JS function.
  Candidates are sorted/deduped; the `unknown-entry` stub keeps reachability total instead of empty.

## 10. Cost model

- Reuse `function_cost_v2` unchanged (present in both v1.7 and v1.8).
- Loop forms: C-style `for`, `for...of`, `for...in`, `while`, `do...while`; `await` does not alter shape.
- **Implemented in B2-W1:** literal trip counts only for `for (let i = 0; i < N; i++)` / `<=` with an
  integer literal `N` (clamped to 1_000_000); `<=` adds one (`i <= 3` ⇒ 4). `for...of`, non-zero starts,
  non-`i++` increments, variable bounds and multi-declarator initializers report `None` (honest unknown;
  `for...of` over an array literal is deferred).
- `alloc_site_count`: `new` expressions + object literals + array literals (the same events populate the
  additive `AllocEvent` stream).
- `trip_count_source` = literal/partial/unknown; omitted when no loops. Saturating u64 arithmetic.
- No new metric fields; no new schema field.

## 11. Determinism guardrails

- Discovery stays lexicographically sorted; `rel_posix` unchanged.
- New maps are `BTreeMap`; no `HashMap` iteration may reach output.
- `serde_json` stays default-features (no `preserve_order`).
- Double-run byte equality on `examples-js` and `examples-ts` is a required test (B2/B3).

## 12. Risks and accepted limitations

| id | Risk / limitation | Mitigation |
|---|---|---|
| `R-tb-001` | Grammar MSRV undeclared (both crates) | compile on 1.92 in B2-W0; fallback pin `tree-sitter-javascript 0.23.1` |
| `R-tb-002` | TS grammar is ABI 14, one generation behind JS 15 | exact pin + node-types diff on upgrade (ADR-0001 G5) |
| `R-tb-003` | TS/TSX node names drift across grammar releases | pin, vendor any queries in-repo, parser snapshot tests |
| `R-tb-004` | `node_modules`/minified noise if discovery is naive | **mitigated (B2-W1):** global directory skips + JS/TS-only name/size cap, locked by the integration discovery test |
| `R-tb-005` | Rule precision ratchet (1.0/1.0) is hard for JS idioms | canary before freeze; 11-rule frozen set; honest FP class per rule |
| `R-tb-006` | No type info → type-aware rules impossible | `syntactic` tier stated in every finding; limitations published |
| `R-tb-007` | Build time/binary size +2 C parsers | accepted; no feature gate |
| `R-tb-008` | Global directory skips change C/Rust/Python discovery for repos with those dirs | documented behavior change; existing corpora contain none; acceptance criterion 6 restated |
| `R-tb-009` | Reporter `HEURISTIC_CATALOG_V1` is A-owned but mechanically required (panics on uncatalogued rule ids, 67-count locks) | **resolved (B2-W2, `D-tb-017`):** founder-approved atomic catalog-only B edit; the 11 entries landed with the registry block and the count/equality/Appendix-A locks moved by exactly 11 |
| `R-tb-010` | TSX vs generic-arrow ambiguity (`<T,>`) degrades parse health | **narrowed (B3):** the pinned TSX grammar 0.23.2 resolves both `<T,>(x: T) => x` and `<T>(x: T) => x` cleanly at expression position (locked by `tests/typescript_backend.rs` + the grammar-link test); `<T,>` remains the portable form, other positions are not claimed |
| `R-tb-011` | CJS/ESM module kind is not inferred; entry detection mixes `require.main` and `import.meta.url` | document as a limitation; entry candidates are sorted/deduped/first-match; no `package.json` `"type"` claim |
| `R-tb-012` | `.d.ts` skip makes type-only modules invisible (imports become unresolved-internal data-only); an unlabeled minified file < 2 MiB is still parsed; extension matching is case-sensitive | documented limitations; no finding emitted for unresolved-internal; size cap bounds the worst case |
| `R-tb-013` | Malformed/unreadable `package.json` could escalate a per-file problem to exit 3 | **mitigated (B2-W1):** malformed/unreadable manifests are ignored, the conventional/in-source ladder still runs, and the scan exits 0 (locked by the integration test) |
| `R-tb-014` | Pinned TS grammar `0.23.2` cannot parse `f<typeof import("x")>()` (220 of 229 gemini-cli parse failures in the canary) | degrade honestly (partial parse); upgrade only via the ADR-0001 G5 node-types review; B3 snapshot tests |
| `R-tb-015` | **Schema staging (two surfaces):** B2-W1 emitted `files[].language == "js"` against a bundled v1.7 artifact whose enums listed only `c`/`rust`/`python` | **resolved (B2-W2):** v1.8 landed in `src/schema/` + byte-identical `docs/schema/` copy; both enums hold all six tokens; v1.7 retained; version-pin sweep applied. A W1 build remains non-shippable (historical) |
| `R-tb-016` | **Lexical rule gates:** `ALLOC-001` models loop-invariance, not a separate escape analysis; `CALL-001` treats any earlier comparison as a visible base case; `LOOP-001` cannot test for an early exit (no break/return events in IR); `LOOP-007` proxies the frozen "small collection" class with loop trip count | gates documented exactly as implemented; 1.0/1.0 is the synthetic precision corpus only — re-run the real-code canary before merge |

**Accepted limitations added by B2-W1:**

- **Class bodies are `StructDef` scope boundaries, not event owners.** Methods and arrow-function fields are
  their own function records; a non-arrow field-initializer expression and a `class_static_block` body belong
  to no record and are not event-scanned at all (so their calls/allocs never reach `stats.functions[]`).
- **Single-file targets follow the same JS/TS skip policy as discovery.** `*.min.js`/`*.min.mjs`/`*.min.cjs`/
  `*.bundle.js` and declaration-only `.d.ts`/`.d.mts`/`.d.cts` are skipped silently instead of being parsed
  as C; `.ts`/`.mts`/`.cts`/`.tsx` are scanned normally (B3) and a genuinely unknown extension keeps the
  documented C fallback.

**Accepted limitations added by B3:**

- **Type-only imports are erased.** `import type` / `import typeof` / `export type` produce no
  `IncludeEdge`, so `PERF-JS-ARCH-001` cannot report a type-only cycle (there is no runtime cycle).
  Inline type specifiers (`import { type X, y }`) keep the value edge.
- **Type land is never scored.** Interfaces, type aliases, ambient `declare` blocks, overload signatures,
  and `typeof import("…")` type positions emit no calls/allocs/loops and no function records; `declare class`
  emits no `StructDef`. A file that the pinned grammar marks partial because of a type position still emits
  no finding from that region.
- **TS parameter types are IR-internal.** `param_types` carries the raw annotation text (leading `:`
  stripped, `this` erased, `"unknown"` when absent); it is not mirrored in `metrics.json` and no rule
  consumes it.

**Accepted limitations (publish in report/docs):** no type checking; no path aliases/`node_modules`; `.d.ts`
and minified bundles skipped; no framework semantics; JSX parsed structurally only; computed
`require`/`import()` invisible; static analysis is approximate — profile to confirm.

## 13. Session B2-W2 implementation (rules + schema v1.8)

**State:** implemented, gated and reviewed; commit pending founder approval. Branch `feat/js-ts` rebased
on `main` @ `9a94905`. TS/TSX scanning landed in B3, so `SUPPORTED_LANGUAGES` is now the six-value set
`["c","js","python","rust","ts","tsx"]` (§16).

### 13.1 Rule → code (all confidence MEDIUM)

| Rule | Severity | Function | Gate actually implemented (heuristic, not proof) |
|---|---|---|---|
| `PERF-JS-ALLOC-001` | info | `js_alloc_001` | object/array literal at loop depth ≥ 1; silent when a referenced name is the nearest loop's induction variable or is assigned inside that loop; silent with no enclosing loop node |
| `PERF-JS-ALLOC-003` | info | `js_alloc_003` | `concat`/`slice` in a loop on a non-empty receiver that the same loop reassigns, **or** an array literal with `spread` (`has_spread`) whose referenced accumulator name the nearest enclosing loop reassigns; snapshot call/literal roots stay silent |
| `PERF-JS-STR-001` | warn | `js_str_001` | `+=` with an identifier left operand **and a string/template-literal right operand** in a loop, or `x = x + <string\|template literal>` **where the binary is the whole RHS**; numeric/identifier `+=` is silent; silent on a canonical trip count < 8; unknown trip fires |
| `PERF-JS-LOOP-001` | info | `js_loop_001` | one finding per function at the earliest loop with `depth >= 2`; `max_depth` counts only loops whose span is contained in that anchored loop's span |
| `PERF-JS-LOOP-002` | warn | `js_loop_002` | known synchronous Node API (23-name set) at loop depth ≥ 1 |
| `PERF-JS-LOOP-007` | warn | `js_loop_007` | `indexOf`/`includes`/`find`/`findIndex`/`some`/`every`/`filter` in a loop with a non-empty receiver root; silent on a canonical trip count < 16; unknown trip fires |
| `PERF-JS-ASYNC-001` | warn | `js_async_001` | `await` at loop depth ≥ 1, only inside a function with `is_async` |
| `PERF-JS-ASYNC-002` | warn | `js_async_002` | known synchronous Node API anywhere in an `is_async` function; **no loop requirement** |
| `PERF-JS-CALL-001` | info | `js_call_001` | bare direct self-call (no receiver root); fires only when no comparison in the function starts before the call span — a visible base case is enough to stay silent |
| `PERF-JS-ARCH-001` | info | `js_arch_001` | resolved relative-import cycles via `break_cycles`; one finding per broken edge at that pair's earliest retained directive span |
| `PERF-JS-TYPE-001` | info | `js_type_001` | `delete obj.prop` with a `MemberAccess` operand; the subscript form (`delete obj["k"]`) and a bare identifier (`delete x`) are excluded |

Wiring: constants in `src/analyzer/finding.rs`; the ascending `PERF-JS-*` registry block and
`RuleLanguage::{Js,Ts}` in `src/analyzer/mod.rs`; the `js|ts|tsx` partition to `js_units` (`Ts` is reserved
and empty in v1); 11 catalog entries and the 67 → 78 count/equality locks in `src/reporter/mod.rs`.

### 13.2 Test and corpus inventory

- `src/analyzer/heuristics/js/tests.rs` — 27 parser-free IR unit tests (positive + control per rule, plus
  the LOOP-003/`PERF-TS-*` absence lock).
- `tests/fixtures/ir/PERF-JS-*__*.json` — 22 fixtures (one positive + one control per rule), run by
  `tests/js_ir_fixtures.rs` with no parser bound.
- `examples-js/` — 15 scanned sources (14 `.js` + 1 `.cjs`) plus `package.json` (`main: index.js`, script /
  HIGH). `tests/examples_js_coverage.rs` pins exactly 11 findings (0 hot / 5 warn / 6 info), each rule once,
  and 18 silent controls.
- `tests/fixtures/precision/js/` — 25 files with `labels.json` entries; all 11 rules measure 1.0 / 1.0.
- `tests/determinism_js.rs` — masked double run over the full artifact set (metrics, report, detail/queue,
  charts, bundled schema); byte-equal modulo `scan.started_at` / `scan.duration_ms`.
- `tests/golden/report-js/` — 20 files; `GOLDEN_TREES` 4 → 5. All five trees carry
  `metrics.schema.v1.8.json`; no v1.7 artifact remains.
- `tests/benchmark/` — the `small` rung is 142 files / 1,655 LOC / 96 findings; 78 active rules (79 scoped
  including retired `PERF-RS-STR-001`); JS rules 1.0 / 1.0.
- Schema: `tests/schema_1_8.rs` (was `schema_1_7.rs`) plus the version-pin sweep in `schema_1_4`,
  `pipeline_e2e`, `resolver_health`, `i1a`/`i1b`/`i1c`, `atlas_fixtures`, `key_views`, `p6_hardening`.

### 13.3 Recorded verification

- `cargo test --locked` — 63 suites, **935 passed**, 0 failed.
- `cargo deny check licenses` — ok.
- `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` and
  `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` — green.
- Native check — green.
- `examples-js` double run — byte-identical modulo the two provenance fields.
- Gate transcript: `docs/evidence/js-w2/gate.txt` (recorded in the tree).

### 13.4 Accepted limitations added or still live

- **TS/TSX scanning landed in B3.** `.ts`/`.mts`/`.cts` → `ts`, `.tsx` → `tsx`, `.d.*` skipped; `.jsx` maps
  to `js` because the JS grammar embeds JSX. `SUPPORTED_LANGUAGES` is the six-value set.
- **Non-arrow class-field initializers and `class_static_block` bodies are not event-scanned** (`D-tb-016`);
  they belong to no function record, so their calls/allocs never reach `stats.functions[]`.
- **Reporter dependency clause** currently renders a JS corpus as bare "js module edges"; label polish is B4.
- **Findings are optimization candidates, not proven bottlenecks.** The tier is syntactic; no type
  information and no semantic resolver claim.
- **`ALLOC-003` member/`this` accumulators and spread-root attribution:** `assigned_within` matches plain
  identifier targets only, so `this.items = [...this.items, x]` is not detected; the spread arm reports
  the alphabetically first loop-reassigned referenced name as `receiver_root` (the spread operand itself
  is not identified in v1). Both are documented accepted limitations.
- **Count synchronization is test-locked:** the reporter count, the benchmark active set, and the golden
  artifact sets move together when a rule lands; the 78 count is asserted in both suites.

## 14. Session B1 DoD (historical)

> **Historical (B1).** This checkpoint fails by construction after B2-W0's pins; it is kept as the B1
> evidence record. The current gate is the plan `js-ts` Definition of Done plus the B2-W0 verification
> table in `state/tracks/js-ts/handoff.md`.

```bash
test -s docs/specs/js-ts.md && test -s docs/plans/js-ts.md \
 && test -s docs/adr/ADR-0011-js-ts-backend.md \
 && test -s docs/architecture/js-ts.md \
 && test -s state/tracks/js-ts/session-state.json \
 && test -s state/tracks/js-ts/handoff.md \
 && test -s docs/tracks/js-ts.md && echo "files: OK"
git diff --quiet main -- src Cargo.toml Cargo.lock deny.toml tests && echo "no-code: OK"
test -z "$(git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock deny.toml)" && echo "no-untracked-code: OK"
jq -e . state/tracks/js-ts/session-state.json >/dev/null && echo "state-json: OK"
~/.cargo/bin/cargo fmt --all -- --check && echo "fmt: OK"
```

Expected: every line prints its `OK` marker and the script exits `0`. (`cargo` is not on the non-interactive
PATH in this environment; the full `~/.cargo/bin/cargo` path is required.)

## 15. Open questions for B2 (must be resolved by the canary, not by assumption)

1. **Resolved (B2-W0):** 11 candidates survive; `PERF-JS-LOOP-003` is cut. Severity defaults:
   Info for `ALLOC-003`/`LOOP-001`/`CALL-001`/`ARCH-001`/`TYPE-001`; MEDIUM for
   `STR-001`/`LOOP-002`; flagship MEDIUM for `LOOP-007`/`ASYNC-001`/`ASYNC-002`;
   `ALLOC-001` Info-first under the mandatory loop-invariance gate. See §8 and
   `docs/evidence/js-canary.md` §9.
2. Does the TS grammar need `tsx` query differences for JSX-heavy files (element kinds vs type args)?
   **Partly resolved (B2-W0):** the link test locks the JSX node kinds and the `<T,>` generic-arrow
   disambiguation parses cleanly; real-corpus behavior is confirmed in B3.
3. Is `require()` static-string extraction worth shipping in v1, or does it add more FP than graph value?
4. Does a JS fixture join `tests/fixtures/e2e-mixed/` (which currently panics on non-C/Rust paths)?
5. Who lands `HEURISTIC_CATALOG_V1` — Track A before B2, or a founder-approved catalog-only edit by B?
   **Resolved (B2-W2, `D-tb-017`):** the founder approved the atomic catalog-only B edit; the 11 entries
   landed with the registry block, so the frozen set is **11** entries, not 12 (`B-tb-003` closed).

## 16. Session B3 implementation (TypeScript/TSX backend)

**State:** implemented, full DoD green, parallel panel recorded; commit pending founder approval. Branch
`feat/js-ts`, base `main @ 9a94905`. No schema bump (v1.8 already admits `ts`/`tsx`), no new rules, no
catalog change, no dependency change.

### 16.1 Reuse shape

- `src/scanner/backends/tree_sitter_javascript/` is parameterized by
  `Dialect { language: fn() -> Language, token, flavor }`; the public `extract` binds the JS dialect and is
  byte-compatible with B2 (`tests/javascript_backend.rs` and the 22 parser-free IR fixtures stay green).
- `src/scanner/backends/tree_sitter_typescript/` owns only the grammar bindings
  (`LANGUAGE_TYPESCRIPT` for `ts`, `LANGUAGE_TSX` for `tsx`), the tokens, and the two entry points
  (`extract`/`extract_tsx`). TypeScript-specific extraction details live beside the JS visitor they modify
  (parameters, wrappers, type-land suppression, ternary, class-field naming, type-only edges).
- Scanner wiring: `LanguageKind::{Ts,Tsx}`, longest-suffix detection (`.tsx` → `tsx`,
  `.ts/.mts/.cts` → `ts`, `.d.*` → skipped), `extract_unit`/`failed_unit`, a single `is_js_family`
  predicate for the shared 2 MiB cap, the relative-specifier resolver, `package.json` entry declarations,
  and `SUPPORTED_LANGUAGES` = six. Instrument alloc-family/error-path arms are `"js" | "ts" | "tsx"`.
- Aggregator `detect_js_entries` and `entry_scoped` cover the whole family; `PERF-JS-ARCH-001`,
  `PERF-JS-LOOP-*`, `PERF-JS-ASYNC-*` etc. are unchanged and receive `js|ts|tsx` units in the same slice.

### 16.2 Test and corpus inventory (B3)

| Artifact | Content |
|---|---|
| `src/scanner/backends/tree_sitter_typescript/tests.rs` | 17 unit tests: node-kind locks, TS params (`this`/optional/rest/destructured), typed canonical `for` trip count, type-land silence, `import type`/`export type` erasure, wrapper transparency, abstract-class members, `declare class`, enums, JSX embedded expressions, ternary complexity, clean parse/tokens, wrapper-aware callee/receiver decomposition, type-position wrapper silence, comment extras, commented `import type` erasure |
| `tests/typescript_backend.rs` | 17 integration tests: detection/tokens, mixed tree, discovery skips (TS + 2 MiB), parse health, package.json `main` HIGH (`.ts` and `.mts`), conventional `index.tsx` MEDIUM, relative import probes (`.ts`→`.tsx`→`/index.ts`), type-only edge erasure, single-file policy, params in metrics, `<T,>`/`<T>` TSX parse health, non-vacuous cross-language scoring (one known LOOP-007 hit), IR-internal field leak guard, double-run determinism, plus the committed negative fixtures (`type_only.ts`, `type_edges.ts`, `typeof_import.ts`) |
| `examples-ts/` | 17 scanned sources (15 `.ts` + 2 `.tsx`) + `package.json`; all 11 shared rules fire exactly once (0 hot / 5 warn / 6 info); type-only/JSX/generic modules silent; 18 documented silent controls |
| `tests/examples_ts_coverage.rs` | exact 11-id set, zero cross-language ids, `["ts","tsx"]`, 15/2 split, 11 findings, per-rule 1, `report.md` §4 catalog, silent-control triple check |
| `tests/determinism_ts.rs` | masked double-run over `examples-ts`; artifact set + bytes identical |
| `tests/golden/report-ts/` | 20-artifact golden tree; `GOLDEN_TREES` 5 → 6; pinned component page `charts/component-01-root.svg` |
| `tests/p6_hardening.rs` | six-tree comparator; existing five trees unchanged by the B3 regeneration |

### 16.3 Recorded verification

Full transcript: `docs/evidence/js-b3/gate.txt` §9 (final tree): 66 suites / **973 passed** (lib 456),
semantic-feature **976 passed**, DoD double-run over `examples-js` + `examples-ts`, native check, golden
identity (five pre-existing trees unchanged), and all guards. Panel (final): **R1, R2, R3, Q1 and Q2 all
APPROVED**, zero unresolved blocking findings; verdict and fix history in the track handoff.
