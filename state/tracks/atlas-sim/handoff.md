# Track D — Atlas Run · Session D1 handoff

**Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` · branch `feat/atlas-sim` (base `821b984`)
**Track:** `/home/shafin/Opencode/Perfpal/docs/tracks/atlas-run.md` (Session D1)
**Status:** **D1 complete — committed `7093c77` (feature); this docs/state follow-up records it.**
D2 (founder review/iterate) is next. D3 (runtime trace spec/ADR) is untouched.

## Objective (D1)

A standalone, scrubbable **static, inferred run story** over the real Atlas map: boot at entry →
representative path → node/function highlights → loop/trip moments → end/stop, with play/pause,
step, scrub, speed, node select, reset; labelled **static, inferred; not runtime; profile to
confirm**; served at `:8768`; prototype + research doc + evidence + this handoff.

## What shipped

- `sim/` — `index.html`, `sim.css`, `sim.js`, `extract.py`, `verify.py`, `make-source.sh`, and five
  deterministic case files plus `data-index.json` (hyperfine, cJSON, and the labelled
  `no-forward-path`, `empty`, `capped` state demos).
  - Layout: masthead + transport, honesty strip, 3-row key strip, 1400×718 SVG stage, inspector
    overlay, narration bar, canvas footer.
  - Story: 13 beats (hyperfine) / 8 beats (cJSON), deterministic durations, no randomness, no
    wall-clock input except the animation clock. Loop beats carry `max_loop_depth`,
    `unknown_trip_loops`, and `trip_source`; unknown trips are labelled ×1. Branch beats name the
    off-path edges and state they are not followed or measured.
  - Inspector: full path, class, cost/ratio/top%, hull group, dominant annotation, real findings
    (severity/rule/line/title), top functions with loop/trip columns.
- `docs/research/atlas-run.md` — vision, D1/D2/D3 split, story model, honesty contract, D2 review
  criteria + companion/in-report decision (new ADR + A12 amendment if it graduates), and the D3
  runtime trace/replay **proposal** (schema, facilities, sandbox, privacy, replay).
- `docs/evidence/atlas-sim/` — eleven 1440×900 frames (eight keyframes including two
  loop/function frames, plus three state demos), `checks.json` (101/101), `NOTES.md`, `VISION.md`,
  `GATE.txt`.

## Provenance incident (important)

The external fixture dir `/home/shafin/Opencode/perfpal-test/out-atlas-v2-*` was **overwritten at
02:06** by a newer build from another worktree (868px canvas + right rail — not this worktree's
committed step-10 layout; the rail does not exist in `src/reporter/` here). The prototype therefore
does not depend on it: `sim/make-source.sh` builds **this worktree** and scans the two fixture repos
into `sim/source/` (gitignored), and `extract.py` fails closed unless the canvas is `1400×718`.
Do not "fix" the sim by pointing it back at the shared out dir.

## Verification

| Claim | Command / artifact | Result |
|---|---|---|
| Source renders are the approved layout | `bash sim/make-source.sh` | 1400×718 full-bleed; hyperfine 19/49, cJSON 7/5 |
| Extraction deterministic | `python3 sim/extract.py` twice + `sha256sum` diff | identical, 6/6 files (`GATE.txt`) |
| Gate | `python3 sim/verify.py` | **101/101 checks, exit 0** (`GATE.txt`) |
| Function highlight | loop-beat node bottom line | `▶ <function> · L<line>` |
| Loop honesty | loop narration | `Inferred trip weight, not measured iterations` |
| Controls | play/pause, step ±, scrub, speed, reset, node select, Escape | all pass, both real cases |
| Honesty | label present; no rendered `ms`/`seconds` | pass |
| Final frame | all nodes at effective opacity ≥ 0.9; footer verbatim | pass |
| Fold | `#canvas-footer` bottom 894 ≤ 900 at 1440×900 | pass |
| Narrow | no horizontal scroll at 1280×800 | pass |
| Reduced motion | starts at the final beat | pass |
| Network | zero non-local requests; no external URL in source | pass |
| State demos | `no-forward-path`, `empty`, `capped` stubs; zero fabricated nodes | pass |
| No production edits | `git status --porcelain src/ docs/schema/` | empty |
| Vision | `docs/evidence/atlas-sim/VISION.md` | **PASS** (reads 3–5) |

## Review panel

| Lane | Verdict |
|---|---|
| R1 — correctness / data fidelity / determinism | **APPROVED** (after the case-switch + findings fixes) |
| R2 — contract / security / provenance / honesty | **APPROVED** |
| Q1 — verbatim scope | **APPROVED** (after the function-highlight + D2-premise fixes) |
| Q2 — acceptance / evidence | **APPROVED** (after the transcript/count fixes) |
| R3 — fresh confirmation | **APPROVED** |

Non-blocking follow-ups (not changed in D1): the function label on ENTRY nodes renders cream because
the entry text rule wins specificity (legible, documented); `verify.py` checks finding-count presence
and compares the cap disclosure to the same payload, so a future filter regression could pass — lock
the known per-case counts; cJSON's four hot findings on undrawn files are excluded (counts are
disclosed, severity loss is not); the reduced-motion context has no `pageerror` listener.

## Vision nits (non-blocking, carried)

- cJSON `WARN ×11 · partial` / `WARN ×6 · unreached · partial` badges sit ~3px apart across the
  depth-1/UNREACHED boundary.
- A two-reachable-file graph leaves the left canvas empty by construction.
- Mid-ellipsis can hide the directory/file boundary on very long paths (tails stay distinguishable).

## Deferred / next

- **D2:** founder watches the prototype; iterate pacing/controls; decide companion vs in-report
  (in-report requires a new ADR and an A12 amendment; the static report stays zero-new-JS and the
  single fixed `REPORT_SCRIPT` stays unchanged).
- **D3:** runtime trace/replay spec/ADR only — no code (post-beta).
- Not touched: production Atlas (`src/reporter/`), schema, deps, the report's static JS, holdouts,
  A2/ship (still blocked by R-pf-109 pending the step-13 gate).

## Commit

`7093c77` — `feat(track-d): Atlas run-story prototype (D1)` (30 files: `sim/` 12, research doc,
evidence 15, `.gitignore`, `CHANGELOG.md`). This handoff + `docs/activeContext.md` +
`state/session-state.json` follow in a docs/state commit. **No push; no merge to `main`.**
