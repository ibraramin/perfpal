# Track B — JS/TS backend — session handoff

> **Session:** B3 — TypeScript/TSX backend + wiring + corpora/goldens. Plan `js-ts`; branch `feat/js-ts`;
> base `main @ 9a94905`; prior commits W0 `e7f5332`, W1 `90d5677`, W2 `edfc98a` (+ `b2f6eae`).
> **Uncommitted; full DoD green; parallel panel recorded.** No push. Commit/merge pending founder approval.

## North-star / scope discipline

Track B remains a **later release, non-blocking**; the birdseye map (Track A) is the core feature and was
not touched. This wave changed `src/scanner` (new TS backend + dialect parameterization + detection/wiring),
`src/aggregator` (JS-family entry detection), `src/scanner/instrument.rs` (family arms), tests/corpora/
goldens, and track docs/state. **No schema bump** (v1.8 already admits `ts`/`tsx`), **no rule/catalog change**
(78 entries, zero `PERF-TS-*`), **no reporter/Atlas edit**, **no dependency change** (`Cargo.toml`/
`Cargo.lock` untouched), no main state.

## Deliverables (this session)

### 1. TS/TSX backend without a second visitor

- `src/scanner/backends/tree_sitter_javascript/` is parameterized by
  `Dialect { language: fn() -> Language, token: &'static str, flavor: Flavor }`; the public `extract`
  binds the JS dialect and the B2 surface is unchanged (JS suite + 22 IR fixtures stay green).
  `pub(crate) extract_with(&Dialect, path, source)` is the reuse seam.
- `src/scanner/backends/tree_sitter_typescript/` owns `LANGUAGE_TYPESCRIPT` (`ts`) and `LANGUAGE_TSX`
  (`tsx`), the two entry points and 17 unit tests. Node-kind handling stays beside the JS visitor it
  modifies, so there is exactly one rule-facing extraction path.
- TS-specific behavior: `required_parameter`/`optional_parameter` unwrapping (`pattern` fallback `name`,
  `this` erased, `type` text with the leading `:` stripped, `"unknown"` when absent);
  `ternary_expression` branch + guard; `abstract_class_declaration`; `public_field_definition.name`
  (JS uses `field_definition.property`); `as`/`satisfies`/`!`/`<T>expr` transparent for shapes/events;
  **type land skipped before any event match** (`TYPE_ONLY_KINDS`, 42 kinds — `type_query` contains
  `call_expression`; `interface_body`/`object_type` contain `method_signature` → identifiers);
  `import type`/`import typeof`/`export type` produce **no IncludeEdge** (child-1 `type`/`typeof` token);
  `declare class` produces no `StructDef`; overloads/ambient declarations produce no records.
- `collect_identifier_names` skips type land, so `typeof someVar` in a type position cannot enter an
  allocation's `referenced_names` (loop-invariance gates).

### 2. Scanner / aggregator / instrument wiring

- `LanguageKind::{Ts,Tsx}`; `language_of`: `.tsx` → `tsx`, `.ts/.mts/.cts` → `ts`, `.d.ts/.d.mts/.d.cts`
  → skipped (checked first, case-sensitive).
- Single-file policy: declaration-only files skipped silently (never C); `.ts/.mts/.cts/.tsx` scanned;
  unknown extensions keep the C fallback.
- One `is_js_family` predicate drives the shared 2 MiB cap, `resolve_js_edges`, and
  `mark_js_entry_declarations`; `SUPPORTED_LANGUAGES = ["c","js","python","rust","ts","tsx"]`.
- `extract_unit`/`failed_unit` emit `ts`/`tsx`; `instrument::populate` receives the real token and the
  `classify_alloc_call`/`detect_error_path` arms are `"js" | "ts" | "tsx"` (idiom hash/normalization
  identical to JS — locked by a unit test).
- Aggregator `entry_scoped` + `detect_js_entries` cover the family (declared HIGH / conventional MEDIUM /
  lib-root MEDIUM / unknown-entry LOW), with a mixed ts+tsx unit test.

### 3. Tests

