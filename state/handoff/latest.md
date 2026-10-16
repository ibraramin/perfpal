# Handoff — Latest

## Objective
Sessions 47+ mega-burst — **`divergence-revamp` instrument + usefulness track**: IR v1.4
(schema → producers → precision), corpus pinning, bird's-eye visualization (static + interactive),
ablation apparatus, and a 9-repo demo sweep. Paper track **PARKED** per founder; M5 still gated.

## Phase
POST-MVP. `divergence-revamp` instrument bullets 1–4 ✅ (additive fields, distributions/DOT/cost,
precision fixes, corpus pins) + usefulness (viz A+B ✅, ablation apparatus ✅, campaign ⏳) +
demo sweep ✅. Paper/stats/Zenodo/draft PARKED. `evidence-beta` M1–M4 ✅ + deep dive ✅; M5 parked.

## Alignment (L6)
- **Goal restated:** Perfpal is a CLI (Rust) that scans a codebase and emits (a) language performance
  heuristics, (b) architecture-level bottleneck visualization, and (c) LLM-consumable quantified
  metrics, so humans see bottlenecks and LLMs optimize with explicit signals (`docs/NORTH-STAR.md`).
- **Phase restated:** POST-MVP, `stage: dev`; instrument + bird's-eye (pillars 1b/2) landed behind
  high-assurance gates; usefulness measured via ablation apparatus, not claimed.
- **How this session serves the north-star:** shipped the deterministic instrument (IR v1.4),
  the human-visible bottleneck view (component pages + layers + interactive HTML), and the tool that
  will *prove* usefulness (ablation) — pillars 1b + 2 made concrete.
- **Drift flag: NONE.** Paper deferral + usefulness focus are founder-directed. Audit counter 7 → 8
  (threshold 10; not due).

## This session — commit chain
1. **`e05c2bc`** (prior): plan `divergence-revamp` + specs `divergence-study`/`divergence-paper` +
   north-star realignment.
2. **`f899c11`** — IR v1.4 skeleton: schema 1.3→1.4, 7 optional per-function fields (serde-defaulted,
   old JSON parses), populated `distributions[]`, deterministic DOT stub. 536 tests.
3. **`37d0981`** — step-2 producers: whole-program call-shape pass, `alloc_family`/`error_path`
   post-pass (`src/scanner/instrument.rs`), FNV-1a `idiom_hash`, `charts/graph.dot` wired,
   cost-weighted coupling overlay. 556 tests. Zero finding changes.
4. **`fb8fe8d`** — 5 precision FP fixes (do-while0 gate, `#if 0` suppression, empty-ctor exclusion,
   realloc guard, LOOP-002 streaming split), silent fixtures + labels, TP/recall pinned. 566 tests.
   Golden impact: exactly ONE removed finding (intended FP).
5. **`24021cc`** — corpus packet (docs-only): 10 pinned + 4 proposed cells (M4 anchors reused,
   every SHA git-verified), `corpus.json`, `scripts/pin-corpus.sh`. No scans run (slots PENDING).
6. **`96c72ba`** — Option A slice 1: per-component SVG pages + §5 index hook.
7. **`5f7f524`** — Option A slice 2: workspace/file layer SVGs; remediated 6 adversarial BLOCKs
   (intra-degree cap, single-winner hot-path, honest coverage, exact page registry, symmetric
   stubs, pinned page sets). 592 tests.
8. **`f1c43f4`** — ablation apparatus (docs+script only): N=26 versioned tasks,paired WITH/WITHOUT,
   non-vacuity scorer guard, append-merge results, N=2 pilot shakedown (not quotable).
9. **`9581575`** — Option B slice 1: static zero-JS `report.html` (collision-free anchors, stem
   allowlist, §4 parity). 602 tests.
10. **`7ac3e22`** — Option B slice 2: pan/zoom + click-filter (fixed 33-line script), anchor-pairing
    fix, semantic masker parity. 606 default / 611 semantic tests.
11. **Demo sweep (uncommitted):** 9 repos (C/Rust/Python × small/med/large) cloned to
    `../perfpal-test`, scanned clean; founder reviewing raw `out-*/report.html` directly.

