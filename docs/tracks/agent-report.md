# Track C — Agent report (LLM depth layer)

**Worktree:** `/home/shafin/Opencode/Perfpal-agent-report` (`feat/agent-report`, created from `821b984`, **rebased onto `main` `9a94905`** at C3).
**Start this tab:** open the worktree, first message:

> `Read /home/shafin/Opencode/Perfpal/docs/tracks/agent-report.md and execute Session C2.`

If the worktree is missing:

```bash
git worktree add ../Perfpal-agent-report -b feat/agent-report 821b984
```

**Track handoff:** `state/tracks/agent-report/handoff.md` · **Spec:** `docs/specs/agent-report.md` · **Plan:** `docs/plans/agent-report.md` · **A-pf-111** in the state/handoff.
**Role:** the full-fidelity LLM layer the simplified Atlas first page points to. Not a human dashboard; no LLM calls.

**Status 2026-10-24 — C1–C4 COMPLETE (merged).** `feat/agent-report`
is rebased onto `main` `9a94905`; code commit `7e5b921`. The CLI now always emits `agent-report.md`
(fixed seven-section skeleton, 25/25 inline caps, ≤80 KB on the §12 envelope) and `agent-actions.json`
(complete action list, 100-row profile cap), both pure functions of `MetricsDocument`: no flag, no schema,
no dependency. `tests/agent_report.rs` adds 9 integration tests (double-run byte equality, completeness
arithmetic, the 11-row state matrix, `detail_anchor` resolution, inertness, artifact set, sample parity,
size budget, human-page link); the 24 unit tests are unchanged. **C1 COMPLETE** (`6d32e67`, hash
`e730d01`). A step 12 (`9a94905`) shipped **without** the C link, so the founder authorized C to add the
two nav rows + the output-contract rows (+ the four `report.html` goldens, +2 lines each); recorded in the
track handoff. DoD 7–20 green; the §13 downstream eval is **deferred by the founder** (verbatim in the
handoff). **Merged --ff-only into `main` as `d097e04` (2026-10-24); no push.** **C4** (the C-owned
prune: `PERFPAL_REGEN_GOLDEN=1` deletes the two root-bearing agent artifacts from every `tests/golden/*`
tree again, so regeneration leaves the golden set byte-clean) merged as **`c73d6ff`**.

## Read first

1. A-pf-111 (`state/session-state.json`, `state/handoff/latest.md`).
2. `docs/specs/atlas-metrics.md`, `docs/schema/` (`metrics.json`), and a real `metrics.json` from `perfpal-test/out-atlas-v2-hyperfine`.
3. `docs/plans/atlas-build.md` step 12 — the human page link slot.
4. The existing `report.md`/`metrics.json`/`findings-detail.*`/`profile-queue.*`.

## Session C1 — contract/spec

**Goal:** define the artifact contract; **no production code**.

**Build:** `agent-report.md` + `agent-actions.json` as a **pure deterministic renderer of `metrics.json`**: rank key, fix hints, downstream eval, provenance/anchors, complete action list, units/caveats; no LLM calls, no network, no JS, no schema change, no new deps.

**Deliver:** `docs/specs/agent-report.md`, `docs/plans/agent-report.md`, `state/tracks/agent-report/handoff.md`; artifact names + a sample; no code.

**Do not:** edit `atlas_render.rs`/`atlas_brief.rs`/`atlas.rs`; no main state; no schema; no metrics duplication; no `mod.rs` wiring until A step 12.

**EOL:** track handoff; commit only with founder approval; no merge to `main`.

## Sessions C2–C3 (signposts)

- **C2 — implementation.** New `src/reporter/agent_report.rs` + tests; output artifacts; deterministic; no network.
- **C3 — tests + A link.** Golden/round-trip tests; A step 12 links to `agent-report.md`/`agent-actions.json`; review panel.
- Merge after A step 12; C is a later add-on.

## Guardrails / ownership

- C owns its module, artifact names, tests, spec/plan, `docs/evidence/agent-report/`.
- A links; D reads; no edits to A/D surfaces.
- Rebase on `main` at session start; no state/CHANGELOG merge.
