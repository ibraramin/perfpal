# Track B — JS/TS backend — session handoff

> **Session:** B4 — docs/contract/canary + label polish. Plan `js-ts`; branch `feat/js-ts`; base
> `main @ 9a94905`; prior commits W0 `e7f5332`, W1 `90d5677`, W2 `edfc98a`, B3 `9dd19bb` (+ docs
> `21ede53`). **B4 committed `6d6bcdf`; plan DoD green; parallel panel APPROVED (R1/R2/R3 + Q1/Q2).** No push. Merge remains
> `A-tb-006` + founder approval, and now also requires the B4-canary `CALL-001` gate fix (`B-tb-004`).

## North-star / scope discipline

Track B remains a **later release, non-blocking**; the birdseye map (Track A) was not touched. B4 changed
docs (architecture/contract/README/roadmap/spec/plan/track), the reporter's language labels, tests and
goldens, and the canary evidence. **No schema, rule, catalog, scanner, analyzer, Atlas or dependency
change.** `README.md` and `docs/report/output-contract.md` were based on `main @ c797004` and then edited
(founder-approved plan decision) because those files moved on main since the `9a94905` base; the branch
itself was not rebased (merge gate `A-tb-006`).

## Deliverables (this session)

### 1. Docs/contract pass

- `docs/architecture/modules.md` — §1 supported-set row (six tokens); §3.2 language detection for C/Rust/
  Python/JS/TS/TSX, the global directory skips and JS/TS minified/2 MiB caps; §4.2 five backend
  directories / six tokens + `js-ts` close-out; §5.2 v1.8 re-emission; §5.3 schema version; §7 layout.
- `docs/report/output-contract.md` — v1.8 schema references, the six-token vocabulary (`.jsx` → `js`,
  `.tsx` → `tsx`, `.d.*` skipped), the discovery-skip policy, and the dependency-label vocabulary
  (C `#include`, Rust `mod/use`, Python `import`, JS/TS `import`/`require`).
- `README.md` — intro/scan/analyze/quickstart/CLI/exit codes/heuristics updated for JS/TS, 78 catalog
  rules (23 C + 22 Rust + 22 Python + 11 JS), the JS/TS rule table, v1.8 layout/`--version`, and the docs
  index; the `main` Atlas/agent-report content is preserved.
- `docs/roadmap.md` — status snapshot schema 1.8 + six languages, `js-ts` plan row, M6+ JS/TS status
  (C++ deferred), benchmark `small` 142 files / 78 rules, versioning note.
- `Cargo.toml` — package description names C/Rust/Python/JS/TS; dependency lines untouched.
- Semver note: **later release, additive MINOR v1.7 → v1.8**; crate version stays `0.2.0-beta.1`.
- CHANGELOG note is recorded below for Track A to apply at merge (the branch CHANGELOG trails main and is
  intentionally untouched).

### 2. `docs/heuristics/js-catalog.md`

New read-code-first catalog for all 11 `PERF-JS-*` rules: severity/confidence, mechanism, fix, exact
evidence keys, accepted FP classes, baseline stats/cost model, published limitations, the B4 canary
result, and the cross-language pointers. No `PERF-TS-*` rule (reserved/empty); `LOOP-003` remains cut.

### 3. Reporter label polish (`B-tb-001`)

`dependency_mechanisms`/`dependency_clause` in `src/reporter/mod.rs` now recognize the JS family once:
`import/require` in the include-graph caption and ``JavaScript/TypeScript `import`/`require` bindings`` in
the architecture prose. `js`/`ts`/`tsx` no longer render as three bare fragments; unknown languages keep
the escaped `<name> module edges` fallback. Unit lock `dependency_labels_cover_the_js_family`.
`report-js`/`report-ts` goldens regenerated (report.md/html only); the other four trees are byte-identical.

### 4. Tests

- `tests/javascript_backend.rs` — `str_001_real_parse_ignores_a_string_plus_nested_in_a_call`: real-parse
  negative `s = f(s + "x")` is silent; `s = s + "x"` fires exactly once (line 9, `runDirect`).
- `src/analyzer/heuristics/js/tests.rs` — `alloc_003_spread_arm_attributes_the_first_sorted_reassigned_name`
  pins the documented spread-root attribution order.
