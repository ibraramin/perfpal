# Atlas sim — Session D1 evidence

**Date:** 2026-10-24 · **Track:** `docs/tracks/atlas-run.md` (Track D) · **Worktree:** `feat/atlas-sim`
**Prototype:** `sim/` (standalone HTML/CSS/JS, no build step, no dependencies, no network)

A scrubbable **static, inferred run story** over the real Atlas entry-flow map: boot at the elected
entry → the representative path → per-node/function highlights → loop/trip moments → off-path
branch points → the hottest file → the stop/summary. It is a companion prototype, not the report.

```bash
python3 -m http.server 8768 --directory sim   # then open http://127.0.0.1:8768/
```

## Provenance (read this first)

The extraction is only reproducible against a **known render**. During D1 the external fixture
directory `/home/shafin/Opencode/perfpal-test/out-atlas-v2-*` was overwritten (02:06) by a newer
build from a different worktree — that render has an 868px canvas plus a right rail and does **not**
match this worktree's committed step-10 layout. The prototype therefore pins its own source:

```bash
bash sim/make-source.sh        # builds this worktree + scans the two fixture repos
python3 sim/extract.py         # -> sim/data-*.json (byte-stable)
```

`make-source.sh` writes `sim/source/out-{hyperfine,cjson}/` (gitignored) and fails closed unless the
canvas is `viewBox="0 0 1400 718"` (the approved full-bleed composition). `extract.py` parses the
viewBox and fails closed on any unexpected stage size. This keeps the sim independent of whatever
the shared `perfpal-test/out-*` directories contain at any moment.

| Case | Source render | Nodes / edges / hulls / columns / path |
|---|---|---|
| hyperfine | `sim/source/out-hyperfine/` | 19 / 49 / 5 / 6 (5 depth + UNREACHED) / 3 |
| cJSON | `sim/source/out-cjson/` | 7 / 5 / 2 / 3 (2 depth + UNREACHED) / 2 |

Data hashes (2026-10-24, two identical runs):

```
bcad0a81cbcd45f892f76a7e9eaaa700c5938a3771f642e55a1ccb77417d43b6  data-hyperfine.json
2bee0a596d81e6d67c48441aae375ad33f66bfb1ed3a6fa975f68523d84bfbfb  data-cjson.json
9aebe16710c68d5f1bad3fee0ead56aac5ae8184a364d9f112cb309adfc37aaf  data-no-forward-path.json
ee405fa98ddf1678c20113ef61e8b7496a72319498dacd9ee82845ed5b277d5e  data-empty.json
c23fdb504545c7a45a8422cbc03b6004d6f826b044f6d2051093d1e9cdcf5ec5  data-capped.json
d42564a03db5977541a60c62b8a43e8f412c0ea5d4b27b0d2864a7e0870dad22  data-index.json
```

## Files

| File | Role |
|---|---|
| `sim/index.html`, `sim/sim.css`, `sim/sim.js` | The prototype: masthead + transport, honesty strip, 3-row key strip, SVG stage, inspector rail (overlay), narration, canvas footer. |
| `sim/extract.py` | Stdlib-only extractor: geometry from `report.html` (nodes, edges, hulls, columns incl. `UNREACHED`, path badges, footer, strip) + function/loop/finding detail from `metrics.json`; fail-closed structural assertions; sorted deterministic JSON. |
| `sim/make-source.sh` | Builds this worktree and regenerates the two source renders into `sim/source/`. |
| `sim/verify.py` | Playwright gate: serves `:8768`, drives every control, checks honesty/no-time/no-network, captures keyframes, writes `checks.json`. |
| `sim/data-*.json` | Generated replay data (two real scans + three labelled synthetic state demos: no-forward-path, empty, capped). |
| `docs/research/atlas-run.md` | Vision, D1/D2/D3 split, honesty contract, D2 review criteria, D3 trace/replay proposal. |
| `docs/evidence/atlas-sim/*.png` | 1440×900 keyframes: start / mid / end for both real cases. |
| `docs/evidence/atlas-sim/checks.json` | Machine-readable gate transcript. |
| `docs/evidence/atlas-sim/GATE.txt` | Command/exit/hash transcript (source renders → extraction ×2 → gate). |

## Story model

Beats are derived deterministically from the data (no randomness, no wall-clock input beyond the
animation clock). Fixed durations: boot 1.6s, enter 2.2s, loop 2.6s, branch 1.8s, hottest 2.0s,
stop 2.0s, summary 3.0s. hyperfine → **13 beats**; cJSON → **8 beats**.

