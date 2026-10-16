# NORTH-STAR.md — Perfpal

> **Founder-owned. Low-churn.** This is the pinned objective for the Perfpal repo; it is the
> anti-goal-dilution anchor (HARNESS-DESIGN **L6**). Only the founder edits it, at a session boundary —
> agents read it, they do not rewrite it. Detail lives in `docs/specs/`, `docs/roadmap.md`, `docs/adr/` and
> the root `AGENTS.md`; this file holds only the durable goal, non-goals, stage, and metrics.
> Every session restates the goal + phase and flags drift (see `AGENTS.md` → North-star (L6)).
>
> **Seed content** derived verbatim from committed docs (`docs/specs/perfpal-core-spec.md`,
> `docs/specs/evidence-beta-spec.md`, `docs/roadmap.md`, `AGENTS.md`). Business success metrics are
> **not sourced** — founder must ratify (see below).

## Goal
A CLI (Rust) that scans a codebase and produces quantifiable, performance-oriented insight — language
heuristics, architecture-level bottleneck visualization, and LLM-consumable metrics — so humans can see
bottlenecks and LLMs can optimize code with **explicit, quantified signals instead of guessing**. The
**maintainer birdseye map is the core feature**: one visual answering where to look first (ranked
hot-spots per KLOC), what breaks if touched (coupling/blast-radius), and where time might plausibly go
(static cost hints, profile-to-confirm) — findings ride as annotations on the map, never as a list
to cross-reference by hand. Beta
framing: **"Perfpal audits what AI wrote"** (primary audience = developers working with AI-generated code).
Perfpal doubles as a **deterministic divergence instrument**: it measures how AI-era code *diverges* from
human code in perf idioms via **rates and distributions, never raw counts** (call-shape, idiom/clone rate,
error-vs-hot stratification, alloc-family histogram). (`docs/specs/perfpal-core-spec.md:7-10`; `docs/roadmap.md:18-22`; `README.md:5`; `AGENTS.md:7-9`.)

## Non-goals
- **MVP** (one language = C): Web UI, SaaS/multi-user, CI-integration focus, languages beyond C, runtime
  profiling ingestion, multi-LLM output variants, terminal-native charts. (`perfpal-core-spec.md:28`.)
- Pre-GA: public/GA product, paid licenses/activation, deep obfuscation, cross-language linking, C++/JS/TS
  backends, T1 signing — and **no release before the publish gate clears**. (`docs/specs/evidence-beta-spec.md:81-83`.)
- Static analysis is **approximate, not ground truth** — outputs are "optimization candidates," never
  certainties; runtime validation is post-MVP. (`AGENTS.md:25-26`; `perfpal-core-spec.md:42`.)
- No full type-checker integration (e.g. mypy/pyright). (`docs/adr/0007`.)
- **No copyleft (GPL-family) dependencies**, ever, in the core tool. (`perfpal-core-spec.md:34-35`.)
- **No pure tool-only publication claim** — publishable novelty goes via the empirical divergence study
  (human vs AI perf idioms) where Perfpal is the deterministic instrument, not the subject.

## Current stage
**dev** — `v0.1.0` published; `v0.2.0-beta.1` code-complete but **publish PARKED behind the gate**;
`evidence-beta` M1✅ M2✅ M3✅ M4✅ + session-46 deep dive (181 prod-`hot` = 9 GENUINE / 124 minor / 48 FP,
no L4 hero); grading burst (correctness APPROVED / plan BLOCKED / depth C-, D-pf-214); entering
**map-first** (spec `maintainer-map`: plans `map-research` R1–R4 then `map-build` I1–I5 — the birdseye
map is the core feature; JS+TS backends and the paper queue behind it). (`state/session-state.json`;
`docs/roadmap.md:64-71`; `state/handoff/latest.md:7`.)

## Success metrics
Engineering ratchets + gate (sourced):
- **Per-rule precision/recall ratchets to 1.0/1.0 and is never lowered.** (`docs/roadmap.md:85,92-93`.)
- Self-scan noise reduction 1219→304 findings; 5 noise rules 1084→168. (`docs/roadmap.md:84`.)
- **Hard publish gate:** no release until the benchmark is built + RUN and the AI-code showcase verified.
  (`docs/specs/evidence-beta-spec.md:23-26`.)
- **Divergence instrument (proposed):** pre-registered 4–5 primary dimensions reported as rates with effect
  sizes + CIs, blinded dual-triage with κ, Zenodo replication packet (URL+SHA+scan-cmd+counts+byte-identical proof).
- **Agent-loop tool (proposed):** precision lift on top FP classes + downstream fix success with/without
  JSON+arch-map ablation.

_Founder-locked suffix:_ **UNSET (founder to ratify)**. `perfpal-core-spec.md:25` states "No formal success
criteria required yet" — there is no authoritative business/quality success section; the founder must
supply and lock it. (License/proprietary + no-copyleft are the only founder-locked invariants today,
`AGENTS.md:11-16`.)