## Process notes (high-assurance mode, founder-ordered)
- Adopted mid-session: adversarial review + independent finding-diff audit + real-repo run with
  eyeball after EVERY major change. Caught pre-commit: 6 BLOCKs (slice 2), 3 (report.html),
  1 mislink (slice-2 JS), 2 (ablation scorer clobber/vacuity).
- Vision backend down all session (`deepseek-v4-flash-vision-exp` missing) — eyeballs were
  structural (XML geometry/labels), never pixels. **Owed: one human pixel glance** (A-pf-076).
- Semantic-only golden failure chased to root cause (missing resolver masker on new comparator),
  fixed with parity masker — not filed away.

## Verification (final gates observed by orchestrator)
| Claim | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 (every burst) |
| Lint | `cargo clippy --locked --all-targets -- -D warnings` (both feature sets) | exit 0, 0 warnings |
| Tests default | `cargo test --locked` | **606 passed; 0 failed** (526 → 606 across session) |
| Tests semantic | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | **611 passed; 0 failed** |
| Licenses | `cargo deny check licenses` | ok |
| Byte-determinism | `! grep -q preserve_order Cargo.lock` + double-scan proofs | exit 0 / identical |
| Findings stability | finding_id diff HEAD vs worktree, all 4 goldens, every code burst | IDENTICAL except 1 intended FP removal (fb8fe8d) |
| 67-rule catalog | `active.len()==67` lock | green throughout; `baseline.json` untouched at git level |

**Dual/adversarial review:** every code burst got review + requirements/adversarial passes; all
BLOCKs remediated and re-verified (ALL-HOLD) before commit. No unresolved disagreement.

## Risks / Open
- **Open risks (hot, key):** **`R-pf-063`** (high; MINISIGN secrets), **`R-pf-067`** (high; EULA/PRIVACY
  contact placeholder), **`R-pf-082`** (medium; synthetic-corpus credibility — complemented by M4),
  **`R-pf-007`** (medium; differentiation claims rest on one survey pass), **`R-pf-094`** (low),
  **`R-pf-095`** (low; Windows residuals).
- **New/changed this session:** vision-backend gap (A-pf-076); bands third layer deferred (A-pf-077);
  ablation campaign unrun (A-pf-078); demo feedback pending (A-pf-079).
- **Blockers (unchanged):** **`B-pf-001`** (contact address); **`B-pf-002`** (repo secrets).
- **Hot-state hygiene:** `state/session-state.json` = **6206 B / 8192 B**; demoted D-pf-197/198/199 +
  A-pf-075/062 to cold `state/archive/ledger.jsonl` (append-only).

## Next Actions
1. **Founder reviews 9 raw demo reports** (`../perfpal-test/out-*/report.html`); feedback drives next
   burst (A-pf-079). Suggested open order: hyperfine → bat → django.
2. **26-task ablation campaign** — needs model-call spend approval (A-pf-078).
3. **Owed pixel glance** at 1 component SVG + report.html (A-pf-076).
4. **M5 un-park** still needs B-pf-001 + B-pf-002 (founder action, A-pf-080).
5. Bands third layer (A-pf-077); carry R-pf-094/095/077/078/080/082.

## Change set (this session)
- **Product code:** `src/ir`, `src/schema`, `src/scanner` (+new `instrument.rs`), `src/aggregator`,
  `src/analyzer/heuristics` (5 FP gates), `src/charts` (+`component.rs`, `layers.rs`, `dot.rs`),
  `src/reporter` (`render_html`, §5 hooks), `src/cli`, `src/writer` (unchanged), `src/graph` (reused).
- **Tests:** 526 → 606 (+80 locks); goldens regen across 4 trees (new artifacts only + 1 intended
  FP removal); `baseline.json` + `labels.json` baselines never lowered.
- **Docs/evidence/scripts:** `corpus-packet.md` + `corpus.json` + `pin-corpus.sh`; `ablation-design.md`
  + `ablation-tasks.json` + `ablation-results.md` + `run-ablation.sh`; output-contract; CHANGELOG.
- **Excluded / untracked:** `brag-output/`, `perfpal-report/`, `docs/research/`, `../perfpal-test/`
  (all outside-the-repo working dirs).

## L2 state split / L6 north-star (history)
Hot/cold split landed 2026-09-29; north-star pinned 2026-10-01 (`north_star_ref` in hot state).
Paper track parked + usefulness focus are founder directives recorded this session (D-pf-201/204).
