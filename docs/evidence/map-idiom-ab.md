# Map-Intelligibility V3 — idiom A/B results and decisions

> Plan `map-intelligibility` V3 · spec `map-intelligibility` (V2-locked slots) · data
> `out-i5b-{tmux,hyperfine,cJSON}/metrics.json` (schema 1.6) · tree after `1f042af`
> (prereg). Pre-registration: `docs/evidence/map-vision/v3/prereg.md` (committed before any
> prototype HTML/PNG was rendered or read). This document records the run, the blind reads,
> the deterministic recall check, the per-slot decision and its **what would change it**
> trigger. **No product code or schema changed. django/postgres were not rendered or read.**

## 1. Protocol actually run

| Item | Value |
|---|---|
| Candidates | 2 per slot: KV1 `brief` vs `cards`; KV2 `bars` vs `matrix`; KV3 `bars` vs `treemap` |
| Renders | 18 HTML prototypes → 18 PNGs, Playwright chromium 1243, 1440 wide, `device_scale_factor=1`; **15/15 non-card pages** have content height ≤900 px (PNGs 920 px); **all 3 KV1-cards pages are 1635 px** |
| Data | tmux 216 files / 7,545 findings / 3,189 fns (median cost 20); hyperfine 53 / 348 / 353 (7); cJSON 101 / 837 / 1,055 (2). SHAs in `map-vision/v3/manifest.json` |
| Blind singles | **18** (6 candidates × 3 repos): one image per task, no source, no sibling, no expected answer; describe → strict JSON `{question_answered, action, blockers, confidence}` |
| Blind pairs | **9** (3 slots × 3 repos): both candidates of one slot+repo, pre-registered A/B order, strict JSON `{winner, why, confidence}` |
| Reads | `docs/evidence/map-vision/v3/reads.json` (structured extraction, V0 convention) |
| Decision rule | Pre-registered in `prereg.md` §5 (pair ≥2/3; split → single-read tie-break; locked-parenthetical conflict → default to V2) |
| Manifest | `docs/evidence/map-vision/v3/manifest.json` sha256 `63f6e0da…` (after the matrix correction below) |

### 1.1 Post-prereg corrections (recorded, not silent)

1. **KV3 ranking key** — the prereg key omitted startup demotion; the locked spec
   (`map-intelligibility.md:124-132`) requires `entry_kind = main/script` demotion. Corrected
   before the reads: `demote_score = startup + no_loop`, ascending, then cost desc.
2. **KV3 scale/row parity** — bars and treemap now use the same selection (8 non-demoted + 2
   demoted rows) and the same scale domain (non-demoted max); demoted rows are shown off-scale
   with full values. First render had 8 vs 12 rows and a bar scale dominated by cJSON's
   2,000,008-unit demoted test row (all production bars <1 px) — a rendering artifact, not an
   idiom difference.
3. **KV2 matrix rows** — first render used raw top-12 out-degree, which for cJSON produced 12
   `tests/*` rows and hid every production dependent of `cJSON.h`. Corrected rows to
   production-first includers (top 8 production out-degree ∪ production dependents of the top
   3 columns, non-production fill only to reach 12), re-rendered the 3 matrices, and re-ran the
   3 matrix singles. No other candidate changed.

**Pair-render provenance.** The matrix correction happened **before** any pair read was
dispatched. All 9 pairs — including the 3 KV2 pairs — compare against the corrected matrix
renders bound by manifest sha256 `63f6e0da…`; the three pre-correction matrix singles were
replaced, not retained, and no read in this artifact used the superseded matrix render. The
KV1 key's implemented tiebreakers add `path`/`span` after `finding_id`; `finding_id` is unique
in all three scans (verified), so the ordering is identical to the documented key.

## 2. Blind singles (18)

`Q` = the reader's `question_answered` matches the slot question; `action` = it names a
concrete path/function/line; `conf` = reader confidence (`—` = omitted by the reader).

