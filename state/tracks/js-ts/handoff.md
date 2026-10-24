# Track B — JS/TS backend — session handoff

> **Session:** B2-W2 — 11 `PERF-JS-*` rules + atomic catalog landing + schema v1.8 + the
> version-pin sweep + `examples-js` + IR fixtures + `report-js` golden + coverage/precision/benchmark
> + `determinism_js`. Plan `js-ts`; branch `feat/js-ts`; **rebased on `main @ 9a94905`** (A step 12
> landed; prior wave tip `c3d9282` replayed cleanly). **Uncommitted; commit pending founder approval.**
> No push.

## North-star / scope discipline

Track B remains a **later release, non-blocking**; the birdseye map (Track A) is the core feature and
was not touched. This wave changed `src/analyzer`, `src/scanner` (the additive `AllocEvent.has_spread` fact and the member-expression property name), `src/reporter` (catalog entries
only), `src/schema`, `src/cli` (version suffix), tests/fixtures/goldens, track docs/state. No Atlas
surface, no main state file, no `views.rs`, no dependency change (`Cargo.toml`/`Cargo.lock` untouched).

## Deliverables (this session)

### 1. Eleven `PERF-JS-*` rules (5 warn / 6 info, all MEDIUM confidence)

| Rule | Gate as implemented |
|---|---|
| `PERF-JS-ALLOC-001` | object/array literal at `in_loop_depth ≥ 1`; nearest enclosing loop required; silent when a `referenced_names` entry is an induction var or is assigned inside that loop (loop-invariance gate) |
| `PERF-JS-ALLOC-003` | `concat`/`slice` calls **or** array literals with spread (`acc = [...acc, x]`, `AllocEvent.has_spread`) whose accumulator root is reassigned inside the same loop; snapshot roots stay silent |
| `PERF-JS-STR-001` | `+=` with an identifier left operand **and a string/template-literal RHS**, or `x = x + <string\|template literal>` where the binary expression is the whole RHS, in a loop; numeric/identifier/subscript/call RHS silent; trip `< 8` silent |
| `PERF-JS-LOOP-001` | `LoopNode.depth ≥ 2`; one finding per function at the earliest qualifying loop; `max_depth` counts only loops inside that anchored loop |
| `PERF-JS-LOOP-002` | call in a loop to a known sync Node API (23 names incl. crypto sync) |
| `PERF-JS-LOOP-007` | `indexOf/includes/find/findIndex/some/every/filter` in a loop, non-empty receiver; silent when known trip `< 16` |
| `PERF-JS-ASYNC-001` | `AwaitEvent` with `in_loop_depth ≥ 1`; requires `record.is_async` |
| `PERF-JS-ASYNC-002` | sync Node API call in an `async` function (no loop requirement) |
| `PERF-JS-CALL-001` | `CalleeKind::Direct` bare self-call; silent when any `ComparisonEvent` starts before the call (visible base case) |
| `PERF-JS-ARCH-001` | resolved import cycles via `graph::break_cycles`; one finding per broken edge at the earliest directive span |
| `PERF-JS-TYPE-001` | `UnaryEvent op="delete"` with a `MemberAccess` operand; subscript form and bare-identifier delete excluded; the property name is carried in evidence |

Additive IR-internal fact (B2-W2, no schema/DTO change): `AllocEvent.has_spread: bool`
(`#[serde(default)]`), populated by the JS backend from `spread_element` children; only
`PERF-JS-ALLOC-003` consumes it. `PERF-JS-LOOP-003` is **cut**; **zero** `PERF-TS-*` exist. IR-only: no engine node kinds cross the
seam; every message states mechanism, fix and accepted-FP class; every finding is an optimization
candidate on the syntactic tier.

### 2. Atomic catalog landing (`B-tb-003` resolved)

- `src/analyzer/finding.rs` — 11 `JS_*_RULE_ID` constants.
- `src/analyzer/mod.rs` — `RuleLanguage::{Js, Ts}`; JS registry block **between C and Python**
  (global ascending-order test); partition `"js" | "ts" | "tsx"`; `Js` dispatch; `Ts` reserved/empty.
