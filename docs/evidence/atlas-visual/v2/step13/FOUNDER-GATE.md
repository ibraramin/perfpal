# Founder gate — Atlas v2 step 13 (1440×900)

**Status: APPROVED — 2026-10-24.** **Decision: the first-page composition is approved.** This is the gate that unblocks **A2 (step 14)**;
ship (step 15) stays blocked until A2 reports (`R-pf-109` / `A-pf-118` stay open as records). Nothing in this session tuned the page.

**Provenance of the decision (recorded honestly):** the founder asked the orchestrator which way to rule and
delegated the call to the evidence below; **the founder's own eye was not applied**. The approval therefore
rests on the measured DOM/geometry gate, the unchanged pixels since the step-10/12 approvals, the A1/strip
reads and the G comparison — not on taste. It is recorded so it can be revisited if the page does not feel
like the designed page.

**Re-rendered 2026-10-24 on the post-Track-C tree** (`main` `c73d6ff`): every measurement below was
re-taken. The **900px `-final.png` first screens and the 1280×800 frames are byte-identical to the
step-12 committed renders** (`step12.json`), and every fixture frame still matches its `step13-reads.json`
image hash, so the first-page composition you are judging has not changed. `-charts.png` and `-full.png` changed because the two new nav rows sit inside the
below-the-fold coverage block; no chart data or geometry changed.

## Look at

1. **Live page (private server, exact build):**
   <http://127.0.0.1:8769/hyperfine/report.html#map> (cJSON:
   <http://127.0.0.1:8769/cJSON/report.html#map>).
   Desktop Chrome/Firefox, window ~1440px wide. This is the **merged `main` (`c73d6ff`)** build;
   the two `agent report` / `agent actions` links below the fold are expected. The served page
   was re-scanned and re-rendered from `main` `c73d6ff` (Track C merged) on 2026-10-24: a fresh scan with
   that binary is **byte-identical** to what is served (`report.html` sha256 `a4fade03…`).
   **Difference from the earlier package:** `report.html` now carries two nav links on one baseline row —
   `agent report` / `agent actions` — for the LLM artifacts that sit beside the page
   (`agent-report.md`, `agent-actions.json`, Track C), below the 900px fold. Nav text only: no chart,
   geometry, label, or data change. **Servers:** `:8769` serves the private gate scan
   (`/tmp/opencode/step13-page`), `:8767` serves the canonical shared `perfpal-test/out-atlas-v2-hyperfine`;
   both serve the same bytes (`a4fade03…`).
2. **First screens:**
   - `docs/evidence/atlas-visual/v2/atlas-v2-step13-hyperfine-final.png`
   - `docs/evidence/atlas-visual/v2/atlas-v2-step13-cJSON-final.png`
3. **G beside step 13 (the baseline diff):**
   - `docs/evidence/atlas-visual/v2/step13/G-vs-step13-hyperfine-1440x900.png`
4. **Below the fold:** `atlas-v2-step13-{hyperfine,cJSON}-full.png` and `-charts.png` in the same
   folder.
5. **States if you want them:** `docs/evidence/atlas-visual/v2/step13/{state}-final.png`.

## What is already proven (not a substitute for your eye)

| Check | Result |
|---|---|
| Canvas fully visible at 1440×900 incl. header/legend/footer | bottom **869** / footer **869** |
| `#map` height ≤5,000 | **1,848** (hyperfine and cJSON) |
| No horizontal scroll 1440×900 / 1280×800 | none (1,440/1,440 · 1,280/1,280) |
| No clipped text / node text overflow | 0 / 0 |
| One definition each (unit, median, structural caveat, unreached, legend, footer) | 1 each |
| No leading-dot truncation / duplicate ids | 0 / 0 |
| Detail reachable on the first page | 12-card rail + exact function/finding links |
| Canvas purity A9 / no new JS A12 | 0 tables/bars/cards · 0 scripts/handlers |
| Currency after the C merge (2026-10-24) | fresh-scan `report.html` sha256 **== served page** (`a4fade03…`) |
| V8 A1 + strip rerun (2 blind reads × 8 states) | **16/16 + 16/16 PASS** |
| G-baseline comparative read | **SAME SYSTEM** |

## What only you can judge

- Does the first page feel like the **designed page** you approved (warm paper, charcoal, burnt
  orange, sans chrome + mono data, masthead), not an instrument panel?
- Is the **composition** right: main graph + small key-metrics strip with 3 START actions, rail on
  the right, charts revealed below?
- Is the page **trustworthy** for the beta framing ("Perfpal audits what AI wrote")?
- Is the **density/scale** of the graph and rail right at 1440×900?

## Known residuals (disclosed, deliberately not fixed here)

1. Canvas basenames can mid-elide (`executi_.ets.rs`) and node labels drop `src/` (parent segment
   only on collision); one rail function row wraps and loses a line number in the visible card (full
   value in `title`/profile queue). Flagged again by the comparative read as the most visible defect.
2. Sparse cJSON state: 2 of 81 files reachable, 38 partial parses, 75 demoted, 133/229 unresolved —
   honest but visually thin (one reachable file, large empty UNREACHED column).
3. Docs leftovers: output contract missing `atlas-weight.html`/`atlas-q2.html`; stale seven-block
   references; whole-tenth vs "one decimal" wording. Step-15 docs pass.
4. Whole-document page height (includes the legacy Explorer/appendix below `#map`): 15,872 / 22,058px.
5. Capped synthetic state: canvas 1,398px > the fold (V8-disclosed; A2-owned).
6. The first below-canvas heading ("Where is the mass?", 890–907) is sliced by the 900px fold as a
   scroll affordance; no element is clipped by its container.
7. Masthead heroes "Atlas", not the repo name (root path omitted from the HTML).

## Decision

- **APPROVED — 2026-10-24.** Step 13 passes: the first-page composition is signed off and A2 (step 14) may
  be scheduled as a separate session. Basis: every measured clause above passed on the merged tree
  (`c73d6ff`), the 900px first screens and 1280 frames are byte-identical to the step-12 committed
  renders (`step12.json`), the weight crops / state frames / G composite still hash to their recorded
  values in `gate-step13.txt` §10 (and the state frames to `step13-reads.json`), the two added nav links
  are the requested Track C links below the fold, and A1 + strip remain 16/16 + 16/16. Delegated approval — the founder's own eye was not applied
  (see the provenance note at the top). Recorded in `state/session-state.json` (D-pf-253) and
  `state/handoff/latest.md`.
- **REJECT (not taken)** would have failed step 13 and required a superseding composition decision with no
  tuning; nothing in the evidence supported it.
- A2/ship remain separate gates: this approval does **not** ship the beta and does **not** mark the plan
  done (step 15).
