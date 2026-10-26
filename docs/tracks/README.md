# Perfpal parallel tracks — operating design (2026-10-24)

Four tracks, four worktrees, four sessions. **One session per worktree; never two sessions in one worktree.**

| Track | Worktree | Branch | Base | First session | Track handoff |
|---|---|---|---|---|---|
| **A Atlas beta** | `/home/shafin/Opencode/Perfpal` | `main` | `821b984` | **A1** step 11 rail | `state/handoff/latest.md` |
| **B JS/TS backend** | `/home/shafin/Opencode/Perfpal-js-ts` (archive) | `feat/js-ts` (merged into `main`) | `9a94905` → merged in merge-train M1 (`v0.3.0`) | **B1–B5 DONE · B6 merge gate DONE** (merged `main` `v0.3.0`) | `state/tracks/js-ts/handoff.md` |
| **C Agent report** | `/home/shafin/Opencode/Perfpal-agent-report` | `feat/agent-report` (merged) · `fix/agent-report-regen` (C4) | `821b984` → merged `d097e04` (C3) + `c73d6ff` (C4) | **C1** spec/contract · **C1–C4 DONE** | `state/tracks/agent-report/handoff.md` |
| **D Atlas Run** | `/home/shafin/Opencode/Perfpal-atlas-sim` | `feat/atlas-sim` | `821b984` | **D1** static run-story prototype | `state/tracks/atlas-sim/handoff.md` |

## How to start a tab

Open opencode in the track worktree and send **one** first message:

1. **A:** `Read /home/shafin/Opencode/Perfpal/docs/tracks/atlas-beta.md and execute Session A1.`
2. **B:** complete — `feat/js-ts` merged into `main` as `v0.3.0` in merge-train M1 (no further B session).
3. **C:** `Read /home/shafin/Opencode/Perfpal/docs/tracks/agent-report.md and execute Session C1.`
4. **D:** `Read /home/shafin/Opencode/Perfpal/docs/tracks/atlas-run.md and execute Session D1.`

The track file is the brief. Do not paste this README into a tab; it is the shared rulebook.

**Track B branch state:** B2-W0/B2-W1/B2-W2 are committed (`e7f5332`/`90d5677`/`edfc98a`); **B3 is
committed `9dd19bb`** (full DoD green, panel APPROVED); **B4 is committed `6d6bcdf`** (docs/contract/
canary + label polish; panel APPROVED: R1/R2/R3 + Q1/Q2); **B5 (post-B4 correctness pass) is committed `1b23198`
(docs `dbda7e3`)** (CALL-001 production 180 → 2, TYPE-001 14 → 2;
panel APPROVED: R1/R2/R3 + Q1/Q2). `B-tb-004` (CALL-001) is
resolved in B5. **B6 (merge gate) is DONE:** `feat/js-ts` was merged into `main` with `--no-ff` in
merge-train M1, the js-ts B6 DoD ran on the merged tree, and `main` is `v0.3.0` (metrics schema `1.8`).

## Shared rules (all tracks)

- **One session = one work-burst.** Commit per task only after the founder's explicit go-ahead; no merge to `main` without the track gate + founder approval; no push.
- **Main state = Track A.** A owns `state/session-state.json`, `state/handoff/latest.md`, `state/archive/ledger.jsonl`, `docs/activeContext.md`, `CHANGELOG.md`. B/C/D use `state/tracks/<slug>/session-state.json` + `handoff.md`; never edit or merge the main state files.
- **Track index:** this file. Each track updates only its row in its own branch; A reconciles rows at merge.
- **Read before write; JIT reads only.** The task gate + parallel review panel after every major task; the track handoff at EOL.
- **The beta is Track A.** B/C/D are parallel and non-blocking: B is a later release, C is a later add-on, D is R&D.

## Ownership matrix — only the owner writes; everyone else reads

| Surface | A | B | C | D |
|---|---|---|---|---|
| `main` state, `docs/activeContext.md`, `CHANGELOG.md`, `docs/tracks/README.md` | **OWN** | no | no | no |
| `src/reporter/atlas_*.rs`, `atlas_brief.rs`, `atlas.rs`, Atlas tests/goldens, `docs/specs/atlas-*`, `docs/plans/atlas-build.md`, `docs/evidence/atlas-visual/` | **OWN** | no | read | read |
| `src/scanner`, `src/parser`, `src/rules`, grammars, JS/TS fixtures, `src/schema` language values, `Cargo.toml`/`Cargo.lock`, `deny.toml`, `docs/specs/js-ts.md`, `docs/plans/js-ts.md`, JS/TS ADR | read | **OWN** | read | no |
| `src/reporter/agent_report.rs`, `agent-report.md`, `agent-actions.json`, agent tests, `docs/specs/agent-report.md`, `docs/plans/agent-report.md`, `docs/evidence/agent-report/` | read/link | no | **OWN** | read |
| `sim/`, `sim.html`, `docs/research/atlas-run.md`, `docs/evidence/atlas-sim/`, sim ADR | read | no | no | **OWN** |
| `src/reporter/mod.rs` / `views.rs` | **OWN until step 12** | language wiring after A step 12 | minimal wiring after A step 12 | no |
| `tests/golden/report*` (C/Rust/Python Atlas) | **OWN** | adds new JS/TS trees | no | no |
| `docs/tracks/<track>.md` | A (its file) | B (its file) | C (its file) | D (its file) |

## Shared contracts (contract-first)

- `metrics.json` schema: **B** owns additive JS/TS language values + ADR; A/C/D read. No field change without an ADR.
- `AtlasView`/renderer: **A** owns; C/D read. No edits.
- `agent-report.md` / `agent-actions.json`: **C** owns the names and contract; A links after step 12; D reads.
- `Cargo.toml`/`Cargo.lock`/`deny.toml`: **B** only. A/C/D add no Rust deps; D’s prototype stays outside Cargo.
- `views.rs`/`mod.rs`: A-owned until step 12; B/C wiring after, with a rebase.
- `CHANGELOG.md`: A owns; B/C/D note their change in the track handoff; A adds it at merge.
- No state-file merges across tracks.

## Merge / rebase

- A commits per task to `main`.
- B/C/D work only on their branch; `git rebase main` at session start and before their gate; merge to `main` only at the gate with founder approval.
- Merge order: **A step 12** (human layer + link slot) → C wiring + D production path + A’s agent link; **B’s schema/ADR** is additive and can land after A step 12; **D’s prototype** is independent; D production integration is post-beta.
- Beta = A only. B/C/D must not delay it.

## Ports / artifacts

- A: `http://127.0.0.1:8767/report.html#map` (current step-10 report; regenerate and refresh).
- D: `http://127.0.0.1:8768/` for the sim prototype.
- B/C: no server.
- Existing: `:8765` (8-state matrix), `:8766` (seven projects).
- Use separate `CARGO_TARGET_DIR`s if building Rust in parallel to avoid Cargo lock contention.

## The four briefs

- `docs/tracks/atlas-beta.md` — Track A, A1–A5.
- `docs/tracks/js-ts.md` — Track B, B1–B6 (B6 merged; track closed).
- `docs/tracks/agent-report.md` — Track C, C1–C3.
- `docs/tracks/atlas-run.md` — Track D, D1–D3.
