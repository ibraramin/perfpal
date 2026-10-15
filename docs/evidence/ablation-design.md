# Tool ablation — pre-registered protocol (plan `divergence-revamp`, paper § Tool ablation)

> **One-line parent spec** (`docs/specs/divergence-paper.md:29-30`): agent fix-rate
> with vs without JSON+arch-map on an AI-heavy sample, served by the same IR v1.4 build.
> This document pre-registers the full protocol. The Phase-1 pilot
> (`docs/evidence/ablation-results.md`, N=2) is a **harness shakedown only** and
> produces **no quotable delta**.

**Status:** design pre-registered; full runs PENDING. Frozen-binary + task list versions
are recorded in `docs/evidence/ablation-tasks.json`.

---

## 1. Question

Does Perfpal output (finding subset + arch-map) causally raise an LLM agent's
fix-rate on known performance defects, versus the same agent given only the bare
source plus a one-line rule prompt?

## 2. Task sample (N≈24–40 full runs)

- **Source:** the committed labeled corpus `tests/fixtures/precision/` — every task
  is one `fire` label (`rule_id` + `path` + `line`) from `labels.json`, i.e. a
  **known true positive** the current binary fires on.
- **Size:** N=26 tasks (one per `fire` label; the corpus holds exactly 26), which
  lands inside the 24–40 window. The versioned list is
  `docs/evidence/ablation-tasks.json` (`version`, `perfpal_version`,
  `labels_sha`, per-task `rule_id/path/line/language/family`).
- **Stratification:** language (C / Rust / Python) × family
  (`loop` / `alloc` / `arch` / `mem` / `str`, taken from the rule-id middle
  token). Primary analysis cells are the 3×3 loop/alloc/arch core; `mem`/`str`
  tasks are reported as secondary strata, not dropped.
- **Never-lowered convention (mirrors `baseline.json`):** the task list is
  append/version-only. A task may be *retired with a recorded reason* (e.g. the
  rule itself was removed, as `PERF-RS-STR-001` was) but its row is never edited
  to make results look better; `version` bumps on any change.

## 3. Design: paired within-task

- **Same task × both conditions.** Each task is attempted once WITH and once
  WITHOUT Perfpal artifacts (2 runs per task, 52 runs at N=26).
- **Randomized order** per task (coin flip recorded in the results JSONL).
- **Fresh context** per run: a new agent session (or explicit context reset) so
  the WITHOUT run cannot read the WITH run's artifacts and vice versa. The
  pilot used *different tasks per condition* instead (documented in
  `ablation-results.md`) — the full runs use same-task pairing.
- **Attempt budget:** 1–3 attempts per run (agent may iterate on rescan output
  within its own condition). Attempts are counted and reported; the primary
  outcome uses the full budget.

### 3.1 Campaign procedure (results accumulation)

