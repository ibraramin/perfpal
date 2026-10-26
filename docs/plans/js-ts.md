---
plan name: js-ts
plan description: JavaScript TypeScript backends
plan status: active
---

## Idea
Add JavaScript (B2), then TypeScript/TSX (B3), as first-class Perfpal backends in the existing Rust core, following the Python tier's shape: a tree-sitter scanner backend, parser-agnostic IR emission, a `PERF-JS-*` rule catalog, a MINOR schema bump, fixtures/goldens, and docs. Execution plan for Track B (branch `feat/js-ts`); B1 (spec + plan + ADR + Phase 0 inventory) is this planning session and ships no production code. Founder-approved B1 decisions: tree-sitter-javascript =0.25.0 (ABI 15, MIT, JSX embedded) + tree-sitter-typescript =0.23.2 (ABI 14, MIT, two languages LANGUAGE_TYPESCRIPT/LANGUAGE_TSX) on the existing tree-sitter =0.26.13 runtime; schema tokens `js`/`ts`/`tsx` following the grammar boundary (.jsx stays `js`, .d.ts skipped); ONE minor bump v1.7 -> v1.8 landing in B2 that declares all three tokens up front (B3 then emits ts/tsx with no second bump); a 12-rule JS candidate catalog frozen by a W0 canary before implementation (M1 lesson: 5 rules once caused 89% of self-noise); JS/TS-only file-name/size skips (*.min.js/*.min.mjs/*.min.cjs/*.bundle.js; 2 MiB cap) on top of **global** directory skips (node_modules, bower_components, dist, build, out, coverage) so C/Rust/Python discovery changes only beneath those directory names (the existing corpora contain none); `syntactic` tier only, no resolver claim. JS and TS share one `PERF-JS-*` analyzer family because the runtime mechanism is identical (V8/Node); `PERF-TS-*` is reserved for TS-only constructs and v1 ships none unless the canary proves one. Rationale for rejecting oxc/swc: oxc latest requires Rust 1.97 (repo floor 1.92) and both pull dozens of crates with fast-breaking version churn, violating ADR-0001 G5 deliberate-upgrade discipline. Ownership: Track B owns src/scanner, src/analyzer rules, src/schema language values, Cargo.toml/Cargo.lock, deny.toml, JS/TS fixtures + goldens, docs/specs/js-ts.md, docs/plans/js-ts.md, ADR-0011; A owns views.rs/mod.rs wiring until A step 12; B never touches Atlas surfaces or main state files. Merge only after A step 12 with founder approval; beta is Track A and is not delayed by B.

