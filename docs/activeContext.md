# Active Context

## Current Task
**Track D1 (Atlas run story) is implemented and gated (2026-10-24): 101/101 checks, vision PASS, panel green; it is
committed `7093c77`.** D2 (founder watches the prototype; companion-vs-in-report) is
next. Separately, Atlas v2 steps 9–10 remain founder-approved; the step-13 final gate is PENDING and
step 11 (G-style detail rail) is the next Atlas work item.
- **D1:** a standalone `sim/` companion (no build step, no deps, no network, no production edits) plays
  a scrubbable **static, inferred** run story over the real 1400×718 Atlas map for hyperfine and
  cJSON: boot at entry → representative path → node/function highlights → loop/trip moments →
  off-path branch points → hottest → stop/summary (leaf, no forward path, partial parse, capped).
  Controls: play/pause, step, scrub, speed, node select + inspector, reset; keyboard; reduced motion
  shows the final frame. Persistent honesty label, no time units, unknown trips ×1.
- **Provenance:** the shared `perfpal-test/out-atlas-v2-*` dirs were **overwritten at 02:06** by a
  step-11 build from another worktree (868px canvas + rail). `sim/make-source.sh` rebuilds the
  approved full-bleed render from **this worktree** into `sim/source/` (gitignored) and `extract.py`
  fails closed on any layout change.
- **Evidence:** `docs/evidence/atlas-sim/` (eleven 1440×900 frames, `checks.json` 101/101, `NOTES.md`,
  `VISION.md` PASS); `docs/research/atlas-run.md` (vision, D2 criteria, D3 proposal);
  `state/tracks/atlas-sim/handoff.md`.

## Recent Decisions
- **D-pf-249 (live, 2026-10-24):** Track D1 Atlas run-story prototype — standalone `sim/`, five cases
  (hyperfine, cJSON, no-forward-path, empty, capped), 101/101 gate, vision PASS, panel R1/R2/Q1/Q2/R3 APPROVED; no production edits,
  no schema/deps/network; committed `7093c77`; D2 next, D3 parked.
- **D-pf-248 (live, 2026-10-23/24):** Atlas v2 steps 9–10 implemented; Option A signed; step-10
  first-page fit/G visual language/key-metrics strip/chrome hygiene; panel R1/R2/Q1/Q2 + fresh R3
  APPROVED; step-10 founder check approved 2026-10-24; step-13 final gate pending; steps 11–12 not
  started.
- **D-pf-247 (live):** plan reordered — steps 9–13 = v2 visual rework; A2/ship 14–15 blocked.
- **R-pf-109 (open, high):** Atlas visual regression; A2/ship blocked until the step-13 gate.
- **D-pf-246/245/244** moved to the cold ledger (historical session status); **A-pf-119** settled.

## In Progress
- **D1 complete, gated and committed (`7093c77`).**
- Atlas v2: step 10 approved; step 11 next; step 13 gate pending.
- **Step 10 committed 2026-10-24 (`468d772` + docs/state); D1 feature committed `7093c77`; no push.**

## Next
- **Founder:** watch D1 (`python3 -m http.server 8768 --directory sim`) and approve the commit
  (`sim/` + `docs/research/atlas-run.md` + `docs/evidence/atlas-sim/` + state).
- **D2:** founder review of pacing/controls; decide companion vs in-report (in-report needs a new ADR
  and an A12 amendment; the static report stays zero-new-JS with the single fixed `REPORT_SCRIPT`).
- **Atlas:** `/new` → Session 5: steps 11–13 (G-style detail rail; progressive charts/F candidate;
  founder visual gate + V8).
- Do **not** start A2 (step 14) or ship (step 15); holdouts vaulted; `map-build` not done; A1b open.

## Blockers
- **Atlas A2 (R-pf-108):** founder timed test on application + library classes; blocked until step 13.
- **Atlas visual gate:** step-10 first page APPROVED 2026-10-24; step-13 final composition remains.
- **Map legibility:** R-pf-104/R-pf-105/R-pf-106; ranking recall A1b open.
- **M5 publish:** B-pf-001 (contact) + B-pf-002 (secrets) — founder-owned.
- **Paper track:** prereg + scans + stats + Zenodo PENDING (PARKED behind the map).

## History
- **L2 split (2026-09-29)** · **L6 pinned (2026-10-01)**, realigned 2026-10-17 twice · **first audit (2026-10-16)**.
- **Sessions 47–50:** instrument mega-burst → vision fixes → group-fold → C-full + audit/archive.
- **Map research:** R1–R4 → D-pf-215.
- **I1–I4:** entries C/Rust/Python + cost-v2 (`e0df47a`); entry map + bands (`d78bb85`);
  annotations + findings stem (`f56d072`).
- **I5:** proof + demo + **A5 FAIL** (`dbb9bbe`→`6f31a4f`), successor plan created.
- **V0–V2:** prereg → diagnosis → recall → review fixes → methodology + spec lock.
- **V3–V6a:** 18 prototypes + 27 blind reads → V4 key views + Explorer → V5 validation + no-loop
  falsifier → V6a KV2 production-first amendment (6/6 reads PASS); V6 timed test scrapped.
- **V7:** F/G prototypes + 12/12 blind reads (`docs/evidence/map-vision/v7/`); founder decision
  D-pf-238 (G core, F supporting); Atlas named.
- **Atlas planning:** needs research → metrics contract → data-gap audit → layout + inclusion rule
  → ADR-0009/0010 + V8 amendments → `atlas-build` plan. Docs only.
- **Atlas ratification (2026-10-23):** ADR-0009/0010 **Accepted**; V8 ratified; D-pf-239/240
  confirmed; D-pf-241/242/243 recorded; library A2 holdout pre-registered (`memchr` crate payload).
  Docs/state only.
- **Atlas build session 1 (2026-10-23):** steps 2–4 implemented + unit tests (`fe58299`/`f76a63b`/
  `3fa7c07`, review fixes `5e6b45e`/`6c0f0f3`); full gate green; panel R1/R2/Q1/Q2 APPROVED. No
  schema/`#map` wiring/holdout reads.
- **Atlas build sessions 2–3 (2026-10-23):** steps 5–8 green (view model/metrics/canvas, brief/
  drills/#map wiring, test/golden lock, V8 16/16 A1 + 16/16 strip); post-V8 visual audit found the
  composition regression (`R-pf-109`); v2 rework planned (steps 9–13), A2/ship moved to 14–15.
- **Atlas v2 Session 4 (2026-10-23/24):** steps 9–10; signed Option A; G-baseline diff + 1440×900
  side-by-sides; first-page fit 862 ≤ 900; warm/sans/mono system; key-metrics strip; chrome hygiene;
  evidence under `docs/evidence/atlas-visual/v2/`. Step-10 founder check approved 2026-10-24; committed
  (`468d772` + docs/state); step-13 final gate pending.
- **Track D1 (2026-10-24):** Atlas run-story prototype `sim/`; reproducible step-10 source renders
  (`sim/make-source.sh`), deterministic extraction (5 cases), deterministic beat engine (13/8 beats),
  inspector, 3-row key strip, unreached lane; gate 101/101; vision PASS (five reads); research doc +
  evidence + track handoff; **committed `7093c77`**; no production/schema/deps/network.
