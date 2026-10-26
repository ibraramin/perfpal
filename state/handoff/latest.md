# Handoff — Merge Train M2 (`feat/atlas-sim` → `main` v0.4.0; 2026-10-27)

**Updated:** 2026-10-27
**Repo:** `/home/shafin/Opencode/Perfpal`
**Session:** merge-train M2 — merge the Atlas Story companion into `main` as v0.4.0 (plan `merge-train` / M2)
**Verdicts:** `@reviewer` APPROVED (R1; R2 APPROVE with non-blocking notes) · `@requirements-reviewer` Q1 APPROVED; Q2 CHANGES_REQUESTED pending the clean-status close (round 3)

**North-star:** `docs/NORTH-STAR.md` — the birdseye map is the core feature. M2 integrates the Atlas
Story **companion** (the temporal/data-flow showcase) into `main` as **v0.4.0** without changing the
report, the map, the schema or A12. **No drift:** the story stays a companion, no in-report
graduation, no A2/ship claim, no push/tag. **M3 (finalization) is next.**

## Objective

Merge `feat/atlas-sim` into `main` with semantic conflict resolution; renumber the Atlas ADRs
(0011→0012, 0012→0013) with the founder's direction-C override addendum; bump **0.4.0**; run the
story gate + full gate; mark the story track done and reconcile state/handoff; no push/tag.

## Phase / plan
- Plan: `merge-train` · M2 · status: active (M2 done; M3 remains)
- Previous milestone: M1 close (`158c7b0`, v0.3.0)
- Task spec: `docs/specs/merge-train.md` (R1–R10)

## Verification (claim → evidence)