## Implementation
- B2-W0 canary + pins: add `tree-sitter-javascript = "=0.25.0"` and `tree-sitter-typescript = "=0.23.2"` to Cargo.toml (tracked Cargo.lock), confirm `cargo deny check licenses` green, add `tests/javascript_grammar_link.rs` + `tests/typescript_grammar_link.rs` (ABI-in-window assert, representative parse with zero ERROR/missing nodes, malformed-source non-vacuity), and run the canary over pinned real JS repos to freeze the 12-rule catalog with a per-rule mechanism/fix/accepted-FP-class disposition in `docs/architecture/js-ts.md`. **B2-W0 result (2026-10-24):** pins compile on 1.92, deny green, six link tests green, canary over express/execa/gemini-cli frozen **11 keep / 1 cut** (`LOOP-003`); evidence `docs/evidence/js-canary.md`, dispositions in `docs/architecture/js-ts.md` §8.
- B2-W1 JS backend: create `src/scanner/backends/tree_sitter_javascript/` implementing the shared `Extraction` contract (FileRecord, ParseHealth parity, IncludeEdge, GlobalVar, StructDef, MacroEvent, FunctionRecord, LoopNode, allocation/call events); wire `LanguageKind::Js` + `language_of` (longest-suffix first, `.d.ts` skip) + `extract_unit` + the language token arm (`src/scanner/mod.rs:307-311`) + `failed_unit` + `SUPPORTED_LANGUAGES` (B2: `["c","js","python","rust"]`) in `src/scanner/mod.rs`; add the **global directory skip set** + **JS/TS-only** file-name/size guard; add the relative-specifier import resolver (`./`/`../` + extension probes + `/index.*`, bare specifiers external, unresolved-internal data-only) and JS entry detection; add `instrument.rs` alloc-family / error-path arms (comment stripping already covers `//` and `/* */`; only `#` is Python-gated — verify idiom hashes are unchanged).
- B2-W2 JS rules + schema v1.8: add `PERF-JS-*` constants (`src/analyzer/finding.rs`), `RuleLanguage::Js`/`Ts`, the ascending registry block and partition/dispatch arms (`src/analyzer/mod.rs`), and `src/analyzer/heuristics/js/**`; **land `HEURISTIC_CATALOG_V1` atomically with the registry** (A supplies the entries in the same change, or the founder approves a catalog-only B edit inside B2-W2; blocker `B-tb-003`; 67-count/catalog==registry/Appendix-A panic tests move by 11); re-emit `metrics.schema.v1.8.json` in `src/schema/` + byte-identical `docs/schema/` copy with `$id`, `title`, `description`, `schema_version.const`, both language enums moved to `["c","js","python","rust","ts","tsx"]`, and **both enum descriptions** updated (the `scan.languages` fallback clause stays generic); move `SCHEMA_VERSION`/`SCHEMA_ARTIFACT_NAME`/`EMBEDDED_SCHEMA`/CLI `VERSION_SUFFIX`; run the **version-pin sweep** (`tests/schema_1_7.rs` becomes the v1.8 lock incl. the required-list comparison, `tests/schema_1_4.rs:88-119,247-291`, `tests/pipeline_e2e.rs:77,85`, `tests/resolver_health.rs:217`, `tests/exit_code_matrix.rs:75,90-93`, `tests/i1a/i1b/i1c_entry_*.rs`, `tests/atlas_fixtures/mod.rs:155`, `tests/key_views.rs:203`) and replace the schema filename in `GOLDEN_ARTIFACTS` + `provenance_free`; regenerate all four existing golden trees; add `tests/fixtures/ir/PERF-JS-*__*.json` + `examples-js/` + `tests/golden/report-js/` (`GOLDEN_TREES` → 5) + `tests/examples_js_coverage.rs` + `tests/determinism_js.rs` (masked double-run) + precision fixtures/labels + benchmark corpus/baseline (active-catalog count `67 → 78`).
- B3 TS/TSX backend: create `src/scanner/backends/tree_sitter_typescript/` using `LANGUAGE_TYPESCRIPT` for `.ts`/`.mts`/`.cts` and `LANGUAGE_TSX` for `.tsx`, reusing the JS extraction visitor where the node shapes coincide and adding TS-specific handling (type annotations/erasure, interfaces/type aliases as non-runtime, JSX elements); emit `ts`/`tsx` tokens; extend `SUPPORTED_LANGUAGES` to six values; add `examples-ts/` (TS + TSX), `tests/golden/report-ts/` (`GOLDEN_TREES` 5 → 6), TS type-only negative fixtures (must not fire) incl. a `<T,>` generic-arrow parse-health case, and `tests/examples_ts_coverage.rs` + `tests/determinism_ts.rs` (masked double-run); no schema bump and no duplicated rule implementations.
- B4 docs + contract: finalize `docs/architecture/js-ts.md` (node-kind -> IR table, frozen disposition table, accepted limitations), update `docs/architecture/modules.md` sections 4.2/5.2, add the v1.7 -> v1.8 entry to `docs/schema/metrics-json-v1.md`, update `docs/report/output-contract.md` language labels, README/roadmap language lists, and record the CHANGELOG note in the track handoff for A to apply at merge; document semver position (later release, additive minor).

  **B4 result (2026-10-26):** docs/contract pass landed (`modules.md` §1/§3.2/§4.2/§5.2/§5.3/§7,
  output-contract language vocabulary + discovery skips, README/roadmap/`Cargo.toml`, semver note);
  new `docs/heuristics/js-catalog.md` (11 rules, read-code-first); reporter JS-family label polish
  (`import/require`) + unit lock; real-parse STR-001 negative (`s = f(s + "x")` silent, `s = s + "x"`
  fires); spread-root attribution pin; six golden trees checked (only `report-js`/`report-ts` labels
  changed); real-code canary re-run (`docs/evidence/js-b4/real-code-canary.md`) — **it found a
  merge-gate blocker in `PERF-JS-CALL-001` (150/180 strict member FPs + 2 wrapped = 152 member-class
  from member/method-name collisions)**, documented as `R-tb-017` and not fixed in B4 (docs/label
  scope). Plan DoD green; parallel panel **APPROVED (R1/R2/R3 + Q1/Q2)**; B4 commit `6d6bcdf`;
  merge remains `A-tb-006` + founder approval.

  **B5 result (2026-10-26):** post-B4 correctness pass. `PERF-JS-CALL-001` production hits **180 → 2**
  (exact bare `callee_path` + `!conditional_guarded` + additive `FunctionRecord.name_is_lexical_binding`);
  `PERF-JS-TYPE-001` production **14 → 2** (additive `UnaryEvent.operand_root_fresh`); `PERF-JS-ALLOC-003`
  spread arm requires an assignment RHS (24 → 22 total); `findings[].symbol` resolves the narrowest record
  (JS/TS/Python goldens re-baselined); `path_class` now catches `.spec.`/`test-*` (canary production
  1,336 → 1,178); `.min.ts`/`.min.mts`/`.min.cts` skip gap closed; p6 compares both Atlas pages
  (`GOLDEN_ARTIFACTS` 17 → 19). Canary re-run + exact pre/post diff: `docs/evidence/js-b5/real-code-canary.md`;
  DoD transcript: `docs/evidence/js-b5/gate.txt`. No schema/catalog-count/dependency/rule-id change;
  78 rules. Committed `1b23198` (+ docs record); merge remains `A-tb-006`.
