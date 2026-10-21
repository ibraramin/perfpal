# Perfpal — I5 Map Proof + 9-Repo Demo (plan `map-build`, spec `birdseye-map` F5)

> **Scope (locked).** Prove the maintainer birdseye map (I1–I4) on real code: full gate,
> double-scan byte-identity, semantic-tier behaviour, true GENUINE calibration (A-pf-095),
> and the 9-repo demo with the A5 three-action maintainer test. **No new map idiom was
> invented here.** Where the proof found the hand-off assumptions false, the negative result
> is recorded verbatim (never smoothed over).
>
> Session: 2026-10-21 · code at `f56d072` + this burst · default build
> `perfpal 0.2.0-beta.1 (schema 1.6)`.

---

## 1. Corpora

### 1.1 Demo set (9 repos, pinned commits in `../perfpal-test/<repo>`)

| Repo | Language | Commit | Files scanned | Findings | hot |
|---|---|---|---:|---:|---:|
| alacritty | Rust | `29dc553` | 88 | 1,378 | 15 |
| bat | Rust (+ vendored Python fixture) | `62b439da` | 78 | 807 | 21 |
| cJSON | C | `6d9f244` | 101 | 837 | 7 |
| click | Python | `2247b35` | 91 | 598 | 0 |
| django | Python | `d430084` | 2,932 | 26,089 | 0 |
| hyperfine | Rust | `c516152` | 53 | 348 | 20 |
| postgres | C | `856a659` | 2,566 | 102,933 | 484 |
| rich | Python | `9d8f9a37` | 213 | 1,337 | 0 |
| tmux | C | `c251547e` | 216 | 7,545 | 6 |

All scans exit 0. Outputs live in `../perfpal-test/out-i5-<repo>` (uncommitted, per
D-pf-205); this dossier is the committed evidence.

### 1.2 A-pf-095 calibration corpora (4 pinned M4 repos)

`/var/tmp/opencode/m4-showcase/{aider,chunkhound,ripgrep,curl}` at the dossier pins
(`a4be6cc`, `a8553ae`, `4649aa9`, `75a2079d5c`). Same production-path filter as
`findings-deep-dive.md` §1.2.

---

## 2. Gate (A1) — all exit 0

| Check | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all -- --check` | exit 0 |
| clippy (default) | `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| tests (default) | `cargo test --locked` | **50 suites / 688 passed / 0 failed** |
| licenses | `cargo deny check licenses` | `licenses ok` |
| clippy (feature) | `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` | exit 0 |
| tests (feature) | `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang` | **50 suites / 691 passed / 0 failed** |
| ratchets | `cargo test --locked --test benchmark --test precision_kpi` | pass; no ratchet file touched |

**Feature-build golden gap found and fixed.** `report_matches_committed_goldens` compared
default-build goldens against a `semantic-clang` build whose resolved-edge tier changes
chart topology and ARCH-003 confidence — unmaskable in SVG. The comparator + regen path are
now `#[cfg(not(feature = "semantic-clang"))]` with the rationale in-code; the semantic tier
is covered by `tests/resolver_health.rs` (10 tests, including real libclang parses) and the
double-run determinism suites. The feature-build job is green again; with libclang present
the comparator could not pass once call-graph-dependent stems existed (the semantic tier
changes chart topology, not just masked rows).

**Semantic-report boilerplate fixed (review G4).** A semantic build's `report.md`/`report.html`
still printed the default-build "purely syntactic tier" caveat and described all cross-file
edges as LOW, contradicting the same report's `resolver_tier: semantic` rows. The reporter now
derives the tier from the data (`semantic_tier_active`): semantic runs disclose the active tier
and describe resolved edges as MEDIUM; the default build keeps the syntactic text. Locked by
`tests/reporter_render.rs::semantic_tier_boilerplate_follows_the_data` (the +1 default test
that takes 687→688).

**A2 intact.** No `metrics.json`, `graph.dot`, or `findings-detail.*` golden changed
(`git status --porcelain` for those paths is empty). The 33 golden files that changed are
the caption-only SVGs/`report.md`/`report.html` from the A4 wording fix (§6).

---

## 3. Determinism (A3)

### 3.1 Committed goldens — double regeneration

`PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens` run twice;
SHA-256 of every file under `tests/golden` after pass 1 == pass 2. **Identical.**
The regen is a no-op on an already up-to-date tree, and the second pass leaves git clean.

