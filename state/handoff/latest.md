# Handoff — Merge Train M3 finalization (`main` @ v0.4.0; 2026-10-27)

**Updated:** 2026-10-27
**Repo:** `/home/shafin/Opencode/Perfpal`
**Session:** merge-train M3 — finalize the three-line merge on `main` (plan `merge-train` / M3)
**Verdicts:** M3 review panel (R1/R2 correctness + contract, Q1/Q2 verbatim scope + evidence); the
final verdicts are recorded by the M3 close commit.

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. M3 closes the merge
train that landed JS/TS and the Atlas Story **companion** on `main` at **v0.4.0** without changing the
report, the map, the schema or A12. **No drift:** no in-report graduation, no A2/ship claim, no
push/tag.

## Objective

Verify all four feature lines are ancestors of `main`; remove the agent-report worktree (reversibly);
prune merged branches (worktree registrations), run the full CI gate + story gate, verify
**0.4.0**/schema **1.8**, CHANGELOG/README, hot state ≤8 KB and clean status; update state/handoff
(EOL); no push/tag without founder go.

## Phase / plan
- Plan: `merge-train` · M3 · status: **done** (M1, M2, M3 all complete)
- Previous milestones: M1 `158c7b0` (v0.3.0) · M2 `646597c` (v0.4.0)
- Task spec: `docs/specs/merge-train.md` (R1–R10)

## Verification (claim → evidence)

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| All four lines are ancestors of `main` | `git merge-base --is-ancestor <b> main` for `feat/agent-report`, `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim` | all four printed `ANCESTOR` | `0` |
| Agent-report worktree removed (reversibly) | `mv /home/shafin/Opencode/Perfpal-agent-report /home/shafin/Opencode/.trash/Perfpal-agent-report-2026-10-27 && git worktree prune` | archive present in trash; `git worktree list` shows main + atlas-sim + js-ts only | `0` |
| Branch refs retained (no deletions) | `git branch -vv` | `feat/agent-report`, `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim` all present | `0` |
| Format | `cargo fmt --all -- --check` | clean | `0` |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | clean | `0` |
| Tests | `cargo test --locked` | 1,090 total (1,089 passed, 1 ignored) / 0 failed | `0` |
| Licenses | `cargo deny check licenses` | clean; five `license-not-encountered` allowance warnings (all allowlisted) | `0` |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | clean | `0` |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 1,092 total (1,091 passed, 1 ignored) / 0 failed | `0` |
| Full CI gate aggregate | `docs/evidence/merge-m3/gate.txt` at `2c01c17` | 6/6 commands `EXIT=0`; **2,180 passed / 0 failed / 2 ignored**; `OVERALL_FAIL=0` | `0` |
| Story gate on the final tree | `bash sim/gate.sh` (HEAD `978a623`) | `verdict=PASS`; 1,089 tests (1 ignored); 304/304 checks; 3k scale PASS; `worktree_dirty_zero` pass | `0` |
| Version / schema | `./target/debug/perfpal --version` | `perfpal 0.4.0 (metrics schema 1.8)` | `0` |
| Schema copies | `cmp src/schema/metrics.schema.v1.8.json docs/schema/metrics.schema.v1.8.json` | identical | `0` |
| Cargo versions | `grep version Cargo.toml`; `Cargo.lock` | both `0.4.0` | `0` |
| CHANGELOG | `grep '^## \[0.4.0\] - 2026-10-27'` / `[0.3.0]` | one each | `0` |
| README | `grep` markers | `Beta — 0.4.0`, metrics schema 1.8, Atlas Story companion, JS/TS sections present | `0` |
| Hot state | `wc -c state/session-state.json` | **4,884 B** (≤ 8,192) | `0` |
| State + ledger validity | `jq` / `json.loads` on hot state and every ledger line | valid; 753 ledger lines | `0` |
| No duplicate hot IDs | `jq group_by` on decisions/risks/actions/blockers | `[]` | `0` |
| ADR numbers unique | `ls docs/adr/` | `0001`–`0013`, one file each | `0` |
| Byte-determinism guard | `grep -c preserve_order Cargo.lock` | 0 | `0` |
| No tag | `git tag --points-at HEAD` | empty | `0` |
| Clean status (post-close) | `git status --porcelain` after the close commit | empty; ancestry/worktree/tag/state/version outputs in `docs/evidence/merge-m3/verify.txt` | `0` |

