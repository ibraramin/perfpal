# Handoff — Track B / Session B2-W0 (grammar pins + canary freeze)

**Updated:** `2026-10-24`
**Repo:** `/home/shafin/Opencode/Perfpal-js-ts` (branch `feat/js-ts`, rebased onto `main` @ `189df91`)
**Session:** B2-W0 — exact grammar pins, pinned-pair link tests, pinned-repo canary, catalog freeze
(plan `js-ts`; no backend/analyzer/schema code yet)
**Verdicts:** R1 **APPROVED** · R2 **APPROVED** · R3 **APPROVED** · R4 **APPROVED** · R5 **APPROVED** ·
Q1 **APPROVED** · Q2 **APPROVED**. Zero unresolved blocking findings. Round-1 Q1/Q2 findings were fixed
(evidence-doc tense, handoff inventory/commands, historical markers, `R-tb-014`, plan `N`→11, script
guards, manifest completeness, `class` node kind, status labels, stale anchors).

## Objective

Put the JS/TS parser pins on a proven footing on the Rust 1.92 floor — add the two exact grammar
dependencies, prove ABI/link/parse health with integration tests, run a reproducible frequency canary
over pinned real JS/TS repos, and freeze the 12-candidate `PERF-JS-*` catalog before any rule code.

## Phase / plan

- Plan: `js-ts` (status active) · track B · B1 done, **B2-W0 done (panel approved)**, **B2-W1 next**.
- `main` advanced to `189df91` (A v2 step 11) during B1; `feat/js-ts` was rebased onto it first this
  session (no conflicts). The rebase rewrote B1's `9c3b0b4` + `98de307` to `3adf115` + `3e414e3`.
  A's step-11 changes are present on the branch but B touched none of them.
- North-star: Track B remains a **later release, non-blocking**; W0 touched no Atlas surface, no
  `views.rs`/reporter wiring, and no main state.

