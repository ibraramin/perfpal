# Handoff — Latest

## Objective
**Map-intelligibility V2 is DONE and dual-APPROVED (docs-only methodology + spec lock).**
Three answer-first view slots are now locked — **KV1 Start here** (Q1, severity/rank),
**KV2 Blast radius** (Q2, coupling; include-based only in the default tier), **KV3 Profile
queue** (Q3, cost) — each with one primary encoding, mandatory units + scan anchors, an
explicit `Explorer` tier for the 8 legacy stems, and a decided field set for V4. The next node
is **V3 (idiom A/B)**, then V4 → V5 → V6 → V7 → V8. `map-build` remains **not done** and has
no I6. M5 still waits on `B-pf-001`/`B-pf-002`.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1–I5 ✅ but A5 ❌; successor
`map-intelligibility`: **V0 ✅ V1 ✅ V2 ✅**; V3–V8 queued. JS/TS backends + paper queue behind
the map.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI emitting heuristics + the maintainer birdseye map
  (core feature) + LLM-consumable metrics (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; the map exists, is deterministic, and the V0/V1 evidence isolated
  H2 (ranking/labels/selection) as the dominant failure. V2 converts that diagnosis into a
  locked acceptance view set before any code is written.
- **How this session serves the north-star:** it prevents another round of polished-but-unreadable
  charts by fixing the *contract* first (question, action, encoding, unit, anchor, field), so
  V3/V4 can only choose idioms inside a slot. **Drift: NONE.**

## This session — V2 lock
### Locked acceptance view set
| Slot | Question | Primary encoding | Unit + anchor | Action it enables |
|---|---|---|---|---|
| **KV1 — Start here** | Where do I look first? | severity (ranked list position) | rank #k; cost `static cost units (v2)`, `× median function`, `top 1%` | open/profile `path:line` first |
| **KV2 — Blast radius** | What breaks if I touch it? | coupling (bar = dependent-file count) | `N dependents`, `% of scanned files`, `× median dependent count`, `top 1%` | do not edit directly; expect N dependents to rebuild/re-import |
| **KV3 — Profile queue** | Where might time plausibly go? | cost (bar = `function_cost_v2`) | `static cost units (v2)` / `(v1)` fallback, `× median function`, `top 1%`, trip flags | profile the named function first (candidate, confirm) |

### Binding rules (full text in `docs/specs/map-intelligibility.md`)
- **First screen = KV1** in plain language; the workspace node-link is not on it.
- **All emitted stems under `Explorer`:** `#architecture` is **renamed** (heading + nav label,
  `#explorer`), not duplicated. Per-component pages stay explorer-tier and get a label when
  reached from a key-view link. `layer-findings` may not claim "density" without a denominator.
- **Units + anchors mandatory** for every measured number (`× median`, `top 1%`,
  `% of scanned files`); ordinal ranks exempt; the 32-char chart-label cap does not apply to
  key-view text rows.
- **No silent caps:** KV1 = 3–5 inline + complete `findings-detail` list; KV3 = complete
  `profile-queue` list ranked by cost within the non-demoted set.
- **Default-tier honesty:** KV2 claims include blast radius only (`0` non-LOW call edges) and
  prints `resolved N of M include edges`.
- **Production-first, never silent:** `path_class` demotion + counted files; KV3 also demotes
  `main/script` and no-loop functions (never excludes them; prints both counts), labels
  unknown-trip estimates, and flags known literal trips (`known trip ×N — verify it actually runs`).
- **One encoding:** KV1 severity, KV2 coupling, KV3 cost; no stacked colour/size channel.
- **No orphan views:** each slot names its question + action; everything else is Explorer.

### Field decisions (`docs/evidence/map-data-coverage.md` §8)
- **V4 additive (schema 1.6→1.7):** `files[].path_class` + `stats.functions[].trip_count_source`
  / `known_trip_value` (optional on the wire; absence = not computed).
- **Chart-side or deferred (with triggers):** `stats.cost_summary` (median/p95 computed once),
  `components[].metrics.coupling_in/out` (reverse include in-degree per file),
  `graph_root` (unusable at 0 resolved call edges + R-pf-098), density guard (Explorer-only),
  uniform call-edge tier (deferred).
- **Why trip provenance became REQUIRED:** `i5-map-triage.json` documents two cost FPs the
  original simple lock could not catch — django `cache.py` (literal-10000 retry → 10,000×
  artifact) and tmux `format.c` (branch-count dispatcher). Founder approved the flip; the guard
  is no-loop demotion + known-literal flag + unknown-trip badge, with the V5 falsifier recorded.

### Reviews
- **Code round 1:** CHANGES_REQUESTED — MAJOR 1 (KV3 known FPs unguarded), MAJOR 2 (KV3 had no
  A1b completeness mechanism) + 8 minors. All fixed; trip-provenance flip founder-approved.
- **Code round 2:** **APPROVED**; minors closed (counts, v1-fallback unit, trip field shape,
  A8 gating, CHANGELOG drift, KV2 target assignment, component-page label, tmux wording).
- **Requirements:** **APPROVED**, 8/8 original bullets.

## Verification
| Claim | Command / evidence | Result |
|---|---|---|
| Docs-only (no code/golden) | `git diff --quiet HEAD -- src tests/golden` | exit 0 |
| 3 slots in both specs | `grep -c 'KV[123] —' docs/specs/map-intelligibility.md` / `maintainer-map.md` | **3 / 3** |
| Trip fields locked | `grep -q 'trip_count_source'` in spec + evidence | pass |
| Explorer rename, no duplicate | `grep -q 'renamed' docs/specs/map-intelligibility.md` | pass |
| No stale `% of TUs` unit | `! grep -q 'TUs' docs/specs/map-intelligibility.md docs/specs/maintainer-map.md` | pass |
| Task gate | `task-gate.sh` (9 commands: diff, slot counts, fields, rename, profile-queue, no-TUs) | **9/9 PASS** |
| Review (code) | reviewer round 2 | **APPROVED** |
| Review (requirements) | requirements-reviewer round 2 | **APPROVED** (8/8) |
| `map-build` not done | `docs/plans/map-build.md:4` | `active` |
| No implementation | `git status --porcelain` | 4 docs files only (before EOL state edits) |

## Risks / Open
- **R-pf-104 (high):** V2 lock applied; founder timed A2 remains the gate (V6). **V5 is the
  falsifier for the KV3 guard** — if the django literal-10000 / tmux dispatcher FPs are still
  named, switch to per-iteration cost or the two-list variant.
- **R-pf-098 (high):** entry detection over-marks lib-roots; key views do **not** use entry
  strata. V7 founder decision.
- **R-pf-099/R-pf-100 (medium):** production-path ranking + tiny-scope density — V7.
- **R-pf-103 (low):** no semantic-tier committed-golden lock.
- **Blockers unchanged:** B-pf-001 (contact), B-pf-002 (secrets) for M5.

## Next Actions — NEW SESSION SIGNPOST
1. **New session** with: **plan `map-intelligibility` (V3), specs `map-intelligibility` +
   `maintainer-map`, handoff `state/handoff/latest.md`, evidence
   `docs/evidence/map-data-coverage.md` §8 + `docs/evidence/map-vision-diagnosis.md`.**
   V2 docs lock: `45fa514`.
2. **V3 first:** prototype 2–3 idioms per locked slot (KV1 text-first brief vs ranked cards;
   KV2 blast-radius bars vs matrix; KV3 cost bars vs treemap); run the blind `@vision` A/B on
   tmux/hyperfine/cJSON; choose and record "what would change it". Keep django/postgres unseen
   for V6.
3. **V4 → V5 → V6 → V7 → V8:** implement the two additive fields + views; two blind reads per
   key view; founder timed holdout; re-triage R-pf-098/099/100; closeout.
4. **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002 (founder-owned).

## Change set (this session)
- **Modified docs:** `docs/specs/map-intelligibility.md` (locked view set + A7/A8),
  `docs/specs/maintainer-map.md` (mirror + reading path), `docs/evidence/map-data-coverage.md`
  (§8 field decisions + triggers), `CHANGELOG.md`.
- **State:** `state/session-state.json` (D-pf-229, phase, A-pf-104), `docs/activeContext.md`,
  this handoff.
- **No code/golden change**; `map-build` stays active.
- **Commits:** docs lock `45fa514`; this EOL commit.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to divergence-instrument
then map-first core; first periodic audit 2026-10-16 (counter 0).