- The full design needs **52 paired rows** (26 tasks × 2 conditions), scored
  across separate invocations (one `--mode score` per fixed tree). The results
  file therefore **appends/merges by default**: each invocation upserts one row
  per task+condition (re-running the same task+condition replaces its prior
  row; every other task's rows are preserved).
- Pass `--fresh` to truncate `results.jsonl` first (fresh campaign start).
- Each row records `non_vacuous` alongside the three classic criteria, so the
  vacuity guard (§5 criterion 4) is auditable per run.

## 4. Conditions (prompt delta = artifact presence ONLY)

**WITH** — the agent receives, for the target file only:

1. the finding subset for the target rule (`rule_id`, message, span, evidence,
   confidence/severity) extracted from `metrics.json`;
2. `charts/graph.dot` (deterministic DOT export, always emitted — see
   `src/charts/dot.rs:20-34`);
3. `report.md` §4 (findings) + §5 (architecture) excerpts covering the target —
   the §4 excerpt keeps target-rule blocks only, the §5 excerpt keeps
   target-file blocks only (ranking-table structure + lines naming the target
   file or its parent dir; chart index links are dropped because the full
   `graph.dot` already ships as item 2).

**WITHOUT** — the agent receives the bare source plus a one-line rule prompt
naming the same rule (e.g. `PERF-C-ALLOC-001: avoid per-iteration heap
allocation inside loops`) with **no location hint beyond what the compiler
gives** (file compiles/parses; compiler warnings, if any, are allowed — they are
available in both conditions). Note the WITHOUT prompt **names the rule** (same
`rule_id` as the WITH arm): this can only help the WITHOUT arm, i.e. it biases
the comparison toward the null — a conservative choice that attenuates the
measured delta, never inflates it.

Everything else is identical: same model, same harness, same attempt budget,
same success criterion.

## 5. Outcomes

**Primary — success per run** (all four must hold):

1. **rescan-clean:** no finding with `(rule_id, path, span.start_line)` matching
   the target label on rescan (TP-matching rule copied from
   `docs/evidence/benchmark-suite.md` §3: intersection on
   `rule_id+path+start_line`, duplicates not double-counted);
2. **no new scoped FP:** no finding of any task-scoped rule without a matching
   `fire` label (same §3 FP definition);
3. **compiles/parses:** rescan reports the file `ok` (zero partial/failed); C
   fixes additionally compile under `gcc -Wall -Wextra -c`, Python/Rust fixes
   parse under their toolchain;
4. **non-vacuous:** the target file must still exist in the rescan (`files[]`,
   matched by `FileEntry.path` with the finding-path fallback), still parse
   `ok`, and still define ≥1 function. This closes the degenerate-fix loophole:
   deleting or emptying the single-file work tree yields 0 findings and parses
   trivially, which criteria 1–3 alone would score as success. An
   emptied/missing/zero-function tree scores **failure** (the harness also
   refuses to score an empty source file outright). Verified: both genuine pilot
   fixes score `non_vacuous=true`, while an emptied-tree negative control
   scores failure (see `ablation-results.md` Repro §4).

**Secondary:** `tries-to-fix` rate (agent produces an edit that parses, whether
or not it cures the finding) and FP-introduced rate per condition.

## 6. Pre-registered confounds

| # | Confound | Control |
|---|---|---|
| 1 | Model + version | recorded per run (`model` field); all runs in one campaign share it |
| 2 | Temperature | 0 (or the provider minimum) for every model call |
| 3 | Attempt budget | fixed 1–3, identical both arms |
| 4 | Perfpal binary drift | frozen binary for the campaign (`cargo run --locked --` at a pinned commit, or `$PERFPAL_BIN`); version recorded |
| 5 | Prompt delta | artifact presence ONLY — same task text, same rule name, same budget |
| 6 | Task leakage | fixture templates are public in-repo; models may have seen Perfpal fixtures. Mitigation: fresh context per run, and the leak (if any) applies symmetrically to both arms — it attenuates, never inflates, the delta |
| 7 | Order effects | randomized WITH/WITHOUT order per task, recorded |

## 7. Statistics (full runs; NOT the pilot)

Paired within-task success (McNemar or conditional logistic on the 2×2
task table), plus absolute fix-rate delta with bootstrap CIs. The pilot (N=2,
unpaired) reports **no inferential statistic** — counts only.

## 8. Honest limits

- **Fixtures lack unit tests.** Success criterion 3 is a parse/compile check
  only; behavioural equivalence of the "fix" is judged by inspection and
  recorded — a fix that changes semantics is scored as a fix with a
  `semantics_changed` flag, not silently accepted.
- **Synthetic snippets vs real code.** Single-file planted TPs understate
  real-world navigation cost, which is exactly where the arch-map should help;
  the measured delta (if any) is therefore plausibly a *lower bound* of the
  real-code effect — disclosed, not claimed.
- **Single-model campaigns.** Each campaign binds one model+version; cross-model
  generalisation requires re-running, not re-interpreting.
- **Scorer scope.** "No new scoped FP" is scoped to task-listed rules; a fix
  that introduces a finding from an *unlisted* rule is flagged in notes but does
  not flip the outcome.