- `src/reporter/mod.rs` — the label unit test.
- `tests/javascript_backend.rs` count is now 21 integration tests (20 + the STR negative).

### 5. B4 real-code canary (pinned corpora)

`docs/evidence/js-b4/real-code-canary.md`. express `v5.1.0`, execa `v9.6.1`, gemini-cli `v0.65.0`;
2,987 files; 230 partial parses (the known `f<typeof import("x")>()` limitation); all **11 rules fire**;
deterministic (charts/schema/metrics identical; `report.md` provenance only). **Blocking finding:**
`PERF-JS-CALL-001` has **150/180 strict member FPs (+2 wrapped)** from member/method-name collisions (`this.foo()` inside
`foo()`, `new X().foo()` inside `foo()`) because `receiver_root_of` returns `None` for non-identifier
receivers; 3/3 express FPs, 1/1 execa TP. Other canary findings: `TYPE-001` fresh-object class (~10/14),
`ALLOC-001` noisy-but-gated (503 prod, 2.25/kLOC), `ALLOC-003` 17/20 `slice`-consumption + 1 error-message
spread FP, `ASYNC-002` test skew (153 prod vs 1,028 non-prod), `ARCH-001` alias-monorepo blindness
(0 gemini, 4 execa). Cross-cutting: `path_class` misses `.spec.` / hyphenated test dirs (≥34 files / 98
findings mislabelled), and `findings[].symbol` is `<module>` for every JS/TS finding in `metrics.json`
(the ranked action list/key views resolve the narrowest function; full-detail rows still show `<module>`
until `A-tb-008`). No rule fix was made in B4.

### 6. State/docs

`docs/specs/js-ts.md` (B4 close-out, semver, B4 locks, canary findings/limitations),
`docs/architecture/js-ts.md` (§13.4 label note, §17 B4 record, new R-tb-017…021),
`docs/plans/js-ts.md` (B4 result), `docs/tracks/js-ts.md` (B4 session), this handoff,
`docs/evidence/js-b4/{real-code-canary.md,gate.txt}`, `state/tracks/js-ts/session-state.json`.

## Verification (claim → evidence)