## Verification (claim → evidence)

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| Branch is rebased onto current `main` | `git merge-base --is-ancestor main HEAD && git log --oneline -1 main && echo REBASE_OK` | `189df91`; `REBASE_OK` | 0 |
| Exact pins present | `grep -n "tree-sitter-javascript\|tree-sitter-typescript" Cargo.toml` | `:24 =0.25.0`, `:25 =0.23.2` | 0 |
| Lock pins the two grammars; no `preserve_order` | `grep -n 'name = "tree-sitter-javascript"\|name = "tree-sitter-typescript"' Cargo.lock && ! grep -q preserve_order Cargo.lock` | `:451`, `:487`; no match | 0 |
| Both grammars compile on the repo floor | `~/.cargo/bin/rustc --version`; `CARGO_TARGET_DIR=/var/tmp/perfpal-js-ts-target ~/.cargo/bin/cargo check --locked --all-targets` | `rustc 1.92.0`; `Finished dev … 12.33s` | 0 |
| Pinned-pair link tests pass (JS + TS + TSX) | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-a ~/.cargo/bin/cargo test --locked --test javascript_grammar_link --test typescript_grammar_link` | JS 2 passed; TS 4 passed; 0 failed | 0 |
| Full existing suite is green after the rebase + deps | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-a ~/.cargo/bin/cargo test --locked` | all suites green (the `--bogus-flag` lines are the exit-code-matrix tests' intentional child runs) | 0 |
| Licences stay copyleft-free | `~/.cargo/bin/cargo deny check licenses` | `licenses ok` (pre-existing `ISC` unmatched-allowance warning) | 0 |
| Formatting | `~/.cargo/bin/cargo fmt --all -- --check` | no output | 0 |
| Lint (all targets, examples included) | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-a ~/.cargo/bin/cargo clippy --locked --all-targets -- -D warnings` | `Finished dev …` no warnings | 0 |
| Semantic feature gate stays green | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-sem ~/.cargo/bin/cargo clippy --locked --all-targets --features semantic-clang -- -D warnings`; `PERFPAL_SEMANTIC_TESTS=1 CARGO_TARGET_DIR=/var/tmp/perfpal-w0-sem ~/.cargo/bin/cargo test --locked --features semantic-clang` | clippy `Finished dev …`; tests exit 0 | 0 |
| Canary is deterministic and reproducible | `docs/evidence/js-canary/run.sh` | clones 3 pinned tags, verifies SHAs, runs counter twice, `cmp` byte-identical; `wrote …/canary_results.json` | 0 |
| Canary artifact hashes | `sha256sum tools/js_canary.rs docs/evidence/js-canary/canary_results.json` | counter `cd69c482…9783d68`; results `7efab4fe…622a4c2` | 0 |
| Raw results are valid JSON | `jq -e . docs/evidence/js-canary/canary_results.json` | valid | 0 |
| Track state valid + within the 8 KB hot budget | `jq -e . state/tracks/js-ts/session-state.json`; `wc -c < …/session-state.json` | valid; `8086` bytes | 0 |
| No A-owned surface or main-state diff | `git diff --quiet 189df91 -- src/reporter/views.rs src/reporter/mod.rs src/reporter/atlas_brief.rs src/reporter/atlas_render.rs state/session-state.json && echo NO_A_SURFACE_DIFF_OK` | empty; `NO_A_SURFACE_DIFF_OK` | 0 |
| Frozen catalog is recorded | `grep -n "FROZEN by the B2-W0 canary\|CUT (v1)" docs/architecture/js-ts.md` | `:209`, `:228` | 0 |
| A-owned demo corpus unchanged after the `tools/` move | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-a ~/.cargo/bin/cargo run --locked --quiet -- examples -o /tmp/pf-ex2 --force && jq '{file_count: .scan.file_count, findings: (.findings\|length)}' /tmp/pf-ex2/metrics.json` | `13` files / `83` findings | 0 |

Non-runnable evidence (pinned corpus, fetched/checked into `/var/tmp/opencode/js-canary` by the driver):

| Corpus | Licence | Tag | Commit | Prod files / lines | Test files / lines | Parse failures |
|---|---|---|---|---:|---:|---:|
| express | MIT | `v5.1.0` | `cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4` | 51 / 4,883 | 91 / 16,281 | 0 |
| execa | MIT | `v9.6.1` | `84e1f36b2c31a463e3efbd37c7505da9438a05c9` | 219 / 13,224 | 329 / 19,685 | 1 |
| gemini-cli | Apache-2.0 | `v0.65.0` | `6006c14a4decfe4b891c6263ad6ad5992e404b5b` | 1,255 / 303,008 | 993 / 402,004 | 229 |
| **total** | — | — | — | **1,525 / 321,115** | **1,413 / 437,970** | **230** |

The 229 gemini-cli failures are dominated by one pinned-grammar limitation: `f<typeof import("x")>()`
(220 files). The recovered tree still yields imports/calls; it is recorded as `R-tb-014`, not hidden.

## Catalog freeze (B2-W0)

**11 ship / 1 cut / 0 deferred.** Full evidence: `docs/evidence/js-canary.md`; frozen table:
`docs/architecture/js-ts.md` §8. Production hits (control in parentheses):

| Rule | Prod hits | Control | Verdict |
|---|--:|---|---|
| `PERF-JS-ALLOC-001` | 1,021 | 21,094 outside loops | KEEP — reframed: loop-invariance gate mandatory, Info-first |
| `PERF-JS-ALLOC-003` | 16 | 32 outside loops | KEEP (Info) |
| `PERF-JS-STR-001` | 73 | 159 outside loops | KEEP (MEDIUM) |
| `PERF-JS-LOOP-001` | 165 | 900 depth-1 | KEEP (Info) |
| `PERF-JS-LOOP-002` | 112 | 759 outside loops | KEEP (MEDIUM) |
| `PERF-JS-LOOP-003` | 24 | 0 array-literal RHS | **CUT (v1)** — identifier RHS is unidentifiable without types |
| `PERF-JS-LOOP-007` | 177 | 857 outside loops | KEEP — flagship |
| `PERF-JS-ASYNC-001` | 395 (278 `for...of`) | 4,363 awaits outside loops | KEEP — flagship |
| `PERF-JS-ASYNC-002` | 270 (882 test) | 462 non-async | KEEP — flagship; async gate mandatory, test-skewed |
| `PERF-JS-CALL-001` | 108 (102 guarded) | — | KEEP (Info) — guard-gated; only 6 unguarded |
| `PERF-JS-ARCH-001` | 6 cyclic modules | 1,519 acyclic | KEEP (Info) — relative-only graph limitation published |
| `PERF-JS-TYPE-001` | 31 | 182 `.delete(...)` | KEEP (Info) |

Consequences downstream: B2-W2 adds **11** `PERF-JS-*` rules, so `HEURISTIC_CATALOG_V1`, the
count/equality locks, the IR fixtures, and `examples-js` move by **11**, not 12. `PERF-TS-*` stays at
zero for v1 (no TS-only candidate was proven).

## Deliverables (changed / added)

- `Cargo.toml` / `Cargo.lock` — exact pins for `tree-sitter-javascript =0.25.0` and
  `tree-sitter-typescript =0.23.2`; transitive closure is MIT + build-only `cc`. `Cargo.toml` also
  declares the canary as an explicit example (`[[example]] path = "tools/js_canary.rs"`).
- `tests/javascript_grammar_link.rs` — ABI-window/link/clean-parse lock for the JS grammar (JSX, ESM/CJS,
  declarations, loops, async, allocation constructs) + malformed non-vacuity.
- `tests/typescript_grammar_link.rs` — same for `LANGUAGE_TYPESCRIPT` and `LANGUAGE_TSX`; TS node-kind
  locks, TSX JSX locks, `<T,>` generic-arrow disambiguation parses cleanly, malformed non-vacuity.
- `tools/js_canary.rs` — deterministic 12-rule + 12-control counter over the pinned grammars
  (prod/test split, import-graph SCCs, seeded-zero ids, sorted JSON; no new dependencies).
  **Deviation (R4):** moved from `examples/` to `tools/` with an explicit `[[example]]` path because
  `examples/` is the A-owned C demo corpus — keeping a 1,296-line Rust file there changed its published
  counts from 13 files / 83 findings to 14 / 104. After the move the demo scan is back to 13 / 83 and the
  counter still runs as `cargo run --example js_canary`.
- `docs/evidence/js-canary/run.sh` — pin-verifying clone driver + double-run determinism proof.
- `docs/evidence/js-canary/canary_results.json` — raw per-repo/per-rule counts.
- `docs/evidence/js-canary.md` — method, corpus, counts, controls, limitations, per-rule recommendation.
- `docs/architecture/js-ts.md` — §6 node-kind confirmations; §8 **FROZEN** disposition table; §14 Q1/Q2
  resolved/partly resolved.
- `docs/specs/js-ts.md` — §6 frozen-catalog paragraph + verdict column; §9 reporter-catalog count = 11;
  `PERF-TS-*` zero for v1.
- `docs/plans/js-ts.md` — B2-W0 result recorded in the W0 bullet.
- `state/tracks/js-ts/session-state.json` — phase/decisions/risks/next actions/artifacts updated
  (D-tb-012, R-tb-014; R-tb-001/R-tb-005 mitigated; A-tb-001 done).
- `docs/tracks/js-ts.md` — B2-W0 close-out + next-session pointer.
- `docs/tracks/README.md` — Track B row + start-tab pointer moved to B2-W1 (B owns its row).
- `state/tracks/js-ts/handoff.md` — this file.

## Decisions (live)

- `D-tb-001` — pins: `tree-sitter-javascript =0.25.0` (ABI 15) + `tree-sitter-typescript =0.23.2`
  (ABI 14) on `tree-sitter =0.26.13`; MIT, C11 only. Verified on 1.92 (R-tb-001 mitigated).
- `D-tb-002` — tokens `js`/`ts`/`tsx` on grammar boundaries; `.jsx`→`js`; `.d.ts` skipped.
- `D-tb-003` — one MINOR schema bump v1.7→v1.8 in B2, all three tokens up front.
- `D-tb-004` — shared `PERF-JS-*` family; `PERF-TS-*` reserved (zero in v1; canary proved none).
- `D-tb-005` — **frozen**: 12 candidates adjudicated → 11 ship, `LOOP-003` cut.
- `D-tb-006`…`D-tb-011` — discovery/tier/schema/fallback/reporter-catalog decisions unchanged.
- `D-tb-012` — **new**: canary freeze record (11/12, gates for ALLOC-001/CALL-001/ASYNC-002; evidence
  `docs/evidence/js-canary.md`).

## Risks (open)

- `R-tb-001` (medium) — **mitigated**: both grammar crates compile on 1.92; fallback `0.23.1` not needed.
- `R-tb-002` (medium) — TS grammar ABI 14 vs JS 15; inside the 13–15 window (link tests lock it).
- `R-tb-003` (medium) — TS/TSX node-name drift; link tests + in-repo queries + B3 snapshots.
- `R-tb-004` (high) — vendored/minified noise; mitigated by the skip set + size cap (W1).
- `R-tb-005` (high) — precision ratchet; **mitigated by the freeze** (11 rules, gates, honest FP class).
- `R-tb-006`…`R-tb-013` — unchanged (syntactic tier, build size, global skips, reporter catalog,
  TSX/generic ambiguity, CJS/ESM, skipped declarations, malformed `package.json`).
- `R-tb-014` (medium, **new**) — pinned TS grammar cannot parse `f<typeof import("x")>()` (220/229
  gemini-cli parse failures); degrade honestly and upgrade only via the ADR-0001 G5 node-types review.

## Blockers

- `B-tb-001` — `views.rs` / reporter `mod.rs` language wiring is A-owned until A step 12. **Open;
  W0 did not touch those files.**
- `B-tb-002` — merge to `main` requires A step 12 + explicit founder approval. **Open.**
- `B-tb-003` — `HEURISTIC_CATALOG_V1` entries (`src/reporter/mod.rs`) must land via Track A, or the
  founder must approve a catalog-only B edit. **Open; gates B2-W2 only.** The frozen set is 11 entries,
  and catalog + registry must land atomically (count/equality locks).

## Next actions

1. `A-tb-002` — **B2-W1**: JS backend + scanner wiring + global directory skips + JS/TS-only file/size
   guard + relative import resolver + entry detection + instrument arms + cost-model loop/trip extraction.
2. `A-tb-003` — **B2-W2**: resolve `B-tb-003`, add 11 `PERF-JS-*` rules + schema v1.8 + fixtures/goldens/
   coverage/precision/benchmark; catalog and registry land together.
3. `A-tb-004` — **B3**: TS/TSX backend + `examples-ts` + `report-ts` golden + TS negatives; no schema bump.
4. `A-tb-005` — **B4**: docs/contract updates + CHANGELOG note for A.
5. `A-tb-006` — gate + parallel review panel + rebase after A step 12 + founder merge approval.

## Uncommitted files

```
M  Cargo.toml
M  Cargo.lock
M  docs/architecture/js-ts.md
M  docs/plans/js-ts.md
M  docs/specs/js-ts.md
M  docs/tracks/js-ts.md
M  docs/tracks/README.md
M  state/tracks/js-ts/session-state.json
M  state/tracks/js-ts/handoff.md
?? docs/evidence/js-canary.md
?? docs/evidence/js-canary/run.sh
?? docs/evidence/js-canary/canary_results.json
?? tools/js_canary.rs
?? tests/javascript_grammar_link.rs
?? tests/typescript_grammar_link.rs
```

## Commits (this session)

- `B2-W0` — grammar pins + link tests + canary + frozen catalog (this change; hash appended after commit).

**Resume in a fresh session:** plan `js-ts`, next task **B2-W1** (JS backend), branch `feat/js-ts`
rebased on `main` @ `189df91`, W0 commit `<pending>`.
