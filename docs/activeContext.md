# Active Context

## Current Task
Session 46 **performance-findings deep dive** (session-45-locked follow-on to `evidence-beta` M4). Hand-triaged **every production-code `hot` finding** on the 4 pinned M4 corpora — **181 candidates** (chunkhound 26, ripgrep 34, curl 121; Aider 0) → **9 GENUINE / 124 REAL_BUT_MINOR / 48 FP**. Deliverable: `docs/evidence/findings-deep-dive.md` + machine-readable verdicts `docs/evidence/showcase/<repo>/triage.json`. **No exact-spot L4 hero found** (strongest: chunkhound `src/db/duckdb_backend/write.rs:239` `serde_json::to_string(emb)` per embedding at loop depth 3 — L2/L3 with adjacent-subsystem corroboration only). **No product code changed.** Dual review ✅ both APPROVED.

## Recent Decisions
- **D-pf-199 (live):** session-46 deep dive COMPLETE. Triaged all **181** prod-`hot` (9 GENUINE / 124 REAL_BUT_MINOR / 48 FP). Scope reconciled: the handoff's ~259 (**curl 199**) was a copy error (`225 − 26`); true prod-`hot` = **181** (curl 121). **Hero verdict: no exact-spot L4 hero** — strongest = chunkhound `write.rs:239`; L4-negatives evidenced (ripgrep `GlobSet::new` PR #1915 maintainer-dismissed; curl `parse_netscape` PR #15826 was a **correctness** fix, not perf). Follow-ups: `#if`/`cfg(test)` reachability + `do/while(0)` guard + no-alloc constructors + `realloc` capacity guard.
- **D-pf-198 (live):** python-backend W4 dual-review gap is **closed** — `@reviewer` on `67fb682` returned APPROVED.
- **D-pf-197 (live):** evidence-beta M4 EVIDENCE RECORD COMPLETE — 4 pinned repos 2056 files/22010 findings, deterministic; hand-verified sample 174 = 70% TP / 26% FP / 4% AMBIG; headline: 72% of volume = `ARCH-003`; Rust backend weakest.
- *(D-pf-195 M3 / D-pf-196 Windows hardening demoted to cold `state/archive/ledger.jsonl` — append-only.)*

## In Progress
- Nothing — session-46 deep dive complete and committed. Plan `evidence-beta` M1–M4 + this follow-on done.

## Next
- **NEXT SESSION (`/new`) = `evidence-beta` M5** (first release) — with M2 run ✅ and M4 verified ✅, un-park `beta-release`: clear the residual beta blockers (EULA/privacy contact `B-pf-001` + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` `B-pf-002`), publish `v0.2.0-beta.1` as a clearly-labelled pre-release (`--prerelease --latest=false`), then T2 packaging. Read state + handoff first.
- **Optional precision follow-ups (from session 46, not gate-blocking):** the 5 `hot`-tier FP-class fixes (`#if`/`#[cfg(test)]` reachability, `do{…}while(0)` guard, `String::new()`/`vec![]` no-alloc, `realloc` capacity guard, streaming-I/O message/severity).
- **Optional M4 precision follow-ups:** Rust name-matching FP classes (`CALL-001`/`TYPE-001`/`TYPE-002`/`STR-002`/`STR-003`) + `ARCH-003` receiver-type awareness.
- **Carry `R-pf-094` (low):** root-package relative imports + LOOP-007 depth≥1 note.
- **Carry `R-pf-095` (low):** Windows semantic-clang tier + true-kernel behaviours unproven.
- **Carry (from M1/M2/M4):** `R-pf-077` (USR-ambiguity), `R-pf-078` (LOOP-008 deref-chain), `R-pf-080` (`r#` receiver), `R-pf-082` (corpus credibility).

## Blockers
- **M5 publish:** EULA/privacy contact address (`B-pf-001`) + `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` (`B-pf-002`) — unchanged.
- **Hygiene:** hot state must stay ≤ 8192 B (now 6790 B).

## History
- **L2 state split (2026-09-29):** hot/cold split — hot `state/session-state.json` compact schema v2 (≤ 8 KB), full history in cold `state/archive/ledger.jsonl`.
- **L6 north-star pinned (2026-10-01):** founder-owned `docs/NORTH-STAR.md`, `north_star_ref` in hot state, alignment-audit counter fields.