| Repo | Slot-idiom | Q | action | conf | main blockers (as read) |
|---|---|---|---:|---:|---|
| tmux | KV1 brief | ✅ | ✅ `format.c:6052` | 0.78 | 4/5 why texts truncated; #1 why is a directory aggregate, not the row |
| tmux | KV1 cards | ✅ | ✅ `format.c:6052` → `getline.c:85` | 0.62 | same mismatch; #1 snippet is `static int`; rank vs cost looks non-monotonic; 1635 px |
| tmux | KV2 bars | ✅ | ✅ do not touch `tmux.h` (160, 74%) | — | no dependent list; no axis; slivers at 1; duplicated caption |
| tmux | KV2 matrix | ✅ | ✅ do not touch `tmux.h` (160) | 0.55 | only ~13 rows; direction can be misread; duplicated subtitle |
| tmux | KV3 bars | ✅ | ✅ `format_replace` first (1331, 67×) | 0.55 | units undefined; 6/8 unknown-trip badges; off-scale demoted rows |
| tmux | KV3 treemap | ✅ | ✅ `format_replace` via complete list | 0.60 | tiles carry no `path:line`; uniform blue; 2511/3189 demoted unexplained |
| hyperfine | KV1 brief | ✅ | ✅ `report.rs:144` `print` | 0.75 | truncated mid-word; units jargon; #1/#2 identical metrics |
| hyperfine | KV1 cards | ✅ | ✅ `report.rs:144` (lines 142-146) | 0.72 | truncated; #1/#2 near-duplicates; Hot-component metric mismatch; 1635 px |
| hyperfine | KV2 bars | ✅ | ✅ `src/quantity/mod.rs` (12, 6.0×) | 0.55 | rebuild fan-in ≠ breakage; 33/238 unresolved; no axis; duplicated caption |
| hyperfine | KV2 matrix | ✅ | ❌ names `src/main.rs` for the 12-dep column | 0.55 | 4 columns labelled `mod.rs`; direction ambiguous |
| hyperfine | KV3 bars | ✅ | ✅ `report.rs:19` `print` (213, 30×) | 0.60 | all rows estimate-incomplete; `scripts/plot_*.py` counted production; misread `8.7×` as `0.7×` |
| hyperfine | KV3 treemap | ✅ | ✅ `print` (213) via list | 0.55 | no paths on tiles; three `main` tiles indistinguishable |
| cJSON | KV1 brief | ✅ | ✅ `cJSON_Utils.c:807` | 0.62 | #1/#4 same line; #2/#3 duplicates; #1 why is a directory aggregate; all in one file |
| cJSON | KV1 cards | ✅ | ✅ `cJSON_Utils.c:807` | 0.72 | duplicates; identical 94 units repeated per line; 1635 px |
| cJSON | KV2 bars | ✅ | ✅ do not touch `cJSON.h` (5) | — | top% ambiguous; no dependent list; mostly empty canvas |
| cJSON | KV2 matrix | ✅ | ✅ check hidden dependents of `cJSON_Utils.h`; treat unity headers as do-not-touch | 0.50 | totals exceed visible dots (5 vs 3); production/test dots identical |
| cJSON | KV3 bars | ✅ | ✅ `cJSON_Delete` first (137, 68×) | 0.55 | all rows estimate-incomplete; demoted 131 hides above most rows |
| cJSON | KV3 treemap | ✅ | ✅ `cJSON_Delete` (137) via list | 0.55 | no paths; near-equal tiles; demoted 131 ≈ top 137 |

**Singles summary:** all 18 reads stated the right question and named an action; **17/18 named
the correct target** (the hyperfine matrix named the wrong file — `src/main.rs` for the
12-dependent `mod.rs` column). The differences are speed/clarity, not comprehension. Mean
confidence: KV1 brief 0.72 vs cards
0.69; KV2 bars (one value 0.55) vs matrix 0.53; KV3 bars 0.57 vs treemap 0.57.

## 3. Blind pairs (9)

