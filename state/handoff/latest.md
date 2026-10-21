# Handoff — Latest

## Objective
I5 (map-build) proof + demo **executed and reviewed round 1**: full gate green (50 suites /
688 tests; feature 691), committed-golden double regeneration byte-identical, 9-repo
double-scan byte-identical modulo provenance, semantic tier verified on real libclang,
A-pf-095 true GENUINE measured (9/172 = 5.2%) plus a supplementary 27-node map calibration,
and the 9-repo demo artifact audit. Evidence: `docs/evidence/i5-map-proof.md`,
`i5-determinism.txt`, `i5-map-triage.json`, `i5-blind-a5.json`.
**Closeout pending: founder A5 quote + round-2 dual review.** Round 1: code APPROVED;
requirements CHANGES_REQUESTED → G4 tier-aware report boilerplate (+ locked test),
G5/G6/G7 doc fixes, G8/G9 persisted evidence + parse-partial disclosure, G2 timing disclosed
unmeasured. The proof surfaced three map-quality defects (entry strata, cost ranking, density
ranking) recorded honestly and queued, not silently fixed.

## Phase
POST-MVP map-first. `map-research` R1–R4 ✅; `map-build` I1 ✅ I2 ✅ I3 ✅ I4 ✅ (`f56d072`);
I5 proof/demo executed (this burst) — closeout pending founder input. Paper + JS/TS parked
behind the map; M5 parked behind B-pf-001/002.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) emitting heuristics + the maintainer birdseye
  map (core feature) + LLM-consumable metrics, with findings as map annotations
  (`docs/NORTH-STAR.md`).
- **Phase restated:** dev; I5 is the proof gate for the map — determinism, semantic tier,
  credibility, and the A5 three-action test.
- **How this session serves the north-star:** the map is now proven deterministic and
  demonstrably usable by blind maintainers, and the proof exposes where the ranking is
  *not* yet decision-grade (entry detection, tests/literal-bound cost, tiny-scope density) —
  exactly the evidence needed before claiming "where do I look first" as solved.
- **Drift flag: NONE.** The negative results are inside the map idiom's A5 reversal trigger,
  not a new direction.

## This session — I5 result
- **Gate:** fmt/clippy/test/deny all exit 0; feature (`semantic-clang`) clippy + tests green.
- **Determinism:** `PERFPAL_REGEN_GOLDEN=1 … regenerate_goldens` twice → identical SHA-256
  set; second pass leaves the tree clean. 9 demo repos scanned twice → 0 diffs (charts
  exact; `metrics.json`/`report.md` masked provenance only). `metrics.json`, `graph.dot`,
  `findings-detail.*` goldens untouched (A2).
- **Semantic tier:** libclang 23.1 runtime dlopen works; `resolver_health` 10/10 including
  real parses. cJSON 837→1,162 findings (778 semantic-tier; ARCH-003 MEDIUM 6→784, LOW
  581→128; 115 resolved edges). curl 10,985→11,807 (5,064 semantic; ARCH-003 MEDIUM
  518→5,579, LOW 6,432→2,190; 1,548 resolved edges).
- **A-pf-095:** current prod-`hot` = 172 (chunkhound 26, ripgrep 29, curl 117, aider 0);
  all 172 ids already hand-triaged; 9 GENUINE → **5.2%** (9 retired ids: ripgrep 5, curl 4).
  New calibration: top-3 files by cost per demo repo (27) = **1 GENUINE / 11
  REAL_BUT_MINOR / 11 TEST_NOISE / 4 FP**.
- **Demo:** 9/9 repos emit all 9 expected stems + 4–50 component pages; `report.html`
  41–58 KB with 153–211 drill anchors. Two blind map-only maintainers named 3 concrete
  actions each (tmux: format.c / tmux.h / tty.c; hyperfine: benchmark/executor.rs /
  command.rs / output/report.rs) → **A5 PASS with caveats**.
- **Negative results (queued, not hidden):**
  - **R-pf-098 (high):** `layer-entry` collapses to one `entry` column in all 9 repos, even
    semantic; `detect_c_entries` marks every exported function in main-less files as
    `lib-root` (curl 612/923 files, postgres 1,592/2,566, django 1,637/2,932).
  - **R-pf-099 (medium):** file-cost rank dominated by tests/examples/generated files and
    literal-loop trip inflation (cJSON 1M-iteration vendored test; django 10,000× retry
    guard; alacritty `#[cfg(test)]` loop).
  - **R-pf-100 (medium):** finding density explodes on tiny scopes (postgres
    `src/include/pch` 1000.0 from 3 findings; rich `.` 333.33 from 1).
- **Fixes landed:** A4 captions (`candidate … — profile to confirm`) on 5 heat captions +
  hot-components + 3 report link captions; README/website chart-set drift; feature-build
  `report_matches_committed_goldens`/`regenerate_goldens` gated to default builds (the
  semantic tier legitimately changes chart topology; this was a pre-existing red CI job).
  33 goldens regenerated (caption-only).

