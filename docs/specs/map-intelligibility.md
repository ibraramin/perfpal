# Spec: map-intelligibility

Scope: feature

# Map Intelligibility — feature spec (successor to `birdseye-map` F5)

## Why (founder verdict, 2026-10-21)

The I5 human A5 test **failed**. After the 9-repo demo the founder said, verbatim:

> "Honestly, the visuals are all posished and nice, but I can't understand anything, I doubt
> anyone will undersand anything. Now, there can be 2 reasons, 1. We simply don't have enogh
> useful information extracted via perfpal. or 2. We have enough information to construct an
> intuituve and easy to understandable birds-eye-view map of the codebase, it's just that the
> graphs and figures we are constructing aren't doing the data extracted by perfpal justice.
> Use a methodology of going back and forth with the vision model to see what's the issue, and
> from now on, while maijng the graph and charts and anything visual, always consult the
> vision model to make use the visual elemtn adheres to our project methodology (not all
> graphs has to be completely simple and understandable, there has to be ones that are simply
> info-dumps but some of the key ones has to be easily understandablem, do you get me >)"

Per `birdseye-map` A5, **the idiom is judged wrong, not the data**. The I5 proof (determinism,
gate, semantic tier, artifact contract) stands; the visual language and the ranking it
expresses do not.

## The two hypotheses (must be separated by evidence, not opinion)

- **H1 — data insufficiency:** Perfpal does not extract the facts needed to answer the three
  maintainer questions (e.g. no runtime weights, no per-function caller/callee chains at a
  useful granularity, no change/churn signal, no build-flag/gating signal).
- **H2 — visualisation insufficiency:** the facts exist in `metrics.json` (findings,
  `function_cost_v2`, entry/reachability, resolved call edges, include edges, components) but
  the current stems (DAGs, bands, cost heat) do not turn them into a maintainer-readable
  answer.

The first deliverable is a **vision-model diagnostic** that decides, per view and per
question, which hypothesis dominates. Neither hypothesis may be assumed. The diagnostic must
also pass a **known-answer recall test**: the 9 I5 GENUINE findings and the pre-registered
actions for the sample repos must be traceable to a view; if the data exists but no view
surfaces it, that is H2, not H1.

## Viewing conditions (fixed, or the test is meaningless)

- Canonical path: `report.html` → `#map` at **1440×900, 100% zoom**, then drill down.
- `@vision` receives **rendered PNG screenshots** (Playwright), never raw SVG text.
- The report's first screen must answer Q1 in plain language; the current 8 stems move
  under an explicit `Explorer` divider and stay available for power users.

## Vision protocol (auditable, blind)

- No source access, no sibling-view access; describe what is seen **before** judging.
- Strict output: `{question_answered, action, blockers, confidence}`; two independent reads
  per key view, both recorded as evidence.
- A view that only makes sense after reading another view is not answer-first.

## Visual methodology (durable rule)

- **Two tiers, explicitly labelled.**
  - **Answer-first key views (must be legible):** one view answers one question in plain
    language; a reader with no repo knowledge must name a concrete action from it. These are
    the views the founder/vision test judges.
  - **Explorer / info-dump stems (allowed):** full-fidelity charts for power users and
    LLM consumption; they may be dense, but they must be labelled as explorers and must not
    be the only path to an answer.
- **Vision gate is mandatory.** Every chart/visual change is reviewed by `@vision` against
  the methodology before it is claimed done: "what does this communicate?", "what can a
  reader not answer?", "which question does it serve?". Transcripts are committed as
  evidence.
- **No orphan views.** Every key view names the maintainer question it answers and the
  action it is supposed to enable. A view that answers nothing is removed or demoted to an
  explorer stem.
- **One primary view, one encoding.** At most 3 answer-first views, one question each. A key
  view uses ONE primary encoding (severity OR cost OR coupling), never all three stacked.
  Every number carries a unit and a comparison anchor (`× median`, `top 1%`), not a bare
  cost. Progressive disclosure: summary first, detail on drill.
