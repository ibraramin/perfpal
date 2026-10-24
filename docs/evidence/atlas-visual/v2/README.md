# Atlas v2 — visual rework evidence (steps 9–13)

**Status: STEP 9 SIGNED 2026-10-23; STEP 10 IMPLEMENTED AND STEP-10 FOUNDER-APPROVED 2026-10-24;
STEP 11 (DETAIL RAIL) COMMITTED `189df91`; STEP 12 (PROGRESSIVE CHARTS) COMMITTED `9a94905`;
STEP 13 APPROVED 2026-10-24 (D-pf-253, delegated evidence-based sign-off; the founder's own eye was
not applied).**
Option A (port G) was signed and steps 10–13 were executed; the founder approved the step-10 first
page and the step-12 commit. **A2 (step 14) may be scheduled; ship (step 15) remains blocked.**
*(Per-step status lines further down — `STEP10/11/12.md`, `VISION-STEP10/12.md`, `gate*.txt` — are
historical snapshots of the session that wrote them; this banner is the current status.)*
This directory holds the signed decision, the G-baseline comparison, the 1440×900 render evidence,
the gate transcripts and the vision records.

**Scope guard:** steps 9–13 only. A2 (step 14) and ship (step 15) are **not** started here. The F
treemap is a candidate prototype; F remains undecided. Holdouts stay vaulted; `map-build` is not done;
A1b stays open.

## Files

| File | What it is |
|---|---|
| `side-by-side-hyperfine-1440x900.png` | G (approved V7 baseline) vs shipped Atlas v1, hyperfine, both 1440×900 first viewports, labelled. |
| `side-by-side-cJSON-1440x900.png` | Same comparison for the cJSON library case. |
| `G-BASELINE-DIFF.md` | Token, typography, geometry and composition diff; defect→step map; what Atlas must keep. |
| `VISION-READ.md` | Fresh independent comparative `@vision` read of the step-9 G-vs-Atlas-v1 composite (2026-10-23). |
| `VISION-STEP10.md` | `@vision` read of the step-10 render (PASS WITH NITS → final PASS) and the fixes applied; step 13 remains the founder gate. |
| `DECISION.md` | The signed step-9 decision record: options, chosen contract, implementation notes, founder sign-off. |
| `decision.json` | Machine-readable signed decision (`status: signed`). |
| `STEP10.md` | Step-10 before/after, acceptance booleans, chrome counts, deviations and verification links. |
| `step10.json` | Reproduced 1440×900 DOM geometry, hashes, text floors, unreached chip/footer checks. |
| `render_step10.py` | Reproducer for the step-10 render/evidence (fails closed on acceptance). |
| `gate.txt` | Command + exit + artifact-hash transcript for the step-10 DoD. |
| `STEP11.md` | Step-11 detail rail: selection/roles/links, measured stage geometry, residuals. |
| `step11.json` | Reproduced 1440×900 + 1280×800 DOM geometry, rail completeness, link resolution, text floors, hashes. |
| `render_step11.py` | Reproducer for the step-11 render/evidence (fails closed on acceptance). |
| `gate-step11.txt` | Full step-11 DoD transcript (every suite line, exits, mtimes, artifact hashes). |
| `VISION-STEP11.md` | `@vision` step-11 read (PASS; two reads + a targeted collision-label re-check). |
| `atlas-v2-step11-{hyperfine,cJSON}-final.png` | Step-11 1440×900 first viewports (`#map`, reduced motion). |
| `atlas-v2-step11-{hyperfine,cJSON}-full.png` | Full-page step-11 renders. |
| `atlas-v2-step11-{hyperfine,cJSON}-1280x800.png` | Step-11 1280×800 stacked-rail renders. |
| `STEP12.md` | Step-12 progressive charts: the four blocks, the retired wall, A8 re-baseline, measured geometry, verification. |
| `step12.json` | Reproduced canonical `#map` geometry, chart heights, text floor, links, hashes, booleans. |
| `render_step12.py` | Reproducer for the step-12 evidence (fails closed on acceptance). |
| `gate-step12.txt` | Full step-12 DoD transcript (every command, exit, suite line). |
| `VISION-STEP12.md` | `@vision` step-12 read (PASS on final crops; step-13 proxy, not the founder gate). |
| `atlas-v2-step12-{hyperfine,cJSON}-final.png` | Step-12 1440×900 canonical `#map` first viewports. |
| `atlas-v2-step12-{hyperfine,cJSON}-full.png` | Full-page step-12 renders. |
| `atlas-v2-step12-{hyperfine,cJSON}-charts.png` | Step-12 below-graph charts at true panel width. |
| `atlas-v2-step12-{hyperfine,cJSON}-1280x800.png` | Step-12 1280×800 stacked-rail renders. |
| `step12/F-CANDIDATE.md` | F treemap candidate decision brief (not shipped; F undecided). |
| `step12/f-treemap-{hyperfine,cJSON}.{html,png}` | F candidate panels at the weight slot. |
| `step12/f-render.py` | Deterministic F prototype renderer (refuses the holdout). |
| `step12/W1-vs-F-{hyperfine,cJSON}.png` | Shipped weight panel vs F treemap side-by-side. |
| `STEP13.md` | Step-13 gate: measured 1440×900/1280×800 composition, all DOM/render assertions, V8 rerun, founder package, residuals. |
| `step13.json` | Reproduced real-page DOM geometry, definitions, clipping/leading-dot/duplicate checks, 1280 stack, hashes, G-baseline record. |
| `render_step13.py` | Reproducer for the step-13 real-page gate (fails closed; writes the renders + G composite). |
| `render_step13_states.py` | Reproducer for the eight-state rerun (preflight + final frames + weight crops). |
| `gate-step13.txt` | Full step-13 transcript (both harnesses, currency hash, assertion summary, scope check, evidence hashes). |
| `VISION-STEP13.md` | V8 rerun summary (A1 16/16, strip 16/16) + the G-baseline comparative read. |
| `step13-reads.json` | Normalized 32-read transcript (2 A1 + 2 strip per state), images + sha256, evaluator decisions. |
| `atlas-v2-step13-{hyperfine,cJSON}-final.png` | Step-13 1440×900 canonical `#map` first viewports. |
| `atlas-v2-step13-{hyperfine,cJSON}-full.png` | Full-page step-13 renders. |
| `atlas-v2-step13-{hyperfine,cJSON}-charts.png` | Step-13 below-graph charts at true panel width. |
| `atlas-v2-step13-{hyperfine,cJSON}-1280x800.png` | Step-13 1280×800 stacked-rail renders. |
| `step13/G-vs-step13-hyperfine-1440x900.png` | G (approved baseline) vs step-13 real page, labelled composite. |
| `step13/FOUNDER-GATE.md` | The founder gate package: what to open, what is proven, what only the founder can judge, decision. |
| `step13/{state}-{final,weight}.png` | Eight-state rerun frames + weight-panel crops. |
| `step13/manifest.json` | Eight-state measurement + frame manifest (hashes, canvas bottom, definitions, floors). |
| `step13/states.json` | Eight-state preflight result (PASS/FAIL + per-state measurements); rewritten on every run. |
| `atlas-v2-hyperfine-final.png` / `atlas-v2-cJSON-final.png` | Step-10 1440×900 first viewports (`#map`, reduced motion). |
| `atlas-v2-hyperfine-full.png` / `atlas-v2-cJSON-full.png` | Full-page step-10 renders. |