- `src/scanner/backends/tree_sitter_typescript/tests.rs` — 17 unit tests.
- `tests/typescript_backend.rs` — 17 integration tests (detection/tokens, mixed tree, discovery skips +
  2 MiB, parse health, `package.json main` HIGH, conventional `index.tsx` MEDIUM, relative imports
  `.ts`→`.tsx`→`/index.ts`, type-only edge erasure, single-file policy, param types in metrics,
  `<T,>`/`<T>` TSX status, no cross-language scoring, IR-internal leak guard, double-run determinism,
  committed negatives `type_only.ts`/`type_edges.ts`/`typeof_import.ts`).
- Updated: `tests/javascript_backend.rs` (TS now discovered; declaration-only skipped),
  `tests/exit_code_matrix.rs`, `tests/schema_1_4.rs` (six-value zero-file fallback),
  aggregator unit tests (fallback + mixed family entry), instrument unit test.

### 4. Corpora, goldens, gates

- `examples-ts/` — **17 scanned sources (15 `.ts` + 2 `.tsx`) + `package.json`** (`main: index.ts`).
  All 11 shared rules fire **exactly once** (0 hot / 5 warn / 6 info); 18 silent controls;
  `14_types_only.ts`, `15_component.tsx`, `16_generic_arrow.tsx` are silent; every file parses `ok`.
- `tests/examples_ts_coverage.rs` — exact 11-id set, zero cross-language ids, `["ts","tsx"]`, 15/2 split,
  pinned total/severity/per-rule counts, `report.md` §4 catalog, direct silent-control triple check.
- `tests/determinism_ts.rs` — masked double-run over `examples-ts`; artifact set + bytes identical.
- `tests/golden/report-ts/` — 20 artifacts; `GOLDEN_TREES` 5 → 6;
  `EXPECTED_COMPONENT_PAGES["ts"] = ["charts/component-01-root.svg"]`. Regeneration left the five existing
  golden trees byte-identical.
- `docs/evidence/js-b3/gate.txt` — full DoD transcript.

### 5. Docs/state

`docs/specs/js-ts.md` (B3 close-out, §4/§5/§9/§10/§12), `docs/architecture/js-ts.md` (§4.1, §5, §6,
§12, §13 state, new §16), `docs/tracks/js-ts.md` (B3 session), this handoff, `state/tracks/js-ts/
session-state.json` (D-tb-018/019; `R-tb-010` narrowed; `A-tb-004` done).

## Verification (claim → evidence)

Transcript: `docs/evidence/js-b3/gate.txt`. `cargo` is not on the non-interactive PATH; `~/.cargo/bin/cargo`.

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Docs/state complete | file test block from plan DoD | `files: OK` | 0 |
| Formatting clean | `cargo fmt --all -- --check` | no output | 0 |
| Lint clean | `cargo clippy --locked --all-targets -- -D warnings` | `Finished`, no warnings | 0 |
| Full suite green | `cargo test --locked` (final tree, §9b) | **66 suites, 973 passed, 0 failed** (lib 456) | 0 |
| Licences clean | `cargo deny check licenses` | `licenses ok` | 0 |
| Semantic-feature lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | `Finished`, no warnings | 0 |
| Semantic-feature tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 66 suites, **976 passed**, 0 failed | 0 |
| DoD double-run `examples-js` | two `cargo run -- examples-js -o … --force` | charts/schema byte-identical; masked metrics identical | 0 |
| DoD double-run `examples-ts` | two `cargo run -- examples-ts -o … --force` | 17 files / 17 ok; charts/schema byte-identical; masked metrics identical | 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | `Finished` | 0 |
| TS coverage gate | `cargo test --locked --test examples_ts_coverage` | 1 passed | 0 |
| TS determinism | `cargo test --locked --test determinism_ts` | 1 passed | 0 |
| TS backend integration | `cargo test --locked --test typescript_backend` | 17 passed | 0 |
| Golden trees | `cargo test --locked --test p6_hardening` | 4 passed (6 trees) | 0 |
| Guard: determinism config | `! grep -q preserve_order Cargo.lock` | pass | 0 |
| Guard: no dependency drift | `git diff --name-only -- Cargo.toml Cargo.lock` | empty | 0 |
| Guard: whitespace | `git diff --check` | clean | 0 |
| Guard: hot state budget | `jq -e . … && wc -c` | valid JSON, 8,118 B (≤ 8,192) | 0 |
| Criterion 6: existing goldens unchanged | `git status --short -- tests/golden`; `git diff --stat -- tests/golden` | only `?? tests/golden/report-ts/`; no tracked diff | 0 |
| Wrapper decomposition | `cargo test --locked --lib tree_sitter_typescript` | 17 passed (incl. wrapped callee/receiver, type-position silence, comment extras) | 0 |

