# Handoff — map-intelligibility V4 DONE (dual-APPROVED)

## Objective
**Perfpal's maintainer birdseye map is the core feature** (`docs/NORTH-STAR.md`): one visual
answering where to look first, what breaks if touched, and where time plausibly goes, with
findings as annotations. **V4 shipped the V3-chosen key views** and the two additive schema
fields; **V5 (two blind reads per key view) is the next node**, followed by the V6 founder
timed test (including the django/postgres holdout).

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1–I5 ✅ but A5 ❌ (not done);
successor `map-intelligibility`: **V0 ✅ V1 ✅ V2 ✅ V3 ✅ V4 ✅**; V5–V8 queued.
JS/TS backends + paper queue behind the map.

## Alignment (L6)
- **Goal restated:** a CLI emitting heuristics + the maintainer birdseye map (core) +
  LLM-consumable metrics (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; the map existed and was deterministic, but the founder's A5 test
  failed on intelligibility. V0–V3 diagnosed and chose idioms; **V4 is the first implementation
  of the locked slots** and the schema support they need.
- **How this session serves the north-star:** it turns the tested brief/bars/bars idioms into
  the shipped first screen and adds complete, deterministic drill paths, so the V6 founder test
  judges the intended views rather than prototypes. **Drift: NONE.**
- **Holdout:** django/postgres were not rendered, read, or used as data (V6/A2b intact).

## This session — V4 implementation
### Commits
| Commit | Scope |
|---|---|
| `bc887ef` | schema 1.7 — `files[].path_class` + `stats.functions[].trip_count_source`/`known_trip_value`, classifier, provenance, both schema copies, docs, field tests, goldens |
| `de93292` | key views — KV1 brief, KV2/KV3 bars, Explorer tier, `profile-queue` + ranked `findings-detail`, output contract |
| `0bd6d08` | keep `ARCH-001`/`ARCH-003` in KV1 (only `ARCH-002` suppressed), full KV2 paths, plural counts |
| `ae50e61` | KV1 rank-key line, same-location notes, demoted bars, `Key views` nav |
| `f896631` | dual bar scale (ranked vs demoted) so outliers cannot flatten the ranked bars |
| `882ddce` | review round 2 — `report.md` KV3 parity, exact `top P%` anchors, cap disclosures, docs/tests |
| `2ec547c` | V4 vision evidence (`reads.json`, `manifest.json`, 12 PNGs) + shipped-key recall reproducer |
| EOL | state/handoff/CHANGELOG |

### What shipped
- **KV1 Start here** — text-first ranked brief, top 5 inline with rank, `path:line`, function,
  rule title, severity/confidence, path class, full rationale, cost unit + anchors; complete
  list at `findings-detail.html#ranked-actions`. Ranking: production → severity →
  confidence → containing-function cost → `finding_id`; only `-ARCH-002` component aggregates
  suppressed (counted).
- **KV2 Blast radius** — horizontal dependents bars, top 8, full paths, `N dependents · % of
  scanned files · top P%`, `× median` only when median > 0, `resolved N of M`; **no
  production-first demotion** (V2 lock; rows label their `path_class`).
- **KV3 Profile queue** — horizontal cost bars, top 8 ranked + 2 labelled demoted, dual bar
  scale, unknown/known-trip badges, startup/no-loop demotions counted; complete list at
  `profile-queue.html`.
- **Tiering/contract** — first screen is KV1; all 8 legacy stems under `<section id="explorer">`
  (`Explorer` nav + `Key views` nav entry); new `profile-queue.md/.html`; `report.md` §1
  carries the ≤5-row briefs and §5 is Explorer; output contract + README updated.
- **Schema 1.7** — additive only (no `required` list grew), optional on the wire; classifier
  order test → example → bench → fixture → generated → vendor → production; `scripts/`,
  `tools/`, `ci/`, `.github/` are `bench`; `compat/` is production; trip provenance
  `literal|partial|unknown` + clamped `known_trip_value`.
- **V4 fix list** — ARCH-002 suppressed from KV1 (not an analyzer-message edit); full
  rationale (no `…`); one KV2 action line; `scripts/` decision; 900px card rule recorded.

### Vision gate
9 blind key-view reads + re-reads + final per-repo gate: **APPROVED** for tmux, cJSON and
hyperfine (all key views + first screens). Evidence: `docs/evidence/map-vision/v4/reads.json`,
`manifest.json`, 12 PNGs. Defects found and fixed: unstated KV1 rank key, duplicate-looking
rows, zero-length demoted bars, then a flattened cJSON KV3 scale (dual-scale fix).

### Recall (do not overclaim)
Shipped-key ranks: tmux Q1 **209/7543**, Q3 **820/3189**; hyperfine Q1 **172/346**, Q3
**58/353**; cJSON Q1 **37/835**, Q3 **60/1055** — still not inline. KV2 is inline in all three
(tmux 1/216, hyperfine 6/53, cJSON 4/101). **A1b is not claimed closed on severity**; V7 owns
the KV1 rank-key re-triage; V5 tests the KV3 no-loop guard. Reproducer:
`docs/evidence/map-vision/v4/recall_ranks_v4.py`.

## Verification
| Claim | Command / evidence | Result |
|---|---|---|
| Schema 1.7 additive + byte-identical | `cargo test --locked --test schema_1_7`; `cmp docs/schema/metrics.schema.v1.7.json src/schema/metrics.schema.v1.7.json` | **4/4 pass**; identical |
| Key views + drill lists | `cargo test --locked --test key_views`; rendered `out-v4-{tmux,hyperfine,cJSON}` | pass; artifacts present |
| Vision gate | `docs/evidence/map-vision/v4/reads.json` → `final_gate` | **APPROVED** (3 repos, 9 views + 3 first screens) |
| Full gate | `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets -- -D warnings`; `cargo test --locked`; `cargo deny check licenses` | **0 / 0 / 52 suites 0 failures / licenses ok** |
| Feature gate | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings`; `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | exit 0 |
| Golden double-regen | `PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens` twice; `git status` after | byte-identical, no diff |
| V3 gates | `check_v3.py`; `recall_ranks.py` | PASS; 9/9 |
| V4 recall | `recall_ranks_v4.py` | 9/9 shipped-key match |
| Dual review | `@reviewer` + `@requirements-reviewer` round 2 | **both APPROVED** |
| Holdout / plan state | grep V4 evidence; `docs/plans/map-build.md:4` | no django/postgres data; `map-build` **active** |

## Risks / Open
- **R-pf-105 (medium):** shipped-key KV1 ranks are still not inline (209/172/37 for Q1; Q3
  820/58/60) → **V7** re-triage; **V5** tests the KV3 no-loop guard.
- **R-pf-104 (high):** map intelligibility; founder timed A2 remains the gate (V6).
- **Founder decisions before V6:** KV2 production-first amendment (cJSON `cJSON.h` is otherwise
  buried by unity test headers); randomized pair re-run (KV1 was B/B/A and partly confirmatory).
- **V4 non-blocking follow-ups:** KV2 `top P%` vs `% of scanned files` wording risk (V5
  observation); mixed v1/v2 rank scale for legacy documents; KV1 comparator recomputes the
  containing function (O(N log N·k)); `report.md` KV2 grammar “1 of 1 files”; the pre-existing
  `perfpal perfpal` meta-line duplication.
- **Blockers unchanged:** B-pf-001 (contact), B-pf-002 (secrets) for M5.

## Next Actions — NEW SESSION SIGNPOST
1. **New session** with: **plan `map-intelligibility` (V5), specs `map-intelligibility` +
   `maintainer-map`, evidence `docs/evidence/map-idiom-ab.md` + `map-data-coverage.md` §8 +
   `docs/evidence/map-vision/v4/reads.json`, handoff `state/handoff/latest.md`.**
2. **V5:** for each key view, two fresh blind `@vision` reads on tmux/hyperfine/cJSON (no source,
   no sibling views); read-aloud description first, then strict JSON
   `{question_answered, action, blockers, confidence}`; iterate until both name the question and
   one concrete action; commit transcripts. The shipped views and the V4 recall ranks are the
   baseline.
3. **Before V6:** founder decides the KV2 production-first amendment and whether to run the
   randomized pair re-run; V6 includes django/postgres (holdout).
4. **Do not mark `map-build` done.** M5 needs B-pf-001 + B-pf-002 (founder-owned).
5. **Do not claim A1b closed on severity.** V7 re-triages the KV1 rank key.

## Change set (this session)
- **Code:** `src/reporter/views.rs` (new), `src/reporter/mod.rs`, `src/aggregator/path_class.rs`
  (new), `src/aggregator/mod.rs`, `src/schema/mod.rs` + `metrics.schema.v1.7.json`
  (docs + embedded copy), `src/cli/mod.rs`, test fixtures, `tests/key_views.rs` (new),
  `tests/schema_1_7.rs` (new), `tests/i4_annotations.rs`, `tests/p6_hardening.rs`, goldens (4 trees).
- **Docs:** `docs/specs/map-intelligibility.md` (V4 notes), `docs/specs/maintainer-map.md` (V3/V4
  mirror), `docs/report/output-contract.md`, `docs/schema/metrics-json-v1.md`, `README.md`,
  `CHANGELOG.md`, `docs/evidence/map-idiom-ab.md` (V4 re-measure note).
- **Evidence:** `docs/evidence/map-vision/v4/` (12 PNGs, `reads.json`, `manifest.json`,
  `recall_ranks_v4.py/.json`, `render_v4.py`).
- **State:** `state/session-state.json` (D-pf-231, R-pf-105, A-pf-106, phase),
  `state/archive/ledger.jsonl` (hot-state demotions), `docs/activeContext.md`, this handoff.
- **No new dependency; no JS; no new SVG chart.**
