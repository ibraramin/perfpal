# Active Context

## Current Task
Session 45 **`evidence-beta` M4** — the **AI-code credibility showcase** (ADR-0005 publish-gate precondition). Four permissively licensed repos scanned at **pinned commits** — Aider `v0.86.0` (`a4be6ccd…`), chunkhound `v6.0.0` (`a8553ae6…`), ripgrep `14.1.1` (`4649aa97…`), curl `curl-8_11_1` (`75a2079d…`) — **2056 files / 22010 findings**, deterministic (byte-identical modulo `scan.started_at`/`duration_ms`). **Hand-verified stratified sample (174 findings): 70% TP / 26% FP / 4% AMBIG.** Deliverables: `docs/evidence/ai-code-showcase.md` (umbrella record), four repro packets `docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md`, `docs/evidence/showcase/<repo>/{counts.json,verified.json}`, and the required `docs/evidence/accepted-limitations.md`. **No product code changed.** The M4 reviewer-gap re-run on `67fb682` also returned **APPROVED** (non-blocking nits only).

## Recent Decisions
- **D-pf-197 (live):** M4 EVIDENCE RECORD COMPLETE. Honest headline: **72% of all findings are the single `ARCH-003`** name-based coupling rule (all `info`); the name-matching rules (`ARCH-003`/`CALL-001`/`MEM-002`) are the FP engine; the **Rust backend is the weakest** (ripgrep 42% TP — `CALL-001` name collisions, `TYPE-001` move≠copy, `TYPE-002` non-FFI padding, `STR-002` O(1) `len()`, `STR-003` `set_len`); the allocation/loop/string/async core is uniformly TP. Follow-ups seeded in `accepted-limitations.md` §D.
- **D-pf-198 (live):** the python-backend W4 dual-review gap is **closed** — `@reviewer` on `67fb682` returned **APPROVED** (content-filter outage cleared); only non-blocking doc/test-robustness nits.
- **D-pf-196 (live):** post-W4 cross-platform (Windows) hardening (commit `cccd91c`) — ungated unix test gated; resolver re-read path join fixed; `discover` junction-cycle guard; `windows-latest` CI job; MSVC `+crt-static`. Residuals → `R-pf-095`.
- **D-pf-195 (live):** M3 COMPLETE — Python is the third first-class language (23 C + 22 Python + 22 Rust = 67 rules, schema 1.3).

## In Progress
- Nothing — M4 evidence record complete and committed. Plan `evidence-beta` M4 done.

## Next
- **NEXT SESSION (`/new`) = `evidence-beta` M5** (first release) — with M2 run ✅ **and M4 verified ✅**, un-park `beta-release`: clear the residual beta blockers (EULA/privacy contact address `B-pf-001` + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` `B-pf-002`), publish `v0.2.0-beta.1` as a clearly-labelled pre-release (`--prerelease --latest=false`), then T2 packaging. Read state files + handoff first.
- **Optional precision follow-ups (from M4, not gate-blocking):** the Rust name-matching FP classes (`CALL-001`/`TYPE-001`/`TYPE-002`/`STR-002`/`STR-003`) and `ARCH-003` receiver-type awareness. Any fix must be precision-measured (M1 KPI + M2 benchmark) before landing.
- **Carry `R-pf-094` (low):** root-package relative imports + LOOP-007 depth≥1 note.
- **Carry `R-pf-095` (low):** Windows semantic-clang tier + true-kernel behaviours unproven.
- **Carry (from M1/M2/M4):** semantic-tier USR-ambiguity guard (`R-pf-077`); `PERF-RS-LOOP-008` deref-chain (`R-pf-078`); `r#` raw-identifier receiver nit (`R-pf-080`); M2 synthetic-corpus credibility (`R-pf-082`).

## Blockers
- **M5 publish:** EULA/privacy contact address (`B-pf-001`) + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (`B-pf-002`) — unchanged.
- **Hygiene:** hot state must stay ≤ 8192 B (see `state/session-state.json`).

## History
- **L2 state split (2026-09-29):** hot/cold split — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in hot state, alignment-audit counter fields.