**Gate tooling note:** `scripts/gates/task-gate.sh` is not present in this repo; the plan's commands were
run directly (L4 claim→evidence is satisfied by the transcript).

## Panel

**Round 1 (2026-10-26):** R2 APPROVED (contract/schema/CLI/determinism/deps); **R1 CHANGES_REQUESTED**
(two blocking correctness findings); **Q1 CHANGES_REQUESTED** (stale architecture claim + premature
"panel recorded" wording); **Q2 CHANGES_REQUESTED** (same panel placeholder + criterion-6 evidence missing
from the transcript). All blocking findings fixed:

1. **R1-1 — wrapper callees/receivers.** `(fn as any)()`, `<any>fn()`, `arr!.includes(x)` now decompose
   through `infer::strip_transparent_wrappers` in `callee_parts`/`receiver_root_of`/`final_segment_of`
   (+ `call_position` parent climb); `Direct` kind and `receiver_root` are preserved. Locked by
   `wrapped_callees_and_receivers_are_decomposed`.
2. **R1-2 — `as`/`satisfies` type child.** `FuncScan::visit` now visits only the value child of
   `as`/`satisfies`/`!`/`<T>expr`/parentheses, so `x as import("./m").T` cannot emit a spurious
   `import` CallEvent; `collect_identifier_names` does the same. Locked by
   `wrapped_type_positions_emit_no_runtime_events`.
3. **R1 non-blocking adopted:** comment-aware `is_type_only_import` (extras skipped) with
   `commented_type_import_marker_is_still_erased`; `js_entry_candidates` now probes `.mts`/`.cts`
   (`package_json_main_entry_mts_…`); the vacuous TS-scoring test now pins one known `LOOP-007` finding.
4. **Q1/Q2 — docs/evidence.** Architecture §13.4/§16 counts and the stale "B3 next"/"four-value fallback"
   text fixed; `docs/tracks/README.md` state updated; the gate transcript gained the full-suite aggregate,
   targeted suites, golden-identity output, and an echoed `CARGO_TARGET_DIR` native check.

**Round 2 (2026-10-26):** R1 **APPROVED** after the comment-extra fix
(`value_child` now skips extras; `comments_inside_wrappers_do_not_hide_the_value`); Q1 **APPROVED**
after the count sweep (17/17 everywhere). **Q2 and R3 both CHANGES_REQUESTED on the same evidence
defect:** `docs/evidence/js-b3/gate.txt` §7 still recorded the pre-comment-test run (16 TS unit tests,
972 total), so the final tree was not evidenced. That was fixed by the §9 re-run on the final tree
(66 suites / **973 passed**, lib **456**, semantic **976**, targeted suites listed, golden identity,
echoed native target) and by regenerating the TS golden after a corpus-comment edit (the five pre-existing
trees stayed byte-identical). Q2/R3 re-checks requested after this update.

**Round 3 (final, 2026-10-26):** **R1 APPROVED** (comment-extra fix verified); **Q1 APPROVED** (count sweep);
**Q2 APPROVED** (final-tree transcript 66/973, lib 456, semantic 976, targeted suites, golden identity);
**R3 APPROVED** (fresh confirmation lane: DoD matches the 17-test tree, holds honored, no blocking findings).

**Reconciliation:** R-family R1 + R2 + R3 APPROVED; Q-family Q1 + Q2 APPROVED; **zero unresolved blocking
findings**. No lane disagreement remained after the fixes — Q2 and R3 independently identified the same
stale-evidence defect and it was fixed in one pass (§9 of the transcript). Non-blocking notes recorded:
§7 is superseded by §9; the five pre-existing goldens are unchanged; the remaining `.min.ts` /
`callee_path` / p6-atlas-comparison items are B4/merge-gate follow-ups, not B3 defects.

