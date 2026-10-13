# Handoff — Latest

## Objective
Session 45 — plan `evidence-beta` **M4** (the **AI-code credibility showcase**): the ADR-0005 publish-gate
precondition *"an AI-code credibility showcase is verified"*. Four permissively licensed repos were scanned at
**pinned commits** — **Aider** `v0.86.0` (`a4be6ccd…`, Apache-2.0), **chunkhound** `v6.0.0` (`a8553ae6…`, MIT),
**ripgrep** `14.1.1` (`4649aa97…`, MIT/Unlicense), **curl** `curl-8_11_1` (`75a2079d…`, MIT-like) — giving
**2056 files / 22010 findings**, every run exit 0 and `metrics.json` **byte-identical modulo provenance**. A
**hand-verified stratified sample of 174 findings** measured **70% TP / 26% FP / 4% AMBIG**. Deliverables: the umbrella
`docs/evidence/ai-code-showcase.md`, four per-repo **repro packets**, machine-readable
`docs/evidence/showcase/<repo>/{counts.json,verified.json}`, and the required **`accepted-limitations.md`**.
**No product code changed.**

## Phase
POST-MVP. Plans: `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ ·
`rust-backend` ✅ · `beta-release` ✅ code (publish **PARKED** = M5) · **`evidence-beta` ACTIVE (M1 ✅ · M2 ✅ · M3 ✅ · M4 ✅ record)** · **`python-backend` ✅ COMPLETE (W0–W4)**.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance heuristics,
  (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified metrics, so humans see
  bottlenecks and LLMs optimize with explicit signals (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; `evidence-beta` M1 ✅ · M2 ✅ · M3 ✅ · **M4 ✅ (evidence record)**.
- **How this session serves the north-star:** the evidence-first pitch ("Perfpal audits what AI wrote") is a *truth
  claim*; ADR-0005 requires a **verified** external showcase before any release. M4 makes precision **hand-checked on
  real AI-authored code** and discloses the false-positive classes honestly (`accepted-limitations.md`) — the
  north-star's "verifiable truth per finding" made concrete.
- **Drift flag: NONE.** M4 is exactly the queued next task (handoff + `activeContext`). Audit counter 5 → 6 (threshold 10; not due).

## This session (45) — evidence-beta M4
- **Corpora (pinned, cloned outside the repo at `/var/tmp/opencode/m4-showcase/`).** Aider `v0.86.0`,
  chunkhound `v6.0.0`, ripgrep `14.1.1`, curl `curl-8_11_1`; all four `rev-parse HEAD` matched the pinned commits.
- **Scan + determinism.** `cargo build --release --locked` → `perfpal 0.2.0-beta.1 (metrics schema 1.3)`; each repo
  scanned **twice**; `metrics.json` is byte-identical after masking only `scan.started_at` + `scan.duration_ms`
  (`root_path` identical). Per-repo masked sha256 recorded in packet §4.
- **Hand-verification.** Four parallel read-only agents (one per repo) opened each candidate's span and judged it
  against the rule's stated semantics; the three most consequential verdicts (filesystem `shutil.copy`; `super()`
  ≠ recursion; libc `strdup` vs `tool_strdup.c`) were **spot-checked by the orchestrator**. Sample plan: ≤3 for the
  dominant rule + ≤2 per other rule, preferring `hot`→`warn`→`info`.
- **Deliverables.** `docs/evidence/ai-code-showcase.md` (umbrella: method, corpora, dry-run, aggregate
  hand-verify, honest findings, reproduce, provenance); `docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md`
  (repro packet each: pinned URL+commit+license, one-command scan, expected counts, determinism hash, full per-rule
  counts + every verdict row); `docs/evidence/showcase/<repo>/{counts.json,verified.json}`; `docs/evidence/accepted-limitations.md`.
- **Docs/state.** CHANGELOG entry; roadmap status snapshot (session 45) + M4 status; `activeContext.md`; hot state
  (`D-pf-197`, `D-pf-198` appended; `D-pf-193`/`D-pf-194` demoted to the cold ledger — 533 → 535 records).
- **Parallel bonus — W4 dual-review gap CLOSED.** `@reviewer` on `67fb682` returned **APPROVED** (the content-filter
  outage cleared); only non-blocking doc/test-robustness nits.

## Verification
| Claim | Command | Result |
|---|---|---|
| 4 repos at pinned commits | `git -C <repo> rev-parse HEAD` | each == the recorded commit |
| Release build | `cargo build --release --locked` | up to date; `perfpal 0.2.0-beta.1 (metrics schema 1.3)` |
| Scans (×4, ×2 runs) | `perfpal <repo> -o out-<repo>` | exit 0; **2056 files / 22010 findings** |
| Determinism | double scan → mask `started_at`/`duration_ms` → sha256 | **IDENTICAL** ×4 |
| Hand-verify | 4 agents + 3 orchestrator spot-checks | sample **174: 122 TP / 45 FP / 7 AMBIG (70%)** |
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lint (default) | `cargo clippy --locked --all-targets -- -D warnings` | exit 0, 0 warnings |
| Lint (semantic-clang) | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| Tests (default) | `cargo test --locked` | **526 passed; 0 failed** |
| Tests (semantic-clang) | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | **531 passed; 0 failed** |
| No copyleft | `cargo deny check licenses` | `licenses ok` |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` | exit 0 |