- `src/analyzer/heuristics/js/{mod.rs,tests.rs}` — rules + 27 parser-free IR unit tests.
- `src/reporter/mod.rs` — 11 `HEURISTIC_CATALOG_V1` entries in the same change; count/equality locks
  `67 → 78` (23 C + 11 JS + 22 PY + 22 RS); Appendix-A panic unchanged; JS prefix ascending check.
  Founder approved the catalog-only B edit in the B2-W2 work order.

### 3. Schema v1.8 + version-pin sweep

- `src/schema/metrics.schema.v1.8.json` + byte-identical `docs/schema/metrics.schema.v1.8.json`
  (23,253 B each; `cmp` identical). v1.7 retained in both places. Only the six expected JSON hunks
  changed: `$id`, title, top description, `schema_version.const`+description, both language enums +
  descriptions; `scan.languages` fallback wording is generic; no required-list change.
- `SCHEMA_VERSION` / `SCHEMA_ARTIFACT_NAME` / `EMBEDDED_SCHEMA` / CLI `VERSION_SUFFIX` = 1.8.
- `tests/schema_1_7.rs` → `tests/schema_1_8.rs` (v1.8 lock, retained v1.7/v1.6/v1.5/v1.4 history);
  sweep in `schema_1_4`, `pipeline_e2e`, `resolver_health`, `exit_code_matrix`, `i1a/i1b/i1c`,
  `atlas_fixtures`, `key_views` (:200), `p6_hardening` (`GOLDEN_ARTIFACTS` + `provenance_free`).
  `docs/schema/metrics-json-v1.md` gained the v1.7→v1.8 delta.

### 4. Corpora, goldens and gates

- `examples-js/` — 15 scanned files (14 `.js` + 1 `.cjs`) + `package.json` (`main: index.js`,
  script/HIGH); ESM + CJS; one module per rule family; 18 silent controls.
- `tests/fixtures/ir/PERF-JS-*__*.json` — 22 fixtures (positive + control per rule) and
  `tests/js_ir_fixtures.rs` (typed `Vec<FileUnit>`, no parser).
- `tests/examples_js_coverage.rs` — exact 11-id set, zero C/PY/RS/TS ids, `scan.languages == ["js"]`,
  15 files (14/1 split), 11 findings (0 hot / 5 warn / 6 info), per-rule count 1, controls silent.
- `tests/determinism_js.rs` — masked double-run over `examples-js`; HTML/charts/schema byte-identical.
- `tests/fixtures/precision/js/` (25 files) + `labels.json` — 11 rules at precision 1.00 / recall 1.00;
  benchmark `small` rung lifts the same fixtures verbatim (paths/lines equal the M1 anchors).
- Benchmark ladder — `active_rule_ids` 67 → 78; `baseline.json` + README extended; `small` rung
  142 files / 1,655 LOC / 96 findings / 79 scoped rules (78 active + retired `PERF-RS-STR-001`);
  `medium`/`large`/`real-code` and the rung determinism check PASS.
- Goldens — `GOLDEN_TREES` 4 → 5 (`tests/golden/report-js/`, 20 files); `EXPECTED_COMPONENT_PAGES`
  extended; all five trees now carry `metrics.schema.v1.8.json` and **no** stale v1.7 artifact;
  regenerated with `PERFPAL_REGEN_GOLDEN=1`.
- Docs — `docs/architecture/js-ts.md` (§8 implementation status + B2-W2 section),
  `docs/specs/js-ts.md`, `docs/tracks/js-ts.md`, `docs/tracks/README.md`; track state updated.

## Verification (claim → evidence)

