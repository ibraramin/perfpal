# Track D — Atlas Story · Session D2 + S1 handoff

**Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` · branch `feat/atlas-sim`
**Track:** `/home/shafin/Opencode/Perfpal/docs/tracks/atlas-run.md`
**Status:** **D2 done + S1 shipped.** D1 presentation was accepted by the founder; the requirement was
re-sealed as a core-fed temporal showcase (`docs/specs/atlas-story.md`), and the first slice (S1) is
implemented. S2 (data movement on the edges) is next. No push, no merge to `main`.

## D2 outcome

- Founder watched D1 at 1440×900: *"It looks good, as well as the animation. Just the info isn't very
  insightful."* Composition and motion pass; the **information** does not.
- Diagnosis (measured, not opinion): the story re-sequences file-level facts the static report already
  states. It never names a function, a finding, a payload, or a comparison. cJSON's climax is
  `testunity.c · 2,000,468 u · 1,000,234× median` (99.8% of ranked cost is non-production); hyperfine's
  hottest file is `command.rs` but the biggest function is `options.rs#from_cli_arguments` (165 u), and
  2 of 3 KV1 actions sit on files the canvas does not draw.
- Founder direction: **Atlas is Perfpal's #1 priority**; it must consume **Perfpal core data**, not a
  separate pipeline; new data needs → add a probe to the core **and record it** for later merge.
- Sealed requirement (`docs/specs/atlas-story.md`): a temporal showcase — entry → stages → loops → I/O
  → stop — with the payload crossing each file boundary labelled; vibecoder-first, senior drill;
  static inference labelled; scenes by directory at scale; shipped as `perfpal <repo> --story`.
- A 15-item questionnaire was drafted and then withdrawn: the docs already specify the requirement
  (A1/A2, KV1/KV3, `AUDIT.md:51-56`). It was never committed.

## What S1 shipped

- **Core (Perfpal):** `perfpal <repo> -o <dir> --story` now also writes `atlas-layout.json`
  (`src/reporter/atlas_layout.rs`, `--story` in `src/cli/mod.rs`). It carries the exact geometry the SVG
  draws — canvas, columns, node rects, edge `d` paths, hulls, aggregate boxes, cap chips — plus node
  presentation strings, state flags, drawn/scan findings counts, KV1 start actions and derived
  `stages`. Layout math is **shared** with the renderer (helpers extracted in `atlas_render.rs`), not
  copied. Additive: `report.html` is byte-identical and `metrics.json` differs only in scan provenance.
- **Story (sim):** `sim/from-layout.py` builds `sim/data-<case>.json` from `atlas-layout.json` +
  `metrics.json`, asserts the drawn-findings count against `findings_included`, and embeds the layout
  SHA-256 in `source.sha256`. `sim/extract.py` (HTML scraping) is **deleted**; `make-source.sh` scans
  with `--story` and rebuilds the data. The page itself is unchanged.
- **Record:** `docs/atlas/core-probes.md` — P4/P5 are `prototype`; P1–P3 remain `not started` (S2/S3).
  Merge protocol: one probe per commit, additive only, story-only emission, no golden churn.

## Verification

| Claim | Command / artifact | Result |
|---|---|---|
| Core gate | `cargo fmt --all -- --check` | exit 0 |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | all suites pass (838 tests) |
| Existing outputs unchanged | `sha256sum` report.html, story-off vs story-on | identical both fixtures |
| metrics.json unchanged | field diff of story-off vs story-on | only `scan.duration_ms`, `scan.started_at` |
| Story artifact | `perfpal … --story` → `atlas-layout.json` | 1400×718; hyperfine 19/49/3 stages, findings 148/348; cJSON 7/5/2, 153/837 |
| Data from core, not HTML | `sim/from-layout.py` | 6/6 data files byte-stable across two runs; `source.sha256` matches |
| Story gate | `SIM_PORT=8769 python3 sim/verify.py` | **101/101, exit 0** |
| sim parity vs D1 | field diff of regenerated `data-*.json` | nodes/edges/findings/hulls/columns/trace identical |
| No production scope leak | `git status --porcelain src/ docs/schema/` | core edits are the recorded probes only |

## Open risks / follow-ups

- `--story` builds the Atlas view twice (render + projection). Costs one extra O(files) pass; fix in a
  single-pass artifact assembly later.
- `findings_included` is defined as the drawn-node severity sum (matches the D1 extraction). It is a
  story-slice count, not a cap disclosure.
- P1/P2/P3 fidelity is unfunded until S2/S3: no data-movement field exists yet in the core.
- Evidence keyframe PNGs are captured 250 ms after a seek and are not byte-stable (known D1 nit;
  transitions still running). Capture after `transitionend` if byte-diffable frames are needed.
- Companion vs in-report is deferred until S2 produces evidence; an interactive in-report story would
  need a new ADR + A12 amendment (A12 already permits CSS/SMIL, not a second script).

## Commits

- `5faacb7` — `feat(core): atlas-layout story projection behind --story [P4/P5]`
- this commit — `feat(sim): story data from Perfpal core, delete HTML extraction [S1]`

## Next

- **S2:** P3 signature summary + P1 call-site payloads (core), then data labels on the film's edges
  with unresolved indirection greyed; add the payoff beats (`hottest` function, `profile`,
  `start-here`, `trust`). Then re-run the panel and the F-gate subset.
- **S3:** P2 I/O effects. **S4:** scenes at 3k files + F1–F8.
- Atlas v2 step 11 stays queued; A2/ship still blocked by the step-13 founder gate (R-pf-109).