## Provenance (sha256)

Sources are the committed G prototype frames and the committed Atlas visual-regression audit captures;
no new Atlas render was taken for step 9 (the audit PNGs match the current tree).

| Input | sha256 |
|---|---|
| `docs/evidence/map-vision/v7/g/g-hyperfine-final.png` | `c52c58e901873479da05839070d75e4f7303443b9070c392fa06d2ca3271add3` |
| `docs/evidence/map-vision/v7/g/g-cJSON-final.png` | `61757319a4677128489f047004e16aeb600ebb048f68c2fe5a13fdbb1bf80496` |
| `docs/evidence/atlas-audit/atlas-hyperfine-final.png` | `b1614dd4f4f5c6d67a7ab0fb32c533979972ca36e6dc07780064e42180cb4bdc` |
| `docs/evidence/atlas-audit/atlas-cJSON-final.png` | `659bdf239326c1f31e369b966468cca2d47a27577bad2dae25e08c166ed72b4e` |
| `side-by-side-hyperfine-1440x900.png` | `a17d2412b3fbf5d54ea7e24f85cee65b9a7591ba98e99d098b0890c1b824939f` |
| `side-by-side-cJSON-1440x900.png` | `3ff641857a5be5d1933220450aee08316f7c1cdf38a2dc074512095349d747ec` |

The Atlas capture is the shipped `report.html#map` at 1440×900, reduced motion, taken by
`docs/evidence/atlas-audit/audit_render.py`; the DOM geometry backing every number in
`G-BASELINE-DIFF.md` is `docs/evidence/atlas-audit/audit.json`.

## Regenerate the composites

```bash
magick montage \
  -label 'G · approved V7 visual baseline · hyperfine · 1440×900' \
  docs/evidence/map-vision/v7/g/g-hyperfine-final.png \
  -label 'Atlas · shipped v1 first viewport (#map) · hyperfine · 1440×900' \
  docs/evidence/atlas-audit/atlas-hyperfine-final.png \
  -tile 2x1 -geometry '1440x900+14+14' -background '#202b35' -fill '#fbf9f4' \
  -font 'DejaVu-Sans' -pointsize 22 -gravity center \
  docs/evidence/atlas-visual/v2/side-by-side-hyperfine-1440x900.png
# repeat with g-cJSON-final.png / atlas-cJSON-final.png for the library case
```

## How to view

Open the two PNGs (or the HTML prototypes) side by side. The canonical question is **not** "which
page has more information" but **"which page looks like a designed product page at 1440×900, and what
must the first page carry?"** The proposed contract and the exact founder choices are in
`DECISION.md`.
