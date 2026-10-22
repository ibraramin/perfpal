# Atlas — developer-needs research

**Status:** research input for `map-shipped` and the Atlas build plan. **No implementation.**
**Date:** 2026-10-23 · **Base:** `60dac49` · **Feature:** Perfpal Atlas.
**Founder decision:** D-pf-238 — G (entry-flow) is the core graph; F is not a fallback and not
discarded; F-derived components are candidate supporting elements, included only if this research
justifies them; no mode switch; graph page = 100% graph; metrics/brief below.

## 1. Question

What does a developer actually want to see **alongside** the main graph — actions, metrics, weight
mass, blast radius, noise — and which of those needs justify a supporting element?

## 2. Evidence base and limits

Internal:

- 12/12 valid blind `@vision` reads of the V7 F/G prototypes on cJSON, hyperfine and tmux
  (`docs/evidence/map-vision/v7/f/reads.json`, `g/reads.json`, `COMPARISON.md`).
- The locked maintainer questions and recall targets (`docs/specs/map-intelligibility.md`;
  `docs/evidence/map-data-coverage.md` §2–§4).
- The V7 prototype notes and manifests (`docs/evidence/map-vision/v7/{f,g}/NOTES.md`).

External (prior art, `@librarian`): Sourcetrail, CodeCharta, NDepend, dependency-cruiser, Understand,
Sillito et al. FSE'06, LaToza & Myers ICSE'10, hairball/readability work, split-attention and
progressive-disclosure evidence. Sources are named in §4.

Limits, stated up front:

- The evidence is three **small design repos** (cJSON ~101 files, hyperfine ~53, tmux ~216). The
  holdout (django/postgres) stays vaulted.
- Two of the three are application-shaped; only cJSON is library-shaped. G's cJSON read answers
  "which entry?" not "where do I start?" — the 12/12 count is a **view-legibility** pass, not a
  complete Q1 pass on libraries (`g/reads.json` cJSON[0].json; `COMPARISON.md:19–25`).
- No maintainer task observation, no clickstream, no time-on-task data. The needs model below is a
  synthesis of stated questions, blind reads and prior art — it is a **contract to falsify**, not a
  proven requirement set. §9 names the falsifiers.

## 3. What the V7 reads actually establish

1. **Readers act on text, not geometry.** Every F read took its action from the headline or KV1
   brief, not the treemap (`f/reads.json` O1). The treemap was read as "a wall of test code" on
   cJSON (`f/reads.json` cJSON A/B).
2. **G's application case works.** hyperfine/tmux reads name the entry, the HOTTEST file and a
   concrete function action; the path trace and depth rings are read correctly (`g/reads.json` O2).
3. **G's library case is an honest dead end.** cJSON: one reachable file, five grey callers, no
   forward path; both reads report it (`g/reads.json` O1; `g/NOTES.md` §cJSON). The page needs a
   non-canvas answer for this state, not a layout trick.
4. **Caps are understood when printed.** tmux 20/106 and the `+N more` boxes were read correctly;
   one reader named the cap as a blocker for deeper flow (`g/reads.json` O4). Caps are a trust
   feature, not noise — hiding them would be the defect.
5. **The file cards are the answer surface G readers used.** They carry top functions, cost and
   finding tags (`g/reads.json` descriptions). Moving cards below the canvas is a layout change
   that must be re-validated; the evidence does not automatically transfer.
6. **Units still fail.** One F read misread `83,354×` as `53,354×`/`3,354×` (`f/reads.json` O4);
   G readers invented `ms`/`µs` where the artifact contains only static cost units
   (`g/render_g.py` has no time conversion; `g/NOTES.md` §Units). Abbreviated or bare numbers are
   a live defect even after the V5 unit fix.
7. **Readers ask for remediation context.** F readers wanted the enclosing function name and
   untruncated hints (`f/reads.json` O5). G card rows are full names; the shipped brief must keep
   `path:line` + function + rule title + why.

## 4. What prior art establishes

