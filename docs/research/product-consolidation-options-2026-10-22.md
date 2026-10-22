# Product consolidation — options brief (2026-10-22)

**Status:** options only; no implementation. Founder agreed the three-consumer model and the
order **agent report → human map → JS/TS → paper** (paper parked until after JS/TS).

## Agreed model

```
scan core → metrics.json (canonical, schema-versioned)
              ├── A  agent report   — ranked actions + evidence + fix hints; downstream eval
              ├── H  human map      — one object + brief strip; A2 + vision gates
              └── P  paper          — parked; frozen schema + manifest; reproducibility gate
```

Rules: one canonical artifact; every consumer derives from it; separate acceptance gates; no
consumer reads another's presentation; static analysis stays candidates, never certainties.

---

## Q3 — Agent report: how to package it

### What the agent needs

A token-efficient, deterministic, **action-ranked** report: top actions with `path:line`, rule,
why, evidence, confidence, a curated fix hint, and an impact anchor; a complete index for
retrieval; a cost-ranked profile queue; and a data-quality header (parse health, unresolved
edges, LOW-confidence caveats). No charts, no prose padding, no visual dependency.

### Options

| # | Option | What it is | Pros | Cons |
|---|---|---|---|---|
| A | **Dedicated always-on artifacts** | Every scan emits `agent-report.md` + `agent-actions.json` alongside `metrics.json` and the human report | Clean separation; token-efficient; stable machine contract; testable by a downstream eval; agents find it by default | +2 artifacts; output contract grows; must stay in sync (both are pure functions of `metrics.json`) |
| B | **Mode flag** | `--agent` switches/selects the report | One directory; user chooses | Hides the separation; multiplies goldens/gates; agents may never see it; flags are a UX tax |
| C | **Extend `metrics.json`** | Add `actions[]` to the canonical JSON | One artifact; no duplication of the source | Bloats an already-large canonical file; no narrative/fix hints; schema churn; high LLM token cost |
| D | **Separate render command** | `perfpal render --agent <dir>` | True pure-function consumer; regenerate without rescan | Extra command; easy to forget; scan should not require a second step |
| E | **Reuse `findings-detail.md` + `profile-queue.md`** | Treat existing artifacts as the agent report | Zero new work | Rule-grouped, not action-ranked; no fix hints; not token-efficient; no agent gate — the status quo that underbuilt job 2 |

### Recommendation — **A, implemented as D's pure renderer**

- **Always emit** (no flag): `agent-report.md` (LLM-facing) + `agent-actions.json` (machine
  contract). The scan already emits the human report; the agent report is cheap text and must be
  discoverable without a flag.
- Both are **pure functions of `metrics.json`**, like `report.md` and `charts/` — byte-deterministic,
  regenerable, never a second source of truth.
- `metrics.json` stays canonical; do **not** add a second ranked list to it.
- Keep `findings-detail.*` and `profile-queue.*` as human/explorer artifacts; the agent report is a
  distinct consumer view (same view model, different template/ranking).

**`agent-actions.json` (machine contract, sketch):** `schema_version`, `tool`, `scan`,
`data_quality`, `rank_key`, `actions[]` (stable `action_id`, rank, rule_id, title, path, span,
symbol, severity, confidence, production/path_class, why, evidence, `fix_hint`, anchors),
`complete_action_count`, `profile_queue[]`. Complete, deterministic, no silent caps.

**`agent-report.md` (LLM-facing, sketch):** header (repo/commit, schema/tool, parse health,
rank key, caveats) → *Start here* (top 10 with why/evidence/fix hint/anchor) → *Complete action
index* (one line each) → *Profile queue* (top N + link) → *What this scan cannot see*.

**Fix hints:** add one curated sentence per heuristic to the built-in catalog (67 rules), joined by
`rule_id`; never invent a fix in the renderer. "Candidate; verify with a profiler" stays in the
header.

**Acceptance gate:** deterministic byte-identical; a downstream eval (agent given the repo +
`agent-report.md` fixes the right issue; measure top-1/top-3 hit, hallucination, time-to-fix);
token budget for the top report; no chart dependency; every action has `path:line` + evidence.

**Weakest assumption:** analyzer precision is sufficient for an agent to act without triage. If
not, the artifact is a **ranked triage queue**, and the eval must say so.

---

## Q4 — Human map: what is the one object on the first screen?

### First-screen contract (proposed)

One screen at 1440×900: a **component-level treemap** as the hero, a **KV1 brief strip** as the
annotation layer, and a one-line plain-language answer. `#map` opens with the answer, not a chart
index. Explorer is opt-in.

