# Handoff — Track B / Session B1 (JS/TS Phase 0 planning)

**Updated:** `2026-10-24`
**Repo:** `/home/shafin/Opencode/Perfpal-js-ts` (branch `feat/js-ts`, rebased onto `main` @ `3a84a0e`)
**Session:** B1 — Phase 0 spec + plan + ADR + inventory (plan `js-ts`)
**Verdicts:** round 1 — R1 CHANGES_REQUESTED · R2 CHANGES_REQUESTED · Q1 **APPROVED** · Q2 CHANGES_REQUESTED. After fixes — R1 **APPROVED** · R2 **APPROVED** · Q2 **APPROVED** · fresh R3 **APPROVED**. Zero unresolved blocking findings.

## Objective

Produce the JS/TS backend specification, execution plan, ADR, Phase 0 inventory and track handoff —
**planning only, zero production code** — for a JS-first then TS/TSX backend in the existing Rust core.

## Phase / plan

- Plan: `js-ts` (status active) · track B · B1 complete, B2 next
- Previous milestone: `3a84a0e` (track briefs checkpoint on `main`); implementation base `821b984`.
  **`main` advanced to `189df91` (A v2 step 11) during B1** — expected parallel-track behavior, not drift;
  B2 must rebase first (already in the plan's gate step).
- North-star: Track B is a **later release, non-blocking**; B1 touched no Atlas surface and no main state.

## Verification (claim → evidence)

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| All B1 deliverables exist and are non-empty | `test -s docs/specs/js-ts.md && test -s docs/plans/js-ts.md && test -s docs/adr/ADR-0011-js-ts-backend.md && test -s docs/architecture/js-ts.md && test -s state/tracks/js-ts/session-state.json && test -s state/tracks/js-ts/handoff.md && test -s docs/tracks/js-ts.md && echo FILES_OK` | `FILES_OK` | 0 |
| Track state is valid JSON and within the 8 KB hot-state budget | `jq -e . state/tracks/js-ts/session-state.json >/dev/null && wc -c < state/tracks/js-ts/session-state.json` | valid JSON; `8175` bytes | 0 |
| No production code touched (committed or uncommitted) | `git diff --quiet HEAD -- src Cargo.toml Cargo.lock deny.toml tests && git diff --quiet 821b984 HEAD -- src Cargo.toml Cargo.lock deny.toml tests` | both empty; exit 0 | 0 |
| Formatting gate green on the untouched tree | `~/.cargo/bin/cargo fmt --all -- --check` | no output; `fmt_exit=0` | 0 |
| Plan links all four repo specs plus the feature spec | `grep -A8 "Required Specs" docs/plans/js-ts.md` | `divergence-study`, `evidence-beta-spec`, `maintainer-map`, `perfpal-core-spec`, `js-ts` | 0 |
| Track docs present after rebase | `git log --oneline -1` ; `ls docs/tracks/` | `3a84a0e`; 5 track files | 0 |
| Deliverable sizes | `wc -c …` | spec 19,303 · plan 9,334 · ADR 10,677 · inventory 24,606 · state 8,175 | 0 |
| No untracked production code | `git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock deny.toml` | empty; `NO_UNTRACKED_CODE_OK` | 0 |
| No A-owned surface or main-state diff | `git diff --quiet 3a84a0e -- 'src/reporter/atlas*' src/reporter/views.rs src/reporter/mod.rs 'docs/specs/atlas-*' tests/atlas_fixtures state/session-state.json state/handoff/latest.md` | empty; `NO_A_SURFACE_DIFF_OK` | 0 |
| Base is an ancestor of HEAD | `git merge-base --is-ancestor 821b984 HEAD` | `BASE_ANCESTOR_OK` | 0 |
| Branch tip vs moving `main` | `git rev-parse --short HEAD main` | `9c3b0b4` / `189df91` — main advanced (A step 11); B2 rebases | 0 |
| Track-state counts | `jq -c '{decisions:(.decisions\|length),risks:(.open_risks\|length),blockers:(.blocked_by\|length),next:(.next_actions\|length),artifacts:(.artifacts\|length)}' state/tracks/js-ts/session-state.json` | `{"decisions":11,"risks":13,"blockers":3,"next":6,"artifacts":7}` | 0 |
| Hot-state budget | `test "$(wc -c < state/tracks/js-ts/session-state.json)" -le 8192` | 8,175 bytes; `STATE_BUDGET_OK` | 0 |
| B1 commit contains no code | `git show --stat --oneline 9c3b0b4` | 7 files: 5 docs + 2 state; zero `src/`, `Cargo*`, `deny.toml`, `tests/` paths | 0 |

Non-runnable evidence (external verification, fetched during B1):

- `https://raw.githubusercontent.com/tree-sitter/tree-sitter-javascript/master/bindings/rust/build.rs` — compiles `parser.c`/`scanner.c` with `cc`, `.std("c11")`; **no C++ requirement** (corrects the librarian's initial C++ claim).
- `https://raw.githubusercontent.com/tree-sitter/tree-sitter-typescript/master/bindings/rust/build.rs` — compiles four C files (`typescript/{parser,scanner}.c`, `tsx/{parser,scanner}.c`) with `-std=c11`; no C++.
- `https://github.com/tree-sitter/tree-sitter-javascript` (MIT, ABI 15, JSX embedded, `LANGUAGE` const) and `https://github.com/tree-sitter/tree-sitter-typescript` (MIT, ABI 14, `LANGUAGE_TYPESCRIPT` + `LANGUAGE_TSX`).
- `https://crates.io/crates/oxc_parser` — latest 0.154.x declares **MSRV 1.97.0** (repo floor 1.92) → rejected.
- `https://crates.io/crates/swc_ecma_parser` — Apache-2.0, MSRV unpublished → rejected on unverified MSRV + major-version churn.

## Deliverables (changed / added)

- `docs/specs/js-ts.md` — feature spec: parser/licence, detection, schema v1.8, 12-rule candidate catalog, cost model, resolution, tests, CLI, acceptance criteria, limitations.
- `docs/plans/js-ts.md` — B2/B3/B4 execution plan (6 steps, runnable gate), linked to all four repo specs + the feature spec.
- `docs/adr/ADR-0011-js-ts-backend.md` — parser pins/ABI/licence decision, grammar-boundary tokens, single v1.8 bump, shared analyzer family, discovery policy, rejected alternatives, reversal triggers.
- `docs/architecture/js-ts.md` — Phase 0 inventory with every touchpoint (`file:line`), node-kind→IR draft, rule disposition table, cost model, risks/limitations, B1 DoD.
- `state/tracks/js-ts/session-state.json` — track state (11 decisions, 13 risks, 3 blockers, 6 next actions, 7 artifacts).
- `state/tracks/js-ts/handoff.md` — this file.
- `docs/tracks/js-ts.md` — B1 close-out line only (B-owned file).

## Decisions (live)

- `D-tb-001` — tree-sitter-javascript `=0.25.0` + tree-sitter-typescript `=0.23.2` (ADR-0011).
- `D-tb-002` — tokens `js`/`ts`/`tsx` on grammar boundaries; `.jsx`→`js`; `.d.ts` skipped.
- `D-tb-003` — one MINOR schema bump v1.7 → v1.8 in B2 declaring all three tokens.
- `D-tb-004` — shared `PERF-JS-*` family; `PERF-TS-*` reserved for TS-only rules (none in v1).
- `D-tb-005` — 12-rule JS candidate catalog to be frozen by a B2-W0 canary.
- `D-tb-006` — vendored/minified JS skipped (corrected by `D-tb-009`: global directories, JS/TS-only files).
- `D-tb-007` — `syntactic` tier only; no semantic claim.
- `D-tb-008` — B1 planning-only scope.
- `D-tb-009` — discovery correction: directory skips are **global**; file-level skips are JS/TS-only.
- `D-tb-010` — `SUPPORTED_LANGUAGES` staged: four values in B2, six in B3.
- `D-tb-011` — reporter catalog is A-owned; B2 is blocked until entries land (`B-tb-003`).

Full text: `state/tracks/js-ts/session-state.json`.

## Risks (open)

- `R-tb-001` (medium) — grammar crates declare no `rust-version`; compile on 1.92 in B2-W0; fallback JS pin `0.23.1`.
- `R-tb-002` (medium) — TS grammar is ABI 14 vs JS 15; still inside the 13–15 window.
- `R-tb-003` (medium) — TS/TSX node-name drift; pin + in-repo queries + parser snapshot tests.
- `R-tb-004` (high) — vendored/minified JS noise; mitigated by the skip set + size cap.
- `R-tb-005` (high) — precision ratchet on JS idioms; mitigated by canary + 12-rule ceiling.
- `R-tb-006` (low) — no type information; limitations published.
- `R-tb-007` (low) — build time/binary size +2 C parsers; accepted.
- `R-tb-008` (medium) — global directory skips change C/Rust/Python discovery where those dirs exist; corpora contain none.
- `R-tb-009` (high) — `HEURISTIC_CATALOG_V1` is A-owned but required; blocker `B-tb-003`.
- `R-tb-010` (medium) — TSX vs generic-arrow ambiguity degrades parse health; snapshot tests + honest degradation.
- `R-tb-011` (medium) — CJS/ESM kind not inferred; documented; candidates sorted/deduped.
- `R-tb-012` (low) — `.d.ts` hides type-only modules; unlabeled minified <2 MiB parsed; extensions case-sensitive.
- `R-tb-013` (medium) — malformed `package.json` must be per-file data, never exit 3.

## Blockers

- `B-tb-001` — `views.rs` / reporter `mod.rs` language wiring is A-owned until A step 12.
- `B-tb-002` — merge to `main` requires A step 12 + explicit founder approval.
- `B-tb-003` — `HEURISTIC_CATALOG_V1` entries (`src/reporter/mod.rs`) must land via Track A before B2, or the founder must approve a catalog-only B edit; B2 cannot ship a rule without them.

## Next actions

1. `A-tb-001` — B2-W0: grammar pins, 1.92 compile check, `cargo deny`, JS + TS grammar-link tests, canary, freeze the catalog.
2. `A-tb-002` — B2-W1: JS backend + scanner wiring + discovery policy + relative import resolver + entry detection + instrument arms + cost-model loop/trip extraction.
3. `A-tb-003` — B2-W2: `PERF-JS-*` rules + schema v1.8 + fixtures/goldens/coverage/precision/benchmark.
4. `A-tb-004` — B3: TS/TSX backend + `examples-ts` + `report-ts` golden + TS negatives; no schema bump.
5. `A-tb-005` — B4: docs/contract updates + CHANGELOG note for A.
6. `A-tb-006` — gate + parallel review panel + rebase after A step 12 + founder merge approval.

## Uncommitted files

```
(clean — B1 artifacts committed as 9c3b0b4; this handoff-hash update is the only follow-up)
```

## Commits (this session)

- `9c3b0b4` — docs(js-ts): B1 Phase 0 spec, plan, ADR-0011, inventory + track state.

**Resume in a fresh session:** plan `js-ts`, next task **B2-W0** (grammar pins + canary), commit `9c3b0b4`.
