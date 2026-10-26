# ADR-0011: Atlas Story remains a companion; no in-report graduation in v1

- **Status:** Accepted (founder decision 2026-10-26)
- **Date:** 2026-10-26
- **Ratified:** 2026-10-26 (founder; companion option)
- **Deciders:** Founder + orchestrator
- **Gates:** resolves `atlas-story` §11 decision 3 and the "not decided here" item in
  `docs/research/atlas-run.md` §5. Does **not** amend `ADR-0009`, `ADR-0010`, A12, or any spec
  requirement; changes no schema, layout version, dependency or code. `atlas-build` steps 11–15 are
  unaffected.

## Context

- The Atlas Story shipped as a **companion** over four slices (S1–S4). Perfpal core emits the data
  (`atlas-layout.json`; `flow`, `purpose`, `profile`, `scenes`); a separate player in `sim/` renders
  the inferred run — entry → stages → loops → I/O → stop — with the payload crossing each file
  boundary labelled (`docs/specs/atlas-story.md:10-20,122-135`).
- **S4 evidence is strong on mechanism, not on product fit.** Final gate `71d0d04` (tree `51548f3`,
  dirty 0): 897 tests, `sim/verify.py` 304/304, 3k scale 44/44, page 463,490 bytes, first frame
  1,706 ms, `verdict=PASS` (`docs/evidence/atlas-sim/GATE.txt`). F1–F7 pass, including F6 byte-identity
  of `report.html` story-off vs story-on and F5 double-scan determinism. The F1/scale vision reads are
  **PASS-WITH-NITS, no blockers** (`docs/evidence/atlas-sim/VISION.md` → Read 8).
- **The companion is not a shipped artifact.** The player is `sim/index.html` + `sim/sim.js`
  (58,261 B) + `sim/sim.css`, and loads its data with `fetch('data-index.json')` and
  `fetch(entry.file)` (`sim/sim.js:1328,1426`) — it requires an HTTP server. `perfpal <repo> --story`
  adds only `atlas-layout.json` (`docs/report/output-contract.md:90-113`); the
  `perfpal story <repo>` subcommand is deferred (`atlas-story.md:157-160`).
- **The report is byte-deterministic and zero-new-JS.** `report.html` carries exactly one fixed
  `REPORT_SCRIPT` (`src/reporter/mod.rs:1718,2443-2447`); the Atlas canvas/SVG/animation artifacts
  carry zero `<script>`, zero handlers and zero network references (A12; `docs/specs/atlas-layout.md:198-201`;
  `ADR-0010:25-33,51-54`).
- **The founder interaction gate has not run.** The pre-registered D2 protocol is a founder review:
  watch at 1440×900 with motion, check reduced-motion and print, use both reference repos, ask the A2
  question, judge composition against the G baseline, then record *iterate / companion / graduate*
  (`docs/research/atlas-run.md:127-159`). F8 is **deferred, not run or claimed**
  (`atlas-story.md:117`; `docs/evidence/atlas-sim/NOTES.md:186`). V8 showed a comprehension-only gate
  can pass a page the founder rejects (`docs/atlas/AGENT-BRIEF.md:49-51`).
- **The report's own composition is unapproved.** `R-pf-109` is open, the v2 step-13 founder visual
  gate is pending, and A2/ship is blocked (`docs/plans/atlas-build.md:7-22`;
  `state/session-state.json` → `R-pf-109`).

## Decision

1. **The Atlas Story remains a standalone companion** for v1. It does not graduate into `report.html`
   in this phase.
2. **A12 is unchanged.** `report.html` keeps exactly one fixed `REPORT_SCRIPT`; the Atlas
   canvas/SVG/animation artifacts keep zero `<script>`, zero event handlers and zero network
   references. No CSS/SMIL story block is added to the report either — a film is not the story
   (option C).
3. **No code, schema, layout or dependency change.** `layout_version` and `flow_version` stay 1; the
   fixed artifact set is unchanged; `metrics.json`, `report.html` and the `--story` output are
   untouched.
4. **Graduation is a separate, founder-ratified decision** and requires all of:
   (a) a founder D2 session on a **self-contained** story artifact using the
   `docs/research/atlas-run.md` §5 protocol, including the F8/A2-style action read;
   (b) the report's step-13 founder visual gate passed (`R-pf-109` closed);
   (c) a scoped player design that is deterministic, offline, no-network, byte-identical on double
   render, with a complete final frame, pause, reduced-motion and print;
   (d) an explicit A12 amendment naming the second script block, its fixed source, its data channel
   and its gates. Until (a)–(d), in-report-with-controls stays blocked.
5. **The companion's packaging is a recorded follow-up, not work in this session.** A precondition for
   any D2 session is a **self-contained story artifact** (data inlined; no `fetch`, no server) that a
   user can open from the CLI (`perfpal story <repo>`, or `--story` writing it). This ADR does not
   scope or start that slice, and it makes **no ship claim** for the companion.