Transcript: `docs/evidence/js-b4/gate.txt`. `cargo` is not on the non-interactive PATH; `~/.cargo/bin/cargo`.

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Docs/state complete | file test block from plan DoD | `files: OK` | 0 |
| Formatting clean | `cargo fmt --all -- --check` | no output | 0 |
| Lint clean | `cargo clippy --locked --all-targets -- -D warnings` | `Finished`, no warnings | 0 |
| Full suite green | `cargo test --locked` | **66 suites, 976 passed, 0 failed** (lib 458) | 0 |
| Licences clean | `cargo deny check licenses` | `licenses ok` | 0 |
| Semantic-feature lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | `Finished`, no warnings | 0 |
| Semantic-feature tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 66 suites, **979 passed**, 0 failed | 0 |
| DoD double-run `examples-js` | two `cargo run -- examples-js -o … --force` | charts/schema byte-identical; masked metrics identical | 0 |
| DoD double-run `examples-ts` | two `cargo run -- examples-ts -o … --force` | charts/schema byte-identical; masked metrics identical | 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target cargo check --locked --all-targets` | `Finished` | 0 |
| Label unit lock | `cargo test --locked --lib dependency_labels_cover_the_js_family` | 1 passed | 0 |
| Spread attribution pin | `cargo test --locked --lib alloc_003_spread_arm_attributes_the_first_sorted_reassigned_name` | 1 passed | 0 |
| Real-parse STR negative | `cargo test --locked --test javascript_backend str_001_real_parse_ignores_a_string_plus_nested_in_a_call` | 1 passed | 0 |
| Golden trees | `cargo test --locked --test p6_hardening` | 4 passed (6 trees) | 0 |
| JS backend | `cargo test --locked --test javascript_backend` | 21 passed | 0 |
| TS backend | `cargo test --locked --test typescript_backend` | 17 passed | 0 |
| JS coverage | `cargo test --locked --test examples_js_coverage` | 1 passed | 0 |
| TS coverage | `cargo test --locked --test examples_ts_coverage` | 1 passed | 0 |
| JS determinism | `cargo test --locked --test determinism_js` | 1 passed | 0 |
| TS determinism | `cargo test --locked --test determinism_ts` | 1 passed | 0 |
| Golden identity | `git diff --stat -- tests/golden` | only `report-js`/`report-ts` report.md/html | 0 |
| Cargo.lock untouched | `git diff --name-only -- Cargo.lock` | empty | 0 |
| Cargo.toml only description | `git diff -- Cargo.toml` | description line only | 0 |
| Canary determinism | `diff -r` charts; `cmp` schema/metrics(masked); report.md mask | identical on all three repos | 0 |
| Canary hit counts | `jq` over `metrics.json` | 11/11 rules fire; CALL-001 180 prod (150 strict member FPs + 2 wrapped = 152 member-class) | 0 |
| State budget | `jq -e . … && wc -c` | valid JSON, **≤ 8,192 B** | 0 |

**Gate tooling note:** `scripts/gates/task-gate.sh` is not present in this repo; the plan's commands were
run directly (L4 claim→evidence is satisfied by the transcript).

## Panel

Four lanes dispatched in one batch (2026-10-26), read-only, disjoint scopes.

- **R1 (correctness/determinism): APPROVED.** No blocking findings. Verified the label semantics/golden
  surface, the real-parse STR-001 positive control, the ALLOC-003 pin, and the canary arithmetic. Non-blocking
  fixes applied: express W0-vs-B4 split wording (51 → 7 production files, named explicitly), the broad
  "human artifacts" claim narrowed to the ranked action list/key views (full-detail rows still `<module>`
  until `A-tb-008`), parse-health assertions added to the STR test, the ALLOC-003 pin reversed the
  assignment order to distinguish sorted-name order, and the unknown-language Markdown caption escape was
  recorded as a not-CLI-reachable follow-up.
- **R2 (contract/security/determinism/deps): APPROVED.** No blocking findings. Verified the six-token
  contract/docs, v1.8 invariants, 78-rule catalog, Cargo.lock untouched, golden surface, and merge-gate
  honesty; ledger append is 16 additions / 0 deletions (orchestrator EOL append). Non-blocking fixes applied:
  symbol wording, premature "panel recorded" language, stale 67-rule sentence (historical annotation),
  output-contract grounding v1.0 → v1.8, js-catalog CALL-001 caveat, duplicate `R-tb-016` removed, gate
  annotation; the pre-existing `AGENTS.md` C+Rust sentence is noted as an A-owned merge follow-up.
- **Q1 (verbatim scope/no scope creep): APPROVED.** All requested B4 items mapped to artifacts; no
  rule/schema/scanner/catalog/dependency/Atlas change; no commit/push; CHANGELOG note is in this handoff
  only; semver note present; merge explicitly blocked. Non-blocking fixes applied: `.playwright-mcp/`
  excluded from the commit plan, js-catalog CALL-001 caveat, architecture §17.4 panel wording, handoff
  loose wording; global state files untouched by design (merge-gate concern).
- **R3 (fresh confirmation): APPROVED (static scope).** Verified the post-panel fixes: narrowed symbol
  wording, parse-health assertions in the STR test, reversed assignment order in the ALLOC-003 pin, the
  express 51→7 split caveat, one `R-tb-016` row, the gate §9 annotation, and the exact R1/R2/Q1 fixes. No
  shell in that lane, so the runtime checks are covered by the orchestrator's post-panel §9 run below. Its
  remaining non-blocking notes were fixed: architecture §17.3/track-doc symbol parentheticals narrowed,
  `docs/tracks/README.md` "B4 is closed" reworded, `.min.ts` gap added to the catalog limitations.
- **Q2 (acceptance/evidence): APPROVED** after two blocking evidence fixes. **B-Q2-1:** the canary's Python
  count was 24; it is **128 findings across 23 Python files** (49 Python files scanned, 14 `PERF-PY-*` rule
  ids), all non-production. **B-Q2-2:** the architecture gemini subcount was `150/176`; it is now
  **147/176 strict (+2 wrapped = 149/176 member-class)**, with the headline **150/180 strict (+2 wrapped =
  152 member-class)**. Final re-check: 180 = 3 express + 1 execa + 176 gemini; 150 strict total includes
  the 3 express; the arithmetic and all cross-doc counts agree; the canary numbers, 230 partial parses,
  determinism, state 7,526 B, golden identity and gate §9 all check out. Non-blocking: §9 re-runs
  fmt/clippy/full/targeted but the original §2d/§3/§4/§5 are pre-panel (docs/tests-only changes since, so
  not invalidated); noted for a strict merge-gate re-run.

**Reconciliation:** R family **R1 + R2 + R3 APPROVED**; Q family **Q1 + Q2 APPROVED**; **zero unresolved
blocking findings**. The canary's `CALL-001` defect is a **merge-gate blocker**, not a B4 scope failure: it
remains `B-tb-004`/`R-tb-017`/`A-tb-007`, and merge `A-tb-006` is still blocked on it plus founder
approval. Non-blocking fixes are applied as listed above; the final-tree execution evidence is
`docs/evidence/js-b4/gate.txt` §9 (66 suites / 976 passed, lib 458; targeted suites; fmt/clippy clean).

## Decisions (live)

- `D-tb-020` — B4 docs/contract/canary + label polish landed without a schema/rule/catalog/dependency
  change; `README.md`/`output-contract.md` based on `main @ c797004` then edited (approved partial doc
  rebase); goldens regenerated for the label change only.
- `D-tb-021` — The B4 canary's `CALL-001` gate defect is a **merge-gate blocker** (`R-tb-017`/`B-tb-004`);
  the fix is a separate rule-code change with a canary re-run and golden re-baseline, not a B4 edit.
- `D-tb-022` — `TYPE-001` fresh-object class, `path_class` `.spec.` gap and `findings[].symbol` first-hit
  are documented follow-ups (`R-tb-018`/`019`/`020`), not B4 changes.
- `D-tb-018`/`D-tb-019` remain live (dialect reuse; TSX generic arrows).

## Risks (open)

- `R-tb-017` (`high`, **merge blocker**) — `CALL-001` member/method-name collision gate (150/180 strict member FPs + 2 wrapped).
- `R-tb-018` (`medium`) — `TYPE-001` fresh-object FP class (~10/14).
- `R-tb-019` (`medium`) — `findings[].symbol` first-hit reports `<module>` for JS/TS in `metrics.json`.
- `R-tb-020` (`medium`) — `path_class` misses `.spec.` / hyphenated test dirs (cross-language).
- `R-tb-021` (`low`) — `ALLOC-001` trivial-literal noise remains (503 prod, 2.25/kLOC).
- `R-tb-002`/`003` (TS ABI/node drift), `R-tb-006` (no types), `R-tb-007` (build size), `R-tb-008`
  (global skips), `R-tb-011` (CJS/ESM), `R-tb-012` (`.d.*`/minified), `R-tb-014` (type-query grammar)
  remain open; `R-tb-016` is narrowed by the B4 canary (synthetic KPI still synthetic-only).

## Blockers

- `B-tb-002` — merge to `main` requires explicit founder approval (`A-tb-006`).
- `B-tb-004` — **`CALL-001` merge blocker**: fix the member-receiver gate, re-run the B4 canary, re-baseline
  the JS/TS goldens, then re-run the merge-gate DoD (`A-tb-007`); `B-tb-001` is resolved by the B4 labels.

## Next actions

1. `A-tb-005` — **B4 DONE** (docs/contract/canary/labels; plan DoD green; panel record below).
2. `A-tb-007` — fix `PERF-JS-CALL-001` (require a bare `callee_path == function name` and/or `this`/`new`
   receiver roots; treat truthiness/`if` guards as visible base cases), re-run the B4 canary, re-baseline
   `report-js`/`report-ts`, and re-check the rule's precision/control fixtures.
3. `A-tb-006` — merge gate: rebase on current `main`, fix `B-tb-004`, run the plan DoD on the final tree,
   musl/Windows check, parallel panel, founder approval.
4. `A-tb-008` — decide/document the `TYPE-001` fresh-object gate, `path_class` `.spec.` extension and
   `findings[].symbol` narrowest-record fix (cross-language); `A-tb-009` — `.min.ts`/`.min.mts`/`.min.cts`
   skip gap and the B3 `callee_path`/p6 comparison carry-ins.

## Uncommitted files

None after the B4 feature commit `6d6bcdf` and this docs-record commit. The only working-tree residue is
untracked `.playwright-mcp/` (unrelated browser-tool output; NOT part of B4).

## Commits (this session)

- `6d6bcdf` — `feat(js-ts): B4 docs, contract, labels and real-code canary` (23 files, +1,290/−255).
- The docs-record commit follows: `docs(js-ts): record B4 commit hash in track handoff`. No push.

## CHANGELOG note for Track A (apply at merge; do not land on this branch)

Under `## [Unreleased] → ### Added`:

> - **JS/TS backends (Track B, later release).** JavaScript (`.js`/`.jsx`/`.mjs`/`.cjs` → `js`) and
>   TypeScript/TSX (`.ts`/`.mts`/`.cts` → `ts`, `.tsx` → `tsx`) are first-class scanner backends with 11
>   shared `PERF-JS-*` rules (5 warn / 6 info), six-value `SUPPORTED_LANGUAGES`, schema v1.8 (additive
>   `js`/`ts`/`tsx` tokens), `examples-js`/`examples-ts`, `report-js`/`report-ts` goldens, 22 IR fixtures,
>   precision/benchmark coverage (78 active rules), and a syntactic-tier-only claim. Declaration-only
>   `.d.*` and minified/vendored JS/TS are skipped; no type information, no `node_modules`/`tsconfig`
>   resolution, no framework semantics. Merge is founder-gated; not part of the published beta.
>
> Under `### Changed`: reporter dependency labels now render the shared JS/TS family as
> `import/require` / ``JavaScript/TypeScript `import`/`require` bindings`` instead of bare `js`/`ts`/`tsx`
> module edges.

**Semver note:** the schema change is an additive MINOR (v1.7 → v1.8; no required-list growth, no field
removal/retype, re-marked v1.7 documents validate). The crate version stays `0.2.0-beta.1` until the
release owner cuts the next version. Track B is a later release and does not delay the beta.