### 3.2 Real 9-repo double scan

Each demo repo scanned twice (pass A/B) into separate directories. Exact compare for
`charts/*` (every SVG + `graph.dot`), `findings-detail.md/.html`, `report.html`, schema copy;
provenance-masked compare for `metrics.json` (`scan.started_at`, `scan.duration_ms`) and
`report.md` (`started … duration … ms`). **0 diffs in all 9 repos** (35/37/36/33/65/27/65/23/19
files compared per repo). `DOUBLE_SCAN_IDENTICAL`.

A4 honesty markers hold in every repo: `showing top S of M`/`…+N more` where capped, no blank
scopes, LOW-confidence call edges captioned, and every heat legend now reads
`candidate … — profile to confirm` (§6). Persisted transcript + per-repo set hashes:
`docs/evidence/i5-determinism.txt` (golden tree SHA-256
`749162395aac267386ff3196022dbdea8db281282ad5571a6d12f6640c26eaec`, identical on both regen
passes; 9/9 repo set hashes identical).

---

## 4. Semantic tier

Built with `--features semantic-clang`; libclang 23.1 found via runtime dlopen;
`PERFPAL_SEMANTIC_TESTS=1` real-parse tests pass (10/10).

| Corpus | Findings synt → sem | Semantic-tier findings | ARCH-003 MEDIUM | ARCH-003 LOW | Resolved call edges |
|---|---:|---:|---:|---:|---:|
| cJSON | 837 → **1,162** | 778 | 6 → **784** | 581 → 128 | 115 |
| curl | 10,985 → **11,807** | 5,064 | 518 → **5,579** | 6,432 → 2,190 | 1,548 |

The tier works as specified: resolved C edges upgrade ARCH-003 from LOW (name-based) to
MEDIUM (semantic), and the extra findings are the documented confidence lift. Both semantic
scans remain parse-partial-heavy in the same way as their default counterparts (cJSON 63 ok /
38 partial; curl 687 ok / 236 partial) — the tool reports this per file and never fabricates
the missing regions. The human-report boilerplate that contradicted the tier is fixed and
locked (§2, G4).

### 4.1 Negative result — `layer-entry` strata are degenerate even on the semantic tier

**The I5 expectation did not hold.** In all 9 demo repos, and in both semantic scans, every
shown node in `layer-entry.svg` sits in a single `entry` column — no `depth 1…8` columns, no
`not reached` region. Root cause is upstream of rendering: **entry detection marks most C
files as entries.**

| Corpus | Files | Files with ≥1 entry | Entry functions (`lib-root` etc.) |
|---|---:|---:|---:|
| cJSON | 101 | 81 | 628 lib-root |
| tmux | 216 | 150 | 1,462 lib-root |
| curl | 923 | **612** | 1,991 lib-root |
| postgres | 2,566 | **1,592** | 13,395 lib-root |
| django | 2,932 | **1,637** | 27,386 lib-root |

`detect_c_entries` (`src/aggregator/mod.rs:567-569`) marks **every exported (non-`static`)
function in a main-less file** as a `lib-root` entry. With 612/923 curl files as roots, the
BFS distance map is dominated by distance 0 and the 50-slot entry-first cap fills with
entries; the reserve cannot surface depth≥1 files. This is an I1 design finding, not an I3/I4
rendering bug — **`R-pf-098`**. Fix options (founder decision, new plan/ADR): (a) define
lib-roots as true graph roots (files with no incoming resolved call edge), (b) cap
entry-first slots and reserve ≥20 for depth≥1 files, (c) accept and label the layer as
"entries only" on such corpora. Not changed in I5 (would alter reachability/cost semantics).

---

## 5. A-pf-095 — true GENUINE share

### 5.1 Hot-tier re-triage (same rubric, current build)

| Repo | Prod-`hot` now | Already hand-triaged | New ids | GENUINE |
|---|---:|---:|---:|---:|
| aider | 0 | 0 | 0 | 0 |
| chunkhound | 26 | 26 | 0 | 6 |
| ripgrep | 29 | 29 | 0 | 1 |
| curl | 117 | 117 | 0 | 2 |
| **Total** | **172** | **172 (100%)** | **0** | **9** |

True share **9/172 = 5.2%** — unchanged from the dossier's 9/181 (5.0%) because the current
hot set is a strict subset: 9 previously-triaged ids (ripgrep 5, curl 4) no longer fire under
the current rules. Verdicts are the hand-read `docs/evidence/showcase/<repo>/triage.json`
entries; no new hand triage was needed for this half.