6. **The story stays the temporal companion to the Atlas map**, not a second map. It may not add a
   second metric encoding, a schema field, a time unit or a runtime claim
   (`atlas-story.md:27-35,71-83`).

## Options considered

### A — companion (chosen)

- **Pros:** preserves the report contract (byte-determinism, one fixed script, offline single file,
  LLM/agent consumption); the story can iterate without report goldens or A12 gates; no second
  surface is added to a composition the founder has not approved; the graduation decision is made on
  founder interaction evidence, as pre-registered; S4 evidence (F1–F7, 3k scale) stays valid.
- **Cons:** the story is not reachable from the default artifact; the companion is currently
  HTTP-only and repo-relative; two surfaces exist (report map + story player) and can drift; the CLI
  entry point is deferred.
- **Mitigation:** one data source and one layout (`atlas-layout.json`, probe P5); the player never
  re-derives geometry; provenance is carried as `source.sha256`; self-containment is a recorded
  precondition, not a nice-to-have.
- **What would change the choice:** (i) a founder D2 read that uses the story to name an action and
  asks for it in the report, with the step-13 composition approved and a scoped deterministic
  player (Decision 4) → graduate; (ii) a D2 read that does not use the story at all → cut or
  re-scope the story rather than graduate; (iii) a report-contract change that removes the
  A12/determinism constraint → option B becomes cheap.

### B — in-report with controls (rejected for now)

- Requires an A12 amendment (second script block, data channel, deterministic timeline,
  offline/no-network, double-render equality, complete final frame, pause/reduced-motion/print, new
  gates), plus porting the `sim.js` player into the Rust emitter and regenerating goldens. Every
  `report.html` would then carry the player and story data, or the story would remain a second
  artifact anyway.
- **Why rejected now:** it spends the A12/determinism surface before the founder has used the story,
  on top of an unapproved report composition (`R-pf-109`). Decision 4 keeps it available once its
  four conditions hold.

### C — in-report, CSS/SMIL only (rejected)

- A12-compliant today (`ADR-0010` already permits CSS/SMIL), and every user would see a film.
- **Why rejected:** it loses play/pause/step/scrub/speed/select, which is the story's product claim
  (`atlas-story.md:12-16`, §6), and the complete-final-frame rule turns the film into a reveal over a
  static page, not an instrument. It would also need a spec amendment to drop the control surface.

### D — hybrid: report film + companion instrument (deferred)

- Could give every user the first act while the companion keeps the controls, but it still needs B's
  report work (timeline, data, gates) and C's spec change. Recorded as a possible later split, not
  chosen.

## Consequences

**Positive**

- `report.html` stays a single deterministic, offline, zero-new-JS artifact; the LLM/agent contract
  and the fixed artifact set are untouched.
- The story keeps iterating in the companion without report goldens or A12 gates.
- The founder's D2 read remains the graduation gate, so the choice rests on interaction evidence,
  not on comprehension reads.
- No schema, dependency or `layout_version` churn; F1–F7 stay green as shipped.

**Negative / mitigated**

- The story is not reachable from the default artifact → recorded as the companion's next work
  (self-contained artifact + CLI entry point); until then it is a track artifact, not a shipped
  surface.
- Two surfaces can drift → same core data, no JS layout fork (P5), provenance hash, and the report's
  map remains the canonical artifact.
- A12 stays a hard boundary → any future graduation must amend it explicitly; no silent
  reinterpretation of the gate.
- The companion's HTTP-only loading is a real distribution defect → self-containment gates D2.

## Gates

- This ADR changes no gate. The report keeps A12/A4/A9/A10; the story keeps F1–F7 and the S4 gate
  (`bash sim/gate.sh`: 304/304 + 44/44, `verdict=PASS`).
- **F8 remains deferred, not claimed.** The D2 session is the founder gate for graduation.
- A2/ship remains blocked by `R-pf-109` until the Atlas v2 step-13 founder visual gate.

## References

- `docs/specs/atlas-story.md` §9–§11 · `docs/atlas/core-probes.md` P5–P8 ·
  `docs/specs/atlas-layout.md` §10 · `docs/specs/atlas-feature.md` A12 ·
  `docs/specs/map-animation.md` · `docs/report/output-contract.md:90-113`
- `docs/adr/ADR-0009-atlas-core.md` · `docs/adr/ADR-0010-declarative-animation.md`
- `docs/research/atlas-run.md:127-159` · `docs/evidence/atlas-sim/{GATE.txt,VISION.md,NOTES.md}` ·
  `docs/evidence/atlas-audit/AUDIT.md`
- `src/reporter/mod.rs:1718,2443-2447` · `sim/sim.js:1328,1426` · `sim/index.html:150`
- `docs/plans/atlas-build.md` steps 11–15 · `docs/atlas/AGENT-BRIEF.md:49-51`