- **Text-first is a legitimate answer.** A ranked maintainer brief (3 actions with
  `path:line`, why, confidence, supporting chart) may be the primary view; charts are then
  evidence. The A/B in the plan includes it.
- **Plain language first.** Jargon (`DAG`, `barycenter`, `stratum`, `resolved pair`) stays in
  the explorer tier; key views use maintainer words ("what breaks if I change this", "where
  is time likely going", "what is safe to ignore").
- **Honesty unchanged.** Candidates, not certainties; caps, stubs and LOW-confidence labels
  survive any redesign.

## Locked acceptance view set (V2, 2026-10-21)

V2 locks **three answer-first view slots**, one per maintainer question. The slot is the
contract; V3 chooses the concrete idiom inside a slot by blind A/B and may not add a fourth
slot or stack a second encoding. Nothing below is implemented in V2.

| Slot | Question (plain language) | Primary encoding | Unit + anchor | Action it must enable |
|---|---|---|---|---|
| **KV1 — Start here** | Where do I look first? | **severity** (ranked list position; no colour/size channel) | rank #k; where cost is shown: `static cost units (v2)`, `× median function`, `top 1%` | open/profile the named `path:line` first |
| **KV2 — Blast radius** | What breaks if I touch it? | **coupling** (bar = dependent-file count, one hue) | `N dependents`, `% of scanned files`, `× median dependent count`, `top 1%` | do not edit directly; expect N dependents to rebuild/re-import |
| **KV3 — Profile queue** | Where might time plausibly go? | **cost** (bar = `function_cost_v2`, one hue) | `static cost units (v2)`, `× median function`, `top 1%`, unknown-trip badge + known-literal-trip flag | profile the named function first (static candidate, confirm) |

Rules that bind the slots:

- **First screen = KV1 (legacy list-first report; superseded for the Atlas page by V8 below).**
  `report.html#map` opens with a plain-language answer to Q1, not a chart index. The workspace
  node-link is not on the first screen.
- **Explorer divider.** The 8 current stems (`include-graph`, `layer-file`, `layer-entry`,
  `component-coupling`, `hot-components`, `layer-workspace`, `layer-bands`, `layer-findings`)
  leave `#map` and are emitted under one explicitly labelled **Explorer** section. The existing
  `#architecture` section is **renamed** to `Explorer` (heading + nav label; `#explorer`), not
  duplicated. They remain available, gated as today (fewer than 8 on small scans), and make no
  answer claim. Per-component detail pages remain explorer-tier evidence, not key views, and
  carry an Explorer-tier label when reached from a key-view link.
- **Text-first candidate.** KV1's primary candidate is the ranked maintainer brief: 3–5
  actions inline with `path:line`, function, rule **title** (never a bare rule ID), why,
  severity + confidence, production class, cost anchor, and a supporting-chart link. The link
  must resolve to content that shows the action (KV3 row, per-component page, or drill row),
  never a dead affordance. Ranked cards are the V3 alternative. Either way the slot's primary
  encoding is severity, expressed as rank.
- **No silent caps (A1b mechanism).** KV1 shows 3–5 inline and links a complete deterministic
  ranked action list (hosted in the existing `findings-detail` artifact); every `hot` finding at
  function granularity stays reachable at a stable rank. KV3 likewise links the complete
  deterministic function list ranked by cost within the non-demoted set (hosted in a
  `profile-queue` drill artifact; output-contract update in V4), so the six pre-registered
  function-level cost targets stay traceable at their rank. No key view may hide a known answer
  behind an undisclosed cap.
- **Default-tier honesty.** KV2 claims include blast radius only. Every default-tier scan
  exports only LOW-confidence call edges, so a call-based claim must carry the LOW/semantic-tier
  label or be absent. KV2 prints include coverage (`resolved N of M include edges`).
- **Production-first, never silent.** KV1/KV2/KV3 rank production code first
  (`files[].path_class`), demote test/example/bench/fixture/generated/vendor files, and print
  `N non-production files demoted`. KV2 slots up to 8 production rows then up to 2 greyed
  demoted rows; demotion never excludes a file from the ordering. KV3 also demotes
  `entry_kind = main/script` (startup) and
  no-loop functions (`max_loop_depth == 0`) below loop-bearing ones — never excluding either —
  and prints `N startup functions demoted` + `N no-loop functions demoted`; it labels
  `unknown_trip_loops > 0` as "estimate incomplete" and prints `known trip ×N — verify it
  actually runs` for any known-literal trip. This three-branch guard set covers the two
  documented cost FPs (django `cache.py` literal-10000 retry; tmux `format.c` branch-count
  dispatcher inflation — demoted if `max_loop_depth == 0`, otherwise labelled).
- **Units + anchors are mandatory.** Every measured numeric value in a key view carries a unit
  and a scan-relative comparison anchor (`× median`, `top 1%`, `% of scanned files`). A bare
  cost or density number is a spec violation. Ordinal ranks are exempt (they are positions, not
  measurements). KV3's unit reads `static cost units (v2)`, or `static cost units (v1)` on
  v1-fallback rows, and anchors are computed within the same scale. The 32-char tail-keep cap
  applies to chart node labels only; key-view text rows show the full `path:line` and function
  name.
- **One encoding.** KV1 = severity (ranked position), KV2 = coupling, KV3 = cost. No key view
  stacks colour + size + position to encode a second metric; severity/confidence are text
  labels, not channels.
- **No orphan views.** Each slot names its question and action (table above). A view that
  answers nothing is demoted to Explorer or removed. `layer-findings` may not be described as
  "density" while it ranks raw severity-weighted counts without a denominator.
- **Field decision.** The fields the locked slots need — and the fields they deliberately do
  not — are recorded in `docs/evidence/map-data-coverage.md` §8 with a "what would change it"
  trigger. `files[].path_class` and trip provenance (`stats.functions[].trip_count_source` +
  `known_trip_value`) are required; the rest are computed chart-side or deferred.

## V3 idiom decision (2026-10-21)

V3 tested two idioms per slot on tmux, hyperfine and cJSON with 18 blind single reads and 9
pre-registered pairwise reads (evidence: `docs/evidence/map-idiom-ab.md`,
`docs/evidence/map-vision/v3/reads.json`). Decisions:

- **KV1 = text-first ranked brief** (2/3 pairs). Cards won tmux only because the brief's #1
  rationale was a directory aggregate; cards also never fit 1440×900 (1635 px) and add
  scanning cost. The brief remains the primary view.
- **KV2 = horizontal dependents bars** (3/3). The matrix loses on basename-only column labels
  (four `mod.rs` columns; one reader named the wrong file) and on totals buried in a dense
  grid. A matrix may survive only as an Explorer stem with full-path labels.
- **KV3 = horizontal cost bars** (3/3). The treemap loses because tiles carry no `path:line`
  and near-equal areas cannot be rank-ordered; bars keep units, anchors and badges.

What would change a decision is recorded in `map-idiom-ab.md` §5. The V3 recall check also
found that the locked KV1 severity ranking does **not** surface the pre-registered Q1 target
inline in any of the three repos (ranks 210/197/35) and that the KV3 no-loop demotion pushes
all three Q3 targets down (811/66/58). The idiom choice stands, but **V4 must not claim A1b
closed on severity alone — V7 owns the KV1 rank-key re-triage and V5 must test the no-loop
guard against the per-byte-leaf evidence**.

**KV2 production-first (V2 amendment, ratified 2026-10-22; implemented V6a).** V3 applied
production-first demotion to KV2 because cJSON's public ABI header (`cJSON.h`, 5 production
dependents) is otherwise buried by unity test headers with 22 dependents. V5 O2 confirmed it:
the two cJSON KV2 reads disagreed on the riskiest file. The founder ratified the amendment
before V6 and it is implemented (see V6a below): production rows first, up to 8 production +
up to 2 greyed demoted rows, demoted count printed, never excluded. V4 shipped the label-only
behavior; that is historical.

