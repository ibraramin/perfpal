# Map-Intelligibility V3 — idiom A/B pre-registration

> **Written before any V3 prototype HTML or PNG was rendered or opened.** Inputs: the V2-locked
> view set (`docs/specs/map-intelligibility.md`), the V1 field decisions
> (`docs/evidence/map-data-coverage.md` §8), and the current scans
> `out-i5b-{tmux,hyperfine,cJSON}/metrics.json` (schema 1.6). No `@vision` read had been run for
> V3 when this file was committed.
>
> Date: 2026-10-21 · plan `map-intelligibility` V3 · data SHAs:
> tmux `62fd2e23…`, hyperfine `5735135b…`, cJSON `99736e9f…`.
> **Holdout untouched:** django and postgres are never rendered, referenced, or read in V3
> (reserved for V6 / A2b).

## 1. Purpose

V3 chooses the **concrete idiom inside each locked slot** (V2: slot, question, encoding,
unit/anchor rule, production-first rule, default-tier claim limit and A1b recall guarantee are
locked). This file fixes the candidates, the prototype-only data transforms, the blind-read
protocol, and the decision rule **before** the experiment runs, so the choice cannot be
reverse-fitted to the vision results.

## 2. Candidates (2 per slot × 3 repos = 18 renders)

| # | Slot | Candidate A | Candidate B |
|---|---|---|---|
| 1 | KV1 Start here (Q1) | `kv1-brief` — text-first ranked brief (5 rows) | `kv1-cards` — ranked action cards + code callout |
| 2 | KV2 Blast radius (Q2) | `kv2-bars` — horizontal dependents bars, one hue | `kv2-matrix` — dependency matrix (DSM) + column totals |
| 3 | KV3 Profile queue (Q3) | `kv3-bars` — horizontal cost bars, one hue | `kv3-treemap` — squarified treemap, area = cost |

Both candidates in a slot carry **identical header text, ranking, row selection, units, anchors,
badges, coverage/demotion lines and footer** — only the idiom differs. Page chrome mimics the
report: light theme, body max-width 1152 px, 1440×900 viewport, `device_scale_factor=1`.

Header (all candidates): kicker `Key view · <KVn> · <repo>`; the plain-language question; the
slot action line ("Open the first path:line and profile it — static candidates, confirm with a
profiler." / "Changing a file here forces re-import of its dependents — do not edit directly." /
"Profile the named function first — static candidate, confirm with a profiler."); honesty
footer "Perfpal static analysis — optimization candidates, not runtime truth."

## 3. Prototype-only data transforms (deterministic; NOT product decisions)

These mirror the V4 fields where possible and are **prototype-only**; final ranking behaviour
is V4/V7. They are recorded so a reviewer can recompute every row from `metrics.json`.

- **`path_class` proxy (V4 field emulation).** Non-production iff the lowercase path matches
  `(^|/)(tests?|testing|specs?|examples?|fuzz(ing)?|benches?|benchmarks?|vendor|third_party|generated)(/|$)`
  or the basename matches `test_*`, `*_test.*`, `*.test.*`. Otherwise production. `compat/`
  counts as **production** (it is compiled source, not test scaffolding). Absent ⇒ production
  (the sample scans have no `path_class`).
- **Cost stats.** `median` and p95 over **all** `stats.functions[].function_cost_v2`
  (including 0). Ratio formatting: `{r:.1f}×` when `r < 10`, else `{r:.0f}×`.
- **Top-percentile label.** For a row at 1-based rank `k` of `N` in its full ordering:
  `pct = max(1, round(100·k/N))`; label `top 1%` (pct ≤ 1), `top 5%` (≤ 5), `top 10%` (≤ 10),
  else `top 25%`.
- **Include in-degree (KV2).** Distinct `from` files per `to` over `graphs.include_graph.edges`
  with `resolved == true`. Coverage line `resolved X of Y include edges`. `× median dependent
  count` is printed **only when the median in-degree across scanned files is > 0** (tmux/cJSON
  medians are 0 → the row uses `% of scanned files` + `top P%` instead; V4 note).