| Claim | Command | Observed result | Exit |
|---|---|---|---|
| `feat/atlas-sim` is merged | `git merge-base --is-ancestor feat/atlas-sim main` | 0 (no output) | `0` |
| All four lines are in `main` | `git merge-base --is-ancestor <branch> main` for `feat/agent-report`, `fix/agent-report-regen`, `feat/js-ts`, `feat/atlas-sim` | all 0 | `0` |
| Version / schema | `./target/debug/perfpal --version` | `perfpal 0.4.0 (metrics schema 1.8)` | `0` |
| Layout/flow versions | `--story` scan → `atlas-layout.json` | `layout 1 flow 1 purpose True profile True scenes True` | `0` |
| Format | `cargo fmt --all -- --check` | clean | `0` |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` | clean | `0` |
| Tests | `cargo test --locked` | 1,089 total (1,088 passed, 1 ignored) / 0 failed | `0` |
| Licenses | `cargo deny check licenses` | clean | `0` |
| Semantic lint | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | clean | `0` |
| Semantic tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | 1,091 total (1,090 passed, 1 ignored) / 0 failed | `0` |
| Schema copy | `cmp src/schema/metrics.schema.v1.8.json docs/schema/metrics.schema.v1.8.json` | identical | `0` |
| Story gate | `bash sim/gate.sh` (`docs/evidence/atlas-sim/GATE.txt`) | `verdict=PASS`; 1,089 tests (1,088 passed, 1 ignored); 304/304 checks; 3k 44/44; F5/F6 PASS; page 463,490 B; first frame max 1,616 ms; dirty 0 | `0` |
| Full gate | `docs/evidence/merge-m2/gate.txt` | every command exit 0; story gate PASS | `0` |
| Hot state | `wc -c state/session-state.json` | 5,094 B (≤ 8,192) | `0` |
| State JSON + ledger | `jq`/`json.loads` on hot state and all ledger lines | all valid; 750 ledger lines | `0` |
| ADR numbers unique | `ls docs/adr/ADR-001*` | one `0011` (js-ts), one `0012` (Atlas Story), one `0013` (reconciliation) | `0` |
| No tag | `git tag --points-at HEAD` | empty | `0` |
| Clean status | `git status --porcelain` | empty after the close commit | `0` |

Non-runnable evidence:
- `docs/evidence/atlas-sim/GATE.txt` — story gate transcript at `ff276fd` (PASS, dirty 0).
- `docs/evidence/merge-m2/gate.txt` — full gate on merged `main`.
- `docs/evidence/merge-m2/resolutions.md` — 12 semantic resolutions + two-hop state-ID remap.
- `docs/evidence/atlas-sim/checks.json` — 304/304 DOM/story checks.
- `docs/adr/ADR-0013-atlas-branch-reconciliation.md` — founder override addendum + final ID table.

## Deliverables (changed / added)

- **Merge commit `646597c`** — `feat/atlas-sim` into `main` (12 conflicts resolved semantically).
- `src/reporter/atlas_render.rs` — kept main's `PAGE_GRAPH_W` embedded canvas and
  `compute_layout(view, width)`; made `Layout`/fields crate-visible for the JSON projection; kept
  main's collision-aware `node_labels`; kept the story's `aggregate_rect`/`cap_chip_rect`;
  `strip_entry_line` `pub(crate)`; one `footer_parts()` source feeds the HTML spans and the JSON
  `footer_text`.
- `src/reporter/mod.rs` — both `atlas_charts` and `atlas_layout` modules; `atlas_layout_json` entry.
- `src/analysis/flow.rs`, `src/reporter/atlas_{layout,purpose,queue,scenes}.rs`, `src/cli/mod.rs`
  (`--story`), `sim/` companion, story tests/goldens — preserved from the branch.
- `docs/adr/ADR-0012-atlas-story-companion.md`, `docs/adr/ADR-0013-atlas-branch-reconciliation.md`
  — renumbered; references swept (specs, README, CHANGELOG, state, track handoff).
- `Cargo.toml`/`Cargo.lock` — **0.4.0**; `CHANGELOG [0.4.0]`; README story section and `--story`
  flag.
- `tests/golden/**` — 24 files regenerated at 0.4.0 (tool.version only; counts 15/9/32/29/11/11);
  `docs/evidence/agent-report/sample/*` regenerated (78 hints, 8,882 B).
- `state/session-state.json` (5,094 B), `state/archive/ledger.jsonl` (append-only union, 750 lines),
  `state/handoff/latest.md`, `docs/activeContext.md`, `docs/plans/merge-train.md` (M2 result),
  `docs/tracks/{README.md,atlas-run.md}`, `state/tracks/atlas-sim/handoff.md`.
- `sim/data-{hyperfine,cjson}.json` — regenerated (only `source.metricsContentSha256`, version
  bump); `docs/evidence/atlas-sim/{GATE.txt,checks.json,*.png}` — 0.4.0 re-run evidence.

## Semantic resolutions (12 conflicts)

| Path | Resolution |
|---|---|
| `src/reporter/atlas_render.rs` | main's `PAGE_GRAPH_W` + width-parameterised `compute_layout`; branch's `pub(crate)` layout/geometry helpers; main's `node_labels` + branch's `aggregate_rect`/`cap_chip_rect`; `strip_entry_line` `pub(crate)`; `footer_parts` single source (4 hunks) |
| `src/reporter/mod.rs` | both `atlas_charts` and `atlas_layout`; no duplicate module |
| `README.md` | main's 1.8 marker + optional-artifact sentence; new story section/flag |
| `CHANGELOG.md` | `[0.3.0]` retained; story entries under `[0.4.0]`; ADR/override record |
| `.gitignore` | union (`/.playwright-mcp/`, `/sim/source/`) |
| `docs/plans/{atlas-build,merge-train}.md`, `docs/atlas/AGENT-BRIEF.md` | main's canonical status; merge-train M2 result added |
| `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md` | main's canonical state kept and updated |
| `state/archive/ledger.jsonl` | append-only union; story records final at `D-pf-267..274` / `A-pf-131..132`; no ID reuse introduced |

## Decisions (live)
- `D-pf-275` — MERGE-TRAIN M2 DONE: merge `646597c`, v0.4.0 / schema 1.8 / layout+flow 1; ADR-0012/0013
  with the founder override; story gate + full gate green; no push/tag.
- `D-pf-251` — F treemap remains undecided; no F code ships.
- Story-line records are cold: `D-pf-267` (ADR-0012), `D-pf-268..272` (D2/S1–S4), `D-pf-273`
  (reconciliation), `D-pf-274` (archive pointer), `A-pf-131/132`.

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
1. `A-pf-127` — **M3 finalization:** verify the four branches are ancestors; remove the
   agent-report worktree; prune merged branches; full CI gate; verify 0.4.0/schema 1.8,
   CHANGELOG, README, hot state ≤8 KB, clean status; EOL; no push/tag.
2. `A-pf-133` — story packaging: self-contained companion (inline data, no `fetch`/server) +
   `perfpal story <repo>` before the §5 founder session; F8/A2 story read deferred; no in-report
   graduation (A12 unchanged).
3. `A-pf-108`/`A-pf-125` — V7 re-triage / post-beta map work; F decision (`D-pf-251`).

## Uncommitted files

```
<filled at the close commit — expected empty>
```

## Commits (this session)
- `646597c` — merge: feat/atlas-sim into main (Atlas Story companion)
- `e1b4f1b` — release: v0.4.0 — Atlas Story companion, ADR renumber, goldens/state
- `ff276fd` — fix(reporter): clippy let_and_return in footer_parts; regen story data at 0.4.0
- `3829e33` — fix(merge-train): M2 panel round 1 — contract truth, version/ADR sweep, ledger records
- final evidence/state commit — docs(merge-train): M2 evidence + panel verdicts

**Resume in a fresh session:** plan `merge-train`, next task **M3 — main finalization**, commit
`ff276fd` (gate-green tree); the evidence/state close commit is the parent of the next session.
