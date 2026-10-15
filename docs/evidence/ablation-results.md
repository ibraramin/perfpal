# Ablation pilot — harness shakedown, N=2 (plan `divergence-revamp`)

> **NOT a quotable delta.** This pilot shakes down the harness
> (`scripts/run-ablation.sh`) and the protocol (`docs/evidence/ablation-design.md`).
> N=2, unpaired, single model (the implementing agent). No inferential statistic is
> reported or implied. Full runs (N=26 paired) are PENDING.

**Model identity:** Muse Spark (`muse-spark-1.3-contributor`), playing the model by
hand-editing temp copies. Temperature N/A (human-driven); attempt budget 3, both runs
finished in 1 attempt. Perfpal `0.2.0-beta.1` via `cargo run --locked --` (same commit
as the pilot; frozen-binary pin recorded in `ablation-tasks.json` at campaign time).

**Pairing note (deviation from the pre-reg):** the two runs used *different tasks*
(ABL-006 WITH, ABL-021 WITHOUT) to avoid cross-condition leakage within one agent
session. Same-task pairing with fresh context applies to the full runs.

## Pilot table

| Task | Condition | Attempts | Outcome | Rescan result |
|---|---|---|---|---|
| ABL-006 `PERF-C-ALLOC-001` `c/alloc.c:10` (malloc in loop; scan also fired co-located `PERF-C-ALLOC-002` `:13`) | WITH (finding subset + `graph.dot` + report §4–5 excerpts) | 1 | **success** | `0 findings (0 hot, 0 warn, 0 info)`; `gcc -Wall -Wextra -c` clean |
| ABL-021 `PERF-PY-ALLOC-001` `python/alloc.py:7` (`dict()` in loop) | WITHOUT (bare source + `PERF-PY-ALLOC-001: avoid per-iteration object construction inside loops.`) | 1 | **success** | `0 findings (0 hot, 0 warn, 0 info)`; `ast.parse` clean |

No failures observed — the pilot therefore does **not** exercise the failure-reporting
path (the harness's `failure` outcome was verified separately on an unfixed tree;
see Repro §4).

## Fixes applied (temp copies only — repo fixtures untouched)

- **ABL-006 (WITH):** replaced per-iteration `malloc(16)`/`free(p)` with a single
  zero-initialised stack buffer `char buf[16]` reused across iterations. This cured
  both the target (`ALLOC-001 :10`) and the co-located `ALLOC-002 :13` in one edit.
  Semantics note: the original read uninitialised `p[0]` per iteration; the fix reads
  a zeroed buffer — recorded under the design's `semantics_changed`-by-inspection
  convention (no unit tests exist for fixtures).
- **ABL-021 (WITHOUT):** hoisted `buf = dict()` above the loop; loop body now calls
  `buf.clear()`, with `del buf` retained after the loop.

## Repro commands

```bash
# 1. Packets + integrity + scoring path, CI-safe (negative control: target present)
scripts/run-ablation.sh --mode dry-run --task ABL-006,ABL-021 --out /tmp/abl-dry
# 2. Emit manual packets for a real run
scripts/run-ablation.sh --mode manual --task ABL-015 --out /tmp/abl-man
#    ... hand-edit the printed work tree copy, then:
scripts/run-ablation.sh --mode score --workdir <worktree> --task ABL-015 --out /tmp/abl-man
# 3. Syntax
bash -n scripts/run-ablation.sh
# 4. Vacuity negative test: empty the source in a temp copy, then score —
#    must refuse/fail, never succeed (criterion 4)
```

Observed harness outputs (pasted):

```text
dry-run OK: target present (total=2), scoring path executes (new_fp=0, parse_ok=true, non_vacuous=true)   # ABL-006
dry-run OK: target present (total=1), scoring path executes (new_fp=0, parse_ok=true, non_vacuous=true)   # ABL-021
score: target_gone=true new_scoped_fp=0 parse_ok=true total=0 non_vacuous=true -> success   # both pilot fixes
score: target_gone=false new_scoped_fp=0 parse_ok=true total=2 non_vacuous=true -> failure  # unfixed-tree negative control
score: target_gone=true new_scoped_fp=0 parse_ok=true total=0 non_vacuous=false -> failure  # EMPTIED-tree negative control (vacuity guard)
score: target_gone=true new_scoped_fp=0 parse_ok=true total=0 non_vacuous=false -> failure  # COMMENT-ONLY-tree negative control (zero functions, same guard branch; run 2026-10-16)
```

Pilot rescans (independent of the harness, run during the fix loop):

```text
scanned 1 files (1 ok, 0 partial, 0 failed) · 0 findings (0 hot, 0 warn, 0 info)  # fixA (C)
scanned 1 files (1 ok, 0 partial, 0 failed) · 0 findings (0 hot, 0 warn, 0 info)  # fixB (Python)
C-COMPILE-OK / PY-PARSE-OK
```

## Honest notes

1. **Too easy.** Both targets are single-statement hoists the WITH message text
   ("hoisting or buffer reuse") nearly dictates; the WITHOUT one-liner was equally
   sufficient. The pilot says nothing about harder families (arch coupling, cfg-gated
   code) — stratification in the full runs exists precisely for this.
2. **Confound observed: fixture comments leak.** The WITHOUT source contains
   `/* TP PERF-C-ALLOC-001 */` comments naming the rule and the anchor ("the finding
   anchors at the call"). The one-line prompt adds little over the leak. Recorded as a
   limitation: full-run WITHOUT deltas are conservative (leak attenuates the delta).
3. **Scorer bug caught by the pilot:** findings carry the path in top-level `path`,
   not `file` — the first dry-run over-counted `new_fp` (2 on a known-clean tree).
   Fixed; dry-run re-green with `new_fp=0`. This is exactly what the shakedown was for.
4. **Non-vacuity guard (added post-pilot; pilot table stands).** The pilot fixes
   were genuine hoists — both fixed trees still contain the file, still parse,
   and still define functions — so both score `non_vacuous=true` under the new
   design §5 criterion 4 and the pilot successes above are unaffected. The guard
   was verified against the pilot trees: genuine fix → `success`, unfixed tree →
   `failure` (target present), emptied tree (`: >` the source) → `failure` via
   `non_vacuous=false` (plus an outright harness refusal to score empty sources).
5. **`results.jsonl` no longer clobbers across invocations.** The pilot-era
   harness truncated the file on every run (`: >`), so separately-scored tasks
   discarded prior rows. The harness now appends/merges by default (one row per
   task+condition, upsert on re-run of the same pair); pass `--fresh` to
   truncate for a new campaign.
6. **Observed (pre-existing, out of pilot scope): single-file materialization
   drops ARCH-003 targets.** `--mode dry-run --task ABL-015` fails with
   "target already gone pre-fix": `c/arch.c` scanned alone yields 0 findings
   because `ARCH-003` (cross-module coupling) needs multi-file context — the
   label fired on the full-corpus scan. The assert is unchanged pilot-era
   behavior, not a regression from the vacuity guard (the failure fires before
   the guard is consulted). Full runs must materialize the coupling context
   for the three ARCH-003 tasks (ABL-003/015/023) — e.g. scan the whole corpus
   dir or the task's import closure — otherwise their WITH/WITHOUT pairing is
   meaningless (target trivially absent). Recorded for the campaign, not fixed
   here.
5. **No model API was used** — `--dry-run` + hand-edits cover CI and the pilot; the
   documented manual/model mode awaits a networked campaign.
