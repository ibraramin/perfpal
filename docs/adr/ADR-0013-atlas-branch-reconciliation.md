# ADR-0013: Reconcile the Atlas Story branch with main before further Atlas work

- **Status:** Accepted — direction C recorded 2026-10-26; **overridden by the founder 2026-10-27**
  (merge now; see the addendum). Merge executed in `main` (v0.4.0).
- **Date:** 2026-10-26
- **Deciders:** Founder (approval) · orchestrator (recommendation)
- **Requirement:** `R-pf-112`
- **Related:** `ADR-0012`; `docs/plans/atlas-build.md`; `docs/specs/atlas-story.md` §9/§11;
  `docs/research/atlas-run.md` §5; `docs/plans/beta-release.md`
- **Supersedes:** the stale "fresh session: Atlas v2 step 11" signpost; **not** `ADR-0012`.

## Context

- **Snapshot.** `feat/atlas-sim` is at `988aea2` (story S1–S4 + `ADR-0012`); `main` is at `c797004`
  (Atlas v2 DONE). Merge base `821b984` (v2 steps 9–10).
- **Actual divergence at the snapshot: 63 story commits ahead, 23 main commits behind**
  (`git rev-list --left-right --count main...988aea2` → `23 63`). This record commit adds one docs
  commit, so at `5250451` the branch is 64 ahead; the reconciliation facts use `988aea2`. The
  recorded "61 ahead / 8 behind" in `state/tracks/atlas-sim/handoff.md` and `docs/activeContext.md`
  is stale; this ADR's count is authoritative.
- **Main is the completed v2 line.** Steps 11–15 are done on main: step 11 `189df91`, step 12
  `9a94905`, step 13 `86c0537` (approved `D-pf-253`), step 14 `9631137` (A2 delegated pass
  `D-pf-255`), step 15 `57d91a2` (memchr residual `D-pf-256`; A2 closure `D-pf-257`; plan done
  `D-pf-258`). `R-pf-109` is closed as a record on main; `atlas-build` is done; no push.
- **This branch is a pre-merge story snapshot.** S1–S4 are committed and gated (`71d0d04`; 897
  tests; `sim/verify.py` 304/304; 3k 44/44; page 463,490 B; first frame 1,706 ms). `ADR-0012` decided
  the story remains a standalone companion; A12 is unchanged; the §5 founder session has not run;
  the companion is fetch/HTTP-only, has no `perfpal story` entry point, and is not shipped. F8 is
  deferred.
- **The branch's own status was stale.** At the snapshot, its `atlas-build` copy was `active` with a
  2026-10-23 "BLOCKED … steps 14–15" banner; its state said "step 11 next" and `R-pf-109` open.
  Main has already closed steps 11–15. **Do not re-run step 11 from this branch.**
- **A one-pass sync is feasible but not free.** `git merge-tree --write-tree --name-only main HEAD`
  reports seven conflicted paths: `README.md`, `docs/activeContext.md`,
  `src/reporter/atlas_render.rs`, `src/reporter/mod.rs`, `state/archive/ledger.jsonl`,
  `state/handoff/latest.md`, `state/session-state.json`. `CHANGELOG.md`,
  `docs/report/output-contract.md`, the Atlas specs and `src/cli/mod.rs` auto-merge. One API
  conflict is known: main's `strip_entry_line` is private; the story projection calls it (the branch
  makes it `pub(crate)`), and the function bodies are otherwise identical. The story's
  `atlas_brief::start_actions` and `atlas::build_atlas_view` survive on main; the story does not use
  main's retired seven-block brief.
- **IDs collide across the two lines.** Branch `D-pf-251/252/253/254/255` mean D2 verdict / S1 / S2 /
  S3 / S4; main uses the same IDs for F-treemap / archived / step-13 approval / archived / A2 PASS.
  Branch `A-pf-124/125` (S4 / `ADR-0012`) collide with main's commit-publish action and post-beta
  follow-ups. At the snapshot, `R-pf-109` was open here and is closed on main. `D-pf-259` was unique
  to the branch then, but M1 later allocated `D-pf-259/260` and `A-pf-126/127` on main — hence the
  merge's second remap (addendum).