| Repo | Slot | A | B | Winner | conf | Why (reader's words, condensed) |
|---|---|---|---|---|---|---:|---|
| tmux | KV1 | brief | cards | **cards** | 0.70 | snippets verify the target and fill the brief's truncated/mismatched rationale |
| tmux | KV2 | bars | matrix | **bars** | 0.85 | bars state the answer in text; matrix buries totals in a dense grid |
| tmux | KV3 | bars | treemap | **bars** | 0.85 | bars give `format.c:6052` + 67× + badges; treemap tiles near-equal, no location |
| hyperfine | KV1 | cards | brief | **brief** | 0.72 | snippets are redundant context; the brief answers with less noise |
| hyperfine | KV2 | matrix | bars | **bars** | 0.93 | bars name `src/quantity/mod.rs`; matrix's top column is an ambiguous `mod.rs` |
| hyperfine | KV3 | treemap | bars | **bars** | 0.82 | bars give file:line, disambiguate three `main`s, show 30× median |
| cJSON | KV1 | brief | cards | **brief** | 0.70 | compact list; cards pad with code without changing the next step |
| cJSON | KV2 | matrix | bars | **bars** | 0.78 | matrix invites a wrong action (unity_config.h 22 is test scaffolding) |
| cJSON | KV3 | bars | treemap | **bars** | 0.85 | bars give `cJSON.c:253` and cost anchors; near-equal tiles omit location |

**Pair vote:** KV1 **brief 2.0 / cards 1.0**; KV2 **bars 3.0 / matrix 0.0**; KV3 **bars 3.0 /
treemap 0.0**.

## 4. Deterministic recall check (not a vote)

Ranks of the 9 pre-registered actions (`docs/evidence/map-vision-prereg.md`) in each
candidate's **full deterministic ordering**:

| Repo | Target | Slot | Rank in full ordering | Inline? | Note |
|---|---|---|---:|---|---|
| tmux | `input.c:970` `input_parse` | Q1 | **210 / 7,545** | ❌ | severity-first puts `format_replace` (documented FP) at #1 |
| tmux | `tmux.h` | Q2 | **1 / 216** | ✅ | 160 dependents, 74% |
| tmux | `screen-write.c:2776` `screen_write_collect_add` | Q3 | **811 / 3,189** | ❌ | no-loop (per-byte leaf), demoted |
| hyperfine | `src/benchmark/mod.rs:346` `Benchmark::run` | Q1 | **197 / 348** | ❌ | `print` (output formatting) ranks first |
| hyperfine | `src/command.rs` | Q2 | **4 / 53** | ✅ | 5 dependents |
| hyperfine | `src/timer/mod.rs:82` `execute_and_measure` | Q3 | **66 / 353** | ❌ | no-loop, demoted |
| cJSON | `cJSON.c:357` `parse_number` | Q1 | **35 / 837** | ❌ | top rows are all `cJSON_Utils.c` |
| cJSON | `cJSON.h` | Q2 | **1 / 101** | ✅ | 5 production dependents (22 test-header dependents demoted) |
| cJSON | `cJSON.c:243` `cJSON_New_Item` | Q3 | **58 / 1,055** | ❌ | no-loop, demoted |

**This is the session's most important result.** The chosen idioms are legible (all 18 reads
named an action), but the **locked KV1 ranking key cannot surface the pre-registered Q1 answer
inline in any of the three repos** — the views confidently point at a different first action
(tmux `format_replace`, hyperfine `print`, cJSON `apply_patch`). That is the V0/V1 failure
recurring at the ranking layer, not the idiom layer. Likewise, all three Q3 targets are
no-loop and therefore demoted by the V2 guard: for tmux `screen_write_collect_add` is a
per-byte leaf whose call frequency is invisible to static cost (H1 for call weights, as V1
predicted). **V4 must not claim A1b closed on severity alone; V7 owns the rank-key re-triage;
V5 must test the no-loop demotion against this evidence.** Reproducer:
`docs/evidence/map-vision/v3/render/recall_ranks.py` writes
`docs/evidence/map-vision/v3/recall_ranks.json` and fails on any rank drift.

**V4 shipped-key re-measure (2026-10-22).** The table above is the V3 prototype ordering. The
shipped V4 key (production → severity → confidence → containing-function cost → finding_id,
with `ARCH-002` excluded; KV3 adds loop-bearing and startup demotion) was re-measured on the
schema-1.7 scans: tmux Q1 **209/7,543**, Q3 **820/3,189**; hyperfine Q1 **172/346**, Q3
**58/353**; cJSON Q1 **37/835**, Q3 **60/1,055** — still not inline, so the recall finding
stands; KV2 stays inline (tmux 1/216, hyperfine 6/53, cJSON 4/101). Reproducer:
`docs/evidence/map-vision/v4/recall_ranks_v4.py` → `docs/evidence/map-vision/v4/recall_ranks_v4.json`.

## 5. Decisions

### KV1 — **text-first ranked brief** (pair vote 2/3)
- Brief wins hyperfine and cJSON; cards win tmux because the brief's #1 rationale was a
  directory aggregate and the snippet verified the real target. Cards also never fit the
  1440×900 reading surface (1635 px) and add scanning cost.
- **What would change it:** (a) if V4's message/truncation fixes land and V5 blind reads still
  fail to name an action, switch to cards; (b) if the founder (V6) prefers inline code
  evidence despite scrolling; (c) if a ≤900 px card layout wins a re-run A/B 2/3.
- **Ranking caveat:** the brief is the right idiom but the wrong rank key (recall §4). V7 must
  re-triage KV1's ranking; an idiom change will not fix it.

### KV2 — **horizontal dependents bars** (pair vote 3/3)
- Bars win every repo; matrix loses on label discipline (basename-only columns, 4× `mod.rs`;
  one reader misattributed the 12-dependent column) and on burying totals in a dense grid.