- Gate + merge: rebase on `main` after A step 12, run the Definition of Done below (every command exits 0), dispatch the parallel review panel (>=2 reviewer + >=2 requirements-reviewer, disjoint lanes) and require >=1 APPROVE per family with zero unresolved blocking findings; merge to `main` only with explicit founder approval.

## Required Specs
<!-- SPECS_START -->
- divergence-study
- evidence-beta-spec
- maintainer-map
- perfpal-core-spec
- js-ts
<!-- SPECS_END -->

## Definition of Done

Every command must exit `0`. Run from the repo root; `cargo` is not on the non-interactive PATH in this
environment, so use `~/.cargo/bin/cargo` (or `${CARGO_HOME:-$HOME/.cargo}/bin/cargo`).

> **B1 checkpoint only:** `git diff --quiet main -- src Cargo.toml Cargo.lock deny.toml tests` was the B1
> proof of "no production code". It is deliberately **not** part of the implementation gate — after B2 it is
> non-empty by design.

```bash
set -euo pipefail

# 1. Docs/state completeness
test -s docs/specs/js-ts.md && test -s docs/plans/js-ts.md \
 && test -s docs/adr/ADR-0011-js-ts-backend.md \
 && test -s docs/architecture/js-ts.md \
 && test -s state/tracks/js-ts/session-state.json \
 && test -s state/tracks/js-ts/handoff.md && echo "files: OK"

# 2. Format, lint, tests, licences
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo clippy --locked --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked
~/.cargo/bin/cargo deny check licenses

# 3. Semantic feature gate (default OFF; must stay green)
~/.cargo/bin/cargo clippy --locked --all-targets --features semantic-clang -- -D warnings
PERFPAL_SEMANTIC_TESTS=1 ~/.cargo/bin/cargo test --locked --features semantic-clang

# 4. Byte-determinism: identical modulo the contract-permitted provenance
#    (scan.started_at / scan.duration_ms; output-contract.md:15,198).
#    Text artifacts are covered by the repo's double-run suites (cargo test --locked), which mask provenance.
for corpus in examples-js examples-ts; do
  out=$(mktemp -d)
  ~/.cargo/bin/cargo run --locked --quiet -- "$corpus" -o "$out/a" --force
  ~/.cargo/bin/cargo run --locked --quiet -- "$corpus" -o "$out/b" --force
  diff -r "$out/a/charts" "$out/b/charts"
  cmp "$out/a"/*.schema.*.json "$out/b"/*.schema.*.json
  jq -S 'del(.scan.started_at, .scan.duration_ms)' "$out/a/metrics.json" > "$out/a.json"
  jq -S 'del(.scan.started_at, .scan.duration_ms)' "$out/b/metrics.json" > "$out/b.json"
  diff "$out/a.json" "$out/b.json" && echo "deterministic (modulo provenance): $corpus"
  rm -rf "$out"
done

# 5. Native check with a separate target dir (not a cross-compile)
CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target ~/.cargo/bin/cargo check --locked --all-targets
```

Expected: every command exits `0`; the `diff`/`cmp` steps print nothing before the `deterministic:` line.
