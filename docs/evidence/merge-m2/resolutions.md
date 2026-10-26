# Merge Train M2 — conflict resolution log

**Merge:** `646597c` (`git merge --no-ff feat/atlas-sim` on `main`)
**Base:** `158c7b0` (M1 close, v0.3.0) · **Feature:** `e3bd45e` (`feat/atlas-sim`)
**Result:** v0.4.0, metrics schema 1.8, layout/flow 1.

Every conflict was resolved by reading both sides and combining behavior — no `--force`,
`-X ours`, `-X theirs`, `--no-verify`, and no side deleted.

| # | Path | Main side | Feature side | Resolution |
|---|---|---|---|---|
| 1 | `src/reporter/atlas_render.rs` (hunk 1) | `SVG_W` private; `PAGE_GRAPH_W = 868` for the embedded page | `pub(crate) SVG_W = 1400` for the JSON projection | one `pub(crate) SVG_W = 1400` + main's `PAGE_GRAPH_W = 868` |
| 2 | `src/reporter/atlas_render.rs` (hunk 2) | `Layout { width, ... }` private; `compute_layout(view, width)` | `pub(crate) Layout`; `compute_layout(view)` | `pub(crate) Layout` with `width`; `compute_layout(view, width)`; `atlas_layout` passes `SVG_W` |
| 3 | `src/reporter/atlas_render.rs` (hunk 3) | collision-aware `node_labels` | `aggregate_rect` + `cap_chip_rect` helpers | kept all three (no overlap) |
| 4 | `src/reporter/atlas_render.rs` (hunk 4) | footer built from 7 escaped spans | `footer_text(view)` helper | one `footer_parts(view)` source; HTML spans + JSON `footer_text` both derive from it |
| 5 | `src/reporter/atlas_render.rs` (visibility) | `fn strip_entry_line` private | `pub(crate) fn strip_entry_line` | `pub(crate)` (story projection calls it) |
| 6 | `src/reporter/mod.rs` | `atlas_charts` module | `atlas_layout` module + `atlas_layout_json` | both modules; `atlas_layout_json` retained |
| 7 | `README.md` | schema 1.8 marker | optional-artifact marker + story text | main's 1.8 sentence + the `layout_version: 1` marker; story section/flag added |
| 8 | `docs/report/output-contract.md` | Atlas v2 page + 1.8 | story artifact + flow/io/purpose/profile/scenes | automatic union; wording corrected so the story canvas (1400) is not confused with the embedded report canvas (868) |
| 9 | `CHANGELOG.md` | Unreleased pending list + `[0.3.0]` | story entries under Unreleased | `[0.3.0]` kept; story entries under `[0.4.0]`; ADR/override record |
| 10 | `.gitignore` | `/.playwright-mcp/` | `/sim/source/` | both |
| 11 | `docs/plans/{atlas-build,merge-train}.md`, `docs/atlas/AGENT-BRIEF.md` | v2 done / M1 result | branch snapshot status | main's canonical status; M2 result + final ADR names |
| 12 | `state/*` (`session-state.json`, `handoff/latest.md`, `activeContext.md`, `ledger.jsonl`) | M1 canonical state | story state + rebuilt ledger | main's canonical state kept and updated; ledger append-only union with story IDs renumbered (below) |

## State ID resolution (two-hop remap)

The branch first remapped story IDs against `main@c797004` (max 258). M1 then allocated `D-pf-259/260`
and `A-pf-126/127` on main, so the merge moved the story block again:

| Story record | Branch ID | First remap | Final |
|---|---|---|---|
| ADR-0012 companion decision | `D-pf-259` | unchanged | `D-pf-267` |
| D2 founder verdict | `D-pf-251` | `D-pf-260` | `D-pf-268` |
| S1 committed | `D-pf-252` | `D-pf-261` | `D-pf-269` |
| S2 committed | `D-pf-253` | `D-pf-262` | `D-pf-270` |
| S3 done | `D-pf-254` | `D-pf-263` | `D-pf-271` |
| S4 done | `D-pf-255` | `D-pf-264` | `D-pf-272` |
| R-pf-112 reconciliation | `D-pf-265` | unchanged | `D-pf-273` |
| archive pointer | `D-pf-266` | unchanged | `D-pf-274` |
| S4 action | `A-pf-124` | `A-pf-126` | `A-pf-131` |
| ADR action | `A-pf-125` | `A-pf-127` | `A-pf-132` |
| packaging follow-up | — | `A-pf-128` | `A-pf-133` |

The branch's `superseded` tombstones are not carried into main's ledger (their old IDs collide with
main's live/archived records); `D-pf-273` and `ADR-0013` preserve the mapping. No ID was reused.

## Panel record (M2)

- **Round 1** (R1/R2/Q1/Q2): all four lanes **CHANGES_REQUESTED** —
  - R1: `atlas-layout.json` canvas 1400 vs report canvas 868; the output contract claimed they were
    the same canvas. Fixed: contract + module docs now name the standalone story canvas and forbid
    overlaying it on the report page; a test locks `canvas.w == SVG_W`.
  - R2/Q1: README/roadmap still said 0.3.0; stale Atlas ADR references in the track handoff and
    ledger records; `D-pf-273` contradicted the merge and had a garbled remap; cold records marked
    `live`. Fixed: version propagated; ADR sweep completed; the reconciliation record rewritten and
    archived (`ref D-pf-275`); story records archived.
  - Q2: test counts paraphrased (1 ignored), CHANGELOG pointed at the wrong transcript, clean-status
    evidence missing. Fixed: exact counts/paths; this log; the handoff records the observed
    `git status --porcelain` result.
- **Round 2:** see `state/handoff/latest.md` verdicts.

## Evidence

- `docs/evidence/atlas-sim/GATE.txt` — story gate on merged main (`ff276fd`): PASS, dirty 0,
  304/304, 3k 44/44, F5/F6, 1,089 tests (1,088 passed, 1 ignored), page 463,490 B, first frame max
  1,616 ms.
- `docs/evidence/merge-m2/gate.txt` — full gate: fmt/clippy/test/deny/semantic all exit 0;
  `perfpal 0.4.0 (metrics schema 1.8)`; schema copy identical.
- `docs/evidence/merge-m1/resolutions.md` — M1 precedent.