- **boot** — elected entry (policy, confidence, alternatives); "nothing is executed".
- **enter** (per path step) — node cost, × median function, top % of files, annotation.
- **function highlight** (on loop beats) — the focused file node's bottom line becomes
  `▶ <function> · L<line>` in bold, so the function is highlighted inside its file. Both real loop
  targets are ENTRY nodes, where the entry text rule wins specificity and the label renders cream;
  on paper nodes it is burnt orange.
- **loop** (when a path function has loops) — max loop depth, unknown-trip count, trip provenance
  (`literal` / `partial` / `unknown`); "inferred trip weight, not measured iterations".
- **branch** (when the node has off-path edges) — how many structural edges leave the path, the two
  largest targets; "the story does not follow them; they are not measured".
- **hottest** — cost, ratio, and the top annotation from the global findings list.
- **stop** — a static leaf / no forward path / partial parse / capped, from the real state flags.
- **summary** — path, caps, findings (including the data-slice cap), "static cost units, not time".
- Synthetic cases (`no-forward-path`, `empty`, `capped`) render a labelled stub only: no fabricated
  nodes, edges, costs or findings.

Controls: play/pause, step back/forward, scrub, speed (0.5/1/2/4×), case select, node select with
an inspector (full path, cost, rank, group, findings, top functions with loop/trip columns), reset.
Keyboard: Space, ←/→, Home, 1–4, Escape. `prefers-reduced-motion` starts at the final frame.

## Honesty contract

- Persistent label: **`STATIC, INFERRED · not a runtime trace · profile to confirm`**.
- No time units anywhere in the rendered text (asserted by the gate); the unit is `static cost
  units (v2)`.
- Trip counts are inferred from literals; unknown trips weight ×1 and are labelled.
- The badge is the **dominant-rule annotation**, not a file finding total — stated in the key strip
  (`badge ×N = dominant rule annotation`) and in the inspector; the inspector lists the real findings
  for the file and says when the data slice is capped (cJSON 153/837 — every drawn node is kept).
- Synthetic states are visibly labelled `synthetic state demo`.

## Verification

Run: `python3 sim/verify.py` (it starts and stops its own server on `:8768`).

| Claim | Check | Result |
|---|---|---|
| Data extraction is deterministic | `extract.py` twice + `sha256sum` diff | identical (6/6 files) |
| Gate | `python3 sim/verify.py` | **101/101 checks, exit 0** |
| Loop honesty | loop-beat narration | contains `Inferred trip weight, not measured iterations` |
| Function highlight | loop-beat node bottom line | `▶ <function> · L<line>` in orange, both cases |
| Findings cap | cJSON final narration | `837 findings in the scan (153 included in this data slice)` |
| Badge semantics | key strip | `badge ×N = dominant rule annotation, not a file total` |
| Data fidelity | node/edge/hull/column/trace counts + on-path edges | 19/49/5/6/3 and 7/5/2/3/2 |
| Controls | play/pause, step ±, scrub, speed, reset, node select, Escape | all pass, both cases |
| Honesty | label present; no rendered `ms`/`seconds` | pass |
| Final frame | every node at effective opacity ≥ 0.9; footer verbatim | pass |
| Fold | `#canvas-footer` bottom 894 ≤ 900 at 1440×900 | pass |
| Narrow | no horizontal scroll at 1280×800 | pass |
| Reduced motion | starts at the final beat | pass |
| Network | zero non-localhost requests; no external URL in source | pass |
| Inputs untouched | data hashes unchanged by the gate | pass |

Artifacts: `checks.json`, `GATE.txt` (the full command/exit transcript),
`hyperfine-{start,mid,loop,end}.png`, `cjson-{start,mid,loop,end}.png`, and the three state-demo
frames `no-forward-path.png`, `empty.png`, `capped.png` (eleven 1440×900 frames).

## Visual system and known nits

The page reuses the shipped Atlas v2 tokens: paper `#f4f1ea`, canvas `#fbf9f4`, charcoal `#1f2d3a`,
burnt orange `#c2410c`, border `#d6d0c4`, muted `#6f6a61`, sans chrome + mono data; node cards keep
the renderer's 150×80 geometry, fills and non-production dashed outline; only on-path edges are
labelled (`×N`), all edges keep their `<title>`.

The `@vision` read is recorded in `VISION.md`. Known nits (non-blocking): dimmed node stat lines sit
near the contrast floor by design; the inspector is an overlay rather than the report's rail;
edge labels can land close to a node border on very short edges.

## Deferred (D2/D3, not D1)

- Founder review of the story pacing/controls; companion-vs-in-report decision (needs an ADR + A12
  amendment if it graduates).
- Runtime trace/replay: schema, instrumentation, sandbox/permissions, privacy, replay — spec/ADR
  only, proposed in `docs/research/atlas-run.md` §6.