**Dual review:** `@requirements-reviewer` **APPROVED** (all 8 requirements present). `@reviewer` **CHANGES_REQUESTED**
(round 1: 2 blocking numeric defects — C `STR-002` wrongly in the "TP across the board" list; `CALL-001` PY ratio
`1/5`→`1/6` — plus 2 nits) → **fixes applied** → **round-2 APPROVED** (numbers re-verified against the committed
JSONs; only cosmetic nits remained, now fixed). **No unresolved reviewer disagreement.**

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY contact
  placeholder), **`R-pf-082`** (medium; M2 corpora synthetic — now complemented by M4's real-repo evidence),
  **`R-pf-007`** (medium; differentiation claims rest on one survey pass), **`R-pf-094`** (low; root-package relative
  imports + LOOP-007 depth≥1), **`R-pf-095`** (low; Windows semantic-clang tier + true-kernel behaviours unproven).
- **NEW follow-up (not a risk record yet):** M4 surfaced concrete **Rust precision defects** — `CALL-001` name
  collisions, `TYPE-001` move≠copy, `TYPE-002` non-FFI padding, `STR-002` O(1) `len()`, `STR-003` `set_len` — and
  `ARCH-003` has no receiver-type awareness. Recorded in `accepted-limitations.md` §D; any fix must be
  precision-measured (M1 KPI + M2 benchmark), never lowered.
- **Blockers (hot, unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Hot-state hygiene:** `state/session-state.json` = **7258 B / 8192 B**; this session demoted `D-pf-193`/`D-pf-194` to cold `state/archive/ledger.jsonl` (append-only; 533 → 535 records).

## Next Actions
1. **NEXT SESSION (`/new`) = performance-findings deep dive** (user-directed after M4): triage the
   **production-code `hot` findings** from the M4 showcase (~**259**: chunkhound 26, ripgrep 34, curl 199;
   aider 0) — dominated by `ALLOC-001` (113) + `LOOP-002` (104) + C `ALLOC-002` (37) — hunting for genuine
   **critical performance bugs** (real + reachable + impactful; ideally corroborated by an upstream fix/issue).
   Goal: concrete evidence that Perfpal finds things maintainers also consider real. Scope + "critical bug" bar
   **pending user decision**. Deliverable: an evidence dossier of the standout bug(s).
2. **Then `evidence-beta` M5** (first release): with **M2 run ✅ and M4 verified ✅**, un-park `beta-release` —
   clear `B-pf-001`/`B-pf-002`, publish `v0.2.0-beta.1` as a clearly-labelled pre-release
   (`--prerelease --latest=false`, never displacing `v0.1.0` as Latest), then T2 packaging.
3. Optional **M4 precision follow-ups** (not gate-blocking): the Rust name-matching FP classes + `ARCH-003` receiver-type awareness.
4. Carry **`R-pf-094`** (low) and **`R-pf-095`** (low).
5. Carry (from M1/M2/M4): `R-pf-077` (USR-ambiguity), `R-pf-078` (LOOP-008 deref-chain), `R-pf-080` (`r#` receiver), `R-pf-082` (synthetic-corpus credibility).

## Change set (this session)
- **Added:** `docs/evidence/ai-code-showcase.md`; `docs/evidence/accepted-limitations.md`; `docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md`; `docs/evidence/showcase/<repo>/{counts.json,verified.json}`.
- **Modified:** `CHANGELOG.md`; `docs/roadmap.md`; `docs/activeContext.md`; `state/session-state.json`; `state/archive/ledger.jsonl`; `state/handoff/latest.md`.
- **Excluded / untracked:** `brag-output/`, `perfpal-report/` (unrelated working dirs) and `docs/research/` (founder note, not M4).

## Commit
1. **`<work>`** — `evidence-beta` M4: AI-code showcase evidence record (umbrella + 4 repro packets + counts/verified JSON + accepted-limitations).
2. **(EOL)** — state/doc truth-up for M4.

## L2 state split / L6 north-star (history)
The L2 hot/cold split landed **2026-09-29**: hot `state/session-state.json` is compact schema v2 and ≤ 8 KB, with full history in cold `state/archive/ledger.jsonl` (greppable, never auto-read). The L6 north-star (`docs/NORTH-STAR.md`, founder-owned goal / non-goals / stage / metrics) was pinned **2026-10-01** and referenced by `north_star_ref` in hot state.