**KV2 anchor.** When the median dependent count is 0 (tmux, cJSON), `× median dependent count`
is undefined; the key view uses `% of scanned files` + `top P%` and keeps `× median` only when
the median is > 0 (hyperfine 6.0×).

## Acceptance (all must hold)

- **A1 — vision-read pass:** for each answer-first view, a fresh `@vision` read (no source,
  no other view) states the question it answers and one concrete action; recorded verbatim.
- **A1b — known-answer recall:** the 9 I5 GENUINE findings and the pre-registered actions are
  traceable to a key view (surfaced, or explicitly out-of-scope with a reason). No known
  answer may be invisible while the data exists. KV1 covers the finding targets; KV3's complete
  ranked cost list covers the six function-level cost targets; KV2 covers the three file-level
  "do not touch" targets.
- **A2 — founder timed test:** a maintainer unfamiliar with the repo names 3 actions from the
  key views alone in **under 10 minutes**, and says the map is understandable. This is the
  same A5 bar that failed; no agent-only substitute closes it.
- **A2b — holdout:** the founder test includes one repo never used during design/tuning
  (django or postgres).
- **A3 — data decision:** for every maintainer question, the plan records either "existing
  field X answers it" (with `file:line`/schema proof) or "missing field Y is added
  additively" (schema note + tests). No question is left implicitly unsupported.
