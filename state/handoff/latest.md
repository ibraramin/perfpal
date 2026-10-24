# Session handoff — Atlas Story (D2 + S1 + S2)

- **Date:** 2026-10-24 · **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (`feat/atlas-sim`)
- **Track:** Atlas Run · **Status:** S2 committed and gated (`2108117..b8359ae`), **not pushed**. S3 is next.
- **Detail:** `state/tracks/atlas-sim/handoff.md` · **Requirement:** `docs/specs/atlas-story.md` (sealed)
- **Probe record:** `docs/atlas/core-probes.md` · **State:** `state/session-state.json`

## Objective

Turn the Atlas map into a **temporal showcase of a codebase**: entry → stages → loops → I/O → stop,
with **the payload crossing each file boundary labelled**; vibecoder-first, senior drill; static
inference labelled; fed only by Perfpal core artifacts. S1 made the story read core JSON; **S2 adds
data movement** (P3 signatures + P1 call payloads), labels every drawn edge, greys unresolved
indirection, and adds the payoff beats.

## Phase

| Slice | Scope | Status |
|---|---|---|
| S1 | P4 stages + P5 `atlas-layout.json` (`layout_version: 1`); sim consumes core JSON; `extract.py` deleted | **done** (`5faacb7`…`63914f9`) |
| S2 | P3 signatures + P1 call-site payloads → `flow` block, edge labels, unresolved grey, payoff beats | **done** (`2108117`…`b8359ae`, gate PASS) |
| S3 | P2 I/O effects (file/env/net, literal targets) | next |
| S4 | scenes at 3k files, F1–F8 gate, full payoff at scale | pending |

## Decisions

- **D-pf-251:** founder verdict — D1 presentation/animation accepted, information not insightful;
  Atlas is Perfpal's #1 priority; Atlas consumes Perfpal core data; core additions allowed when recorded.
- **D-pf-252:** spec sealed; S1 committed; P4/P5 `prototype`.
- **D-pf-253 (S2):** `flow` stays story-only and additive (`flow_version: 1`, `layout_version` 1);
  P3/P1 are `prototype`; edge labels are computed **in the core**, never re-classified in the sim;
  the resolver is deliberately conservative (unique direct names only; ambiguous/method/fn-pointer/
  macro ⇒ `unresolved`); the drawn-slice profile queue is labelled as such.
- **S2 deviations (recorded in `core-probes.md`):** the flow probe re-parses **only the drawn files**
  under `--story` (the spec's "no second parse" ideal is deferred; bounded by the canvas cap);
  `out` is unused (write-only cannot be inferred syntactically); the profile beat is the drawn-slice
  top-3-per-file candidate set, not a whole-repo KV3 queue.

## Risks / carried nits

- **Vision PASS with nits** (`docs/evidence/atlas-sim/VISION.md` Read 6): cJSON `cJSON · type-only`
  sits ~50 px from its arrow beside the `cJSON · 2` hull label; the hyperfine top band is dense;
  `…Option<Information>` is display-truncated (full payload in the tooltip); `name · literal-observed`
  grazes the `cJSON_Utils.c` border; pre-existing `PATH · 3 steps · re…` chrome truncation; cJSON
  still has ~1/3 dead space.
- **Data limits:** field-access arguments are untyped (no field type in the local table); a
  pointer-to-array C return is best-effort; `unresolved` means "callee unresolved or every argument
  untyped"; no cross-file type inference; C macros are never expanded.
- `--story` builds the Atlas view twice (single-pass later); no aggregate/cap fixture test.
- `sim/verify.py` is not run by CI (`.github/workflows/ci.yml`).
- F1 (blind-read purpose), F7 (3k scale) and F8 (founder A2) remain S4; A2/ship still blocked by the
  Atlas v2 step-13 founder gate (R-pf-109); step 11 queued.

## Next actions

1. **S3:** P2 I/O effects (read/write, target file/env/net/stdio, literal when written) as one core
   commit + registry row; then render data stores on scenes and the trust line.
2. **S4:** scenes/aggregation at 3k files, F1–F8 gate; only then decide companion-vs-in-report
   (ADR + A12 if interactive).
3. Keep Atlas v2 step 11 as the parallel queue; no A2/ship before step 13.

## Verification (this session)

| Claim | Command | Result |
|---|---|---|
| Core gates | `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | 853 passed, 0 failed, 1 ignored |
| Licenses | `cargo deny check licenses` | exit 0 (`licenses ok`) |
| F6 additive | story-off vs story-on | `report.html` byte-identical both fixtures; `metrics.json` content-equal after masking `scan.started_at`/`scan.duration_ms` |
| F2 labels | `sim/verify.py` data + DOM | every drawn edge labelled: hyperfine 49/49 (44 `unresolved`, 4 `type-only`, 1 `literal-observed`), cJSON 5/5 (1/2/2) |
| F4 honesty | grep over story JSON + final frame | no forbidden claim, no time units, `runtime` only negated |
| F5 determinism | double scan + two page loads | `atlas-layout.json`/`flow` identical; final-frame `#stage` DOM identical (hyperfine `2f1a3834…`, cJSON `65cd5446…`); PNGs are rasterization-dependent |
| Story gate | `SIM_PORT=8770 python3 sim/verify.py` | **162/162**, exit 0 |
| Full gate | `bash sim/gate.sh` | `Commit at run: b8359ae` · tree `8889ecac…` · dirty 0 · **verdict=PASS** |
| Vision | `docs/evidence/atlas-sim/VISION.md` Read 6 | **PASS** (all three S2 blockers fixed; nits carried) |
| Review panel | R1/R2/Q1/Q2 final round | 4/4 APPROVE, zero unresolved blocking findings |

**Uncommitted:** none. **No push, no merge to `main`.**