Run from the repo root; `cargo` is not on the non-interactive PATH, so `~/.cargo/bin/cargo` is used.
Full transcript: `docs/evidence/js-w2/gate.txt`.

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Rebase clean | `git rebase main` | replayed 6 commits onto `9a94905` | 0 |
| Baseline green before edits | `cargo test --locked --test p6_hardening` | 4 passed | 0 |
| Formatting clean | `cargo fmt --all -- --check` | no output | 0 |
| Lint clean | `cargo clippy --locked --all-targets -- -D warnings` | `Finished`, no warnings | 0 |
| Full suite green | `cargo test --locked` | 63 suites, **935 passed**, 0 failed | 0 |
| Library tests | `cargo test --locked --lib` | **437 passed**, 0 failed | 0 |
| Licences clean | `cargo deny check licenses` | `licenses ok` | 0 |
| Semantic-feature lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | `Finished`, no warnings | 0 |
| Semantic-feature tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 63 suites, **938 passed**, 0 failed | 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | `Finished` | 0 |
| JS coverage gate | `cargo test --locked --test examples_js_coverage` | 1 passed | 0 |
| Parser-free IR fixtures | `cargo test --locked --test js_ir_fixtures` | 1 passed (22 fixtures) | 0 |
| JS determinism | `cargo test --locked --test determinism_js` | 1 passed | 0 |
| Precision ratchet | `cargo test --locked --test precision_kpi` | 1 passed (11 JS rules 1.00/1.00) | 0 |
| Benchmark ladder | `cargo test --locked --test benchmark` | 1 passed (15.06 s) | 0 |
| Golden trees | `cargo test --locked --test p6_hardening` | 4 passed | 0 |
| DoD double-run | two `cargo run -- examples-js -o … --force` runs | charts/schema byte-identical; masked metrics identical | 0 |
| Schema copies identical | `cmp src/schema/…v1.8.json docs/schema/…v1.8.json` | identical (23,253 B) | 0 |
| Guard: determinism config | `! grep -q preserve_order Cargo.lock` | pass | 0 |
| Guard: no dependency drift | `git diff --name-only -- Cargo.toml Cargo.lock` | empty | 0 |
| Guard: whitespace | `git diff --check` | clean | 0 |
| Guard: hot state budget | `jq -e . … && wc -c` | valid JSON, 8,174 B (≤ 8,192) | 0 |

**Gate tooling note:** `scripts/gates/task-gate.sh` is not present in this repo; the plan's commands
were run directly (L4 claim→evidence is satisfied by the transcript).

## Panel

- **Round 1 (2026-10-24):** R2 APPROVED (contract/schema/determinism/security); Q1 APPROVED (verbatim
  scope); Q2 APPROVED (acceptance/evidence); **R1 CHANGES_REQUESTED** with two blocking rule-gate
  defects: (1) `PERF-JS-STR-001` fired on numeric `+=` while the B2-W0 canary counted only string/
  template-literal RHS; (2) `PERF-JS-ALLOC-003` omitted the frozen `spread` accumulation shape.
- **Fixes:** STR-001 is string/template-gated (both `+=` and `x = x + <literal>` forms, the latter only when
  the binary is the whole RHS; numeric silent); `AllocEvent.has_spread` + the spread-accumulator arm
  landed; TYPE-001 is MemberAccess-gated and carries the property name; LOOP-001 `max_depth` is
  span-scoped; the catalog text, tiny-loop controls, CLI `about` and the two second-arm precision/
  benchmark TPs were updated. Goldens regenerated; post-fix full suite **935 passed** (semantic
  **938**). Evidence: `docs/evidence/js-w2/gate.txt` → "Round-2 fixes" + "Post-round-2 polish".
- **Round 2:** R1 re-check APPROVED; fresh R3 APPROVED; Q2 CHANGES_REQUESTED (docs counts/assignment)
  → fixed; final confirmation lane pending. Verdicts appended to the evidence transcript.

## Decisions (live)

- `D-tb-017` — B2-W2 implemented: 11 `PERF-JS-*` rules (5 warn / 6 info, all MEDIUM) with IR-only
  gates in `heuristics/js`; catalog **78** landed atomically after founder approval (`B-tb-003`
  resolved); schema **v1.8** with six tokens (v1.7 retained); `examples-js` 15 files / 11 findings;
  22 IR fixtures; `report-js` golden (5 trees); precision + benchmark 1.00/1.00.
- `D-tb-003` (schema) and `D-tb-004` (shared `PERF-JS-*` family, `RuleLanguage::Ts` reserved) are now
  implemented as decided.
- Archived to the ledger previously: `D-tb-005/006/008/009/012`; stubs remain in hot state.

## Risks (open / changed)

- `R-tb-015` — **resolved**: schema v1.8 (six tokens) is emitted and pinned; v1.7 retained; goldens
  regenerated. No release may precede this point (this point is B2).
