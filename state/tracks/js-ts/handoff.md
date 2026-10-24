# Handoff — Track B / Session B2-W1 (JS backend + wiring)

**Updated:** `2026-10-24`
**Repo:** `/home/shafin/Opencode/Perfpal-js-ts` (branch `feat/js-ts`, based on `main` @ `189df91`;
session HEAD `2166c8f`)
**Session:** B2-W1 — JavaScript tree-sitter backend, scanner wiring, discovery/resolver/entry/instrument,
and a new integration suite; **no `PERF-JS-*` rules and no schema bump** (B2-W2).
**Verdicts:** round 2 — **R1 APPROVED · R2 APPROVED · Q2 APPROVED · R3 APPROVED**, zero blocking findings;
final confirmation (round 3) **APPROVED** after the comment-extra guard. Round 1 was
R1/R2/Q1 APPROVED · Q2 CHANGES_REQUESTED; all round-1/2/3 fixes are in this tree.
**Phase:** **B2-W1 implemented; full DoD green; panel APPROVED; commit pending.**

## Objective

Implement the JavaScript backend and every B2-W1 wiring point from plan `js-ts`, and lock them with a
truthful integration suite that exercises the real grammar, the real scanner, the real aggregator, and the
real CLI. Rules, schema v1.8, examples and goldens are deliberately out of scope.

## Phase / plan

- Plan: `js-ts` (status active) · track B · B1 done, B2-W0 done, **B2-W1 implemented; full DoD green;
  panel APPROVED; commit pending**, **B2-W2 next**.
- North-star: Track B remains a **later release, non-blocking**. No reporter/Atlas surface, no
  `views.rs`/`mod.rs` language wiring, no main state files touched.
- `src/` already contained the B2-W1 implementation when this session started (working-tree changes).
  This pass applied the round-1 correctness fixes inside the JS backend/wiring, added tests, and corrected
  docs/state; it did **not** edit examples, goldens, schema, or main state.

## Deliverables (this session)

- `tests/javascript_backend.rs` — **20 integration tests**:
  1. detection/tokens (`.js`/`.jsx`/`.mjs`/`.cjs`; `.ts`/`.tsx`/`.d.*` skipped; mixed C+JS languages);
  2. discovery skips (six global dirs, `vendor/` kept, four minified name forms, >2 MiB JS skipped while
     >2 MiB C still scans);
  3. parse health (`Partial` + `error_node_count > 0` for malformed; clean stays `ok`; summary counts);
  4. entry ladder (`package.json` `main`; `bin` string/object/array; conventional roots; both in-source
     guards; malformed manifest → exit 0 + conventional rung; no candidate → `unknown-entry`/`LOW`);
  5. loops + cost (`literal` 10, `<=` 4, `while` unknown, literal+unknown `partial`, omitted when no loops,
     nested `max_loop_depth == 2`);
  6. events/instrument (`alloc_site_count` 3 across new/object/array, `throw`/try `error_path`, present
     `idiom_hash`, `#private` hash sensitivity, `alloc_family`, `function_cost_v2`);
  7. exact include-graph edges for resolved `./util.js`, unresolved-internal `./missing.js` (`to: null`),
     external `react` excluded, `export … from` and static `require` resolved;
     extensionless `./util` → `util.js` and directory `./lib` → `lib/index.js` (probe order locked);
  8. zero JS findings in W1; IR-internal `allocations`/`unaries`/`entry_declared` do **not** appear in
     `metrics.json`;
  9. two CLI runs in separate temp dirs byte-identical after masking the two provenance fields;
  10. single-file policy (`*.min.*`/`.bundle.js` and unsupported TS/declaration suffixes skipped; unknown
      extension keeps the C fallback; single `src/util.js` → 1 file/`["js"]`) and the nested
      `package.json` `"main": "../../outside.js"` root-escape guard (never HIGH).
