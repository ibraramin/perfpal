# Handoff — Latest

## Objective
Session 46 — **performance-findings deep dive** (the session-45-locked Next Action 1): hand-triage
**every production-code `hot` finding** Perfpal emitted on the four pinned M4 corpora and hunt for a
genuine **L3+L4 hero bug**. Deliverable: `docs/evidence/findings-deep-dive.md` + machine-readable
per-repo verdicts `docs/evidence/showcase/<repo>/triage.json`. **No product code changed.**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ ·
`rust-backend` ✅ · `beta-release` ✅ code (publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 ✅ · M4 ✅)** · **`python-backend` ✅ COMPLETE (W0–W4)** · session-46 deep dive ✅ (follow-on).

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance heuristics,
  (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified metrics, so humans see
  bottlenecks and LLMs optimize with explicit signals (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; `evidence-beta` M1–M4 ✅; session-46 deep dive is the user-locked
  follow-on before M5.
- **How this session serves the north-star:** it stress-tests the **truth claim** ("Perfpal finds things
  maintainers also consider real") on real code by hand-checking the highest-severity tier and reporting an
  **honest negative** on the L4 bar — the north-star's "verifiable truth per finding", made concrete.
- **Drift flag: NONE.** Deep dive is exactly the queued next task. Audit counter 6 → 7 (threshold 10; not due).

## This session (46) — findings deep dive
- **Corpora.** The four pinned M4 checkouts at `/var/tmp/opencode/m4-showcase/` (Aider `v0.86.0` `a4be6ccd…`,
  chunkhound `v6.0.0` `a8553ae6…`, ripgrep `14.1.1` `4649aa97…`, curl `curl-8_11_1` `75a2079d…`), all four
  **un-shallowed to full history** (`git fetch --unshallow`) for the L4 search.
- **Scope reconciliation (259 → 181).** The session-45 lock said "~259 (curl 199)". chunkhound 26 ✓ and
  ripgrep 34 ✓ reproduce exactly under the production filter (exclude `tests/`,`testing/`,`testdata/`,`benches/`,
  `examples/`,`fixtures/` + files `test_*`/`*_test`/`testutil.*`/`util.rs`); **curl is 121, not 199** — `199 =
  225 − 26` (chunkhound's total subtracted from curl's). **True prod-`hot` total = 181** (chunkhound 26,
  ripgrep 34, curl 121, aider 0); curl's raw→prod drop of 104 = `tests/*` (81) + `docs/examples/*` (23).
- **Triage.** 5 read-only agents (chunkhound, ripgrep, curl × 4 rule-group lanes) opened every span and classified
  **GENUINE / REAL_BUT_MINOR / FP** with a cited reason. Result: **9 GENUINE / 124 REAL_BUT_MINOR / 48 FP**.
- **Hero-bug hunt (L2→L3→L4).** Strongest = chunkhound `src/db/duckdb_backend/write.rs:239` — `serde_json::to_string(emb)`
  serialises the whole embedding vector per row inside a depth-3 loop on the DB-write hot path (**L2** reachable via
  `write_batch_incremental:615` / `write_batches_in_one_txn:684`; **L3** per-embedding serialize→parse round-trip,
  ~10–30 KB text × every stored embedding). **Verdict: NO exact-spot L4 hero** — chunkhound `main` (tip `a131f5b`)
  still contains the line (unchanged since the initial Rust port `5f9ddfd "Main rust flow (#380)"`); L4-negatives
  documented: ripgrep `GlobSet::new` (PR #1915 — maintainer: allocations there are "a pittance"); curl
  `parse_netscape` (PR #15826, exact spot, but reworked for a **const-input crash**, not perf).
- **Deliverables.** `docs/evidence/findings-deep-dive.md` (method + L1–L4 ladder + full verdict inventory +
  FP-class catalogue + hero hunt + honest limitations); `docs/evidence/showcase/{chunkhound,ripgrep,curl}/triage.json`.
- **Docs/state.** CHANGELOG entry; `activeContext.md`; hot state (demoted `D-pf-195`/`D-pf-196` to cold ledger,
  appended `D-pf-199`; ≤ 8 KB).

## Verification
| Claim | Command | Result |
|---|---|---|
| Prod-`hot` set reproduces | filter script over M4 `out1-*/metrics.json` | **181** (aider 0 / chunkhound 26 / ripgrep 34 / curl 121) |
| Triage coverage | sum of `docs/evidence/showcase/<repo>/triage.json` | **181 = 9 GENUINE / 124 REAL_BUT_MINOR / 48 FP** |
| Format | `$HOME/.cargo/bin/cargo fmt --all -- --check` | exit 0 |
| Lint (default) | `$HOME/.cargo/bin/cargo clippy --locked --all-targets -- -D warnings` | exit 0, 0 warnings |
| Tests (default) | `$HOME/.cargo/bin/cargo test --locked` | **526 total passed; 0 failed** |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` | exit 0 |
| No product code changed | `git status` (this session) | only docs/evidence + state files |

**Dual review (both APPROVED).** `@requirements-reviewer` **APPROVED** (every prod-`hot` candidate triaged;
deliverable path exact; 259→181 deviation disclosed & justified). `@reviewer` **CHANGES_REQUESTED (round 1)** —
1 blocking factual item: §6.1 named the wrong caller of `insert_embeddings_txn` (`write_batch_inner` →
**`write_batch_incremental`**, verified: defs `write_batch_inner:510` / `write_batch_incremental:585` /
`write_batches_in_one_txn:662`) — plus nits (an absent `testutil.rs` citation, unstated curl exclusions, a §6.4
GlobSet tension, a typo, an unverifiable PR). **All fixed** → **round-2 APPROVED** (no numeric drift).
**No unresolved reviewer disagreement.**

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY contact
  placeholder), **`R-pf-082`** (medium; synthetic-corpus credibility — complemented by M4), **`R-pf-007`** (medium;
  differentiation claims rest on one survey pass), **`R-pf-094`** (low), **`R-pf-095`** (low; Windows residuals).
- **NEW follow-up (not a risk record yet):** the deep dive distilled concrete **precision opportunities** for the
  `hot` tier — (1) suppress inactive `#if 0`/`#ifdef`/`#[cfg(test)]` regions (≈19 FP); (2) guard the `do{…}while(0)`
  idiom (4 FP); (3) treat `String::new()`/`vec![]` as no-alloc (≈6 FP); (4) capacity-guard `realloc` (≈5 FP);
  (5) rule-recommendation ≠ detection for streaming/channel I/O. Any fix must be **precision-measured** (M1 KPI +
  M2 benchmark) and never lower a ratchet.
- **Blockers (hot, unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Hot-state hygiene:** `state/session-state.json` = **6790 B / 8192 B**; session 46 demoted `D-pf-195`/`D-pf-196`
  to cold `state/archive/ledger.jsonl` (append-only; 535 → 537 records).

## Next Actions
1. **NEXT SESSION (`/new`) = `evidence-beta` M5** (first release): with **M2 run ✅ and M4 verified ✅**, un-park
   `beta-release` — clear `B-pf-001`/`B-pf-002`, publish `v0.2.0-beta.1` as a clearly-labelled pre-release
   (`--prerelease --latest=false`, never displacing `v0.1.0` as Latest), then T2 packaging
   (winget portable, own Scoop bucket, own Homebrew tap, AUR `-bin`, cargo-binstall). Read state + this handoff first.
2. **Optional precision follow-ups** (from this session, not gate-blocking): the 5 FP-class fixes above
   (`#if`/`cfg(test)` reachability, `do/while(0)` guard, no-alloc constructors, `realloc` capacity guard).
3. Optional **M4 precision follow-ups**: Rust name-matching FP classes (`CALL-001`/`TYPE-001`/`TYPE-002`/`STR-002`/`STR-003`)
   + `ARCH-003` receiver-type awareness — precision-measured before landing.
4. Carry **`R-pf-094`** (low) and **`R-pf-095`** (low).
5. Carry (from M1/M2/M4): `R-pf-077` (USR-ambiguity), `R-pf-078` (LOOP-008 deref-chain), `R-pf-080` (`r#` receiver), `R-pf-082`.

## Change set (this session)
- **Added:** `docs/evidence/findings-deep-dive.md`; `docs/evidence/showcase/{chunkhound,ripgrep,curl}/triage.json`.
- **Modified:** `CHANGELOG.md`; `docs/activeContext.md`; `state/session-state.json`; `state/archive/ledger.jsonl`; `state/handoff/latest.md`.
- **Excluded / untracked:** `brag-output/`, `perfpal-report/`, `docs/research/` (unrelated working dirs).

## Commit
1. **`<work>`** — findings deep dive: 181 prod-`hot` triage + `findings-deep-dive.md` + per-repo `triage.json`.
2. **(EOL)** — state/doc truth-up for session 46.

## L2 state split / L6 north-star (history)
The L2 hot/cold split landed **2026-09-29**: hot `state/session-state.json` is compact schema v2 and ≤ 8 KB, with full history in cold `state/archive/ledger.jsonl` (greppable, never auto-read). The L6 north-star (`docs/NORTH-STAR.md`, founder-owned goal / non-goals / stage / metrics) was pinned **2026-10-01** and referenced by `north_star_ref` in hot state.