- **KV1 ranking key:** production first; then severity `hot < warn < info`; then confidence
  `HIGH < MEDIUM < LOW`; then containing-function `function_cost_v2` desc (span containment,
  else name match, else 0); then `finding_id` asc. The containing function provides the cost
  anchor; a finding with no containing function shows rank + severity/confidence only (ordinal
  ranks are exempt from the unit/anchor rule). Rows shown inline: **top 5**; footer
  `Complete ranked list — all N findings (production-first) →` + `N non-production files
  demoted`.
- **KV2 ranking key:** production first; then in-degree desc; then path asc. Inline **top 8**
  production rows; footer `N non-production files demoted · resolved X of Y include edges`.
  (V2 currently names production-first for KV1/KV3 only; the prototype applies it to KV2 to
  surface production hubs — e.g. cJSON `cJSON.h` behind 22-dependency test headers. If the
  vision/decision evidence supports it, this is a **proposed V2 amendment for founder
  ratification**, recorded in the results doc; the prototype does not silently change the lock.)
- **KV3 ranking key:** production first; then loop-bearing (`max_loop_depth > 0`) before
  no-loop; then `function_cost_v2` desc; then path asc. Inline **top 8 non-demoted** + **2
  greyed demoted rows** (first no-loop / non-production rows) labelled `no-loop — demoted` /
  `non-production — demoted` (never excluded). Badge `unknown-trip ×N — estimate incomplete`
  when `unknown_trip_loops > 0`. Footer counts: `N startup functions demoted`
  (`entry_kind ∈ {main, script}`), `N no-loop functions demoted`, `N non-production files
  demoted`, `Complete cost-ranked list (N functions) →`. The known-literal-trip flag is a V4
  field and is **not computable** in this prototype; the badge vocabulary is stated in the
  footer legend, not fabricated on rows.
- **Row labels.** Key-view text rows show full `path:line` + function name (the 32-char cap is
  chart-label-only). Matrix column headers use basenames (rotated text is unreadable); matrix
  row labels use repo-relative paths.

## 4. Blind read protocol (`@vision`)

- Renderer: Playwright chromium 1243, viewport 1440×900, `device_scale_factor=1`.
- **Singles (18 = 6 candidates × 3 repos):** one image per task. Prompt: read the PNG cold, no
  other file; (1) describe what is seen in 3–5 sentences; (2) output strict JSON
  `{question_answered, action, blockers[], confidence}`. The slot's expected answer is **not**
  given. The action line is part of the rendered page (A8 requires it on the view) and is not
  repeated in the prompt.
- **Pairs (9 = 3 slots × 3 repos):** both candidate PNGs of one slot+repo in one task, shown as
  A and B; prompt asks for strict JSON `{winner: "A"|"B"|"tie", why, confidence}`; no source,
  no other views. **Pre-registered order** (A first):

  | Repo | KV1 | KV2 | KV3 |
  |---|---|---|---|
  | tmux | brief | bars | bars |
  | hyperfine | cards | matrix | treemap |
  | cJSON | brief | matrix | bars |

- Every single and pair read is stored verbatim in `docs/evidence/map-vision/v3/reads.json`.

## 5. Decision rule (pre-registered)

1. **Pair vote:** winner = 1, tie = 0.5 per repo. A candidate wins a slot if it scores ≥ 2.0 of
   3 (i.e. ≥ 2 wins, or 1 win + 2 ties).
2. **Tie-break (split vote, both < 2.0):** among the slot's single reads, prefer the candidate
   with (a) more reads whose `question_answered` matches the slot question, then (b) more reads
   whose `action` names a concrete path/function/line, then (c) higher mean confidence. If still
   equal → **founder decides**, recorded as such.
3. **Locked-parenthetical conflict:** if the winning idiom contradicts a V2 slot-table
   parenthetical (`KV2 "bar = dependent-file count"`, `KV3 "bar = function_cost_v2"`), the
   result records the required spec amendment and **defaults to the V2-locked parenthetical**
   until the founder ratifies the change at the next gate.
4. **Recall check (deterministic, not a vote):** for each repo, record whether each of the 9
   pre-registered actions (`docs/evidence/map-vision-prereg.md`) appears in each candidate's
   full ranking, at what rank, and whether it is inline. Rank defects are V7 inputs, not idiom
   vetoes.
5. **Record per slot:** winner, rationale, and a **"what would change it"** trigger.

## 6. Explicit non-goals

No product code, no schema change, no new chart idiom beyond the 6 candidates, no django/
postgres use, no founder timed test (V6), no final ranking decision (V4/V7).