## Deferred / follow-ups (carry into the merge gate)

- `CALL-001` gate fix + canary re-run (`B-tb-004`/`A-tb-007`) — **must land before merge**.
- The B4 EOL hot-state compaction appended 16 cold records to `state/archive/ledger.jsonl` (16 additions,
  0 deletions, orchestrator append per the global STATE-SCHEMA cold-memory rule); A reconciles the shared
  ledger at merge. No main `state/session-state.json` / `state/handoff/latest.md` / `docs/activeContext.md`
  edit was made.
- Root `AGENTS.md` still says the product "scans C and Rust codebases" (pre-existing; Python already
  shipped). A-owned doc, update at merge.
- `TYPE-001` fresh-object class, `path_class` `.spec.` gap, `findings[].symbol` narrowest-record fix.
- `ALLOC-001` trivial-literal class (optional gate), `ALLOC-003` `slice`-arm breadth/attribution.
- `.min.ts`/`.min.mts`/`.min.cts` skip gap; `callee_path` / p6-atlas-comparison carry-ins from B3.
- `is_innermost_event`/`enclosing_loop` re-walks (quadratic in body size, shared with Python).
- Reporter Markdown include-graph caption: unknown-language names are not escaped on the Markdown path
  (HTML path escapes). Not reachable from the CLI (the scanner emits fixed tokens), but `render_report` is
  `pub` over a hand-built `MetricsDocument`; defensively escape or document the fixed-token invariant
  (R1 non-blocking).
- Deep-recursion stack safety; ~2 MiB cap bounds practical depth.
- A skipped single-file JS/TS target still produces an empty report whose `scan.languages` is the six-value
  supported set; the usage hint remains a label-pass item.

**Resume in a fresh session:** plan `js-ts`, next task **merge gate `A-tb-006`** after **`A-tb-007`
(CALL-001 fix + canary re-run)**; branch `feat/js-ts`, base `main @ 9a94905`, W0 `e7f5332`, W1 `90d5677`,
W2 `edfc98a`, B3 `9dd19bb`, **B4 `6d6bcdf`**.