- **A4 — determinism/contract:** double-scan byte-identical; goldens regenerated; output
  contract updated; no new dependency; no copyleft; explorer stems do not regress.
- **A5 — stop criterion:** two distinct design idioms failing the founder test stop the
  polishing loop and escalate to an ADR-0008 amendment (text-first brief or a different
  primary view).
- **A6 — dual review:** `@reviewer` + `@requirements-reviewer` APPROVED on the final tree.
- **A7 — units/anchors:** a key-view read finds no bare measured number (ordinal ranks are
  exempt); every measured number carries a unit and a scan-relative anchor. Evidence: V5
  transcripts + a check of the rendered key views.
- **A8 — first screen + explorer/no-orphan (legacy list-first report; superseded for the Atlas
  page by V8 below):** `report.html#map`'s first content block is KV1 in plain language; every emitted legacy stem renders under the labelled `Explorer` divider; every
  key view states its question and action in this spec and on the view. Evidence: committed PNG +
  DOM assertion.

## Explicit non-goals

Force-directed full-corpus layout; flame graphs; history/diff views; JS layout engines;
runtime truth claims; new languages; any change that makes key views denser than today.

## Relationship to the roadmap

I5 was the last node of `map-build` (I1–I5; there is no I6). This spec governs the next
map node, **before** JS/TS backends and the paper queue: a core-feature that nobody can read
is not a shippable core feature. M5 remains gated by `B-pf-001`/`B-pf-002`.
## V4 implementation notes (2026-10-22)

- **Idioms shipped** (V3 decisions, committed `bc887ef` schema + this commit): KV1 = text-first
  ranked brief (≤5 inline rows, full rationale, complete list in
  `findings-detail#ranked-actions`); KV2 = horizontal dependents bars (≤8, integer CSS widths,
  one action line); KV3 = horizontal cost bars (≤8 non-demoted + up to 2 labelled demoted rows,
  complete list in `profile-queue.*`). No cards and no treemap ship.
- **Component rows suppressed from KV1.** Only `-ARCH-002` ("Hot-component aggregation" — the
  directory/component aggregate) is excluded from the action ranking (it remains in
  `findings-detail`'s rule groups) and counted as "`N` component-level finding(s) in Explorer".
  `ARCH-001` (include fan-out/cycles) and `ARCH-003` (cross-file call coupling) are file-level
  actions and stay in KV1. The ARCH-002 message defect is therefore fixed by suppression, not by
  changing analyzer wording.
- **Rationale truncation:** key views render the full `finding.message`; the 32-char tail-keep cap
  remains chart-label-only. Duplicate KV2 captions are gone (one action line).
- **KV2 production-first was NOT applied in V4** — the V2 lock stood and the rows labelled
  their `path_class` (including test/bench) instead of demoting. The amendment was ratified by
  the founder on 2026-10-22 and implemented in V6a (below); this V4 note is historical.
- **`scripts/` classification:** repo tooling is `bench` in schema 1.7 (aggregator-owned); the
  decision is documented rather than re-derived in the reporter.
- **No card idiom ships.** One line for the record: any future card idiom must fit one 900×900
  reading surface (≤900 px tall) or it is explorer-tier, never a key view.
- **A1b is not claimed closed on severity.** The V3 prototype ranks 210/197/35 remain the V7
  rank-key problem (shipped-key ranks below); KV3's no-loop guard (which pushes the Q3 targets
  down) remains the V5 falsifier. The complete lists satisfy traceability, not "look here first".
