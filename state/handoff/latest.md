# Session handoff — Atlas Story (D2 + S1 + S2 + S3)

- **Date:** 2026-10-26 · **Worktree:** `/home/shafin/Opencode/Perfpal-atlas-sim` (`feat/atlas-sim`)
- **Track:** Atlas Run · **Status:** S3 committed (`0ddf4ad` + `b63c6f6` core, `9e23aa6` + `f3e8804` story), gated; **not pushed**. S4 is next.
- **Detail:** `state/tracks/atlas-sim/handoff.md` · **Requirement:** `docs/specs/atlas-story.md` (sealed)
- **Probe record:** `docs/atlas/core-probes.md` · **State:** `state/session-state.json`

## Objective

Turn the Atlas map into a **temporal showcase of a codebase**: entry → stages → loops → I/O → stop,
with **the payload crossing each file boundary labelled**; vibecoder-first, senior drill; static
inference labelled; fed only by Perfpal core artifacts. S1 made the story read core JSON; S2 added
data movement (P3 signatures + P1 call payloads); **S3 adds I/O effects** (P2 read/write, target
file/env/net/stdio, literal only when the source writes a plain string), rendered as `Stores:` on the
trace beats, a per-file DATA STORES inspector, and an I/O trust line.

## Phase

| Slice | Scope | Status |
|---|---|---|
| S1 | P4 stages + P5 `atlas-layout.json` (`layout_version: 1`); sim consumes core JSON; `extract.py` deleted | **done** (`5faacb7`…`63914f9`) |
| S2 | P3 signatures + P1 call-site payloads → `flow` block, edge labels, unresolved grey, payoff beats | **done** (`2108117`…`b8359ae`, gate PASS) |
| S3 | P2 I/O effects (file/env/net/stdio, literal targets) → stores + trust line | **done** (`0ddf4ad` + `9e23aa6`, gate PASS) |
| S4 | scenes at 3k files, F1–F8 gate, full payoff at scale | pending |

## Decisions

- **D-pf-251:** founder verdict — D1 presentation/animation accepted, information not insightful;
  Atlas is Perfpal's #1 priority; Atlas consumes Perfpal core data; core additions allowed when recorded.
- **D-pf-252:** spec sealed; S1 committed; P4/P5 `prototype`.
- **D-pf-253 (S2):** `flow` stays story-only and additive (`flow_version: 1`, `layout_version` 1);
  edge labels are computed **in the core**; the resolver is conservative; the profile beat is the
  drawn-slice queue.