## Decisions (live)

- `D-tb-018` — B3 implemented as a dialect over the JS visitor; type-only import/export edges erased;
  TS parameter annotations recorded (IR-internal, no schema/DTO change).
- `D-tb-003` (schema) and `D-tb-004` (shared `PERF-JS-*` family) now fully implemented for all three tokens.
- `D-tb-019` — `R-tb-010` narrowed: the pinned TSX grammar parses both `<T,>` and `<T>` generic arrows at
  expression position; `<T,>` remains the portable form.

## Risks (open / changed)

- `R-tb-016` (medium) — rule gates remain lexical; 1.00/1.00 precision is synthetic-only. **Real-code canary
  re-run over the 11 rules is still owed before merge** (carry-in).
- `R-tb-002`/`R-tb-003` (TS grammar ABI 14 + node-name drift) — exact pin + in-repo node-kind locks
  (backend unit tests + grammar-link tests) + `TYPE_ONLY_KINDS` list; upgrade only via ADR-0001 G5.
- `R-tb-010` — **mitigated/narrowed** (above); other TSX positions are not claimed.
- `R-tb-014` — `f<typeof import("x")>()` still fails on the pinned grammar; `typeof import("…")` in a type
  position is covered by a committed negative (no finding even when the file is partial).
- `R-tb-006`/`R-tb-012` — no type information; `.d.ts` invisible; documented.
- Open unchanged: `R-tb-002`, `R-tb-003`, `R-tb-006`…`R-tb-008`, `R-tb-011`, `R-tb-012`, `R-tb-014`.

## Blockers

- `B-tb-002` — **merge to `main` requires explicit founder approval** (`A-tb-006`).
- `B-tb-001` — reporter/views label polish for JS/TS dependency clauses; B4 owns it (TS now renders as
  `ts module edges` through the generic label).

## Deferred / follow-ups (carry into B4 / merge gate)

- **Real-code canary re-run** (express/execa/gemini-cli pins) against the 11 implemented rules before merge,
  including the 11-rule hit/FP rates vs the B2-W0 dispositions.
- Real-parse negative for `s = f(s + "x")` (STR-001 real-parse check) and the spread-root attribution
  limitation (`ALLOC-003` reports the alphabetically first loop-reassigned name, not the spread operand).
- README/roadmap/`Cargo.toml` description + output-contract language labels; `docs/heuristics/js-catalog.md`;
  CHANGELOG note for A at merge; semver note (later release, additive minor).
- `is_innermost_event`/`enclosing_loop` re-walk event streams per event (quadratic in body size, shared
  with Python) — shared performance pass.
- Deep-recursion stack safety shared with C/Python/Rust; ~2 MiB cap bounds practical depth.
- A skipped single-file JS/TS target still produces an empty report whose `scan.languages` is the six-value
  supported set; the usage hint remains a B4 label-pass item.

## Next actions

1. `A-tb-004` — **B3 DONE** (above).
2. `A-tb-005` — **B4**: docs/contract updates (`modules.md` §4.2/5.2 + §1, `metrics-json-v1` delta,
   output-contract labels, README/roadmap counts, `Cargo.toml` description), `docs/heuristics/js-catalog.md`,
   CHANGELOG note for A, semver note, JS/TS label/hint follow-ups, real-code canary re-run.
3. `A-tb-006` — merge gate: re-run the plan DoD after any further `main` movement, musl/Windows check,
   parallel review panel, founder merge approval.

## Commits (this session)

- **None yet** — commit pending founder approval. Working tree: 16 modified files, 7 new paths
  (`examples-ts/`, `src/scanner/backends/tree_sitter_typescript/`, `tests/typescript_backend.rs`,
  `tests/examples_ts_coverage.rs`, `tests/determinism_ts.rs`, `tests/fixtures/typescript/`,
  `tests/golden/report-ts/`).

**Resume in a fresh session:** plan `js-ts`, next task **B4** (docs/contract/canary), branch `feat/js-ts`,
base `main @ 9a94905`, W0 `e7f5332`, W1 `90d5677`, W2 `edfc98a`, B3 uncommitted.