- **Shipped-key recall (V4 artefact, 2026-10-22):** tmux Q1 **209/7543**, Q3 **820/3189**;
  hyperfine Q1 **172/346**, Q3 **58/353**; cJSON Q1 **37/835**, Q3 **60/1055**. KV2 is inline in
  every repo: tmux **1/216**, hyperfine **6/53**, cJSON **4/101**. These ranks differ from the V3
  prototype ranks (210/197/35 for Q1, 811/66/58 for Q3) because V4 uses the shipped key and
  excludes `ARCH-002` from the action list. Evidence:
  `docs/evidence/map-vision/v4/recall_ranks_v4.json` + `recall_ranks_v4.py` (committed `2ec547c`).
- **`PERF-RS-ARCH-002` (I5 GENUINE #6)** is a component aggregate and is explicitly out of KV1's
  action scope: it remains reachable through `findings-detail` and the Explorer
  `hot-components` stem, not the ranked action brief.

## V5 vision validation (2026-10-22)

Two fresh blind reads per key view (3 repos × 3 views = 18 valid reads; 3 first attempts
returned empty agent messages and were retried once) on the V5 renders, which are
**byte-identical** to the V4-approved key views (manifest sha256; fresh HEAD scans reproduce
the schema-1.7 V4 metrics modulo run metadata). Protocol: one image per task, no source, no
sibling views, no expected answer; describe first, then strict JSON
`{question_answered, action, blockers, confidence}`. **18/18 valid reads state the slot
question and one concrete `path:line`/function action** → A1 PASS. Evidence:
`docs/evidence/map-vision/v5/reads.json`, `manifest.json`, 9 PNGs, `render_v5.py`.
Non-blocking observations: (O1) all six KV2 reads call the `top N%` anchor ambiguous (reads
like a rank; denominator unstated next to `% of scanned files`) — confirms the V4-deferred
wording risk, a V7/founder follow-up, not a V5 code change; (O2) cJSON KV2 is dominated by
unity test headers and buries `cJSON.h` at #4 — evidence for the pending KV2 production-first
amendment, and the two cJSON KV2 reads even disagree on the riskiest file (test header vs the
production ABI header), strengthening the ambiguity case; (O3) KV3's per-block scale is understood from the caption but still invites
cross-block comparison; (O4) readers want in-view definitions of the cost unit/anchors;
(O5) one reader perceived a missing space in cJSON KV1 #4 (the DOM has a space); (O6) tmux
KV1 #5 is HOT at 0.7× median because severity is rule severity, not a cost claim.

**KV3 no-loop falsifier (result, not a fix):** all three pre-registered Q3 targets
(`screen_write_collect_add`, `execute_and_measure`, `cJSON_New_Item`) have
`max_loop_depth == 0` and are demoted, but **removing only the no-loop term does not restore
any of them to the inline top 8** (shipped→counterfactual ranks 820→325, 58→46, 60→45).
The guard is a contributing demotion, not the sole cause of the Q3 recall miss: the static
cost key cannot surface per-byte/per-call leaves (the V1 H1 prediction). V7 must adopt
call-weighted/per-iteration ranking or a separate call-frequency view — not merely drop the
guard; a two-list variant of the same static cost is insufficient. cJSON `cJSON_New_Item` is
also `reachable_from_entry = false`, so a reachable-only Q3 (R-pf-098) would hide it further.
Reproducer: `docs/evidence/map-vision/v5/no_loop_falsifier_v5.py` → `.json` (exits non-zero on
rank drift). **No V5 code change**; no read exposed a blocking defect. Holdout untouched;
`map-build` remains NOT done; A1b is not claimed closed.

## V6a — KV2 production-first amendment (2026-10-22, founder-ratified)

The founder ratified the V3-proposed KV2 production-first amendment before V6, using V5 O2 as
supporting evidence (cJSON `cJSON.h` at #4 behind 22-dependent unity test headers; the two V5
cJSON KV2 reads disagreed on the riskiest file). Implementation:

- KV2 ordering: production first (`files[].path_class`), then distinct resolved dependents desc,
  then path asc. `top P%` ranks follow this production-first ordering; the median stays over every
  scanned file, zeros included (unchanged honesty).
- Inline slotting mirrors KV3: up to 8 production rows + up to 2 greyed demoted rows; demoted
  rows are never excluded from the ordering. HTML/Markdown print `N non-production files demoted`
  alongside `showing top X of Y files with resolved dependents` and the `resolved N of M include
  edges` coverage line.
- No schema change; KV1/KV3 semantics, the `top N%` wording (O1, V7) and the holdout are
  untouched.

Vision gate: 3 fresh blind renders (tmux/hyperfine/cJSON, 1440×900 element screenshots) and 6
fresh blind `@vision` reads (2 per view, one image per task, no source/siblings/expected answer).
**6/6 PASS** — each states "What breaks if I touch it?" and one concrete path action. `cJSON.h`
now ranks #1 and both readers name it as the highest-risk production edit; the 22-dependent unity
`TEST` headers are visibly greyed and both readers correctly explain the demotion. Hyperfine
readers likewise read the greyed `BENCH` rows (9/8 dependents) as demoted-but-never-excluded below
production rows (12/10). Evidence: `docs/evidence/map-vision/v6/kv2-amendment/` (`reads.json`,
`manifest.json`, 3 PNGs, `render_v6_kv2.py`). The V5 "byte-identical to V4" claim is historical:
KV2 now differs by design; KV1/KV3 renders are unchanged.

- Tests: `kv2_demotes_non_production_without_excluding`,
  `kv2_test_file_with_more_dependents_ranks_below_production`,
  `kv2_slots_eight_production_then_two_demoted`, plus the integration
  `kv2_demoted_rows_are_greyed_with_demoted_caption`; four golden trees regenerated (KV2 caption
  lines only) and stable on a second regeneration.
- Holdout: fresh HEAD scans of django + postgres released **only** for the V6 timed test; structural
  preflight only (`docs/evidence/map-vision/v6/preflight.py`, `preflight.json`). No agent reads.

## V8 — Atlas page (2026-10-23, founder D-pf-238)

**What changed.** The founder corrected the model: G (entry-flow) is the **core graph**; F is not a
fallback and not discarded — its components are supporting elements, included only where the
developer-needs research justifies them; the graph page is **100% graph**; metrics/brief live
below; no mode switch. The research is `docs/research/atlas-developer-needs.md`; the data decision
is `docs/research/atlas-data-gap-audit.md`; the metric contract is `docs/specs/atlas-metrics.md`;
the composition is `docs/specs/atlas-layout.md`; the ADRs are ADR-0009 (core) and ADR-0010
(animation).

**Ratified 2026-10-23.** Founder ratified ADR-0009 (Atlas core) and ADR-0010 (declarative
animation) and the V8 amendments in this section: the A8 first-screen amendment (first block =
canvas carrying entry + HOTTEST + path; the one `path:line` action in the first scroll, answer line
+ KV1), the structural-edge disclosure (LOW name-based + MEDIUM semantic, `N MEDIUM / M LOW`
printed, never `resolved`/`reachable_from_entry` for that set), and A12 (exactly one fixed existing
`REPORT_SCRIPT`; zero new scripts in the Atlas canvas/SVG/animation artifacts).

**Placement (amends this spec's slot table for the Atlas page only).**

| Slot | Question | Atlas placement | Encoding (unchanged) |
|---|---|---|---|
| **KV1 — Start here** | Where do I look first? | below the canvas, block 2 (3–5 actions) | severity (ranked position) |
| **KV2 — Blast radius** | What breaks if I touch it? | below the canvas, block 4 + Q2 drill | coupling (dependent files) |
| **KV3 — Profile queue** | Where might time go? | **splits:** overview = weight strip (file/component `% of scanned cost`, block 3); drill = complete function-level profile queue | cost (one encoding; no second channel) |

The slot questions, the one-encoding rule, the production-first rule, the default-tier claim limit
and the A1b recall guarantee are unchanged. Only placements move, and KV3 gains progressive
disclosure (overview + drill) — it is not a fourth slot and it does not stack a second encoding.

**First screen (amends A8).** The first content block of the Atlas page is the **canvas**; the
first answer-bearing block is the **answer line** directly below the canvas, within the first
scroll. The canvas itself must carry the entry badge, the HOTTEST file and the path trace so a
blind read of the first screen can name Q1 — entry, HOTTEST file, representative path — without
scrolling; the one `path:line` action is in the first scroll (answer line + KV1). The old A8 rule
(KV1 as the first block) applied to the list-first V4 page; it is amended for the Atlas page, not
deleted for the legacy report.

**Canvas purity.** The canvas is graph chrome only: entry badge, file nodes (name, cost,
`× median function`, HOTTEST, finding annotation `HOT ×N`/`WARN ×N`/`INFO ×N`, M-18), depth columns, call edges (forward/back, distinct structural-edge
count), component hulls, unreached column, numbered path trace, legend, caps chip, canvas footer.
No cards, tables, bars, metric panels, metric selectors or a second map. The V7 G right-rail file
cards move below the canvas (block 5).

**No mode switch.** Every scan renders the same canvas + seven below-graph blocks
(`atlas-layout.md` §4–§5). Library, no-entry, all-test, empty, capped, partial-parse and single-file states
change labels/content, never page shape. The F weight mass is an always-on block; the F treemap is
`weight-treemap` in Explorer. The six-test inclusion rule is
`docs/research/atlas-developer-needs.md` §7.

**Structural edges.** The default tier exports 0 resolved call edges, so the Atlas canvas draws
the default-tier call graph (LOW + MEDIUM) and labels it `structural (LOW name-based + MEDIUM
semantic), may over/under-approximate`; it prints `N MEDIUM / M LOW`; it never prints `resolved` or
`reachable_from_entry` for that set. Canvas reachability is **estimated structural reachability**
(ADR-0009 §Decision 5).

**Data.** No schema change in Atlas v1 (`atlas-data-gap-audit.md` §4). I/O sinks are downscoped;
loop-I/O findings stay in KV1 as heuristics.

**Additional acceptance (binds the Atlas build).**

- **A8 (amended):** first block = canvas; first answer block = answer line below; canvas carries
  entry + HOTTEST + path. Evidence: DOM assertion + committed PNG.
- **A9 (canvas purity):** the canvas element contains no `<table>`, bar/card list or metric
  selector. Evidence: DOM assertion.
- **A10 (state-unconditional):** application, library, no-entry, empty, all-test, capped,
  partial-parse and single-file fixtures render the same canvas + chart stack. Evidence: render matrix.
  (Step 12 replaced the seven-block brief with the four below-graph charts; the stack is unchanged
  across states.)
- **A11 (denominators):** every visible `×`/`%` names its denominator; a blind read must state it
  and must not mistake `× median function` for time. Evidence: `atlas-metrics.md` §10.
- A1/A2/A4/A7 are unchanged and apply to the Atlas page; A2 runs with animation enabled and
  includes a library-shaped repo (pre-registered; the django/postgres holdout stays vaulted until
  the A2 test).

**Explorer.** The legacy 8 stems remain under the labelled `Explorer` divider and the F treemap is
added as `weight-treemap`; all are labelled Explorer-tier and make no answer claim. Per-component
pages remain Explorer-tier evidence.
