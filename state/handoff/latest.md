# Handoff — Latest

## Objective
I3 (map-build) DONE: the entry-point-stratified cost map as the third layer stem
(`layer-entry.svg`) plus the deferred middle-bands layout (`layer-bands.svg`,
A-pf-077). Both are deterministic, capped, stubbed, and honest about LOW-confidence
call edges and default-scan reachability limits.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1 ✅ (entries, 3 langs),
I2 ✅ (cost-v2, 3 langs), I3 ✅ (map layer + bands); I4 annotations next.
Paper + JS/TS parked behind the map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer
  birdseye map (core feature) + LLM-consumable metrics, findings as map annotations
  (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; map-build I3 shipped the third layer and the middle bands.
- **How this session serves the north-star:** I2's cost-v2 + reachability is now
  rendered as an entry-stratified map, and the bands layer makes cross-component
  file edges visible at file resolution (the A-pf-077 honesty gap).
- **Drift flag: NONE.** Build sequence per ADR-0008.

## This session — I3 burst result
- **`layer-entry.svg`** (`src/charts/layers.rs::render_layer_entry`): files capped
  entry-first (REAL entry files only — `unknown-entry` stubs are excluded, captioned,
  and cannot seed BFS or earn the reserve), laid out in BFS-depth columns from
  non-LOW resolved call pairs (`entry`, `depth 1`…), depth capped at 8 with deeper
  files folded into `depth ≥ 9` + caption; unreached files sub-layered by
  `topo_layers`; no-real-entry scans fall back to a plain topo call DAG; heat =
  per-file function cost (v2 where available, v1 fallback, normalized over shown
  files); single hottest shown entry-reachable file outlined; LOW edges excluded
  and counted; stub on empty files.
- **`layer-bands.svg`** (`render_layer_bands`, A-pf-077): capped components
  (`rank ≤ 50`, `(rank, prefix)` order) as bands; per-band file rows cost-ordered
  cap 10 + `…+N more`; intra-band include/non-LOW-call edges deduped + cycle-broken;
  cross-band file edges aggregated into one counted ribbon per ordered band pair;
  outside-scope + LOW counts captioned; file-less bands stubbed.
- **Carried nits fixed:** stale “top files by degree” caption + false “(v2)” heat
  claim (reporter, SVG subtitle, output-contract, tests).
- **Review cycle:** rev-1 blocked on B1 (false “entries only” caption when a path
  crosses a capped-out intermediate), B2 (`unknown-entry` stubs treated as entries),
  B3 (unbounded depth axis) and F1 (bands subtitle claimed a hot-path that is not
  drawn); all fixed and re-reviewed. req-1 APPROVED; rev-1 final APPROVED.
- **Data reality (R-pf-096):** default scans have zero non-LOW call edges, so the
  entry layer honestly degrades to entry/not-reached bands with
  `0 resolved call edges; reachability = entries only (semantic tier off)`. The
  bands layer carries today's structural value; true depth strata need the semantic
  tier at I5.

## Verification
| Claim | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| tests | `cargo test --locked` | 679 passed / 0 failed |
| licenses | `cargo deny check licenses` | ok (exit 0) |
| ratchets | `cargo test --locked --test benchmark --test precision_kpi` | pass; baseline 1.0/1.0 untouched; only `tests/benchmark/mod.rs` chart-count (6→8) + comment changed |
| determinism A3 | `cargo test --locked --test p4_determinism --test p6_hardening --test v14_step2` + `tests/i3_map_layer.rs` | all pass; graph.dot + all SVG stems byte-equal UNMASKED |
| goldens | `PERFPAL_REGEN_GOLDEN=1 cargo test --locked --test p6_hardening regenerate_goldens` + comparator | 2 stems added per tree; `layer-file.svg` subtitle + report links changed only; `metrics.json`/`graph.dot` untouched |
| review | `@reviewer` + `@requirements-reviewer` | both APPROVED (B1–B3/F1 fixed) |
| commit | `git log --oneline -1` | `d78bb85` (independent I3 commit) |

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`**
  (high; EULA/PRIVACY contact), **`R-pf-082`** (medium; synthetic corpora),
  **`R-pf-007`** (medium; differentiation), **`R-pf-094`** (low), **`R-pf-095`**
  (low; Windows residuals), **`R-pf-096`** (low; default-scan zero non-LOW call
  edges → entry layer degrades honestly; semantic tier needed).
- **Blockers (unchanged):** **`B-pf-001`** (contact); **`B-pf-002`** (secrets).
- **Follow-ups:** C octal bound `i < 010` still parses decimal (reject or radix-8 +
  test); IR/schema prose still stops at v1.3 (`docs/architecture/ir-v1.md`,
  `docs/schema/metrics-json-v1.md`); old `layer-file` reserve cost-desc branch is
  partly untested (the new entry-layer reserve is covered); static-SVG
  label/uncertainty thresholds still need the I5 eyeball; pixel glance re-owed.

## Next Actions
1. **NEXT: I4 annotations** — finding dots on nodes (severity+confidence glued),
   `report.html` drill path header → workspace/file/component → `findings-detail`,
   output-contract registry +1 stem; django humans bounded, detail byte-identical.
2. Then I5 proof + demo (incl. A-pf-095 true GENUINE manual triage) and the
   semantic-tier check that makes `layer-entry` depth strata non-trivial.
3. M5 publish needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Change set (this session)
- **Code (`d78bb85`):** two new chart renderers + wiring, reporter stems/captions,
  cli/modules/output-contract docs, 11 `tests/i3_map_layer.rs` locks, frozen
  chart-set tests, 8 golden stems added, `layer-file.svg` subtitle truth fix.
- **State (this commit):** `state/session-state.json` (D-pf-221, R-pf-096,
  A-pf-096; I3/A-pf-077/A-pf-084/089/093/094 closed to ledger), this handoff,
  `docs/activeContext.md`.

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted); first periodic
audit 2026-10-16 (counter 0).