- `R-tb-009` — **resolved**: `HEURISTIC_CATALOG_V1` landed atomically with the registry under the
  founder-approved catalog-only B edit; 78-count/equality/Appendix-A locks green.
- `R-tb-016` (medium, **new**) — the rule gates are lexical heuristics: `ALLOC-001` models
  loop-invariance (not a separate escape analysis) and `collect_identifier_names` descends into nested
  function bodies (a closure referencing the induction var can suppress a real hit); `CALL-001` treats
  any earlier comparison as a visible base case; `LOOP-001` cannot test "no early exit" (no break/return
  events in IR); `LOOP-007` uses loop trip count as the proxy for the frozen "small collection" FP
  class; the spread arm of `ALLOC-003` accepts any referenced name reassigned in the loop, and both
  `ALLOC-003` arms miss member/`this` accumulators (`assigned_within` matches plain identifier targets
  only), and the spread arm reports the alphabetically first loop-reassigned referenced name as
  `receiver_root` (the spread operand itself is not identified in v1). `STR-001` and `ALLOC-003` now match the B2-W0 canary predicates (string/template RHS; whole-RHS
  assignment; concat/slice/spread).
  The 1.00/1.00 precision result is on the synthetic precision corpus only — re-run the real-code
  canary before the merge gate.
- `R-tb-004` / `R-tb-013` — remain mitigated and integration-locked.
- Still open: `R-tb-002`, `R-tb-003`, `R-tb-006`…`R-tb-008`, `R-tb-010`…`R-tb-012`, `R-tb-014`.

## Blockers

- `B-tb-001` — reporter/views label polish for JS dependency clauses (`dependency_clause` renders a
  JS corpus as bare `js module edges`); B4 owns the label pass. A step 12 landed and the branch is
  rebased.
- `B-tb-002` — **merge to `main` requires explicit founder approval** (A step 12 is now landed).
- `B-tb-003` — **resolved**.

## Deferred / follow-ups

- **Real-code canary re-run** (express/execa/gemini-cli pins) against the 11 implemented rules before
  merge; compare prod/test hit rates with the B2-W0 dispositions (`R-tb-016`).
- `is_innermost_event`/`enclosing_loop` re-walk event streams per event — quadratic in body size,
  shared with the Python backend; revisit in a shared performance pass.
- Deep-recursion stack safety is shared with C/Python/Rust; the ~2 MiB cap bounds practical depth.
- A single skipped single-file JS target still produces an empty report whose `scan.languages` is the
  four-value supported set; the usage hint planned for B2-W2 was **not** added to avoid reporter churn
  mid-wave — carry it into B4 with the label pass.
- `docs/heuristics/js-catalog.md` (read-code-first rule catalog) is not written yet; B4 docs.
- The TS-first probe-order test and TS negatives land with B3, when TS targets enter the scanned set.

## Next actions

1. `A-tb-004` — **B3**: TS/TSX backend (`LANGUAGE_TYPESCRIPT`/`LANGUAGE_TSX`), `SUPPORTED_LANGUAGES`
   six values, `examples-ts/`, `report-ts` golden (`GOLDEN_TREES` → 6), TS type-only + `<T,>` negatives,
   `examples_ts_coverage.rs`, `determinism_ts.rs`; no schema bump.
2. `A-tb-005` — **B4**: docs/contract updates (`modules.md` §4.2/5.2 + §1, output-contract labels,
   README/roadmap counts, `Cargo.toml` description), `docs/heuristics/js-catalog.md`, CHANGELOG note
   for A, semver note, and the JS label/hint follow-ups above.
3. `A-tb-006` — merge gate: re-run the plan DoD after any further `main` movement, musl/Windows check,
   parallel review panel, founder merge approval.

## Commits (this session)

- **Pending founder approval** — B2-W2 tree (rules + catalog + schema v1.8 + corpora/goldens/gates +
  docs/state). Expected shape: one `feat(js-ts): B2-W2 …` commit plus a docs follow-up recording the
  hash. No push.

**Resume in a fresh session:** plan `js-ts`, next task **B3** (TS/TSX backend), branch `feat/js-ts`,
base `main @ 9a94905`, W0 `e7f5332`, W1 `90d5677`, W2 pending.