- **Product constraint.** `beta-release` is active but the publish gate is founder-owned and parked
  (`B-pf-001` contact, `B-pf-002` signing secrets). Merging the story now would put an HTTP-only,
  unshipped companion and the opt-in `--story` flag on the release line before the founder has used
  the post-S4 story. `ADR-0012` Decision 5 makes packaging (self-contained artifact + CLI entry) the
  precondition for the §5 session and makes no ship claim.

## Options considered

### A — Rebase the story onto main, then merge it (story becomes mainline)

- **Pros:** one line; the branch stops drifting; packaging/§5 work happens on current main; the
  story's 63 commits stay as a linear series.
- **Cons:** a literal rebase replays 63 commits over main's step 11/12 renderer and 7 conflicted
  paths, so conflicts can recur per commit; every story SHA is rewritten, breaking the committed
  evidence citations (`71d0d04`, `2ebeabd`, `8f0249e`, `63914f9`, …); it puts the unshipped companion
  on the release line before §5. A merge-based variant (**A′**: merge main into the branch once,
  resolve, gate, then fast-forward main) removes the SHA and replay costs but still merges the story
  before the §5 product gate.
- **What would change my mind:** the founder decides the companion ships in the beta; or §5 has
  already happened and the story is kept; or the branch-sync burden exceeds one session.

### B — Cherry-pick S1–S4 onto main as one story series (branch stays an archive)

- **Pros:** clean main history (4 curated commits); no rebase of 63 commits; the original branch
  remains an archive with its evidence SHAs.
- **Cons:** the per-probe commit chain and gate-to-commit mapping are lost from main (the evidence
  still points at the archived branch, not main); four conflict resolutions instead of one; it still
  merges an unproven, unshipped companion into the release line before §5; the story's tests/evidence
  must be re-mapped.
- **What would change my mind:** the founder wants a single curated story series in main for the beta
  and accepts that main's story history is four squashed slices, with the archive branch as the
  evidence source.

### C — Keep the branch separate until the §5 founder session (recommended)

- **Pros:** main stays the proven, gated v2 line and the beta publish gate stays unentangled; `ADR-0012`
  is honored (companion, no ship/graduation claim); the branch is unpushed, so a main→branch sync can
  happen without rewriting 63 SHAs; the packaging slice can proceed on current story code once
  synced; the §5 session (not a merge) decides whether the story is kept, cut, or graduated.
- **Cons:** the story is not in main until after §5; the branch remains a second line and needs a
  sync (7 conflicted paths, one visibility fix) before packaging; if §5 is delayed, drift returns.
- **Mitigation:** sync main into the branch **before** the packaging slice and after any main change;
  re-decide A/B at §5; archive the branch if §5 rejects the story.
- **What would change my mind:** the founder says the companion ships in the beta; §5 uses the story
  and asks for it in main; the sync cost exceeds two sessions or blocks packaging; or the story is
  cut (then no merge).

## Decision

1. **Recommendation: C — no story→main merge now.** Keep `feat/atlas-sim` separate from `main` until
   the §5 founder session. Reconcile the metadata now (IDs, plan status, state, `R-pf-109`) so no
   agent re-runs v2 work or trusts the stale snapshot.
2. **If C is approved, the next implementation is the story packaging slice, preceded by a
   main→branch sync:** merge `main` into `feat/atlas-sim` (one merge commit; resolve the seven
   paths; restore `strip_entry_line` to `pub(crate)`; keep `start_actions`/`build_atlas_view`), run
   the full story gate (fmt, clippy, tests, deny, `sim/gate.sh`, F5/F6), then build the
   self-contained `story.html` + `perfpal story <repo>` entry and take it to §5. No story→main
   merge, no push, no A2/ship claim.
3. **If the founder chooses A:** execute **A′** (merge main into the branch, resolve once, gate,
   then fast-forward main), not a literal rebase, unless a rewritten linear history is explicitly
   wanted. If the founder chooses B: cherry-pick S1–S4 as four curated commits, keep the branch as
   the archive, and re-map the evidence.
