# Track B — JS/TS backend — session handoff

> **Session:** B5 — post-B4 correctness/follow-up pass. Plan `js-ts`; branch `feat/js-ts`; base
> `main @ 9a94905`; prior commits W0 `e7f5332`, W1 `90d5677`, W2 `edfc98a`, B3 `9dd19bb`, B4 `6d6bcdf`
> (+ docs `683adab`). **B5 committed `1b23198` (feat) + `dbda7e3` (docs); no push.**
> No push, no rebase, no merge (`A-tb-006` remains founder-gated).

## North-star / scope discipline

Track B remains a **later release, non-blocking**; the birdseye map (Track A) was not touched. B5 is a
correctness pass on the already-shipped JS/TS tier and closes the B4 canary's merge-gate blocker. It changed
rules, two additive IR-internal fields, the shared aggregator (`findings[].symbol`, `path_class`), the JS/TS
skip list, tests, goldens and docs. **No schema version/catalog-count/dependency/rule-id/severity change**
(catalog stays 78); no `views.rs`/Atlas feature work; no main state file was edited.

## Deliverables (this session)

### 1. `PERF-JS-CALL-001` — merge blocker closed (`B-tb-004` / `R-tb-017` / `A-tb-007`)

`src/analyzer/heuristics/js/mod.rs::js_call_001` now requires ALL of:

- `callee_kind == Direct`, `callee_name == func.record.name`, `receiver_root.is_none()`;
- `callee_path.as_deref() == Some(func.record.name)` — the **exact bare callee**, rejecting `this.f()`,
  `new X().f()`, `(this.x as any).f()` (the 150 strict + 2 wrapped member collisions);