- **D-pf-254 (S3):** P2 `flow.io[]` uses a **fixed v1 call-name table**. A bare C name defined in the
  drawn set, a macro, or **any local/parameter value** (function pointer, typedef'd pointer or closure)
  is never I/O; the C bare-name rules are language-gated; Rust matches the complete written path.
  A literal is recorded only for a plain `string_literal`/`raw_string_literal` (never `format!`,
  `concat!`, C adjacent-string concatenation or `sprintf`); a missing literal stays `null` and renders
  as `unknown`. Non-literal modes/flags, **non-zero numeric `open` flags** (platform-specific bits) and
  variables emit no row (direction unknown, never guessed).
  `layout_version` and `flow_version` stay 1; existing scans are byte-identical.
- **S3 limits (recorded):** a wrapper defined outside the drawn set can still match; a Rust local
  `File`/`fs` wrapper with the same written path can still match until imports/type resolution lands;
  `read`/`write` are fd-based so `target: file` is the call class (may be pipe/socket); method calls
  (`OpenOptions::open`, `write_all`) are not matched; `env::args*` is an `env` read; `connect` is the
  outbound `write` net effect; `std::fs::copy` is read+write with no single literal, and `rename`/
  `hard_link` emit no literal (source vs destination would be a guess); literal values are source
  text (quotes stripped, escapes not decoded, display-capped at 160); stdio rows never carry a literal.

## S3 shipped

- **Core (`0ddf4ad` + `b63c6f6`):** `src/analysis/flow.rs` gains `Flow.io[] =
  {file, line, callee, op, target, literal}` and the fixed `IO_TABLE_VERSION = 1` table. Guards:
  drawn-name wrapper, macro, any local/parameter value (incl. typedef'd fn-pointer), exact Rust
  paths, C-rule language gate, plain-string-only literals, `freopen` mode index, numeric-flag
  conservatism; deterministic sort `(file, line, callee, op, target, literal)`.
  `tests/pipeline_e2e.rs` asserts the emitted shape, sorted rows, drawn call-site files, stdio-null
  literals and a conservative wrapper/formatted-target fixture. 862 tests.
- **Story (`9e23aa6` + `f3e8804`):** `sim/from-layout.py` projects per-node `io` summaries +
  `trust.io` (totals, stdio, literal examples); `sim/sim.js` renders `Stores:` on trace beats, a
  DATA STORES inspector section (up to 6 rows/file, `showing N of M`; stdio = `stream · no path`),
  and the I/O trust line (literal examples, `+N` remainder, stdio count); `sim/verify.py` adds 54
  checks and stores/trust/inspector keyframes; `sim/gate.sh` prints the io summary and asserts the
  full dirty count is 0. Regenerated `sim/data-hyperfine.json` / `sim/data-cjson.json`.
- **Fixture rows:** hyperfine 8 rows / 5 drawn files (5 read, 3 write; literal `TERM`, `NO_COLOR`,
  `/dev/null`; 5 target-unknown); cJSON 24 rows / 4 drawn files (4 read, 20 write; 0 literal,
  2 target-unknown; 22 stdio).

## Risks / carried nits

- **S3 limits:** wrapper outside the drawn set; fd-based target class; method-based file writes not
  matched; `env::args*` as `env` read; `connect` as outbound; literal display cap. Recorded in
  `docs/atlas/core-probes.md` (P2 record).
- **Carried S2 vision nits:** cJSON detached `type-only` label, dense hyperfine top band, truncated
  payload display, `PATH` truncation, cJSON dead space. Untouched in S3.
- **Carried data limits:** drawn-file re-parse, local types only, field-access args untyped,
  pointer-to-array ret; profile is drawn-slice; `sim/verify.py` is not in CI.
- `--story` builds the Atlas view twice; S4 owns scale/aggregation.
- F1 (blind-read purpose), F7 (3k scale) and F8 (founder A2) remain S4; A2/ship is blocked by the
  Atlas v2 step-13 founder gate (R-pf-109).

## Next actions

1. **S4:** scenes/aggregation at 3k files, F1–F8 gate; only then decide companion-vs-in-report
   (ADR + A12 if interactive).
2. **Atlas v2:** step 11 (G-style detail rail) remains the parallel queue; no A2/ship before step 13.
3. Keep the S3 evidence chain (`GATE.txt`, `checks.json`, stores/trust/inspector frames, `VISION.md`,
   4-lane panel) with the S3 close commit.

## Verification (this session)

| Claim | Command / artifact | Result |
|---|---|---|
| Core gates | `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --locked` | 862 passed, 0 failed, 1 ignored |
| Licenses | `cargo deny check licenses` | exit 0 (`licenses ok`) |
| F6 additive | story-off vs story-on | `report.html` byte-identical hyperfine + cJSON; `metrics.json` content-equal after masking scan provenance |
| Core flow determinism | two `--story` scans | `atlas-layout.json` byte-identical; `flow.io` sorted `(file,line,callee,op,target,literal)` |
| Story gate | `SIM_PORT=8770 python3 sim/verify.py` | **216/216**, exit 0 |
| Full gate | `bash sim/gate.sh` | final transcript on the S3 docs commit, `dirty 0`, **verdict=PASS** (pending at this commit; the gate now fails on a non-zero full dirty count) |
| Vision | `docs/evidence/atlas-sim/VISION.md` Read 7 | **PASS** (first pass PASS-WITH-NITS; stdio total, `+N` marker and `stream · no path` fixed; re-read PASS) |
| Review panel | R1/R2/Q1/Q2 + fresh confirmation | round 1: R1/R2 CHANGES_REQUESTED (freopen index, numeric flags, typedef pointer), Q1 APPROVED, Q2 pending final gate; fixes `b63c6f6`/`f3e8804`; round 2 pending |

**Uncommitted:** none after the S3 docs/state commit. **No push, no merge to `main`.**
