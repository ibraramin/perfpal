# Track C — agent report · handoff

**Track:** C (LLM depth layer) · **Branch:** `feat/agent-report` · **Worktree:** `/home/shafin/Opencode/Perfpal-agent-report`
**Session:** C2 (module + unit tests) · **Date:** 2026-10-24 · **Plan:** `docs/plans/agent-report.md` · **Spec:** `docs/specs/agent-report.md`
**Status:** **C2 COMPLETE** — `src/reporter/agent_report.rs` (views + 67 hints + JSON + MD + 24 unit tests) with visibility-only `mod.rs`; **no CLI wiring** (waits on A step 12). C1 COMPLETE (`6d32e67`, hash recorded in `e730d01`). No merge, no push. C2 uncommitted pending founder approval.

## Objective and phase

Build the C2 renderers for the agent-report artifact — the LLM-facing depth layer the simplified Atlas
first page will point to (A step 12). C2 is module-only: `src/reporter/agent_report.rs` + unit tests; no
CLI emission, no schema change, no dependency, no artifact-set change.

**North-star alignment:** C serves the pinned goal ("LLM-consumable metrics … so LLMs can optimize code
with explicit, quantified signals") and the proposed agent-loop metric ("downstream fix success with the
JSON + arch map"). Beta remains Track A; C is a later add-on and blocked nothing.

## Branch state

- `feat/agent-report` was fast-forwarded to `main` at session start (`3a84a0e`, the docs/tracks
  checkpoint commit) per the track's "rebase on `main` at session start". `main` is unchanged.
- **C1 commits:** ``6d32e67`` (docs + evidence only) + ``e730d01`` (hash record). **C2 is uncommitted**
  pending founder approval.
- Worktree (C2): `src/reporter/agent_report.rs` (new), `src/reporter/mod.rs` (visibility/module declaration
  only), `docs/specs/agent-report.md`, `docs/plans/agent-report.md`, `docs/tracks/agent-report.md`, and the
  `derive_sample.py` predicate alignment. `Cargo.*`, `deny.toml`, `docs/schema/`, `tests/golden/`,
  `docs/evidence/agent-report/sample/*.sample.*` and `src/cli/` are untouched.

## C2 — what landed

| Deliverable | Path | Notes |
|---|---|---|
| Renderer module | `src/reporter/agent_report.rs` | `render_actions_json` + `render_markdown`; reuses `views::complete_actions` / `complete_profile_queue` (single rank source, no second key); `data_quality`; §9.9 escaping; fixed seven-section MD; 25/25 caps; `MD_SIZE_BUDGET = 81,920` |
| Fix hints | `AGENT_FIX_HINTS` | all 67 spec §8 sentences verbatim; `PERF-RS-STR-001` absent; unknown `rule_id` → JSON `null` + MD stub |
| Wiring seam | `src/reporter/mod.rs` | `pub mod agent_report;` and `pub(crate)` on `HEURISTIC_CATALOG_V1`, `finding_anchor`, `finding_anchor_assignments` — visibility only, no behavior or emission change |
| Unit tests | `agent_report.rs` `#[cfg(test)]` | 24 tests: rank/profile parity, hint set-equality + hygiene, anchor byte-parity, `detail_anchor` parity, integral cost, no clock, double render, empty/semantic/all-component/capped skeleton, v1 scale, nullable/threshold shapes, 200 B no-truncation, exact size pins, text safety, forbidden claims, committed-sample byte parity |

**Founder decision at C2 kickoff (2026-10-24):** the §12 budget was raised **64 KB → 80 KB** because the
literal 200 B envelope (worst-case widths) measures **76,416 B**; the bound change keeps the width honest
instead of shrinking the fixture. `contract_version` stays 1.
**Deferred (not done here):** CLI emission, artifact-set assertions, `tests/agent_report.rs` integration
suite, sample regeneration gate, downstream eval, and A's step-12 link — all C3/wiring.

## Verification (C2)

All commands from the repo root with `PATH="$HOME/.cargo/bin:$PATH"`:

| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Unit tests | `cargo test --locked --lib agent_report` | 24 passed, 0 failed |
| Test-count gate | `test "$(cargo test --locked --lib agent_report -- --list \| grep -c ': test$')" -ge 10` | exit 0 (24) |
| Full suite | `cargo test --locked` | 55 `test result: ok` groups, exit 0 |
| Licenses | `cargo deny check licenses` | `licenses ok` |
| No schema/deps/fixtures/CLI | `test -z "$(git status --short Cargo.toml Cargo.lock deny.toml src/schema/ docs/schema/ tests/golden/ docs/evidence/ src/cli/ CHANGELOG.md)"` | exit 0 (the `derive_sample.py` predicate update is listed separately) |
| Sample parity | `agent_report_sample_matches_committed_contract` | byte-equal for both artifacts |
| Size | exact pins in `agent_report_size_budget_golden_fixtures` | report 13,469 · mixed 8,889 · python 26,889 · rust 25,848 · synthetic 76,416 B (budget 81,920) |

**Review panel (C2):** R1/R2/R3/R5 reviewer lanes **APPROVED**; Q1/Q2 requirements lanes **APPROVED**;
zero blocking findings. Round-1 findings fixed: unproven stubs/unknown-rule, v1 cost path, nullable and
threshold shapes, capped tails, zero-branch aggregates, >100 profile cap, all-`-ARCH-002` state, exact
size pins, `u64` cost serialization, table/backslash/`rule_id` escaping, the semantic-tier false claim,
the worst-case fixture (unknown-trip suffix + 3-key evidence), and the state/handoff counts.

## C1 — what landed

| Deliverable | Path | Notes |
|---|---|---|
| Contract spec | `docs/specs/agent-report.md` | 16 sections + the 67-rule fix-hint table; `contract_version 1`; frozen rank keys, envelope/action/**profile-row** fields, formatting, escaping, determinism, size envelope, state matrix, eval protocol, acceptance |
| Plan | `docs/plans/agent-report.md` | C1 DONE; C2 (module + renderers), C2 wiring (post-A-step-12), C3 (integration + evidence + eval + link); DoD below |
| Sample | `docs/evidence/agent-report/sample/{agent-report.sample.md,agent-actions.sample.json,derive_sample.py}` | complete artifacts for `tests/golden/report-mixed/metrics.json` (9 actions, 4 profile rows), reproducible |
| Gate | `docs/evidence/agent-report/c1-gate.sh` + `c1-gate.txt` | executable gate; the transcript is the script's own stdout |
| Evidence README | `docs/evidence/agent-report/README.md` | provenance + what the sample does/does not prove |
| Track brief | `docs/tracks/agent-report.md` | status block (7 lines), C owns this file |

**Ratified decisions (founder, C1 gate):** D1 bounded MD + complete JSON; D2 C-owned hint table with a
67-id **set-equality** lock; D3 no clock provenance (same-command byte-identity; `root_path` is CLI
provenance); D4 names `agent-report.md` / `agent-actions.json`; D5 rank keys = shipped KV1/KV3 (no new
weights); D6 numbers reuse shipped key-view strings; D7 JSON values verbatim, MD escaped per §9.9;
D8 the evidence-only reproducer is permitted (never shipped).

**Contract in one paragraph:** actions ranked by production-first → severity → confidence →
containing-function `function_cost_v2` desc → `finding_id` asc (`-ARCH-002` excluded and counted);
profile queue by the shipped KV3 key with its own §6.2 row schema; cost anchors verbatim from
`views::ScaleOrdering`; the MD is a fixed seven-section skeleton (top 25 action details + a ≤67-row
rollup over **all** actions + 25 profile rows). Scanner text is single-line-escaped and a backtick
becomes `ˋ` (U+02CB) because CommonMark code spans ignore backslash escapes; `<`/`>` are neutralised in
prose only. The **80 KB** assertion covers the committed golden fixtures plus a 25+67+25 synthetic
at-envelope fixture (measured 76,416 B); the external `perfpal-test` scans run only with
`PERFPAL_EXTERNAL_SCANS=1` (C3). The
JSON holds every action (uncapped), the top 100 profile rows, a data-quality block, and no clock fields.

## Verification

Command: `bash docs/evidence/agent-report/c1-gate.sh` → exit 0. Full output: `docs/evidence/agent-report/c1-gate.txt`.

| Claim | Gate check | Result |
|---|---|---|
| Deliverables exist | §1 `test -f` × 7 | 7/7 ok |
| Sample reproduces byte-identically | §2 reproducer + `diff` | both identical |
| JSON invariants (counts, ranks, hints, profile schema, no clock) | §3 `jq -e` | ok |
| Hint table = 67 rows and **set-equal** to `HEURISTIC_CATALOG_V1` | §4 Python set comparison | `spec rows=67 catalog ids=67`, sets equal |
| Worked example is a real datum | §5 spec JSON block vs sample action 1 | equal |
| Fixed skeleton | §6 exact H1/H2 list in order | 7/7, no extras |
| No production code/schema/deps | §7 `git status` | empty |
| MD size | §8 `wc -c` ≤ 65536 | 8,889 B |
| Gate exit | script exit + `GATE_EXIT=0` | 0 |

## Review panel (C1) and resolutions

Panel: R1 correctness **APPROVED**; R2 contract **CHANGES_REQUESTED** (6 major); Q1 scope
**CHANGES_REQUESTED** (1 minor + 2 observations); Q2 evidence **CHANGES_REQUESTED** (8 findings).

| Finding | Resolution |
|---|---|
| R2-B1 profile rows undefined; `function` key had two types; all-test state contradicted §7; caps ambiguous | Added §6.2 profile-row table (`function_name`, `cost`, `cost_scale`, `cost_anchor`, `unknown_trip_loops`, `known_trip_value`); profile line now prints `path_class`; `data_quality` adds `startup_functions`/`no_loop_functions`; `caps` = MD caps only, `profile_queue_cap` = JSON cap |
| R2-B2 `function.cost` v1 rule + number form unpinned | §6.1: cost = `function_value` (v2 else chart-side v1), JSON number, integral emitted without a fractional part; example `15` |
| R2-B3 no MD escaping/injection rule | New §9.9 text safety (single-line escapes, backtick closure, prose `<`/`>` neutralised, table `\|`); D7 reworded (JSON verbatim, MD escaped); C2 acceptance item 10 adds a hostile-path fixture |
| R2-B4 64 KB "any scan" contradicted no-truncation and was untestable | §12 scopes the budget to the named fixtures + a width envelope (paths ≤200 B, messages ≤500 B, evidence ≤300 B); no truncation; C2 DoD asserts it incl. a synthetic at-envelope fixture |
| R2-B5 empty-scan `loc_total`/`parse_health`/`languages` | §6/§10: `loc_total: object\|null`, `parse_health: object\|null` (+ `parse: not recorded`); the `languages: none recorded` stub added here was **superseded in round 2** (languages are verbatim; stub removed); §11 updated |
| R2-B6 `root_path` breaks the byte-identity claim | D3/§12 rescoped to **same command**; `root_path` is verbatim CLI provenance (documented, `main` untouched in tests) |
| Q1-1 `derive_sample.py` vs "no code" | Founder-ratified as evidence-only tooling (D8), never shipped; production code remains C2-only |
| Q1-2 handoff said "status line only" | Corrected to "status block (7 lines)" |
| Q1-3 gate HEAD ≠ brief base | Explained: fast-forward to `main`'s checkpoint commit; no C1 commit, no merge |
| Q2-F1 unreachable stubs (`no entry`, `no resolved dependents`, `no cost`) | Removed from the agent-report stub list; added the reachable `parse: not recorded` / `languages: none recorded` |
| Q2-F2 size criterion underspecified | §12 names the fixtures + envelope; C2 DoD gains named tests |
| Q2-F3 sample regen "or diffed" too weak; A-link ungated | §14.12 requires an **empty diff**; C2 DoD adds sample-regen + A-link tests |
| Q2-F4 eval under-defined (N=6 thresholds, hallucination, time, `bat` question) | §13 rewritten (definitions, N-aware thresholds, recording fields); `bat` locked as the third fixture |
| Q2-F5 forbidden-claim matcher undefined | §9.7 item 7: exact regex + exemption lines; reproducer implements it |
| Q2-F6 jq evidence overstated | Gate §3 now asserts rank contiguity + both clock fields; plan DoD #3 matches |
| Q2-F7 lib test filter can pass with zero tests | Plan DoD #11 asserts ≥ 10 listed tests |
| Q2-F8 hint check was count-only | Gate §4 now proves set equality against `HEURISTIC_CATALOG_V1` |
| R1 nitpicks (stale `views.rs` line, §9.3 ref, spec/sample bulk wording, README in-place reproduce, v1 reproducer) | Fixed: `views.rs:459-468`, §9.6, skeleton bullets match the sample, README uses a temp dir + diff, `derive_sample.py` **fails closed** on any v1 function |

### Round 2 (after the first fix pass)

R2 re-review: **CHANGES_REQUESTED** (1 critical, 4 major, 1 partial); Q2 re-review:
**CHANGES_REQUESTED** (C1 green, C2–C3 DoD gaps); Q1 re-review: **APPROVED**.

| Round-2 finding | Resolution |
|---|---|
| R2-N5 **critical**: `` \` `` does not escape a backtick inside a CommonMark code span — the injection defence was bypassable | §9.9 now **substitutes** every backtick with U+02CB `ˋ` (lossy by design; JSON keeps the original); the acceptance fixture asserts no span can be closed |
| R2-N1 the width envelope did not guarantee 64 KB (path rendered at several sites, title/name/rollup unbounded) | §12 stops claiming a universal bound: the hard assertions are the committed goldens + a synthetic **25 actions + 67 rollup + 25 profile** at stated widths; beyond the envelope the artifact is still correct but may exceed the target, and `agent-actions.json` is authoritative |
| R2-N2 zero-file `languages` contradiction (the producer emits the supported set) | `scan.languages` is always verbatim; the `languages: none recorded` stub is removed from §7/§10/§11 and the reproducer |
| R2-N3 §7 rollup sentence missing from the sample | Removed from the normative skeleton (the table + heading are the fixed shape) |
| R2-N4 `data_quality.caps.profile_json` did not exist | The clause is removed; `profile_queue_cap` is the sole JSON cap |
| R2 residual: plan said "double scans" without the same-command qualifier | Plan wording fixed |
| Q2-N1 named integration tests could match zero tests and exit 0 | Plan DoD #14–16 add `--list \| grep -qx '<name>: test'` presence guards; #13 asserts an integration-test count |
| Q2-N2 `grep -c` did not enforce ≥ 10 | Plan DoD #11 wraps the count in `test "…" -ge 10` |
| Q2-N3 named fixtures included user-local paths | §12: the goldens always run; the `perfpal-test` scans run only with `PERFPAL_EXTERNAL_SCANS=1` (a skip is printed, never a silent pass) |
| Q2-N4 synthetic fixture had no cardinality | §12 fixes it at 25 actions + 67 rollup rules + 25 profile rows |
| Q2-N5 duplicate §6.3 | Serialization is now §6.4 |
| Q2-N6 `seconds` forbidden by policy but absent from the regex | Added to the §9.7 predicate and the reproducer |
| Q2-N7 track block said D1–D7 | Now D1–D8 |

### Round 3 (fresh confirmation)

| Lane | Verdict | Notes |
|---|---|---|
| R4 (fresh reviewer) | **APPROVED** | Zero blocking. 5 non-blocking notes; 3 fixed (below). Verified §7 headings/cap lines vs the sample, all 19 action keys + 10 profile-row keys with declared types, stub string parity, §9.9 escaping tiers, and gate↔DoD mapping. |
| Q2 (requirements) final | **APPROVED** | All N1–N7 resolved; gate re-run green. |
| Q1 (requirements) | **APPROVED** | All C1 clauses PRESENT; no scope creep; D8 ratified. |
| R1 (correctness) | **APPROVED** | Round-1; its 2 citation/README nitpicks were fixed in round 2. |
| R2 (contract) | Blockers fixed; final read stale | Its round-2 blockers (N1–N5) are all fixed. Its last two replies cited `spec:252` "All actions are counted here…" — **`grep` proves the sentence is absent from the spec and the sample** (`sed -n '238,264p'` shows the heading → table with no sentence). The lane was answering from a pre-fix cached read; superseded by R4's fresh APPROVED. |

R4's non-blocking notes and resolutions:

| Note | Resolution |
|---|---|
| Capped-state tail paragraphs were in the reproducer but not in §7 | §7 now defines both conditional tails (`only when N > cap`) and the rules list says they are part of the fixed skeleton |
| Gate labelled fixture-specific null checks as "contract invariants"; §6 heading check ignored `###` | Gate §3 relabelled "fixture invariants"; gate §6 now also fails on any `###` that is not a `### {rank}.` block |
| README claimed the sample proves prose-HTML neutralisation | Softened: that half is code-only here; §14.10's hostile fixture is the real proof |
| `no curated hint for this rule yet` unreachable in C1 evidence | Documented in the README's "does not prove" list; §14.8 already allows a direct renderer call in C2 |
| Gate temp dir not cleaned; transcript has a timestamp | Accepted (the DoD greps `GATE_EXIT=0`; the transcript is not claimed byte-reproducible) |

## Risks / open

- **C2 is module-only:** no CLI emission, no artifact-set update, no `tests/agent_report.rs`. Those are
  C3/wiring and must land **after A step 12**; the DoD items 13–16 are not runnable yet by design.
- **`detail_anchor` parity:** C2 reuses the shipped `finding_anchor_assignments` (`pub(crate)`), and
  `agent_report_detail_anchor_matches_shipped_assignments` locks it.
- **Sample drift until C3:** the C1 sample is still produced by the evidence-only reproducer; C3 must
  regenerate it from the Rust renderer and prove an empty diff (the C2 sample-parity unit test already
  proves today's renderer matches it byte-for-byte).
- **Non-blocking panel follow-ups:** §9.7 does not yet exempt the MD header (`scan root: …`) from the
  forbidden-time scan, so a root path containing `runtime`/`ms` could false-fail the predicate; a
  `dynamic`-only `resolver_tiers` value falls into the syntactic wording (no producer emits it today);
  the synthetic fixture keeps `scan.file_count = 4` in its header (width-neutral); the C1 gate's local
  64 KB constant and the C1 README's "no production code" prose remain historical evidence.
- **Size on very large scans:** actions are uncapped in JSON (≈1.7 KB/action); accepted for a machine
  artifact.
- **Ratification open (spec §16):** profile cap 100; hint wording; eval thresholds; whether A links from
  `report.html` only; formatter consolidation; whether the synthetic becomes a golden.

## Next actions

1. Founder approval to commit C2 on `feat/agent-report`; no merge, no push.
2. **Wait for A step 12**, then C2 wiring: emit both artifacts via `Artifact::text`, update the fixed
   artifact-set assertions, and run the full DoD (items 13–20).
3. **C3:** `tests/agent_report.rs` (double-run byte equality, state matrix, `detail_anchor` resolution,
   no-script/network, emitted artifact set), regenerate the sample from Rust and byte-diff, then the
   downstream eval or the founder's recorded deferral.
4. No merge to `main`, no push without the track gate + founder approval.

## Change set (C2, uncommitted)

- **New code:** `src/reporter/agent_report.rs` (renderers + 67 hints + 24 unit tests).
- **Visibility only:** `src/reporter/mod.rs` (module declaration + three `pub(crate)` exports).
- **Docs:** `docs/specs/agent-report.md` (§7/§9/§10/§11/§12/§14 amendments incl. the 80 KB budget),
  `docs/plans/agent-report.md` (80 KB + state-matrix wording), `docs/tracks/agent-report.md` (status).
- **Evidence:** `docs/evidence/agent-report/sample/derive_sample.py` (predicate aligned with §9.7; the
  committed sample output is unchanged).
- **No** schema, `Cargo.*`, `deny.toml`, `src/cli/`, `tests/golden/`, CHANGELOG, main state, or A/D surface
  changes.

## Change set (C1 commit)

- **New docs:** `docs/specs/agent-report.md`, `docs/plans/agent-report.md`,
  `docs/evidence/agent-report/{README.md,c1-gate.sh,c1-gate.txt}`,
  `docs/evidence/agent-report/sample/{agent-report.sample.md,agent-actions.sample.json,derive_sample.py}`,
  `state/tracks/agent-report/handoff.md`.
- **Edited:** `docs/tracks/agent-report.md` (status block only).
- **No** `src/`, schema, `Cargo.*`, `deny.toml`, main state, `CHANGELOG.md`, or A/D surface changes.