- `!call.conditional_guarded` — an `if`/ternary/`catch`/`&&`/`||` ancestor is a visible base case;
- `func.record.name_is_lexical_binding` — new additive `FunctionRecord` field (serde-default). A class/object
  method or a property/field arrow derives its name from a property key, not a binding, so a bare same-name
  call there is **delegation**, not recursion (the canary's residual 18/20 class);
- no `ComparisonEvent` strictly before the call (unchanged).

`receiver_root_of` is deliberately unchanged (a change would perturb ALLOC-003/LOOP-007 receiver semantics).

**Canary:** production **180 → 2**, total 212 → 2; express 3 → 0, execa 1 → 0, gemini 176 → 2. The two
remaining hits are true recursion (`trackerService.ts` `check` const-arrow DFS; `process-utils.ts`
`getAllDescendants` const-arrow tree walk). Removed: 167 member/wrapped + 11 if/catch-guarded recursion
(FN class, §Risks) + 32 test/non-prod delegation.

### 2. `PERF-JS-TYPE-001` — fresh-object gate (`R-tb-018`)

Additive `UnaryEvent.operand_root_fresh` (serde-default), computed by the JS visitor: true when the deleted
member's receiver root was bound in the same function, in source order, to an object/array literal or `new`
(`fresh_bindings` tracks declarators and plain-identifier assignments; a later non-fresh assignment clears
it). The rule skips the finding. `process.env.*` deletes stay firing (residual accepted class; most are
subscript-form and were already excluded).

**Canary:** production **14 → 2** (both execa parameter-object TPs), total 78 → 40. Suppressed: 6
`mcpUtils.ts` `mergedConfig` + 2 `geminiChat.ts` `newPart` + 1 express `opts` literal + test-file fresh
deletes (9 fresh-object production FPs in total). The three `test-setup.ts` hits are **reclassified as
`test` by `path_class`, not suppressed** — they remain in `metrics.json` with `path_class: "test"`.

### 3. `PERF-JS-ALLOC-003` — spread-arm tightening

New `assigned_from_literal`: the spread array literal must be the RHS of an assignment whose target
identifier is the attributed root (assignment span contains the literal span), conjoined with the existing
`assigned_within`. Kills the error-message FP (`[...visited, current]` inside a `throw`), keeps
`acc = [...acc, item]` (precision suite green). Member-target accumulators
(`currentTurn.content.parts = [...]`) are now silent instead of mis-attributed to `currentTurn` — the old
evidence claim was already false (B4's documented attribution limitation). Canary: 24 → 22 total,
20 → 18 production.

### 4. `findings[].symbol` — narrowest record (`R-tb-019`)

`src/aggregator/mod.rs::findings_with_symbols` now selects the narrowest containing record (line-range
containment, width ascending, `record.id` ascending), mirroring `views.rs::containing_function`. JS/TS
goldens: 10/11 findings resolve to a real function; the single `<module>` is the module-level `ARCH-001`
anchor. Python goldens improved the same way. Re-baselined only `report-js`, `report-ts`, `report-python`
(metrics + detail md/html); `report`, `report-mixed`, `report-rust` and every chart/Atlas artifact are
byte-identical; all six finding counts unchanged.

### 5. `path_class` — `.spec.` / hyphenated test dirs (`R-tb-020`, founder-approved shared change)

`src/aggregator/path_class.rs`: `is_test_basename` adds `test-*`, `.spec.`, `-spec.`; a new
`is_test_segment` adds `test-*` prefixes and `-test(s)`/`-spec(s)` suffixes to the exact segment set.
`src/test-data` keeps `fixture` (test DATA is more specific); production lookalikes (`latest`, `contest`,
`attestation`, `spectrum`, `specification`, `prospect`) are unit-locked as production.
Canary production denominator **1,336 → 1,178** (113 execa `test-d/` + 45 gemini-cli files reclassified).

### 6. Discovery and golden-comparator gaps

- `JS_SKIP_FILE_SUFFIXES` 4 → 7: `.min.ts`/`.min.mts`/`.min.cts` added (discovery + single-file targets),
  with tests for both paths (`A-tb-009`).
- `tests/p6_hardening.rs`: `GOLDEN_ARTIFACTS` 17 → 19, adding `atlas-q2.html`/`atlas-weight.html` to the
  byte-exact comparison and to the regenerator's provenance-free assertion. All six committed trees pass
  unchanged; no masking was needed (`atlas` carry-in closed).

### 7. Tests added/updated

- `src/analyzer/heuristics/js/tests.rs`: CALL-001 member-collision/conditional/lexical-binding silent
  cases; TYPE-001 fresh silent case; ALLOC-003 non-assignment spread silent case; updated helpers.
- `src/scanner/backends/tree_sitter_javascript/tests.rs`: `delete_operand_root_fresh_flag_tracks_fresh_bindings`;
  `name_is_lexical_binding_distinguishes_declarations_from_method_properties`.
- `tests/javascript_backend.rs`: minified-TS discovery + single-file skips; real-parse CALL-001 regression
  (bare fires once; `this.`/`new X().`/truthiness-guarded silent). `tests/typescript_backend.rs`: wrapped
  `(this.storage as any).dispose` silent.
- `src/aggregator/mod.rs`: four narrowest-symbol tests (module-first, nested, module-level, equal-width id
  tie-break). `src/aggregator/path_class.rs`: JS/TS naming + production-controls test.
- Precision: `call_001_member_silent.js`, `call_001_guard_silent.js`, `call_001_delegation_silent.js`,
  `type_001_fresh_silent.js` + labels.
- `tests/fixtures/ir/PERF-JS-CALL-001__unguarded-recursion.json`: `callee_path` + `name_is_lexical_binding`.

### 8. Docs/state

`docs/evidence/js-b5/{real-code-canary.md,gate.txt}` (new); `docs/heuristics/js-catalog.md`,
`docs/specs/js-ts.md`, `docs/architecture/js-ts.md` (§18), `docs/architecture/modules.md`,
`docs/report/output-contract.md`, `README.md`, `docs/adr/ADR-0011-js-ts-backend.md`,
`docs/plans/js-ts.md`, `docs/tracks/js-ts.md`, `docs/tracks/README.md` (B row/start), this handoff,
`state/tracks/js-ts/session-state.json`.

## Verification (claim → evidence)

Transcript: `docs/evidence/js-b5/gate.txt`. `cargo` is not on the non-interactive PATH; `~/.cargo/bin/cargo`.

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Docs/state complete | file test block from plan DoD | `files: OK` | 0 |
| Formatting clean | `cargo fmt --all -- --check` | no output | 0 |
| Lint clean | `cargo clippy --locked --all-targets -- -D warnings` | `Finished`, no warnings | 0 |
| Full suite green | `cargo test --locked` | **66 suites, 991 passed, 0 failed** | 0 |
| Licences clean | `cargo deny check licenses` | `licenses ok` | 0 |
| Semantic-feature lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | `Finished`, no warnings | 0 |
| Semantic-feature tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 66 suites, **994 passed**, 0 failed | 0 |
| DoD double-run `examples-js` | two `cargo run -- examples-js -o … --force` | charts/schema byte-identical; masked metrics identical | 0 |
| DoD double-run `examples-ts` | two `cargo run -- examples-ts -o … --force` | charts/schema byte-identical; masked metrics identical | 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | `Finished` | 0 |
| JS backend | `cargo test --locked --test javascript_backend` | 22 passed | 0 |
| TS backend | `cargo test --locked --test typescript_backend` | 19 passed | 0 |
| JS/TS coverage + determinism | four targeted suites | 1 passed each | 0 |
| Precision KPI | `cargo test --locked --test precision_kpi` | 1 passed (JS 1.0/1.0 retained) | 0 |
| IR fixtures | `cargo test --locked --test js_ir_fixtures` | 1 passed | 0 |
| Golden trees | `cargo test --locked --test p6_hardening` | 4 passed (6 trees; 19 artifacts incl. Atlas) | 0 |
| Aggregator locks | `cargo test --locked --lib aggregator` | 34 passed | 0 |
| JS rule locks | `cargo test --locked --lib analyzer::heuristics::js` | 33 passed | 0 |
| JS backend locks | `cargo test --locked --lib tree_sitter_javascript` | 27 passed | 0 |
| Golden identity | `git diff --stat -- tests/golden` | only `report-js`/`report-ts`/`report-python` metrics+detail | 0 |
| Cargo.lock untouched | `git diff --name-only -- Cargo.lock` | empty | 0 |
| Canary determinism | `diff -r` charts; `cmp` schema/metrics(masked) | identical on all three repos | 0 |
| Canary CALL-001 | `jq` over `metrics.json` (B4 binary vs B5 binary) | production 180 → 2; both TPs; 210 removed | 0 |
| Canary TYPE-001 | `jq` over `metrics.json` | production 14 → 2; both TPs; total 78 → 40 | 0 |
| Canary production split | `jq` over `files[].path_class` | 1,336 → 1,178 | 0 |
| State budget | `jq -e .` + `wc -c` | valid JSON, **≤ 8,192 B** | 0 |

## Panel

Four lanes dispatched in one parallel batch after the DoD (2026-10-26, read-only, disjoint scopes):
R1 correctness/determinism, R2 contract/security/determinism, Q1 verbatim scope, Q2 acceptance/evidence.

**Round 1 verdicts:** **R1 APPROVED** (no blocking findings; non-blocking: CLASSIFYING the canary counter's
`__tests__`/`__mocks__`/`e2e` divergence, line-only symbol on same-line nesting, TYPE-001 wrapper-strip gap,
ALLOC-003 concat-arm looseness, block-scoped `fresh_bindings`, missing delegation/clear-on-rebind tests,
`switch_case` guard gap, year-specific p6 check, IR leak tests not extended). **R2 APPROVED** (contract clean:
no schema/catalog/dependency/exit-code change; determinism sound; non-blocking: state claimed the panel was
recorded, canary counter skip-list divergence, guarded IR fixture hygiene, year-specific p6 check, 5-byte
state headroom). **Q1 CHANGES_REQUESTED** (two blocking bookkeeping items: the B5 cold-ledger append edited
a main state file contrary to `docs/tracks/README.md`, and the panel section was still a placeholder while
other artifacts claimed it was recorded). **Q2 CHANGES_REQUESTED** (same panel-placeholder defect; plus
non-blocking 152-vs-167 reconciliation, test-setup "suppressed" wording, stale `docs/tracks/README.md`).

**Fixes applied for the confirmation lane:**

- `state/archive/ledger.jsonl` append **reverted** (Track B does not edit main state; cold reconciliation is
  A's at merge, per the B4 precedent). The handoff's "no main state file was edited" claim is true again.
- This panel record filled; the four surfaces that pointed at it now say **round 1 recorded, confirmation
  pending**. Final verdicts are appended below after the confirmation lane.
- Canary §3 gained the exact **152-vs-167** reconciliation (B4 hand count vs B5 mechanical callee-slice
  classification; both derive from the same 212 → 2 delta).
- Handoff TYPE-001 wording corrected: the three `test-setup.ts` hits are **reclassified as test**, not
  suppressed.
- `docs/tracks/README.md` B row + start message advanced to B5/B6.
- Review-driven code/test hardening: TYPE-001 `is_fresh_value` now strips TS transparent wrappers
  (`const x = { … } as T`), with a TS real-parse test; backend tests for clear-on-rebind and source order;
  a real-parse same-name delegation case (object property arrow + class method) and a
  `call_001_delegation_silent.js` precision fixture; the guarded CALL-001 IR fixture now carries
  `callee_path` + `name_is_lexical_binding` so it isolates the comparison gate.
- Hot state trimmed to **8,186 B** (≤ 8,192) after the R-tb-021/phase update. The finer `D-tb-025`…`028`
  breakdown lives in this handoff only; the hot state consolidates them into `D-tb-023`/`D-tb-024` for the
  8 KB cap (the track ledger is A-owned and was not touched).

**Confirmation round (2026-10-26):** **Q1 APPROVED** (both round-1 blockers and both residual
bookkeeping items — stale DoD citations + incomplete commit inventory — verified fixed; every W1–W4 item
maps to an artifact; no scope creep; holds intact). **Q2 APPROVED** (stale citations fixed; final numbers
991/994; canary re-scan hashes unchanged; state 8,186 B; no commit/push/rebase/merge). **R3 (fresh)
APPROVED** (post-review code/test delta: TYPE-001 wrapper strip, clear-on-rebind/source order, delegation
coverage, guarded IR fixture; final-tree consistency).

**Reconciliation:** R family **R1 + R2 + R3 APPROVED**; Q family **Q1 + Q2 APPROVED**; **zero unresolved
blocking findings**. Non-blocking notes recorded: `docs/tracks/README.md` historical paragraph (now updated
to B5), global EOL artifacts remain Atlas (track-local state authoritative), and the Q lanes' full-cargo
re-execution limitation (transcript-only in their sandbox; the orchestrator ran the DoD).

## Decisions (live)

- `D-tb-023` — CALL-001 closes via rule-local gates: exact bare `callee_path`, `!conditional_guarded`, and
  the additive `FunctionRecord.name_is_lexical_binding`. `receiver_root_of` was deliberately not changed.
- `D-tb-024` — TYPE-001 gains the additive `UnaryEvent.operand_root_fresh`; freshness is same-function,
  source-order, literal/`new` provenance; `process.env.*` remains the published residual class.
- `D-tb-025` — `path_class` extension approved by the founder in this session (shared/cross-language); the
  `test-data` fixture carve-out and production lookalikes are unit-locked.
- `D-tb-026` — ALLOC-003 spread arm requires an assignment RHS to the attributed identifier root; member-target
  accumulators are accepted-silent rather than mis-attributed (restoring them needs a member-root field).
- `D-tb-027` — p6 compares both Atlas pages byte-exact (`GOLDEN_ARTIFACTS` 19); no masker needed.
- `D-tb-028` — `findings[].symbol` is the narrowest containing record, mirroring the reporter key views;
  goldens for the three affected trees were re-baselined in this session.
- `D-tb-020`…`D-tb-022` from B4 remain historical; `D-tb-018`/`D-tb-019` remain live (dialect reuse; TSX arrows).

## Risks (open / changed)

- `R-tb-017` **resolved** (CALL-001: 180 → 2 production, both TPs).
- `R-tb-018` **resolved** (TYPE-001: 14 → 2 production, both TPs).
- `R-tb-019` **resolved** (symbol narrowest; goldens re-baselined).
- `R-tb-020` **resolved** (path_class; canary denominator corrected).
- `R-tb-021` **open, unchanged** (ALLOC-001 remains noisy but gated: 485 production / 2.30 per kLOC on the
  re-based denominator).
- `R-tb-022` **new, low** — CALL-001 accepted FN: recursion whose only guard is an `if`/`catch` ancestor is
  silenced (11 observed: 10 gemini-cli production + 1 execa catch-retry). Documented; candidates are still
  visible in the code and the rule is Info.
- `R-tb-023` **new, low** — TYPE-001 residual `process.env.*` deletes (host-owned global); accepted class.
- `R-tb-024` **new, low** — ALLOC-003 member-target accumulators are silenced rather than attributed; needs
  a member-root attribution field to restore without false evidence.
- `R-tb-002`/`003` (TS ABI/node drift), `R-tb-006` (no types), `R-tb-007` (build size), `R-tb-008`
  (global skips), `R-tb-011` (CJS/ESM), `R-tb-012` (`.d.*`/minified), `R-tb-014` (type-query grammar)
  remain open; `R-tb-016` is narrowed by the B5 canary (synthetic KPI still synthetic-only).

## Blockers

- `B-tb-002` — merge to `main` requires explicit founder approval (`A-tb-006`); the merge gate still needs
  a rebase on current `main` and a final DoD on the rebased tree.
- `B-tb-004` — **resolved in B5** (CALL-001 gate fixed, canary re-run, JS/TS goldens re-baselined).
- `B-tb-003` — closed in B2-W2 (catalog atomic edit); no change.

## Next actions

1. **Commit** the B5 feat + docs changes once the founder gives the go-ahead (two commits, no push:
   rule/IR/aggregator/tests/goldens; then docs/state).
2. `A-tb-006` — merge gate: rebase on current `main`, run the plan DoD on the final tree, musl/Windows
   check, parallel panel on the rebased tree, founder merge approval.
3. Follow-ups (not merge-blocking): `ALLOC-001` triviality gate (optional), ALLOC-003 member-root
   attribution, CALL-001 conditional-guard precision (could recover the 11 guarded recursions),
   `is_innermost_event`/`enclosing_loop` re-walks, deep-recursion stack safety, reporter Markdown
   unknown-language escaping (unreachable from the CLI).

## Committed files

`1b23198` (feat) holds the rule/IR/aggregator/scanner/test/golden changes; this docs commit holds the
evidence, docs and track state. `git status --short` is clean apart from the pre-existing unrelated
untracked `.playwright-mcp/` (NOT part of B5; exclude from any commit).

- Feat `1b23198`: `src/aggregator/{mod.rs,path_class.rs}`, `src/analyzer/heuristics/js/{mod.rs,tests.rs}`,
  `src/analyzer/heuristics/python/mod.rs`, `src/ir/mod.rs`, `src/scanner/mod.rs`,
  `src/scanner/backends/tree_sitter_{c/mod.rs,python/mod.rs,rust/mod.rs}`,
  `src/scanner/backends/tree_sitter_javascript/{infer.rs,mod.rs,scan.rs,tests.rs}`,
  `tests/{analyzer_registry,common/mod.rs,javascript_backend,p6_hardening,typescript_backend}.rs`,
  `tests/fixtures/ir/PERF-JS-CALL-001__{unguarded,guarded}-recursion.json`,
  `tests/fixtures/precision/labels.json`, nine golden files under `tests/golden/{report-js,report-ts,report-python}/`,
  plus the docs listed in §8 (including `docs/tracks/README.md`).
- Docs commit: `docs/evidence/js-b5/{real-code-canary.md,gate.txt}` and the docs/state listed in §8.
- Untracked (NOT part of B5): `.playwright-mcp/` — excluded.

## Commits (this session)

- `1b23198` — `feat(js-ts): B5 rule gates, symbol/path_class fixes and goldens`.
- `dbda7e3` — `docs(js-ts): record B5 correctness pass, canary and panel`.
- This docs-record commit records both hashes. No push, no rebase, no merge.

## CHANGELOG note for Track A (apply at merge; do not land on this branch)

The B4 note stands (JS/TS backends, six tokens, v1.8, 78 rules, later release, founder-gated). Add under
`### Fixed` at merge:

> - **JS/TS rule precision:** `PERF-JS-CALL-001` no longer reports member/method-name collisions or
>   same-name delegation; `PERF-JS-TYPE-001` no longer reports deletes from freshly built per-call objects;
>   `PERF-JS-ALLOC-003` no longer reports non-accumulator spreads. `findings[].symbol` now names the
>   narrowest enclosing function, and `path_class` recognizes `.spec.`/`test-*` files. Discovery also skips
>   `.min.ts`/`.min.mts`/`.min.cts`.

**Semver note:** unchanged — additive MINOR v1.7 → v1.8 from B2; crate stays `0.2.0-beta.1`. B5 adds only
IR-internal serde-defaulted fields; no wire-schema change.

## Deferred / follow-ups (carry into the merge gate)

- ALLOC-001 trivial-literal class (optional gate; 485 production hits on the re-based denominator).
- ALLOC-003 member-target accumulators (needs a member-root attribution field).
- CALL-001 guarded-recursion recall (if/catch ancestors; 11 observed).
- `is_innermost_event`/`enclosing_loop` quadratic re-walks (shared with Python).
- Reporter Markdown include-graph caption: unknown-language names are not escaped on the Markdown path
  (HTML path escapes); not CLI-reachable, defensive fix or documented invariant.
- Deep-recursion stack safety; ~2 MiB cap bounds practical depth.
- A skipped single-file JS/TS target still produces an empty report whose `scan.languages` is the six-value
  supported set.
- Root `AGENTS.md` "scans C and Rust codebases" sentence (A-owned doc; Python/JS/TS already shipped).
- The B4 EOL cold-ledger append remains A's to reconcile at merge; no main state file was touched here.

**Resume in a fresh session:** plan `js-ts`, next task **B6 merge gate** (`A-tb-006`) after the founder's
commit/merge go-ahead; branch `feat/js-ts`, base `main @ 9a94905`, W0 `e7f5332`, W1 `90d5677`, W2 `edfc98a`,
B3 `9dd19bb`, B4 `6d6bcdf` (+ docs `683adab`), B5 `1b23198` (+ docs `dbda7e3`). Signposts: B5 canary
`docs/evidence/js-b5/real-code-canary.md`, gate `docs/evidence/js-b5/gate.txt`.
