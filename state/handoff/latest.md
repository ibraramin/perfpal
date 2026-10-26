# Session handoff — Atlas Story (D2 + S1 + S2 + S3 + S4)

- **Date:** 2026-10-26 · **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (`feat/atlas-sim`)
- **Track:** Atlas Run · **Status:** **S4 built and gated at `61ef8c9`** (dirty 0, 892 tests, 302/302,
  3k 44/44, `verdict=PASS`; evidence `97c4cda`); the final 4-lane panel is running. **Not pushed,
  not merged.**
- **Detail:** `state/tracks/atlas-sim/handoff.md` · **Requirement:** `docs/specs/atlas-story.md`
  (sealed) · **Plan:** `docs/plans/atlas-s4.md` · **Probes:** `docs/atlas/core-probes.md` (P6–P8)
- **State:** `state/session-state.json` · **Evidence:** `docs/evidence/atlas-sim/`

## Objective

Turn the Atlas map into a **temporal showcase of a codebase**: entry → stages → loops → I/O → stop,
with **the payload crossing each file boundary labelled**; vibecoder-first, senior drill; static
inference labelled; fed only by Perfpal core artifacts. S4 makes the story scale: **directory scenes
at 3k files**, the **whole-repo KV3 queue**, a **README purpose sentence**, scene-aware start-here and
a scale-aware trust line, with the 3k F2–F7 gate.

## Phase

| Slice | Scope | Status |
|---|---|---|
| S1 | P4 stages + P5 `atlas-layout.json` (`layout_version: 1`); sim consumes core JSON | **done** |
| S2 | P3 signatures + P1 call payloads → `flow`, edge labels, payoff beats | **done** |
| S3 | P2 I/O effects → stores + trust line | **done** |
| S4 | P6 purpose + P7 KV3 queue + P8 scenes; 3k scale/payoff; F1–F7 | **done and gated** (`5f13b04`) |

## Decisions

- **D-pf-255 (S4):** P6 `purpose` (README first prose paragraph, structural fallback), P7 `profile`
  (authoritative KV3 order, top 12 production non-demoted rows + counts) and P8 `scenes` (depth-2
  directory partition, 24-scene/5-file caps, production-cost-first, omitted totals, shared
  `atlas::file_cost`, `nodes[].scene`/`actions[].scene`) are **story-only, additive, `layout_version:
  1`**. Limits: README first-only; 12-row queue; 24/5/depth-2 scenes; scene cost is static, not
  runtime; a 3k scene can be omitted by the cap (counts disclosed); the 3k fixture is synthetic.
  F8 (founder A2) is deferred, not claimed.
- **D-pf-254 (S3):** fixed v1 I/O call-name table; literals only for plain string arguments; unknown
  stays unknown. **D-pf-253 (S2):** `flow` story-only/additive; labels computed in the core.
  **D-pf-252/D-pf-251:** sealed spec; Atlas is Perfpal's #1 priority; core additions recorded.

## S4 shipped

- **Core P6 (`9263c19`):** `purpose {purpose_version:1,text,source,kind}` from the scan-root README
  (README.md/.rst/.txt/README; 64 KiB/200-char caps; front matter/badges/TOC/fences skipped) or the
  entry line structural fallback. Hyperfine `A command-line benchmarking tool.`; cJSON
  `Ultralightweight JSON parser in ANSI C.`
- **Core P7 (`e55221c`):** `profile {queue_version:1,total_ranked,production_ranked,candidate_total,
  demoted_production,shown,rows[]}` — authoritative `complete_profile_queue` order, production
  non-demoted top 12; no re-rank.
- **Core P8 (`3c80ae9`):** `scenes {scene_version:1,scene_cap:24,file_cap:5,total_scenes,shown_scenes,
  total_files,shown_files,rows[],omitted{...}}` — root/top-level/split depth-2 partition, shared file
  cost, top-5 files, production-cost-first, entry/hottest, node/action `scene`.