### 5.2 Map top-node calibration (new measurement; supplementary — not an A-pf-095 deliverable)

Machine-readable verdicts: `docs/evidence/i5-map-triage.json` (27 nodes, `file:line`
evidence per verdict). A-pf-095 itself is the hot-tier share in §5.1; this calibration is an
extra map-quality probe added by I5 and is not counted toward that action.

Top-3 files by summed `function_cost_v2` in each demo repo (27 nodes), each read at its
cited span and classified. Rubric: GENUINE / REAL_BUT_MINOR / FP / TEST_NOISE (test, example,
bench, fixture, tools, or generated path — valid code that must not drive a maintainer's
first look).

| Verdict | Count | Share |
|---|---:|---:|
| GENUINE | **1** | 3.7% |
| REAL_BUT_MINOR | 11 | 40.7% |
| TEST_NOISE | 11 | 40.7% |
| FP (cost-model artifact) | 4 | 14.8% |
| **Total** | **27** | |

The single GENUINE is `bat` `src/printer.rs:648` `print_line` (per-line width/chunk loops on
the output path). Representative failures, all `file:line`-verified:

- `cJSON` rank 1 `tests/unity/test/tests/testunity.c:4571` — literal `num_tested < 1000000`
  loop in a vendored, macro-gated test harness.
- `postgres` rank 1–2 `src/common/unicode/{case,category}_test.c` — literal
  `code <= 0x10ffff` exhaustive ICU conformance tests (`#ifdef USE_ICU`).
- `django` `django/contrib/sessions/backends/cache.py:54` — `for i in range(10000)` retry
  guard that returns on iteration 1; cost-v2 charges 10,000×.
- `alacritty` rank 1 `window_title` — `#[cfg(test)]` loop `for _ in 0..4097`.
- `tmux` `format.c:6086-6328` — 838-branch dispatcher; fn_cost 1,331 is statement-count
  inflation, not iteration; `reachable_from_entry=false` is also wrong (callers at
  `format.c:6839,6909`).
- `rich` rank 1 `tools/profile_divide.py:12` — 10,000-iteration local benchmark loop.
- `hyperfine`/`click` entries are startup/help/teardown paths — real but one-shot.

**Reading:** the *file cost* tier is not decision-grade on real repos as currently ranked
(`R-pf-099`). Two compounding causes: (a) test/example/generated files are scanned and
ranked like production code; (b) canonical literal trip counts (e.g. 1,000,000) multiply
test loops into dominating heat. The finding-density workspace layer shows the same class of
problem on tiny scopes (`R-pf-100`): postgres rank 1 `src/include/pch` density **1000.0**
from 3 findings; rich rank 1 `.` density 333.33 from 1 finding; click rank 1
`tests/test_exceptions` 250.0 from 1 finding. Small denominators need a minimum-findings
guard or a different normalization before "rank 1" is credible.

The map is still **usable** for the A5 test (§7): the blind maintainers named 3 concrete
actions from the layered/finding views, not from the raw cost rank.

---

## 6. A4 wording fix (F4 acceptance)

Five heat captions lacked the mandatory "candidate" framing: `src/charts/layers.rs:209,415,
840,1194`, `src/charts/component.rs:258` (component-page ramp), plus the
`hot-components.svg` subtitle (`src/charts/mod.rs:1110`) and three `report.md` link captions
(`src/reporter/mod.rs:2313-2319`). The six in-chart legends now read
`candidate … — profile to confirm`; the report-level link captions read `candidate …`
(link prose, no suffix). `docs/report/output-contract.md` was updated to match. This is the
only **A4** code change; I5 also fixes the feature-build golden gate and the semantic-report
boilerplate (§2). Goldens were regenerated (`33` files: 4×
`layer-{workspace,file,entry,bands}`, `hot-components`, `report.md`, `report.html`, and the
component pages that carry the ramp).
`README.md` + `docs/website/scope-and-content.md` were updated from the stale 3-chart prose
to the current 8-stem overview set + component pages + `graph.dot` (R-pf-097, partially
closed; ADR-0002 stays untouched as a historical record).

---

## 7. 9-repo demo + A5 three-action maintainer test

### 7.1 Artifact audit (all 9 repos)