4. **The founder overrode direction C on 2026-10-27 — merge now.** See the addendum: the companion
   merges into `main` as `v0.4.0` without an A12 change and without a graduation claim. The rest of
   this record (facts, options, ID resolution) stays as the decision context.

## ID, plan and state resolution (applied with this record; final IDs at merge)

| Old (branch) | Meaning | Main collision | Final |
|---|---|---|---|
| `D-pf-251` | D2 founder verdict | main F-treemap decision (live) | `D-pf-268` |
| `D-pf-252` | S1 committed | main archived step-13 decision | `D-pf-269` |
| `D-pf-253` | S2 committed | main step-13 approval | `D-pf-270` |
| `D-pf-254` | S3 done | main archived step-13 decision | `D-pf-271` |
| `D-pf-255` | S4 done | main A2 PASS | `D-pf-272` |
| `D-pf-259` | `ADR-0012` decision | main `D-pf-259` (M1 merge) | `D-pf-267` |
| `D-pf-260` | D2 verdict (first remap) | main `D-pf-260` (M1 fix) | `D-pf-268` |
| `D-pf-261..264` | S1–S4 (first remap) | none | `D-pf-269..272` |
| `D-pf-265` | this reconciliation | none | `D-pf-273` |
| `D-pf-266` | archive pointer | none | `D-pf-274` |
| `A-pf-124` | S4 done | main commit/publish action | `A-pf-131` |
| `A-pf-125` | `ADR-0012` action | main post-beta follow-ups | `A-pf-132` |
| `A-pf-126/127` | S4 / ADR actions | main M2 / M3 actions | `A-pf-131/132` |
| `A-pf-128` | next action (sync + packaging) | none | `A-pf-133` |