- **Story (`30a3bba`, `49627b2`, `5f13b04`):** purpose masthead + source label, enter-beat scene
  context, whole-repo profile beat, scene-aware start-here, compact trust narration, below-fold
  SCENES / PROFILE QUEUE / TRUST ACCOUNTING, unresolved-calls inspector, `window.__sim.ready`, 1280
  wrap fixes, scan-wide test/bench counts.
- **3k fixture + scale gate (`4006315`, `5f7579d`):** deterministic 3,000 scanned files (2,999 C + 1
  Rust; 2,731 production + 269 test/bench), C fn-pointer + Rust `dyn` engineered into the drawn slice,
  loops/I/O/README; `sim/scale.py`/`scale.sh` F2–F7. Page **463,230 bytes**; first frame **max
  1,570 ms** (1,570/420/413); no h-scroll at 1280/1440; double-scan/load determinism; story-off/on
  byte identity.
- **Docs/evidence:** `core-probes` P6–P8, spec §9 S4, output contract, `NOTES` S4, `VISION` Read 8
  (two independent reads: PASS-WITH-NITS, no blockers), `CHANGELOG`.

## Risks / carried nits

- **S4 limits:** README first-only/first-prose (a stale/marketing README is quoted and labelled); P7
  12 rows (demoted production counted, not shown); P8 depth-2/24/5 (3k: `src` direct-files and the
  269-file `tests` scene are omitted by the production-first cap; counts disclosed); scene cost is
  static; mixed v1/v2 scene scale; the Rust `dyn` file is forced into the drawn slice as the unreached
  hottest (C entry cannot reach Rust across the language-scoped call graph).
- **Carried vision nits:** third START item scene-attribution tail; `PATH` meta ellipsis at 1440;
  dense 3k depth-1 column + clipped edge labels; trust findings run-on; cJSON unreached START/empty
  canvas; hyperfine edge-label density. F1 purpose + first `path:line` and the 1280 legend/START/footer
  are readable.
- **Prior:** S3 limits (out-of-slice wrappers, Rust path ambiguity, fd-based class, method writes);
  S2 nits; `--story` still builds the view twice; `sim/verify.py` not in CI; F8/A2 deferred.

## Next

1. Reconcile the 4-lane panel (R1/R2/Q1/Q2), fix any blockers, rerun the affected lanes + one fresh
   confirmation; final gate on the close commit; commit evidence/state.
2. Companion-vs-report ADR (+A12 if interactive) now that S4 evidence exists; then Atlas v2 step 11
   remains the parallel queue. No A2/ship before step 13 (R-pf-109).

## Verification (this session)

| Claim | Command / artifact | Result |
|---|---|---|
| Core gates | `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | **892 passed**, 0 failed, 1 ignored |
| Licenses | `cargo deny check licenses` | exit 0 |
| F6 additive | story-off vs story-on | `report.html` byte-identical; `metrics.json` content-equal after provenance masking |
| Core determinism | two `--story` scans | `atlas-layout.json` byte-identical; layout/flow/data sorted |
| Story gate | `SIM_PORT=8770 python3 sim/verify.py` | **302/302**, exit 0 |
| 3k scale gate | `bash sim/scale.sh` | **44/44**, exit 0; page 463,230 bytes; first frame max 1,570 ms (1,570/420/413); no h-scroll; F2–F6 PASS |
| Full gate | `bash sim/gate.sh` | `5f13b04` · tree in `GATE.txt` · dirty 0 · 892 tests · 302/302 · 3k 44/44 · **verdict=PASS** |
| Vision | `VISION.md` Read 8 (F1 + scale) | **PASS-WITH-NITS, no blockers**; purpose/1280/scene/queue/trust fixes landed; carried nits listed |
| 3k fixture | `sim/fixtures/gen-3k.py` + `sim/scale.py` | 3,000 files; C fn-pointer + Rust `dyn` drawn and unresolved; 24 scene cards / 12 queue rows |
| Review panel | 4 lanes (R1/R2/Q1/Q2) | **running at close**; result recorded in the S4 state close |

**Uncommitted at this handoff:** docs/evidence close WIP (`core-probes`, spec §9, contract, NOTES,
VISION, CHANGELOG, activeContext, this handoff, GATE/checks/PNGs/scale frames). **No push, no merge.**
