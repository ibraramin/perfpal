# Track D — Atlas Run (simulation)

**Status:** **DONE — merged into `main` as `v0.4.0` in merge-train M2 (2026-10-27).** D1–D2 + S1–S4
are complete and gated; the companion stays a companion (`ADR-0012`), A12 unchanged, no graduation
claim. The branch/worktree is an archive; no further D session.

**Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (archive; `feat/atlas-sim`, base `821b984`,
merged `main` `v0.4.0`).

**Track handoff:** `state/tracks/atlas-sim/handoff.md` · **Vision:** `docs/research/atlas-run.md` · **Evidence:** `docs/evidence/atlas-sim/` · **Prototype:** `sim/` or `sim.html`.
**Core:** make the map *feel like the program is running* — startup, flow, loops, work, end/stop — playable, scrubbable, speedable. **D1 is a static inferred run, not execution.** D2 is the runtime trace/replay product.

## Read first

1. The current hyperfine report + `metrics.json` (`/home/shafin/Opencode/perfpal-test/out-atlas-v2-hyperfine`).
2. `AtlasView` (`src/reporter/atlas.rs`) — entry, path, nodes/edges, costs, top functions, findings.
3. `docs/evidence/atlas-visual/v2/` (G baseline/decision), `docs/specs/atlas-layout.md` animation, ADR-0010, A12.
4. `docs/evidence/atlas-audit/AUDIT.md` for the visual system.

## Session D1 — static run-story prototype

**Goal:** a standalone `sim.html`/JS that visually plays the hyperfine run story on the Atlas map:

- boot at entry → representative path → node/function highlights → loop/trip-count moments → end/stop (no forward path, capped, empty).
- controls: play/pause, step, scrub, speed, node select; reset.
- label: **static, inferred; not runtime; profile to confirm.**
- serve at `:8768`, e.g. `python3 -m http.server 8768 --directory sim/`.

**Deliver:** the working prototype + a short `docs/research/atlas-run.md` (vision + D2 requirements) + `docs/evidence/atlas-sim/` (screenshot/notes) + `state/tracks/atlas-sim/handoff.md`.

**Do not:** edit production `src/reporter/`/Atlas; execute the target; use the network; add schema/deps; add JS to the static report. The prototype is standalone.

**EOL:** track handoff; commit only with founder approval; no merge to `main`.

## Sessions D2–D3 (signposts)

- **D2 — founder review/iterate.** The founder watches the prototype; iterate the story/controls; decide whether it stays a companion or graduates into the report (a new ADR; A12 amendment if in-report).
- **D3 — runtime trace/replay spec/ADR.** Trace schema (calls/returns/loops/alloc/IO/locks/threads/stack/source), instrumentation options (eBPF/perf/ftrace, ETW/DTrace, Python `sys.settrace`, Node inspector, LLVM XRay), sandbox/security/permissions, languages, privacy/reproducibility, replay/breakpoints; **spec/ADR only, no code**. D2/D3 are post-beta; the live debugger is later.

## Guardrails / ownership

- D is standalone; no production Atlas edits; no target execution; no network; no schema/deps.
- A/C read the prototype; no edits to A/C surfaces.
- The static report stays no-new-JS; D1 is a separate companion. If it graduates, the ADR/A12 decision is explicit.
