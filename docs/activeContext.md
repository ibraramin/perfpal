# Active Context

## Current Task
Session 34 **CLOSED**. B6 unblocked half committed as **`b3a1105`**. A founder **direction reset** is recorded in `docs/roadmap.md` + `state/` and is **PENDING FORMALIZATION** (planning-toolkit spec+plan in the next session).

## Recent Decisions
- **Story:** *"Perfpal audits what AI wrote"* (AI-assisted large codebases).
- **Language order:** **C → Rust → Python → JS → TS → C++** (Python promoted; C++ demoted). JS/TS separate.
- **Publish gate:** NO release until the benchmark suite is **built + run** AND the AI-code showcase is **verified**.
- **Sequence:** M1 Precision Gate → M2 Benchmark (build+RUN) → M3 Python → M4 showcase (Aider/chunkhound) → M5 first release → M6+ JS→TS→C++ → cross-language.
- **Shipping:** T0 + T2 (winget, own Scoop bucket, own Homebrew tap, AUR `-bin`, cargo-binstall); **T1 deferred to GA**.
- **Cross-language linking:** its own milestone, after the language sequence.
- **Beta:** `v0.2.0-beta.1` code complete, **publish parked**.

## In Progress
- Nothing — planning only; no code in flight.

## Next
- **NEXT SESSION (`/new`) = PLANNING-ONLY:** planning-toolkit (`create_spec` → `create_plan`) + `docs/roadmap.md` rewrite + ADRs. Read `state/session-state.json` + `state/handoff/latest.md` + this file FIRST, then `readPlan beta-release`.

## Blockers
- **M5 publish:** EULA/privacy contact address + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` secrets.
- **GA (T1):** Azure Artifact Signing likely ineligible (Bangladesh); macOS notarization $99/yr.
- Carried: R5 cross-language ARCH-003 substitution needs founder sign-off.