- **What would change it:** (a) a matrix variant with **full-path column labels + a ranked
  totals row at the top** wins a re-run 2/3; (b) Q2 moves to component units; (c) the founder
  wants the dependent-file list inline (bars+list hybrid), which is an explorer drill, not a
  new encoding.
- **Proposed V2 amendment (founder ratification, not silently applied):** extend
  **production-first demotion to KV2**. Evidence: cJSON's production bars lead with `cJSON.h`
  (5 dependents); without demotion the unity headers (22/21/22 test includers) dominate and the
  public ABI hub is invisible. The V2 text names production-first only for KV1/KV3.
- **Anchor correction:** `× median dependent count` is undefined on tmux/cJSON (median
  in-degree = 0). Use `% of scanned files` + `top P%`; keep `× median` only when median > 0
  (hyperfine 6.0×). V4 note.

### KV3 — **horizontal cost bars** (pair vote 3/3)
- Bars win every repo; treemap loses because tiles carry no `path:line`, near-equal areas are
  hard to rank, and uncertainty badges have no home.
- **What would change it:** (a) V5 shows the no-loop demotion hides a genuine hot leaf
  (`screen_write_collect_add` rank 811, demoted) → switch to per-iteration cost or the
  two-list (known-trip vs unknown-trip) variant **before** changing idiom; (b) a treemap
  variant with mandatory `path:line` + rank labels wins a re-run 2/3; (c) the founder wants
  module grouping (a different question than Q3).
- **Guard status:** the known-literal-trip flag remains a V4 field; this prototype showed only
  the unknown-trip badge. The V5 falsifier stands.

## 6. Findings that feed V4/V5/V7 (not idiom choices)

1. **Rule-message defect (V4, product):** `Hot component` findings carry a *directory*
   aggregate message (`directory '.' aggregates complexity …`) while the row anchors a
   file:line (tmux #1, cJSON #1). Both idioms' readers flagged it. Fix message selection or
   suppress component-level rows from finding-level KV1.
2. **Truncation (V4, CSS):** 4/5 brief rationales end in `…`; key-view text rows are exempt
   from the 32-char cap, so show the full why.
3. **KV1 rank key (V7):** see §4. The complete list satisfies traceability but not "look here
   first".
4. **KV3 no-loop guard (V5/V7):** see §4; the guard can demote a genuinely hot no-loop leaf.
5. **`scripts/` classification (V4 classifier):** hyperfine `scripts/plot_*.py` count as
   production under the prototype proxy; decide whether repo tooling is demoted.
6. **KV2 duplicate caption (V4):** the action line renders twice (header + caption); keep one.
7. **Matrix labels (Explorer only):** basename-only columns are ambiguous; if a matrix
   survives as an explorer stem, use full paths.
8. **KV1 cards one-screen rule:** 1635 px at 1440 width; any future card idiom must fit 900 px
   or be labelled explorer-tier.

## 7. Limitations

- Vision reads are a model proxy for the founder's timed test (A2/V6), not a substitute; the
  sample is 3 repos × 6 candidates with one single read per candidate.
- The pair order was fixed per repo by the prereg, not randomized per read; KV2/KV3 are 3/3
  and unlikely to be order artifacts. The KV1 winners followed a **B/B/A** pattern (tmux B,
  hyperfine B, cJSON A) and brief was already V2's named candidate, so the 2/3 is partly
  confirmatory; the tmux cards win is the only counter-signal. A randomized re-run is the
  cheapest way to falsify position/confirmation bias before V6.
- The pre-registered Q1/Q3 answers come from source reasoning, not runtime profiles; the
  recall ranks measure "does the view surface this", not "is this the true hot path".
- django/postgres were not rendered, referenced as data, or read (V6 holdout intact).

## 8. Evidence index

- `docs/evidence/map-vision/v3/prereg.md` — pre-registered candidates, keys, protocol, rule
- `docs/evidence/map-vision/v3/manifest.json` — data SHAs, 18 render entries, PNG dims, holdout
- `docs/evidence/map-vision/v3/reads.json` — 18 singles + 9 pairs + corrections
- `docs/evidence/map-vision/v3/recall_ranks.json` — 9 recall ranks with pre-registration labels
- `docs/evidence/map-vision/v3/render/recall_ranks.py` — recall reproducer (exits non-zero on drift)
- `docs/evidence/map-vision/v3/<repo>/<slot>-<idiom>.png` — 18 renders (1440 px wide)
- `docs/evidence/map-vision/v3/render/generate_v3.py` + `render_v3.py` — deterministic repro
- V2 lock: `docs/specs/map-intelligibility.md` → "Locked acceptance view set"
