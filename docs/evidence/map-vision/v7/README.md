# V7 — F vs G human-map A/B (prototype evidence)

> **ARCHIVED EVIDENCE — 2026-10-26.** The V7 F/G prototypes are the historical A/B that selected
> the Atlas visual system. **G's system shipped** (Atlas v2 steps 9–13, approved D-pf-253);
> **F remains undecided** (D-pf-251) and no F treemap ships. These HTML/PNG artifacts are evidence
> only: they are **not current output**, are not linked from the shipped report, and must not be
> tuned or served as the product. The current contract is `docs/specs/atlas-layout.md` /
> `atlas-feature.md` / `atlas-metrics.md`; current evidence is
> `docs/evidence/atlas-visual/v2/` and `docs/evidence/map-vision/v8/`.

**Status:** archived prototype evidence; no core `src/` change. Holdout (django/postgres) was
released for the A2 evidence only (D-pf-255) — do not tune against it.
**Plans:** `map-f-treemap`, `map-g-flow`. **Animation spec:** `map-animation`.

## Candidates

- **F — weight map:** component-level treemap (area = static cost, one cost ramp, severity outlines,
  top-50 production-first), KV1 brief strip, component DSM + focus+1-hop drill.
- **G — entry-flow map:** entrypoint at centre (production `main`; libraries use the highest fan-in
  production `lib-root`), file nodes by resolved call edges in BFS depth rings, per-file
  key-function cards, component/domain hulls, unreached tail, numbered path trace.

Both carry the mandatory declarative animation: CSS keyframes/SMIL, no JavaScript, CSS-only pause,
`prefers-reduced-motion` and print fallbacks, complete static final frame.

## Inputs

`perfpal-test/out-v6-{cJSON,hyperfine,tmux}` (schema 1.7), design repos only. `tmux` is the density
stress case. Do not read/render django/postgres.

## Blind read protocol

- One image per task: the **final frame** at 1440×900; no source, no sibling views, no expected
  answer, no candidate name.
- Describe first (4–8 sentences), then strict JSON:
  `{question_answered, action, blockers, confidence}`.
- Two fresh reads per candidate per repo (F and G separately). Records: `f/reads.json`,
  `g/reads.json`.
- The reads judge comprehension, not animation preference. The founder test judges the animation.

## A/B criteria (record per candidate)

1. **Q1 — where do I look first?** Is there one obvious start point + a concrete `path:line`/file?
2. **Q2 — what breaks if I touch it?** Is the drill reachable and does it answer *which* dependents?
3. **Q3 — where might time plausibly go?** Is cost/heat legible without a second encoding?
4. **Whole-codebase grasp** — does the view show the repo as one object (not a dashboard)?
5. **Data honesty** — are caps, LOW-confidence call edges, parse health and demotions disclosed?
6. **Animation value** — does the progression aid comprehension, and does pause/reduced-motion/print
   still leave a complete static frame?

## Verdict rule

The **founder timed test** (10 minutes, 3 concrete actions, "understandable") decides. Blind reads
are evidence, not a vote. Record the losing candidate's reusable pieces and the exact reason it lost;
do not polish both endlessly. One A/B, then the winner goes to Rust + the ADR-0002 animation
amendment + the A2 holdout test.

## Expected files

```
docs/evidence/map-vision/v7/
├── README.md                 (this file)
├── f/ render_f.py, f-<repo>.html, f-<repo>-{start,mid,final}.png,
│     f-<repo>-drill.html, manifest.json, NOTES.md, reads.json
└── g/ render_g.py, g-<repo>.html, g-<repo>-{start,mid,final}.png,
      g-<repo>-drill.html, manifest.json, NOTES.md, reads.json
```

## Guardrails

- No `<script>` or event handlers in the artifacts; the animation is CSS/SMIL only.
- The static final frame contains everything; no information exists only mid-animation.
- Deterministic ordering/coordinates/palette; no random; double-render byte-identical.
- No new dependency; no holdout read; no `map-build` done claim; A1b stays open.
