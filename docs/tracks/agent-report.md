# Track C — Agent report (LLM depth layer)

**Worktree:** `/home/shafin/Opencode/Perfpal-agent-report` (`feat/agent-report`, base `821b984`; rebase after A step 12).
**Start this tab:** open the worktree, first message:

> `Read /home/shafin/Opencode/Perfpal/docs/tracks/agent-report.md and execute Session C1.`

If the worktree is missing:

```bash
git worktree add ../Perfpal-agent-report -b feat/agent-report 821b984
```

**Track handoff:** `state/tracks/agent-report/handoff.md` · **Spec:** `docs/specs/agent-report.md` · **Plan:** `docs/plans/agent-report.md` · **A-pf-111** in the state/handoff.
**Role:** the full-fidelity LLM layer the simplified Atlas first page points to. Not a human dashboard; no LLM calls.

**Status 2026-10-24 — C1 COMPLETE (contract only; no production code).** Contract locked as
`contract_version 1`: artifacts `agent-report.md` (bounded, fixed seven-section skeleton, ≤64 KB) +
`agent-actions.json` (complete action list, 100-row profile cap named in-file); KV1/KV3 rank keys
reused verbatim; 67 curated fix hints with a completeness lock; no clock provenance; sample +
gate transcript under `docs/evidence/agent-report/`. Founder-ratified decisions D1–D8 in spec §2.
Uncommitted pending founder approval. **C2 is next** — and its `mod.rs`/CLI wiring still waits on
A step 12.

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