- **Treemap:** top-50 components; area = share of static cost; colour = a single ramp on the
  **same** cost metric (redundant encoding) so the object stays one-metric; hottest cells outlined;
  component labels; every cell links to a component drill page.
- **Brief strip:** 3–5 ranked actions (reuse KV1: `path:line`, function, rule title, why,
  severity/confidence, anchor), each linked to its cell/drill. This is the north star's
  "annotations on the map", not a peer dashboard.
- **Drill page:** click a component → file-level treemap + a **focus + 1-hop** dependents list/DSM
  (Q2). The component DSM is the Q2 hero here, not on the first screen.
- **Explorer:** opt-in (`--explorer` or a linked `explorer.html`); the 8 stems remain for power
  users, never the default path.

### Options

| # | Option | First screen | Pros | Cons |
|---|---|---|---|---|
| A | **Treemap-only** | Treemap + brief | One object; whole codebase; Q1/Q3; reuses ADR-0008 L2 | Weak Q2; needs a drill anyway |
| B | **DSM-only** | Matrix + brief | Best Q2; reuses ADR-0008 L0; cheapest spatial upgrade | Weak Q1/Q3; not a "picture of the codebase"; first screen answers Q2, not Q1 |
| C | **Two objects same screen** | Treemap + DSM | Each question has its object | Dashboard risk returns; two heroes; convolution |
| D | **Toggle/modes** | Treemap/DSM switch | Flexible | Two products shipped; zero-JS toggle complexity; ambiguous first screen |
| E | **Brief-only** | KV1 + drills | Cheapest; KV1 works | No visual abstraction; contradicts "I can't see the codebase" and the north star |
| F | **Treemap hero + brief strip + DSM drill** | Treemap + brief; DSM one click | One object; Q1/Q3 answered at a glance; Q2 answered properly on drill; matches the north star | Build cost: new repo treemap renderer, labels, drill navigation |

### Recommendation — **F** (orchestrator recommendation; superseded by the founder's G-core decision)

> Founder decision 2026-10-22: **G is the core graph**; F is not a fallback and not discarded.
> F's components (weight mass, production-first area, component cells) remain candidate supporting
> elements, included only if the developer-needs research justifies them. The F material below is
> kept as the component brief, not as the hero.

- **Hero:** component treemap (area = cost, colour = the same cost ramp; severity as outlined
  annotations, not a second colour channel).
- **Annotation layer:** KV1 brief strip.
- **Q2:** one click away — component DSM + focus + 1-hop file list/DSM.
- **Q3:** the area itself; the exact function list lives in the drill/agent report, not a peer bar
  chart.
- **Explorer:** opt-in only.

**Why not B?** The first screen must answer Q1 ("where do I start") and show the codebase as a
whole; a matrix answers Q2 best but is not a "where to start" object. **Why not A alone?** Q2 is a
graph property; it needs the DSM drill. **Why not C?** Two heroes on one screen is how the current
dashboard happened.

### What happens to the current views

| Current | Fate |
|---|---|
| KV1 brief | **Kept** as the brief strip + seed of the agent report |
| KV2 bars | Retired from the first screen; the Q2 drill (DSM + focus+1-hop) replaces it; bars may remain in Explorer |
| KV3 bars | Retired from the first screen; profile queue becomes a drill table + agent report; bars may remain in Explorer |
| 8 Explorer stems | Kept, **opt-in**; not in the default reading path |
| V6a KV2 production-first | Still useful for drill ordering/evidence; not the first screen |

**Acceptance gate:** vision gate on the first screen + drill; A2 timed test on the holdout (10
minutes, 3 actions); one object/one metric; Explorer opt-in; determinism/contract.

**Weakest assumption:** a treemap is the right "whole codebase" abstraction for a maintainer. If
the founder's mental model is "what connects to what", the DSM-first variant B may win; the test is
the A2 run.

---

## Q4 addendum — Option G: entry-flow map (founder direction, 2026-10-22)

The founder did not select F. Proposed instead: **entrypoint at the centre (say `main`), files
branch out, each file carries a small card listing its major functions/components, component
"section" shapes overlap the files to show what general work they do, and the graph animates the
program's progression from the entry to the output.** This is a *program-story* map, not a weight
map. **Founder correction (2026-10-22): G is the core graph; F is not a fallback and not
discarded — F-derived components are candidate supporting elements, included only if
developer-needs research justifies them. No mode switch.**

### Formulation

- **Nodes:** files (top level), aggregated from function-level call edges. For very large repos,
  component nodes first, then a file drill (honest cap: top 50 nodes, `showing top S of M`).