- **Round-1 + round-2 correctness fixes:** loop-context ancestor walks stop at function/class boundaries
  (`scan.rs` `nearest_enclosing_loop`/`in_loop_header`); destructured parameter names resolve
  (`infer.rs` `pattern_name`/`leftmost_identifier`); identifier-less destructuring params (`{}`, `[]`)
  still count (`collect_params`); legacy-octal bounds (`010`) stay unknown (`parse_js_int`); single-file
  JS/TS skip policy and scan-root entry confinement (`scanner/mod.rs`); stale comments corrected.
  Locked by 7 new backend unit tests (nested arrow in loop, `if` in loop, arrow in `for` header,
  destructured params, identifier-less destructured params, legacy octal, comments between params).
- `docs/architecture/js-ts.md` — status header (B2-W1 implemented / B2-W2 next); §4.2 backend exists;
  §4.4 resolver + JS lane; §4.6 three additive IR facts (`AllocEvent`, `UnaryEvent`, `entry_declared`) with
  no schema/DTO change; §7 implemented skips; §9 implemented resolver + entry ladder and no `"type"`
  analysis; §10 implemented cost rules; §12 `R-tb-004`/`R-tb-013` mitigated + new `R-tb-015`.
- `docs/specs/js-ts.md` — status B2-W1 implemented / full DoD green / round-2 pending; additive
  IR-internal facts; `js`-precedes-v1.8 disclosure; W1 test-inventory pointer.
- `docs/tracks/js-ts.md` — B2-W1 close-out; start-tab pointer to B2-W2.
- `docs/tracks/README.md` — Track B start-tab pointer moved to B2-W2.
- `state/tracks/js-ts/session-state.json` — phase/decisions/risks/next actions (artifacts are cold per
  STATE-SCHEMA §1.1; `D-tb-016` added; 7874 bytes).
- `state/archive/ledger.jsonl` — six superseded B2-W0 entries from the prior pass plus `D-tb-009`
  (superseded by `D-tb-015`) archived verbatim, and all 14 artifacts moved cold (track `js-ts`). The hot
  file keeps one-line `{"status":"archived","title":"see ledger"}` stubs so no array loses history.
- `state/tracks/js-ts/handoff.md` — this file.

## Verification (claim → evidence)