| Finding | Source | Consequence for Atlas |
|---|---|---|
| Graph tools never stand alone: Sourcetrail pairs graph + search + code snippets + opt-in legend; CodeCharta pairs treemap + metric bar + inspector; NDepend wraps the graph in actions and keeps metric treemaps separate; Understand answers blast radius in an Information Browser. | Sourcetrail DOCUMENTATION.md; codecharta.com user controls; ndepend.com dependency graph/treemap docs; support.scitools.com Information Browser | A below-graph answer stack is the norm, not a compromise. The canvas stays clean. |
| Developers ask 44 question types in four stages: focus → expand → subgraph → across; newcomers start with "where is code for this behaviour?" then "who calls/depends on X?" then "what breaks if I change X?" | Sillito et al., FSE'06 | Entry-first answers focus; blast radius answers change impact; both must be present. |
| Reachability is the hard, frequent core: 460 professionals asked reachability questions >9×/day; 82% rated at least one hard. Static call graphs fail on indirection. | LaToza & Myers, ICSE'10 | Blast radius must be a first-class below-graph block with coverage/caveats, not a hover. |
| Hairballs fail on crossings/overlap; the validated fix is filter + aggregate + skeletal backbone, not more ink. | Dunne et al., IBM JRD; Microsoft "Trimming the Hairball" | Keep the canvas capped, aggregate the tail, and never dump full reachability lists onto it. |
| Split attention penalises separated text+diagram; redundancy penalises duplicated content. Below-graph text must **add** information, not restate topology. | Chandler & Sweller 1992; Ayres & Sweller 2021 | The brief states actions, shares, deltas and caveats — never "node A is left of node B". |
| Progressive disclosure: few key options up front, detail on request; performance tools (pprof/flamegraph) keep the canvas clean and push tables to switchable views. | Nielsen Norman Group; pprof UI | One canvas, one brief, drills for the rest. No metric selector on the canvas. |
| Static call graphs silently miss function-pointer/thread-entry edges by design; tools must disclose unsoundness. | VS Code cpptools call-hierarchy issue | The Atlas must print LOW/structural edge share and never imply runtime truth. |
| Prioritise by activity × quality; disclose noise explicitly with filters or hatched non-indexed nodes. | CodeScene hotspots docs; Structure101 help; Sourcetrail | Production-first ranking + counted demotions + a noise footer. |

## 5. The needs model

| ID | Developer need (plain language) | What answers it | Where it must live |
|---|---|---|---|
| **N1** | *Where do I start?* | entry, HOTTEST file, numbered path, 3–5 ranked actions | canvas (entry/HOTTEST/path) + below brief (actions) |
| **N2** | *What breaks if I touch it?* | reverse include dependents (file), caller/callee counts, coverage | below (blast-radius block) + drill (full list) |
| **N3** | *Where might time go?* | `function_cost_v2`, `× median function`, `top P%`, trip provenance, profile-to-confirm | below (actions/cards) + drill (complete profile queue) |
| **N4** | *Where is the weight?* | production-first file/component cost share; test/bench share as a labelled band | below (weight strip); the treemap is Explorer-only |
| **N5** | *What is missing / safe to ignore?* | parse health, unresolved imports/includes, LOW/MEDIUM edge mix, demoted counts, caps, unreached | canvas caps chip + below noise footer |
| **N6** | *What do I do now?* | plain-language action rows with `path:line`, function, rule title, why, confidence, anchor, drill | below brief (KV1), full list drill |

Out of scope for Atlas v1 (no data, or explicitly non-goal): runtime truth/time, churn/history,
ownership, cross-repo, cross-language linking, build-flag impact. `maintainer-map.md:23` and the
north-star already exclude them.

### N1–N6 → page placement

| Placement | Contents | Rule |
|---|---|---|
| **Canvas** | entry badge, file nodes (name, cost, anchor, HOTTEST, finding annotation `HOT ×N`/`WARN ×N`/`INFO ×N`), depth columns/rings, call edges (forward/back, distinct-pair count), component hulls, unreached column, path trace, caps chip, legend | graph chrome only; no cards, tables, bars, metric panels or selectors; the finding annotation is text, not a colour/size channel |
| **Below graph** | answer line, KV1 actions, weight strip, blast-radius block, file cards, noise footer, drill links | adds information; one encoding per block; state-unconditional |
| **Drill** | complete ranked action list, complete profile queue, full dependents/callers, weight detail | answer-first header; back-link to Atlas |
| **Explorer** | legacy 8 stems + the F treemap (`weight-treemap`) + per-component pages | labelled Explorer; no answer claim |
| **Never** | runtime/time numbers, a second full map, a mode switch, a metric selector on the canvas | hard rule |

## 6. What belongs below the graph (the Atlas brief)