## Verification
| Claim | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| tests | `cargo test --locked` | **50 suites / 688 passed / 0 failed** |
| licenses | `cargo deny check licenses` | `licenses ok` |
| feature clippy | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| feature tests | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | **50 suites / 691 passed / 0 failed**; resolver_health 10/10 |
| golden double regen | `PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens` ×2 | tree SHA-256 `749162…` identical; `docs/evidence/i5-determinism.txt` |
| 9-repo double scan | charts exact + masked `metrics.json`/`report.md` | 0 diffs in 9/9; per-repo set hashes in `i5-determinism.txt` |
| A2 untouched | `git status --porcelain -- 'tests/golden/*/metrics.json' 'tests/golden/*/charts/graph.dot' 'tests/golden/*/findings-detail.*'` | empty |
| semantic tier | cJSON/curl semantic scans | 115 / 1,548 resolved edges; ARCH-003 upgrades; tier-aware report locked |
| A-pf-095 | hand verdicts (+ supplementary top-node calibration) | 9/172 = 5.2%; calibration 1/27 GENUINE |
| A5 blind tests | independent map-only agents (tmux, hyperfine) | 3 actions each, `ambiguous=false`; timing not measured |
| founder A5 | tmux map `../perfpal-test/out-i5-tmux/report.html#map` | **PENDING — founder quote** |
| review (code) | `@reviewer` round 1 | **APPROVED** (non-blocking notes; caption overstatement + evidence retention addressed) |
| review (requirements) | `@requirements-reviewer` round 1 | **CHANGES_REQUESTED** → G4/G5/G6/G7/G8/G9 fixed, G2 disclosed; round 2 pending |

## Risks / Open
- **R-pf-098 (high):** entry-detection over-marking → degenerate strata; needs an ADR/spec
  decision (true graph roots vs cap reserve vs honest label) — A-pf-098.
- **R-pf-099 (medium):** production-path + known-trip ranking policy — A-pf-099.
- **R-pf-100 (medium):** density denominator guard — A-pf-100.
- **R-pf-103 (low):** semantic build has no committed-golden lock after the feature gate
  (covered by resolver_health + double-run determinism) — A-pf-103.
- **R-pf-101 (low):** ADR-0002 stays historical; README/website updated; workspace figure
  duplicated in `report.html` (`#map` + `#architecture`) — deferred.
- **R-pf-097:** partially closed (README/website/captions); django "humans bounded" remains
  structural only (no in-repo django corpus); workspace duplication moved to R-pf-101.
- **Blockers (unchanged):** **B-pf-001** (contact); **B-pf-002** (secrets).
- **Follow-ups:** the three map-quality fixes are the next burst; ADR-0008 reversal trigger
  was not tripped (A5 passed) but the ranking caveats are recorded in the dossier.

## Next Actions
1. **Close I5:** founder names 3 actions from the tmux map (timed) → append §7.3; round-2
   dual review on the final tree; `markPlanDone map-build`; commit.
2. **New session — map-quality burst:** A-pf-098/099/100 (entry semantics, production
   ranking, density guard) + A-pf-103 (semantic goldens); each needs its own spec/ADR
   decision and golden regen.
3. M5 publish needs B-pf-001 + B-pf-002 (founder-owned, A-pf-080).

## Change set (this session)
- **Code:** `src/charts/{layers,component,mod}.rs`, `src/reporter/mod.rs` (caption wording +
  tier-aware report boilerplate), `tests/p6_hardening.rs` (default-build-only golden flow +
  cfg'd helpers/const), `tests/charts_render.rs`, `tests/reporter_render.rs` (pinned wording
  + G4 semantic-boilerplate lock).
- **Goldens:** 33 files (layer-{workspace,file,entry,bands}, hot-components, report.md/html,
  component pages with the ramp) — caption-only.
- **Docs:** new `docs/evidence/i5-map-proof.md`, `i5-determinism.txt`, `i5-map-triage.json`,
  `i5-blind-a5.json`; `docs/report/output-contract.md`, `README.md`,
  `docs/website/scope-and-content.md`, `CHANGELOG.md`.
- **State:** `state/session-state.json` (D-pf-223/224; R-pf-098/099/100/101/103;
  A-pf-098/099/100/103; older decisions/risks/actions demoted to ledger),
  `docs/activeContext.md`, this handoff, ledger append.
- **Untracked build dir:** `/target-semantic` ignored (`.gitignore`).

## L2 state split / L6 north-star (history)
Hot/cold split 2026-09-29; north-star pinned 2026-10-01, realigned to
divergence-instrument then map-first core (both founder-permitted); first periodic
audit 2026-10-16 (counter 0).