Every repo emits all 9 expected stems (`include-graph`, `layer-file`, `layer-entry`,
`component-coupling`, `hot-components`, `layer-workspace`, `layer-bands`, `layer-findings`,
`graph.dot`), the bundled schema copy, `findings-detail.*`, and 4–50 component pages.
`report.html` stays bounded (41–58 KB) with 153–211 drill anchors into
`findings-detail.html`; `findings-detail.html` carries the uncapped rows. No repo has a
missing artifact or a blank scope.

### 7.2 Blind test (independent, map-only)

Verbatim machine-readable results: `docs/evidence/i5-blind-a5.json`.

Two independent subagents with no prior repo context read only `report.md`, `report.html`,
and the 8 SVGs. Both returned `ambiguous=false` and named three actions:

**tmux (C):**
1. *look first* → `format.c` (hot-path outline; `PERF-C-ARCH-002:format.c:6052:1`);
2. *do not touch* → `tmux.h` (graph convergence node; 192 parse-error nodes);
3. *profile* → `tty.c` (entry stratum, 1 hot / 7 warn, 144 candidates).

**hyperfine (Rust):**
1. *look first* → `src/benchmark/executor.rs` (16/16 hot ALLOC-001; exemplar `:276:9`);
2. *do not touch* → `src/command.rs` (max-cost outline, highest degree, 575 LOC);
3. *profile* → `src/output/report.rs` (rank-1 component `src/output` 84.81; 2 hot).

Both noted the same caveats this dossier records: the entry layer is degenerate (0 resolved
edges in the default build; strata = entries only), and `fuzz`/`examples`/`tests`
components can rank #1 on tiny denominators. **A5 verdict: PASS with documented caveats** —
the map is usable, but the ranking needs the `R-pf-098/099/100` fixes before "rank #1" is
trustworthy. **Timing:** not measured for these asynchronous agent runs; the spec's
"under 10 minutes" bound is therefore unverified for the blind tests and must be timed on the
founder run (§7.3).

### 7.3 Founder test (verbatim)

**PENDING** — founder to name 3 actions from the tmux map (`../perfpal-test/out-i5-tmux/`:
`report.html` → `#map`). Verdict/quote to be appended before the plan is marked done.

---

## 8. Findings queued (not silently fixed in I5)

| Id | Severity | Finding |
|---|---|---|
| `R-pf-098` | high (map) | Entry detection marks most C files as `lib-root` entries (612/923 curl, 1,592/2,566 postgres) → `layer-entry` strata collapse to one `entry` column even on the semantic tier. Needs a founder decision (true graph roots vs cap reserve vs honest label). |
| `R-pf-099` | medium (map) | File cost rank is dominated by tests/examples/generated files + literal-bound trip inflation; top-3 calibration = 1/27 GENUINE. Needs a production-path policy and/or a known-trip sanity guard before ranking. |
| `R-pf-100` | medium (map) | Finding-density rank explodes on tiny scopes (postgres `src/include/pch` 1000.0 from 3 findings; rich `.` 333.33 from 1). Needs a minimum-findings/denominator guard. |
| `R-pf-101` | low | ADR-0002 remains the historical 3-chart record (intentionally not edited); README/website now point at the current 8-stem set. |
| `R-pf-102` | low | Workspace figure still emitted twice in `report.html` (`#map` + `#architecture`) — cosmetic, deferred. |
| `R-pf-103` | low | The feature golden comparator is default-build-only, so the semantic build now has no committed-golden lock (it is covered by `resolver_health` + double-run determinism). A semantic-specific golden set would restore regression coverage. |

---

## 9. Reproduce

```bash
# gate
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo deny check licenses
cargo clippy --locked --all-targets --features semantic-clang -- -D warnings
PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang

# goldens: double regeneration must be a no-op
PERFPAL_REGEN_GOLDEN=1 cargo test --test p6_hardening regenerate_goldens   # twice
git status --porcelain tests/golden

# demo double scan (per repo)
target/release/perfpal ../perfpal-test/<repo> -o ../perfpal-test/out-i5-<repo>
target/release/perfpal ../perfpal-test/<repo> -o ../perfpal-test/out-i5b-<repo>
# compare charts exact; mask scan.started_at/duration_ms in metrics.json + report.md
```

**Determinism hashes (final tree).** Golden double-pass SHA-256 set identical; 9/9 demo
double-scans identical; `metrics.json`/`graph.dot`/`findings-detail.*` goldens untouched.
Persisted transcript + hashes: `docs/evidence/i5-determinism.txt`.