Order is fixed; every block renders in every scan state (zero-state stub when empty).

1. **Answer line (2–3 sentences).** Entry + why that entry; HOTTEST file; the one action to open
   first; the one caveat that most affects trust. No topology restatement.
2. **KV1 — Start here (3–5 actions).** `path:line`, function, rule title, severity + confidence,
   production class, cost anchor, why, drill link. Ranked production-first; complete list linked.
3. **Weight strip (Q3 mass overview).** Top 5 production files and top 3 **production** components
   by `% of scanned static cost`; non-production (test/bench/other) mass as a separate labelled
   band; units + denominator printed. This is the surviving F component (§7) and it is always
   present.
4. **Blast radius (Q2).** For the selected or entry file: `N dependent files` (resolved include
   reverse edges), `% of source files`, `× median dependent count` only when the median is > 0;
   top 5 dependent files; caller/callee file counts from structural call edges; include coverage
   line; full list drill.
5. **File cards.** For the selected/top files: top 3 functions by cost with `top P%`, dominant
   finding rule, file cost + `× median function`, production class, drill link. The G prototype's
   right rail moves here.
6. **Noise & confidence footer.** Parse `ok/partial/failed`, unresolved imports/includes,
   `N MEDIUM / M LOW` structural edges, `N non-production files demoted`, `N of M files shown`,
   `N edges drawn of M pairs`, `N unreached production`, `N unreached non-production`. Counts with
   denominators, no anchors.
7. **Drill links.** Full action list; complete profile queue; full dependents; weight detail; per-
   component pages. Every link resolves to content that shows the action, never a dead affordance.

## 7. Component-inclusion rule (falsifiable, no mode switch)

An F-derived component joins the **Atlas page** only if **all six** hold:

1. **Distinct question.** It answers a pre-registered need N1–N6 that the graph canvas and the
   existing brief cannot. Restating G's cost signal does not qualify.
2. **State-unconditional.** It renders for every scan shape with a zero-state stub. Data changes
   its content, never whether it exists or where it sits. A component that appears only when
   reachability is low is a mode switch in disguise and is rejected.
3. **No second encoding on the canvas.** It lives below the graph or in a drill; it never adds a
   measured colour/size/area channel to the canvas.
4. **Evidence.** 2/2 blind reads on at least two design repos, including a library/test-heavy
   case, state its answer and a concrete action; no read may need another view to make sense of it.
5. **No duplication.** It does not repeat a number G already carries unless it adds a denominator,
   share, blast-radius dimension or production split.
6. **Reversible.** Removing it leaves a valid page; adding/removing it is a design decision, not a
   data-keyed branch.

Applied to the F candidates:

| F candidate | Verdict | Why |
|---|---|---|
| **Weight/treemap mass** | **PASS as a below-graph weight strip (provisional pending V8 strip-specific reads); FAIL as a treemap on the page.** | The *need* (N4, where is the mass) is real and split from G's path cost (`COMPARISON.md` Q3). The *treemap* fails tests 3 and 5 (second encoding, duplicates cost) and test 4 on cJSON (93.5% test wall; production 0.2% unreadable; `f/reads.json` O2). The strip keeps the mass answer without the area hijack. Test 4 is **provisional** for the strip: the V7 reads cover F's treemap/brief, not the redesigned strip; the V8 gate must run strip-specific reads, and the §9 falsifier removes it if they fail. |
| **Production-first area** | **PASS as an ordering rule** in the weight strip and KV1/KV3; **not** as an area encoding. | Production-first is already ratified (V6a KV2) and fixes the cJSON failure (`f/NOTES.md` §cJSON). It is a ranking rule, not a visual channel. |
| **Component cells** | **FAIL as cells; PASS as component hulls + component rows.** | Cells are the treemap's encoding. Component *structure* belongs on the canvas as hulls (approximate, disclosed); component *mass* belongs in the weight strip as rows. |
| **Full F treemap** | **Not on the Atlas page. Kept as `weight-treemap` Explorer stem.** | Respects "not discarded" without shipping a failed hero. Explorer is the labelled info-dump tier (`map-intelligibility.md` §Visual methodology). |

**No mode switch.** Every scan renders the same seven-block page. cJSON's library state changes
labels and content (`library root`, `no forward path`, grey callers) — not the page shape.

## 8. Metric implications (hand-off)

The needs model requires a standardized metric set, not ad-hoc labels. The contract is
`docs/specs/atlas-metrics.md`. The non-negotiables it must carry:

- every measured number has a **unit** and an **anchor with its denominator adjacent**;
- **no time units** (`ms`/`µs`/runtime) anywhere; static cost is never converted to time;
- **one measured encoding** on the canvas (cost); severity/confidence are text;
- **production-first**, with counted demotions, never silent;
- **caps and coverage printed**, never silent;
- **structural edges** are named structural, never "resolved" or "runtime";
- the weight strip uses **% of scanned cost**, not area, and keeps non-production (test/bench/other) mass in a labelled
  band;
- huge multiples use thousands separators and a full-value `title`/aria, never an abbreviated
  number that can be misread.

## 9. Falsifiers / what would change this

1. **First-screen falsifier.** A fresh blind read of the shipped first screen must name Q1 — the
   entry, the HOTTEST file and the representative path — **without scrolling** (the canvas carries
   all three). The one `path:line` action is allowed in the first scroll (answer line + KV1). If
   the canvas cannot name Q1, the graph-first model fails and the brief must move above the canvas
   (founder decision).
2. **Library falsifier.** A library-shaped holdout (single-file C lib or Rust lib crate with no
   bin) where the canvas yields no action unaided. The below-graph brief is allowed to carry the
   answer; if it cannot, Atlas needs an explicit reverse-context state — a founder decision, not a
   silent fallback.
3. **Weight-strip falsifier.** If no blind read uses the strip to answer N4 or to choose a
   different action than KV1, remove it (test 1/5 fail). If readers compare its `% of scanned cost`
   to the canvas `× median function` as if they were the same scale, drop the percentage-share
   comparison or restate it as a share of a named denominator.
4. **Anchor falsifier.** Any read that misstates the unit or denominator (e.g. reads `× median
   function` as milliseconds, or `83,354×` as `53,354×`) reopens the formatting rule.
5. **Noise-threshold falsifier.** If the noise footer makes readers distrust the whole map rather
   than calibrate it, reduce it to three lines (parse health, edge confidence, caps) and move the
   rest to a drill.
6. **A2.** The founder timed test on the holdout (application + library class) is the decisive
   gate; no agent-only substitute closes it (`map-intelligibility.md` A2/A2b).

## 10. Decisions recorded

- **D-pf-239 (session decision, founder ratification pending):** the Atlas page is a fixed stack:
  the canvas plus seven below-graph blocks (answer line, KV1 actions, weight strip, blast radius,
  file cards, noise footer, drill links). The F weight mass ships as an always-on below-graph
  strip; the F treemap is Explorer-only; production-first is an ordering rule; component cells are
  hulls/rows, not cells. No mode switch. The six-test inclusion rule in §7 is the standing rule for
  any future supporting component.
- **D-pf-240 (session decision, founder ratification pending):** the metric contract is
  `docs/specs/atlas-metrics.md`; no time units, denominator-adjacent anchors, one measured encoding
  on the canvas, structural edges labelled as such.

## 11. References

- `docs/evidence/map-vision/v7/COMPARISON.md`, `f/reads.json`, `g/reads.json`, `f/NOTES.md`,
  `g/NOTES.md`, `README.md`.
- `docs/specs/map-intelligibility.md` (KV slots, acceptance, V4–V6a history);
  `docs/specs/maintainer-map.md` (three questions, honesty contract);
  `docs/specs/map-animation.md` (declarative animation contract);
  `docs/plans/map-shipped.md`; `docs/adr/ADR-0002-visualization.md`;
  `docs/adr/ADR-0008-map-idiom-decision.md`.
- `docs/evidence/map-data-coverage.md` §4–§8 (H1/H2, field decisions).
- Prior art: Sourcetrail DOCUMENTATION.md · codecharta.com/docs/visualization/user-controls ·
  ndepend.com/docs/visual-studio-dependency-graph · github.com/sverweij/dependency-cruiser ·
  support.scitools.com Information Browser · Sillito et al. FSE'06 (doi 10.1145/1181775.1181779) ·
  LaToza & Myers ICSE'10 (doi 10.1145/1806799.1806829) · Dunne et al. IBM JRD (doi
  10.1147/JRD.2015.2411412) · Microsoft "Trimming the Hairball" · Chandler & Sweller 1992 (doi
  10.1111/j.2044-8279.1992.tb01017.x) · NN/g progressive disclosure · pprof UI.
