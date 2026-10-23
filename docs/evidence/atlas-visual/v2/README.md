# Atlas v2 — visual rework evidence (steps 9–11)

**Status: STEP 9 SIGNED 2026-10-23; STEP 10 IMPLEMENTED AND STEP-10 FOUNDER-APPROVED 2026-10-24;
STEP 11 (DETAIL RAIL) IMPLEMENTED 2026-10-24 — STEP-13 FOUNDER GATE PENDING.**
Option A (port G) was signed and step 10 was executed; the founder approved the step-10 first page
and authorized the commit. The **step-13 final composition/V8 gate is still pending**. This directory
holds the signed decision, the G-baseline comparison, the step-10 1440×900 render evidence, the DoD
transcript and the vision records.

**Scope guard:** steps 9–11 only. Progressive charts + F prototype (step 12), the founder visual
gate/V8 rerun (step 13), A2 (step 14) and ship (step 15) are **not** started here. Holdouts stay
vaulted; `map-build` is not done; A1b stays open.

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