- **Two hops, one final block.** The branch first remapped `D-pf-251..255` → `260..264` and
  `A-pf-124/125` → `126/127` against `main@c797004` (max 258). M1 then landed `D-pf-259/260` and
  `A-pf-126/127` on main, so the merge moved the whole story block to `D-pf-267..274` /
  `A-pf-131..133`. The branch's `superseded` tombstone wrappers are **not** carried into main's
  ledger (their old IDs collide with main's live/archived records); the live records use
  `D-pf-267..274` and `A-pf-131..133`, and this table plus `D-pf-273` preserve the mapping.
- **`R-pf-109`:** closed as a record, matching main's 2026-10-26 closure (v2 rework + A2); the branch
  snapshot does not contain steps 11–15 and must not claim them. `R-pf-112` closes at this merge:
  the companion is in `main` and the report gate is satisfied; packaging stays a recorded follow-up.
- **`atlas-build`:** **done** (matching main `D-pf-258`) with no snapshot banner on main after this
  merge; **do not execute steps 11–15 here**. The story track is `atlas-s4` (done) and the story
  companion merges as a track artifact, not a second plan.
- **Historical ledger:** the branch's `superseded` tombstone wrappers are **not** carried into main's
  ledger because their old IDs collide with main's live/archived records; `D-pf-273` records the
  remap and the final IDs. Live docs/state use the final IDs. This is a
  documented, bounded exception to `STATE-SCHEMA.md` §2 ("never reused or renumbered"): the branch
  and main allocated the same IDs independently while divergent; the remap is closed after this
  merge and no further renumbering is authorized. The hot file keeps only live/open/active entries;
  closed/done records are cold (`resolved`/`settled`/`archived`).

## Risks

- **Sync conflicts:** seven paths; `atlas_render.rs`/`mod.rs` are the real code conflicts. Mitigation:
  resolve once, restore `strip_entry_line` visibility, run the full story gate.
- **Story/report drift:** main's step 11/12 rail/charts change the report; the story only consumes
  `atlas-layout.json` and must stay visually independent. Re-run F5/F6 and the 3k gate after the
  sync; verify `report.html` story-off/on byte identity.
- **ID remap:** live docs must use the new IDs; the ledger keeps old IDs + the remap. A stray old
  reference is a documentation defect.
- **Beta entanglement (accepted at merge):** `--story` and `sim/` are now on the release line before
  packaging/§5; they are documented as an experimental companion (opt-in flag, no ship or graduation
  claim), not as a released feature.
- **Unchanged constraints:** no A2/ship claim from this branch; F8 deferred; A12 unchanged (one fixed
  `REPORT_SCRIPT`; zero scripts/handlers/network in canvas/SVG/animation); `layout_version` 1; no new
  dependencies; holdouts vaulted; static-inference honesty; visuals `@vision`-gated.

## Addendum (2026-10-27) — founder override: merge now

**The founder overrode direction C.** The companion merges into `main` as **`v0.4.0`** now; the
branch stops being a separate line.

1. **Direction:** C → **merge now** (`git merge --no-ff feat/atlas-sim` on `main`). A/B were not
   chosen; the merge is the merge-based integration, executed on main, not a rebase or cherry-pick.
2. **No A12 change.** `report.html` keeps exactly one fixed `REPORT_SCRIPT`; the Atlas
   canvas/SVG/animation artifacts keep zero `<script>`, zero handlers and zero network references.
   `--story` adds only the opt-in `atlas-layout.json`; the `sim/` player stays a separate companion
   and is not embedded in the report.
3. **No graduation claim.** The story does **not** graduate into the report. `ADR-0012` Decision 4
   still governs in-report graduation: the §5 founder session on a self-contained artifact, the
   report visual gate, a deterministic offline player and an explicit A12 amendment are all still
   required. F8/A2 remains deferred and unclaimed; no ship claim.
4. **Packaging remains a follow-up.** The companion is still fetch/HTTP-only and has no
   `perfpal story` entry point; self-containment (inline data, no server) and a CLI entry point are
   the precondition for the §5 session (`ADR-0012` Decision 5).
5. **ID resolution is final at this merge** (`D-pf-267..274`, `A-pf-131..133`; see the table above).
   `R-pf-112` closes: the branch is in `main` and the report gate is satisfied. The story gate is
   re-run on the merged tree; evidence lands under `docs/evidence/merge-m2/`.
6. **No push/tag.** The merge commit and the `v0.4.0` bump stay local; publishing is the
   founder-owned gate (`B-pf-001`/`B-pf-002`).

## Verification (record only)

| Claim | Command / artifact | Result |
|---|---|---|
| Divergence (snapshot) | `git rev-list --left-right --count main...988aea2` | `23 63` (main-only / branch-only) |
| Divergence (record HEAD) | `git rev-list --left-right --count main...5250451` | `23 64` (the record commit adds one) |
| Merge base | `git merge-base main 988aea2` | `821b984` |
| Conflict set | `git merge-tree --write-tree --name-only main 988aea2` | 7 paths (listed above) |
| Main v2 status | `git show main:docs/plans/atlas-build.md`; `git show main:state/session-state.json` | plan done; `D-pf-258`; `R-pf-109` closed |
| Branch gate | `docs/evidence/atlas-sim/GATE.txt`; `NOTES.md:184` | PASS; 897 tests (NOTES), 304/304, 3k 44/44 |
| State JSON + budget | `jq empty state/session-state.json`; `test "$(wc -c < state/session-state.json)" -le 8192` | exit 0; 6,337 B at the close |
| Record scope | `git diff --name-status 988aea2..5250451` | docs/state only; no `src/`, `sim/`, schema, Cargo |
| Worktree | `git status --porcelain` | clean at the M2 close commit |
| Merge gate | `docs/evidence/merge-m2/gate.txt` | story gate + full gate exit 0 on merged `main` |

## References

- `docs/adr/ADR-0012-atlas-story-companion.md` · `docs/specs/atlas-story.md` §9/§11 ·
  `docs/research/atlas-run.md` §5 · `docs/plans/atlas-build.md` (main + branch) ·
  `docs/plans/atlas-s4.md`
- `state/tracks/atlas-sim/handoff.md` · `state/handoff/latest.md` · `state/session-state.json`
- Main evidence: `docs/evidence/atlas-visual/v2/`; story evidence: `docs/evidence/atlas-sim/`
