# Handoff — Latest

## Objective
Session 34 **CLOSED**. Two things:
1. **`beta-release` B6 — unblocked half** committed as **`b3a1105`** (draft-safe pre-release publish + tag discipline + 19-asset website inventory).
2. A **founder direction reset** (planning conversation): the path to the first release is re-scoped to an **evidence-first AI-code beta**. Recorded here + in `docs/roadmap.md` `## Direction`; **PENDING FORMALIZATION** (planning-toolkit spec+plan next session).

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · **`beta-release` ✅ code (B0–B6, `b3a1105`); PUBLISH PARKED** (see Direction).

## The direction (founder, session 34 — supersedes the prior roadmap)
- **Story:** *"Perfpal audits what AI wrote"* — performance landmines in large **AI-assisted** codebases. (The "100%-AI-written C/Rust" claim is dropped: no famous large mostly-AI-generated C/Rust OSS exists.)
- **Language order:** **C → Rust → Python → JS → TS → C++** (Python promoted, C++ demoted). Data: AI-code share Python 41.7%, JS 15.4%, C++ 7.8%, C 3.7%, Rust not top-8 (arXiv 2603.27130). JS/TS are separate milestones. **Cross-language linking = a separate milestone AFTER** the sequence.
- **Publish gate: NO release until (a) the benchmark suite is built AND run, and (b) the AI-code credibility showcase is verified.**
- **Sequence:** M1 Precision Gate (C+Rust) → M2 Benchmark (build + RUN) → M3 Python backend → M4 AI-code showcase (Aider / chunkhound) → **M5 First release** → M6+ JS→TS→C++ → cross-language.
- **Shipping: T0 + T2** — the existing 6 unsigned targets + packaging (winget, own Scoop bucket, own Homebrew tap, AUR `-bin`, cargo-binstall). **T1 signing/notarization deferred to GA.**
- **Evidence discipline:** repro packet (pinned URL+commit+license, one-command scan, expected counts, determinism proof) + hand-verified true positives + an accepted-limitations page; showcase only MIT/Apache/BSD.

## Version snapshot
- **Published:** `v0.1.0` (C-only). **Unreleased:** `0.2.0-beta.1` (C+Rust), schema `1.2`. `main` = **`b3a1105`**, **unpushed**.
- Beta **code complete** (B0–B6); **publish parked** behind the evidence gate.

## Evidence
- **B6:** 393 tests / 0 fail; fmt + clippy + actionlint v1.7.12 clean; musl smoke test `examples/` 83, `examples-rust/` 41.
- **Dogfood (R4):** `perfpal src` → 33 files, 1219 findings, **~89% from 5 rules** → the precision gate is M1 and existential for the pitch.
- **Research:** `lib-1` (AI-code language share; showcase feasibility; attention) and `lib-2` (2026 signing/packaging). Key facts recorded in `state/session-state.json` decisions.

## Risks
- **Pipeline never run on GitHub** — first-run risk now deferred to M5 (build-only dry-run can validate build+SBOM legs; the publish leg needs `MINISIGN_*`).
- **Precision credibility** — raw output on a famous repo backfires; hence M1 before M2/M4.
- Carried: R5 cross-language ARCH-003 substitution still needs founder sign-off.

## Blockers
- **At M5 publish:** EULA/privacy **contact address** + **`MINISIGN_SECRET_KEY`**/**`MINISIGN_PASSWORD`** secrets.
- **At GA (T1):** Azure Artifact Signing likely ineligible (Bangladesh not listed; Individual = US/Canada only); macOS notarization $99/yr.

## Next Actions
1. **NEXT SESSION (`/new`) = PLANNING-ONLY.** Formalize the direction via planning-toolkit: `create_spec` (evidence-first AI-code beta) → `create_plan` (M1…M6+) → rewrite `docs/roadmap.md` `## Direction` into real milestones → record ADRs (language-order change; cross-language deferral; evidence/publish gate; T0+T2 packaging). Read `state/session-state.json` + this file + `docs/activeContext.md` FIRST, then `readPlan beta-release`. **No implementation code in that session.**

## Change set (this commit, docs-only)
- **Modified:** `docs/roadmap.md`, `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`.
- **Commit message:** `docs(roadmap): record founder direction — evidence-first AI-code beta (C→Rust→Python→JS→TS→C++), publish gated on benchmark+showcase, T0+T2 shipping (pending formalization)`.