Non-runnable evidence:
- `docs/evidence/merge-m3/gate.txt` — full CI gate transcript at `2c01c17` (all exit 0).
- `docs/evidence/merge-m3/verify.txt` — post-close verification transcript (ancestry ×4, worktree list,
  branch refs, tag, version, hot-state bytes, state/ledger validity, duplicate-ID scan, clean status).
- `docs/evidence/atlas-sim/GATE.txt` — story-gate transcript re-run on the final tree at `978a623`
  (`verdict=PASS`).
- `docs/evidence/atlas-sim/checks.json` — 304/304 DOM/story checks at `978a623`.
- `docs/evidence/merge-m2/gate.txt`, `docs/evidence/merge-m2/resolutions.md` — M2 merge evidence.
- `state/archive/ledger.jsonl` — cold record: `D-pf-275` archived, `A-pf-127` done, `PHASE-pf-004`.

## Deliverables (changed / added)

- `state/session-state.json` — M3 phase label; live decision `D-pf-276`; `A-pf-127` closed; M2
  decision `D-pf-275` demoted to the cold ledger; **4,884 B**.
- `state/archive/ledger.jsonl` — append-only: `D-pf-275` (archived), `A-pf-127` (done),
  `PHASE-pf-004` (M3 record); 753 lines; valid JSONL.
- `state/handoff/latest.md`, `docs/activeContext.md` — M3 close / EOL.
- `docs/plans/merge-train.md` — M3 result; plan marked done.
- `CHANGELOG.md` — M3 pending item removed; `[0.4.0]` **Finalization** entry added.
- `docs/evidence/merge-m3/gate.txt` — full CI gate at `2c01c17`.
- `docs/evidence/atlas-sim/{GATE.txt,checks.json,*.png}` — story-gate re-run at `978a623`.
- Trash archive: `/home/shafin/Opencode/.trash/Perfpal-agent-report-2026-10-27` (outside the repo;
  reversible; the git worktree registration was pruned).

## Decisions (live)
- `D-pf-276` — MERGE-TRAIN M3 DONE: all four lines ancestors of `main`; agent-report worktree archived
  (registration pruned; branch refs retained per the founder); full CI gate 2,180/0/2; story gate
  re-run PASS at `978a623`; v0.4.0 / schema 1.8; clean status; no push/tag.
- `D-pf-251` — F treemap remains undecided; no F code ships.
- Story-line records are cold: `D-pf-267..274`, `A-pf-131/132`; M2 `D-pf-275` is now cold too.

## Risks (open)
- `R-pf-063` (high) — MINISIGN secrets must exist/match before tag.
- `R-pf-067` (high) — EULA/PRIVACY contact placeholder before beta tag.
- `R-pf-105` (medium) — V4 shipped-key recall; V7 re-triage.
- `R-pf-106` (medium) — static cost cannot surface per-byte/per-call Q3 leaves.
- `R-pf-108` (high) — structural LOW/MEDIUM edges on the hero canvas (option A reversal trigger).
- Story-specific accepted limits (cold, `state/tracks/atlas-sim/handoff.md`): fetch/HTTP-only
  companion, no `perfpal story` entry, 12-row queue, 24/5 scenes, synthetic 3k fixture, F8 deferred.

## Blockers
- `B-pf-001` — founder contact address for EULA/PRIVACY before tag.
- `B-pf-002` — repo secrets `MINISIGN_SECRET_KEY`/`MINISIGN_PASSWORD` before tag.

## Next actions
1. `A-pf-133` — story packaging: self-contained companion (inline data, no `fetch`/server) +
   `perfpal story <repo>` before the §5 founder session; F8/A2 story read deferred; no in-report
   graduation (A12 unchanged).
2. `A-pf-108`/`A-pf-125` — V7 re-triage / post-beta map work; F decision (`D-pf-251`).
3. Publish gate (`B-pf-001`/`B-pf-002`) — founder-owned; tag/publish only on explicit founder go.

## Uncommitted files

```
(none — the M3 docs/state/evidence close set is the final commit of this session)
```

## Commits (this session)
- `978a623` — docs(evidence): M3 full CI gate on main at 2c01c17 (fmt/clippy/test/deny/semantic green)
- `c84a3b2` — docs(evidence): story gate re-run at 978a623 (PASS; 1,089 tests, 304/304, scale PASS)
- M3 close commit (this handoff) — state/handoff/EOL + plan done + CHANGELOG finalization

**Resume in a fresh session:** the merge train is closed on `main` at v0.4.0; next work is `A-pf-133`
(story packaging) or the founder-gated publish path. No push/tag has been made.