Commands run from the repo root. `cargo` is not on the non-interactive PATH here, so the absolute
`~/.cargo/bin/cargo` path is used.

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Formatting is clean | `~/.cargo/bin/cargo fmt --all -- --check` | no output | 0 |
| Lint is clean | `~/.cargo/bin/cargo clippy --locked --all-targets -- -D warnings` | `Finished`, no warnings | 0 |
| Full suite green (orchestrator gate) | `~/.cargo/bin/cargo test --locked` | 59 suites, **889 passed**, 0 failed | 0 |
| Full suite green (post-round-1-fix tree) | `~/.cargo/bin/cargo test --locked` | 59 suites, **898 passed**, 0 failed (+9 new tests) | 0 |
| Full suite green (round-2 nit tree) | `~/.cargo/bin/cargo test --locked` | 59 suites, **900 passed**, 0 failed (+2 nits) | 0 |
| Full suite green (final tree) | `~/.cargo/bin/cargo test --locked` | 59 suites, **901 passed**, 0 failed (incl. comment-extra guard) | 0 |
| Licenses clean | `~/.cargo/bin/cargo deny check licenses` | `licenses ok` (pre-existing ISC warning) | 0 |
| Semantic-feature lint | `~/.cargo/bin/cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | `Finished`, no warnings | 0 |
| Semantic-feature tests | `PERFPAL_SEMANTIC_TESTS=1 ~/.cargo/bin/cargo test --locked --features semantic-clang` | 59 suites, **892 passed** (round 1); **901** (round 2); **904** (final), 0 failed | 0 |
| Native check | `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target ~/.cargo/bin/cargo check --locked --all-targets` | `Finished` | 0 |
| JS integration suite | `~/.cargo/bin/cargo test --locked --test javascript_backend` | **20 passed**, 0 failed | 0 |
| CLI proof (`/var/tmp/pf-w1-corpus`) | pipeline run | `scan.languages == ["js"]`, 2 files, 0 findings, `<module>` `script/HIGH`, `helper` `lib-root/MEDIUM`, one resolved include edge, `trip_count_source=literal`, `known_trip_value=10`, `function_cost_v2=35` | 0 |
| Determinism | two runs | masked `metrics.json` identical; `charts/*` + bundled schema identical; `report.html`, `findings-detail.*`, `profile-queue.*` byte-identical; `report.md` differs ONLY in the provenance line (`> started … · duration … ms`), masked comparison identical | 0 |
| Skip proof | tree with `node_modules`/`dist`/`vendor` | only `vendor/v.js` scanned | 0 |
| Single-file proof | `.../src/util.js` | 1 file, `["js"]` | 0 |
| Guard: no schema/reporter edits | `git status --short src/schema docs/schema src/reporter` | empty | 0 |
| Guard: determinism config | `! grep -q preserve_order Cargo.lock` | true | 0 |
| Guard: no dependency drift | `git diff --name-only -- Cargo.toml Cargo.lock` | empty | 0 |
| Guard: hot state budget | `jq -e . state/tracks/js-ts/session-state.json && wc -c < state/tracks/js-ts/session-state.json` | valid JSON; **8144 bytes** (≤ 8192) | 0 |
| Guard: whitespace | `git diff --check` | clean | 0 |
| Panel | round 2 + final | **R1 APPROVED · R2 APPROVED · Q2 APPROVED · R3 APPROVED**, zero blocking findings; final R3 confirmation APPROVED after the comment-extra fix | — |

**Gate tooling note:** `scripts/gates/task-gate.sh` does not exist in this repo, so the equivalent
commands were run directly (L4 claim→evidence is still satisfied by the commands above).

**Full transcript:** `docs/evidence/js-w1/gate.txt` (round-1/2/3 commands, exit codes, CLI JSON,
determinism/skip/single-file proofs, panel verdicts).

## Decisions (live)

- `D-tb-013` — additive IR-internal facts: `AllocEvent` (`new`/`object`/`array` + sorted
  `referenced_names`), `UnaryEvent` (`delete`), `FunctionRecord.entry_declared`. All serde-defaulted;
  **no schema/DTO change** (wire surface stays entry/reachability + existing instrument fields).
- `D-tb-014` — entry design: scanner reads `package.json` `main` (string) / `bin` (string, object, array)
  and in-source `require.main === module` / `import.meta.url` guards → `entry_declared`; aggregator ladder
  `script`/HIGH → conventional `script`/MEDIUM → `lib-root`/MEDIUM → `unknown-entry`/LOW. No
  `package.json` `"type"` analysis.
- `D-tb-015` — discovery/resolver/instrument implemented: global directory skips, JS-only minified/size
  skips, relative specifier resolver (bare = external, internal miss = `to: null`), JS alloc-family and
  `throw` error-path arms; `#private` survives idiom normalization.
- `D-tb-016` — **single-file policy + class limitation:** a single-file `*.min.*`/`.bundle.js` or an
  unsupported TS/declaration suffix (`.ts`, `.mts`, `.cts`, `.tsx`, `.d.ts`, `.d.mts`, `.d.cts`) is skipped
  silently; a genuinely unknown extension keeps the C fallback. Non-arrow class field initializers and
  `class_static_block` bodies are not event-scanned (only arrow-function fields are their own records).
- Archived to ledger: `D-tb-005`, `D-tb-006`, `D-tb-008`, `D-tb-009` (superseded by `D-tb-015`), `D-tb-012`
  (stubs remain in hot state); all 14 `artifacts` moved cold per STATE-SCHEMA §1.1.

## Risks (open / changed)

- `R-tb-015` (medium, **new**) — **schema staging**: B2-W1 emits `files[].language == "js"` while the
  bundled artifact is v1.7 (`c`/`rust`/`python`) and there is no in-repo validator. W1 is deliberately
  **not shippable**; v1.8 lands in B2-W2 before any release.
- `R-tb-004` — **mitigated**: global directory skips + JS/TS-only name/size cap, integration-locked.
- `R-tb-013` — **mitigated**: malformed/unreadable `package.json` is ignored, the conventional/in-source
  ladder still runs, exit 0 (integration-locked).
- `R-tb-001` / `R-tb-005` — were already mitigated; records archived to the ledger.
- Still open: `R-tb-002`, `R-tb-003`, `R-tb-006`…`R-tb-012`, `R-tb-014` (unchanged).

## Blockers (unchanged)

- `B-tb-001` — `views.rs` / reporter `mod.rs` language wiring is A-owned until A step 12. Open; not touched.
- `B-tb-002` — merge to `main` requires A step 12 + explicit founder approval. Open.
- `B-tb-003` — `HEURISTIC_CATALOG_V1` entries must land via Track A, or the founder approves a
  catalog-only B edit; gates **B2-W2** and must land atomically with the registry (count/equality locks).

## Deferred / follow-ups

- `is_innermost_event` re-walks the loop body per event (quadratic in body size) — same shape as the
  Python backend, but a real large-body cliff; revisit together in a shared performance pass.
- Deep-recursion stack safety is shared with the C/Python/Rust backends; the ~2 MiB input cap bounds the
  practical depth today. A shared iterative-walk hardening is deferred.
- A single skipped single-file target produces an empty report whose `scan.languages` is the four-value
  fallback; add a usage hint in B2-W2 rather than a schema special case.
- The TS-first probe-order test lands with B3, when TS targets actually enter the scanned set.

## Next actions

1. `A-tb-003` — **B2-W2 (first)**: resolve `B-tb-003`, add the 11 `PERF-JS-*` rules, schema v1.8 + the
   version-pin sweep, IR fixtures, `examples-js`, `report-js` golden, coverage/precision/benchmark, and the
   `tests/determinism_js.rs` double-run suite.
2. `A-tb-004` — **B3**: TS/TSX backend, six-language supported set, `examples-ts`, `report-ts`, TS
   negatives; no schema bump.
3. `A-tb-005` — **B4**: docs/contract updates, CHANGELOG note for A.
4. `A-tb-006` — gate: rebase after A step 12, full DoD, parallel review panel, founder merge approval.

## Uncommitted files

- `src/`: `src/ir/mod.rs`, `src/scanner/mod.rs`, `src/scanner/instrument.rs`,
  `src/scanner/backends/mod.rs`, new `src/scanner/backends/tree_sitter_javascript/` (4 files:
  `mod.rs`, `infer.rs`, `scan.rs`, `tests.rs`), `src/scanner/backends/tree_sitter_c/mod.rs`,
  `src/scanner/backends/tree_sitter_python/mod.rs`, `src/scanner/backends/tree_sitter_rust/mod.rs`,
  `src/aggregator/mod.rs`, `src/analyzer/mod.rs`, `src/analyzer/heuristics/python/mod.rs`.
- `tests/`: new `tests/javascript_backend.rs`, `tests/common/mod.rs`, `tests/analyzer_registry.rs`,
  `tests/schema_1_4.rs`, `tests/exit_code_matrix.rs`.
- Docs: `docs/architecture/js-ts.md`, `docs/specs/js-ts.md`, `docs/tracks/js-ts.md`,
  `docs/tracks/README.md`.
- Evidence: new `docs/evidence/js-w1/gate.txt`.
- State: `state/tracks/js-ts/session-state.json`, `state/tracks/js-ts/handoff.md`,
  `state/archive/ledger.jsonl`.
- No `.playwright-mcp/` files. Commit only with founder approval.

**Resume in a fresh session:** plan `js-ts`, next task **B2-W2** (rules + schema v1.8), branch
`feat/js-ts`, base `main @ 189df91`, session HEAD `2166c8f`; W0 commit `6a3b7b1`.