- **Entry policy:** production `main` if one exists (tmux, hyperfine); otherwise the highest fan-in
  production `lib-root` (cJSON has no production `main`; 628 lib-roots). Label the choice on the
  view: `entry: main (tmux.c:…)` or `entry: library root (cJSON_Parse)`.
- **Edges:** file→file **resolved call edges** (caller → callee), direction outward, grouped into
  BFS depth rings from the entry. Cycles/back-edges styled distinctly. The default tier's call
  edges are all **LOW confidence**, name-based (`graphs.call_graph.edges`); label that.
- **File card:** path, `path_class`, cost anchor, top 3–5 functions by `function_cost_v2`/fan-in,
  top finding rule; the card is the detail layer under the node.
- **Section shapes:** component/domain hulls around the files, labelled by component path prefix.
  Current components are directory-prefix-based (tmux: 4, cJSON: 21, often test-dominated), so
  production-first filtering and a better domain derivation are part of the work.
- **Encoding:** one primary measured metric = cost (node/card heat); severity as outlined markers;
  edges are structural (call counts, not a second performance metric); one legend.
- **Progression:** v1 **static numbered path trace** (entry → hottest/representative path, steps
  1→2→3), not animation. See the animation constraint below.
- **Q2:** the forward flow does not answer "what breaks if I touch it"; keep the component DSM +
  focus+1-hop dependents drill for Q2. Do not overload the flow map with reverse edges.
- **Unreached tail:** files/functions not reachable from the entry are shown greyed with a count;
  that is an honest signal (libraries, tests, dead code), not a failure.

### Data reality (design repos, measured)

| Repo | Files | Functions | Call edges (all LOW) | File-level call pairs | Reachable from entry |
|---|---:|---:|---:|---:|---:|
| tmux | 216 | 3,189 | 6,731 | 1,455 | 1,556 |
| hyperfine | 53 | 353 | 297 | 92 | 342 |
| cJSON | 101 | 1,055 | 587 | 73 | 659 |

The story is feasible on small/medium repos; tmux's 1,455 file pairs require a top-50 cap and edge
aggregation. C function pointers/macros are unresolved; the map must say "static, LOW-confidence
call edges" and not imply runtime truth. The holdout remains vaulted for the final A2 test.

### Animation constraint

ADR-0002 and the project invariants are **static, zero-JS, byte-deterministic**. A moving graph
either violates that or becomes a separate opt-in artifact. Recommendation:

1. **v1 static** with a numbered path trace — the progression without motion; print-safe, vision-
   gateable, readable in the A2 test.
2. **v2 optional** CSS/SMIL step-through or pre-rendered frames in the Explorer, only after the
   static map passes A2; not in the default report, not in the A2 gate.
3. Making animation a core feature requires an ADR-0002 amendment and a determinism story; do not
   bundle it into the v1 scope.

### Spike before committing

1. Data check on design repos only (no holdout): entry policy, reachability, file-level edge
   density after the top-50 cap, component sizes, cycles.
2. Prototype G on cJSON + hyperfine (static radial/BFS, file nodes + function cards + component
   hulls + numbered path trace); tmux as the density stress case.
3. Two fresh blind `@vision` reads per prototype + a quick "what would you do first" check; build a
   simple F treemap on the same repos for comparison.
4. Pick G or F with evidence; only then amend ADR-0008/spec and build.

**Weakest assumption for G:** the static call graph is complete enough to tell a true story. If the
spike shows fragmented flow, G becomes a "reachable/estimated flow" view, and F is the more honest
hero. The spike decides.

---

## Sequence and gates

1. **Agent report** (first): artifact contract + fix hints + ranking + renderer + downstream eval.
2. **Human map** (second): spec/ADR-0008 amendment (one object, one metric, brief strip, Explorer
   opt-in) → treemap + brief + DSM drill → vision + A2.
3. **JS/TS** (third): fix the aggregator's duplicated per-language families before adding two more
   languages (external review §6).
4. **Paper** (last): parked; frozen schema + manifest; reproducibility gate.

## Open decisions

1. Agent report: approve **A** (always-on dedicated artifacts, pure renderer)? Any objection to
   `agent-report.md` + `agent-actions.json` as the names?
2. Human map: approve **F** (treemap hero + brief strip + DSM drill)? If not, B is the fallback.
3. Treemap colour: **same-cost ramp** (strict one-metric) or **severity colour** (requires a spec
   amendment to allow one primary + one annotation channel)?
4. Explorer: opt-in flag vs separate `explorer.html` link vs delete entirely?
