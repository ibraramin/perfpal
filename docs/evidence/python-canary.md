# W0 canary — Python rule-set frequency evidence (plan `python-backend`)

> **Scope.** `python-backend` **W0** is a *pre-freeze canary*: before committing a
> Python catalog, measure whether the candidate `PERF-PY-*` rules actually fire on
> real AI-assisted Python at a frequency that justifies them — and whether their
> **silent controls** stay quiet. This document is the evidence record. It changes
> **no product code** (`src/`, `Cargo.toml`, schema, goldens are untouched); it
> produces the frequency × plausibility ranking that feeds the W1 disposition table.
>
> **Why it exists.** The M1 lesson: five rules produced ~89% of the C/Rust self-scan
> noise. M3 must not repeat that on Python. ADR-0007 records this canary as the
> mitigation that keeps M4 from being the *first* Python precision test.
>
> **Reproduce:** see §10. The counter (stdlib-only, deterministic) is committed at
> `docs/evidence/python-canary/counter.py` (sha256
> `00157c4c841d0b9caf11680e0acd967284b7812a1d14a0f77719364ee79292b5`); raw counts at
> `docs/evidence/python-canary/canary_results.json`. Every rule id **and** control id
> is pre-seeded with `0`, so "counted zero times" is visible rather than ambiguous.

---

## 1. Method

Three inputs, all pinned and reproducible:

1. **Two AI-assisted Python corpora** (the evidence-first target audience — see
   `evidence-beta-spec` §1):
   - **Aider** `v0.86.0` — Apache-2.0, ~88% AI-written (the M4 showcase repo).
   - **chunkhound** `v6.0.0` — MIT, 100% AI-written (the M4 showcase repo).
2. **A throwaway pattern counter** — a single-pass CPython `ast` walker
   (`counter.py`) that counts each candidate rule **and an explicit silent control**
   per rule, split **production vs test** files. Stdlib-only (zero deps), byte-stable
   file ordering, one `ast.parse` per file. It covers the **full** candidate set:
   the C/Rust mirrors, the Python-native adds, the pandas family, `LOOP-009`
   (try/except-in-loop, surfaced by the Ruff baseline), and graph-derived `ARCH-001`.
   Every rule/control id is pre-seeded to `0` and the totals are merged with a plain
   dict (not `Counter` addition), so zero counts survive into `*_total` too.
3. **A Ruff `PERF` baseline** — `ruff check --select PERF` (ruff `0.16.9`) as the
   closest existing shipped comparison, to (a) cross-validate the counter where an
   analogue exists and (b) show where Perfpal has no analogue at all.

**This is frequency evidence, not a precision measurement.** The counter is a
*syntactic pattern matcher*, not the real analyzer: its job is to answer "how often
does this shape occur in real AI Python?" so implausible rules die before
implementation. Precision is measured later (W4 KPI + M2 benchmark). §9 lists the
counter's known approximations.

---

## 2. Corpus (repro packet)

| Corpus | License | AI share | Pinned commit | Python files | LOC (prod / total) |
|---|---|---|---|---|---|
| Aider `v0.86.0` | Apache-2.0 | ~88% AI-written | `a4be6ccd87ebaa59b361f3f028d116ce1761b626` | 146 (105 prod / 41 test) | 25,776 / 38,123 |
| chunkhound `v6.0.0` | MIT | 100% AI-written | `a8553ae644686d8ee55b94ee74ff560a38e6cc7d` | 868 (381 prod / 487 test) | 130,361 / 262,847 |

Both parse clean: **0 `SyntaxError`** across 1,014 files (Aider 146 + chunkhound 868).

Test classification: a file counts as *test* if its path contains a `/tests/`,
`/test/` or `/testing/` segment, its basename starts with `test_`, or it is
`conftest.py`. Vendored/build dirs (`site-packages`, `.venv`, `node_modules`,
`build`, `dist`, caches) are excluded.

---

## 3. Candidate-rule frequency (the ranking)

`total` = all files; `prod/kLOC` = hits per 1,000 lines of **production** Python
(the decision-relevant number). Rows are ordered by overall production signal
(informal, both corpora considered). `ARCH-*` rows are **graph-derived and
repo-wide** (not statement counts; see §6/§9); `ARCH-002/003` are **not measured**
(§6).

| Candidate (mirror →) | aider total | aider prod/kLOC | chunk total | chunk prod/kLOC | Ruff analogue | Disposition |
|---|--:|--:|--:|--:|---|---|
| `ALLOC-001` (object churn) | 361 | **13.35** | 1187 | 5.43 | — | **REFRAME** — 98% f-string noise (§3.1) |
| `ALLOC-003` (superlinear growth) | 185 | 7.02 | 947 | **6.05** | — | **KEEP — flagship** |
| `LOOP-002` (setup/I-O in loop) | 184 | 6.36 | 197 | 0.53 | — | **REFRAME** — drop `print` (§3.2) |
| `CALL-002` (complexity ≥ 15) | 67 | 2.44 | 329 | 1.83 | — | KEEP (Info) |
| `STR-001` (`str +=` in loop) | 52 | 2.02 | 12 | 0.05 | `PERF201`* | **KEEP** (MEDIUM/LOW; corpus-dependent) |
| `LOOP-009` (try/except in loop) | 49 | 1.79 | 153 | 0.99 | `PERF203` | **ADD — validated** (§5) |
| `ASYNC-002` (await in loop) | 0 | 0.00 | 264 | **1.36** | — | **KEEP — native** |
| `COMPREHENSION-001` (manual comp.) | 16 | 0.62 | 56 | 0.38 | `PERF401` | **KEEP — validated** (§5) |
| `N1-001` (DB/HTTP in loop) | 9 | 0.31 | 75 | 0.51 | — | **KEEP — native flagship** |
| `LOOP-007` (membership scan) | 4 | 0.16 | 104 | 0.64 | — | **KEEP — flagship**, loop-scope it |
| `MEM-002` (copy in loop) | 14 | 0.39 | 29 | 0.21 | `PERF402`* | KEEP (Info) |
| `LOOP-001` (nesting ≥ 3) | 7 | 0.27 | 15 | 0.09 | — | KEEP (Info, rare) |
| `CALL-001` (recursion) | 5 | 0.16 | 12 | 0.08 | — | KEEP (Info) |
| `LOOP-003` (`range(len())`) | 3 | 0.12 | 2 | 0.02 | — | KEEP (Info, near-zero) |
| `ASYNC-001` (blocking in `async`) | 0 | 0.00 | 44 | 0.13 | — | **KEEP — native** |
| `PANDAS-001` (iterrows/apply axis=1) | 0 | 0.00 | 4 | 0.03 | — | **ADD** (near-zero on this corpus) |
| `PANDAS-002` (concat in loop) | 0 | 0.00 | 0 | 0.00 | — | **ADD** (0 hits; no pandas-heavy corpus) |
| `EXECUTOR-001` (executor/loop per iter) | 0 | 0.00 | 1 | 0.01 | — | KEEP (Info, near-zero) |
| `ARCH-001` (cycles/deep chains)† | 3 | 0.12 | 488 | 3.74 | — | **KEEP — not decision-grade in W0; measure in W2/W4** (§6) |
| `ARCH-002` (hot component)† | — | — | — | — | — | **DEFER — not measured** (§6) |
| `ARCH-003` (name coupling)† | — | — | — | — | — | **DEFER — not measured** (§6) |

\* Ruff's `PERF201` (bytes-concatenation-in-loop) is a **partial** analogue to
`STR-001`; `PERF402` (manual list-copy) partially overlaps `MEM-002` — so the
"Perfpal has no analogue" claim holds for `ALLOC-003`/`N1`/`ASYNC`/`LOOP-*`, not for
`STR-001`.  † Graph-derived (repo-wide); `ARCH-002` (hot-component aggregation) and
`ARCH-003` (name-based *call* coupling) are **not** derivable from an import graph
and are therefore recorded as not measured.

### 3.1 `ALLOC-001` is a noise cannon as naively defined

The mirror's raw hit count is **masked by f-strings**: `ALLOC-001::fstring` is
**354 / 361** (aider, 98.1%) and **1182 / 1187** (chunkhound, 99.6%). The intended
signal (a fresh empty container constructed inside a loop) is only 7 and 5 hits
respectively.

**Consequence:** a rule that fires on "f-string (or constructor call) inside a loop"
would emit ~13 hits/kLOC on production Aider — the largest single noise source in the
canary. `ALLOC-001` must ship **reframed** as a *loop-invariant* construction rule
(the constructed value must not depend on the loop variable) or not at all. This is
the single biggest precision risk in the mirror set and is carried to W1 as a hard
obligation.

### 3.2 `LOOP-002` is dominated by `print`

`LOOP002::print` is **124 / 184** (aider) and **162 / 197** (chunkhound); adding
`input` (9 / 1) makes `print`+`input` **72–83%** of the rule's hits. The real I/O
families are rare by comparison (all-file counts): Aider non-`print`/`input` I/O =
**51** hits (`open` 11, `write` family 13, `requests.get` 6, `patch` 5, `post` 3,
`read` family 5, `os.get` 3, `subprocess.run` 2, `readlines`/`send`/`check_output` 1
each); chunkhound non-`print`/`input` I/O = **34** hits (`read` family 11, `readline`
family 9, `open` 6, `write` family 5, `os.get` 3).

**Consequence:** drop `print`/`input` from the setup/I-O family (or demote to Info).
Keeping them would make `LOOP-002` the second-largest noise source.

---

## 4. Silent-control evidence (why the gates exist)

Each control is a shape that is **lexically similar but semantically benign** — the
thing that must *not* fire. Counts are `total` with `prod` in parentheses. The `+=`
controls (`CTL-INT-ACC`/`CTL-AMBIG-ACC`/`CTL-EXPR-ACC`) are counted **inside loops
only** — the same scope as `STR-001` — so the comparison is like-for-like. Every
candidate rule has a control except where a control is uninformative (explicitly
listed as N/A below).

| Control | gates | aider | chunkhound | What it proves |
|---|---|--:|--:|---|
| `CTL-SLEEP-SYNC` | `ASYNC-001` | 249 (166) | 291 (119) | Blocking calls in **sync** functions are ubiquitous. Ungated, `ASYNC-001` would be ~6/kLOC of pure noise on Aider vs **0** real hits. The `async def` gate is the single highest-leverage gate in the canary. |
| `CTL-AWAIT-NOLOOP` | `ASYNC-002` | 0 | 2677 (709) | Awaits **outside** loops outnumber in-loop awaits ~4:1 (prod). Loop-gating is mandatory. |
| `CTL-IO-NOLOOP` | `LOOP-002` | 646 (490) | 1124 (382) | I/O is far more often *outside* a loop than inside it (aider 490 vs 164); the loop gate discards the majority. |
| `CTL-FSTRING-NOLOOP` | `ALLOC-001` | 662 (589) | 4292 (2737) | F-strings outside loops dwarf the in-loop count (chunkhound 2737 vs 708). The loop gate does real work — yet the residual in-loop churn is still ~13/kLOC (aider) / ~5.4/kLOC (chunkhound), which is why `ALLOC-001` must be reframed (§3.1). |
| `CTL-TRY-NOLOOP` | `LOOP-009` | 229 (223) | 1157 (982) | `try/except` is very common outside loops; only the loop-scoped subset is a rule. |
| `CTL-INT-ACC` | `STR-001` | 51 (51) | 118 (110) | In-loop `+=` with a numeric-constant RHS. On chunkhound these outnumber string `+=` 110 : 7 — the discrimination is indispensable. |
| `CTL-AMBIG-ACC` | `STR-001` | 26 (26) | 41 (37) | In-loop `x += <name>` — **kind unknown** (could be str or numeric); deliberately *not* mislabelled numeric. |
| `CTL-EXPR-ACC` | `STR-001` | 45 (45) | 65 (56) | In-loop `+= <binop/other-call>` — ambiguous; neither str nor numeric. |
| `CTL-STR-NOLOOP` | `STR-001` | 50 (50) | 35 (33) | String-ish `+=` **outside** a loop. |
| `CTL-IN-SET` | `LOOP-007` | 0 | 81 (72) | Membership in a **set/dict** is nearly as common as in a **list** (chunkhound 72 vs 83). Without the binding/kind inference to tell list from set, `LOOP-007` would be ~46% FP. Directly justifies ADR-0007 §5 (inference ships in M3). |
| `CTL-APPEND-LOCAL` | `ALLOC-003` | 24 (24) | 81 (69) | `append` to a receiver built **inside** the same loop is benign. |
| `CTL-APPEND-OUTER-RESET` | `ALLOC-003` | 10 (10) | 56 (54) | `append` to a receiver reset in an **outer** loop frame — freshly rebuilt per outer iteration, so the superlinear premise fails. |
| `CTL-APPEND-MULTI` | `COMPREHENSION-001` | 55 (53) | 276 (221) | Multi-statement loop bodies containing `append` — *not* collapse-able to a comprehension; the single-statement gate matters. |
| `CTL-COPY-NOLOOP` | `MEM-002` | 56 (35) | 283 (143) | Copies/materialisations **outside** loops. |
| `CTL-N1-NOLOOP` | `N1-001` | 17 (17) | 468 (332) | DB/HTTP calls outside loops (chunkhound 332 vs 66 in-loop). |
| `CTL-EXECUTOR-NOLOOP` | `EXECUTOR-001` | 0 | 126 (45) | Executor/task creation outside loops. |
| `CTL-LEN-IN-LOOP` | `STR-002` *(retired)* | 117 (117) | 423 (334) | `len()` in a loop is very common **and O(1)** in CPython — the evidence that retiring `STR-002` was correct (shipping it would be ~2.5/kLOC of dead advice). |
| `CTL-RANGE-PLAIN` | `LOOP-003` | 30 (28) | 228 (137) | Plain `range(n)` vastly outnumbers `range(len(x))`; the `len()` requirement keeps `LOOP-003` near-silent. |
| `CTL-DEPTH2` | `LOOP-001` | 75 (70) | 255 (225) | Depth-2 nesting is common; only depth ≥ 3 is the rule — so `LOOP-001` stays rare/informative. |
| `CTL-LISTADD-NOLOOP` | `ALLOC-003` | 31 (31) | 9 (8) | `x += [..]` list concat **outside** a loop. |
| `CTL-COMPILE-IN-LOOP` | `re.compile` *(CUT)* | 0 | 0 | **Zero** in either corpus (~301k LOC) — confirms the ADR-0007 decision to cut the `re.compile`-outside-loop rule as folklore. |

**Controls intentionally N/A (documented, not silent omissions):**
- `CALL-001` (recursion): the near-miss is "a call to a *different* function" — every
  function has those, so a counter adds nothing. The gate is the self-name match.
- `CALL-002` (complexity ≥ 15): the control is the sub-threshold complement of the
  rule (uninformative as a count).
- `PANDAS-001` / `PANDAS-002`: only 4 / 0 hits total; a "non-pandas apply" control
  would need receiver-type evidence the counter does not have.
- `ARCH-001/002/003`: deferred (graph proxy not decision-grade — §6).

---

## 5. Ruff `PERF` baseline + cross-validation

`ruff check --isolated --select PERF --statistics` (ruff `0.16.9`):

| Corpus | Rule | all files | prod only (`--exclude tests`) |
|---|---|--:|--:|
| Aider | `PERF203` try-except-in-loop | 19 | 18 |
| Aider | `PERF401` manual-list-comprehension | 13 | 13 |
| Aider | **total** | **32** | **31** |
| chunkhound | `PERF401` manual-list-comprehension | 49 | 43 |
| chunkhound | `PERF203` try-except-in-loop | 44 | 34 |
| chunkhound | `PERF102` / `PERF403` | 1 / 1 | 1 / 1 |
| chunkhound | **total** | **95** | **79** |

**Cross-validation.** Ruff's `PERF401` (manual list comprehension) is the direct
analogue of `COMPREHENSION-001`: **16 vs 13** (Aider, ~23%) and **56 vs 49**
(chunkhound, ~14%) — the counter tracks an independently-maintained tool to within
~15–25% with no tuning. That calibration is the main reason to trust the other counts.

**`LOOP-009` vs Ruff `PERF203` — a documented divergence.** Ruff confirmed
(probe, §10) that `PERF203` fires on a `try/except` that is a **direct** child of a
sync `for`/`while` body, and does **not** fire for `try/finally` (no `except`), for a
`try` nested inside a `with`, or for `async for`. The counter implements the same
direct-child shape, yet counts **49 vs 18** (Aider) and **153 vs 34** (chunkhound) —
a 2.7–4.5× gap. Ruff's remaining exclusions (it appears to report at most one
violation per loop body in the probe) were **not fully reverse-engineered**. The
`LOOP-009` figure is therefore an **upper estimate** and `PERF203` the lower; the true
value needs the real analyzer in W4. The direction (a real, non-trivial footprint) is
unambiguous.

**Differentiation.** Ruff's PERF set has **no** analogue for `ALLOC-003`, `N1-001`,
`ASYNC-001/002`, `LOOP-001/002/003/007`, `CALL-001/002`, or the `ARCH-*` family —
Perfpal's edge on AI Python (accumulator growth, membership scans, N+1 I/O,
blocking-in-async) stands. It *does* partially cover `STR-001` (`PERF201`
bytes-concat-in-loop) and `MEM-002` (`PERF402` manual list-copy), and it covers
`LOOP-009` (`PERF203`) — the last of which Perfpal had **no** analogue for, hence the
proposed add.

---

## 6. Import-graph / ARCH evidence (indicative only)

| Metric | Aider | chunkhound |
|---|--:|--:|
| Modules | 146 | 868 |
| Internal import edges | 301 | 2323 |
| Cycles (detected) | 1 | 71 |
| Max import depth | 8 | 39 |
| Modules with depth ≥ 8 | 2 | 417 |
| Max hub fan-in | 57 (`aider.dump`) | 189 (`chunkhound.core.types.common`) |

`ARCH-001` (graph-derived) = cycles + deep-chain modules = **3** (Aider) / **488**
(chunkhound). The raw JSON also records `arch001_hub_subcase` — whether max hub
fan-in crossed the ARCH-001 `HUB_FANOUT = 12` floor (1 on both corpora).
`ARCH-002` (hot-component aggregation over `FunctionRecord`s) and
`ARCH-003` (name-based *call* coupling) are **not derivable from an import graph** and
are recorded as **not measured** — do not read the `ARCH-001` number as covering them.

**Not decision-grade.** The resolver is deliberately crude (no `TYPE_CHECKING`
handling, no `__init__` re-export resolution, no conditional-import handling), and the
longest-path metric is memoized (an approximation, **not sound under cycles** — §9).
The 71 chunkhound cycles are an **upper bound**, likely inflated by the unresolved
re-export/`TYPE_CHECKING` patterns. The `ARCH-001` figure must **not** be read as a
finding count.

What the graph *does* establish qualitatively: internal import graphs in real AI
Python are **large and deep** (chunkhound depth 39 ≫ the C/Rust `ARCH-001` threshold
of 8), so the `ARCH-*` family has abundant structure — but `ARCH-001` thresholds,
`ARCH-002` hot-component ranking, and `ARCH-003`'s name-based coupling must be
**measured on Python with the real pipeline in W2/W4**, and `ARCH-003` is expected
**Info/low** (ADR-0007: duck typing + ubiquitous names). A production Python resolver
is the T2 `python-resolver` milestone; W2 ships best-effort resolution only.

---

## 7. Finalized dispositions (input to the W1 table)

**Unchanged from ADR-0007 §6 — retire 10:** `TYPE-001`, `TYPE-002`, `MEM-001`,
`ALLOC-002`, `STR-002`, `STR-003`, `LOOP-004`, `LOOP-005`, `LOOP-006`, `LOOP-008`.
The canary *reinforces* two: `CTL-LEN-IN-LOOP` (§4) shows the `STR-002` premise is
common-but-harmless (O(1) `len`), and `LOOP-005`-style global writes are dict writes
in Python.

**Keep (reframed mirrors), with canary adjustments:**

| Rule | Decision |
|---|---|
| `ALLOC-003` | **Flagship.** Dense on both corpora (7.0 / 6.0 per kLOC prod), no Ruff analogue. |
| `LOOP-007` | **Flagship.** Must be **loop-scoped** (the counter counted list-literal membership anywhere — §9) and must use the kind inference to exclude set/dict membership (§4 `CTL-IN-SET`). |
| `N1-001` | **Native flagship.** Real production footprint (chunkhound 66 prod, `conn.execute`/`fetchone`/`fetchall` in loops; `CTL-N1-NOLOOP` 332). |
| `ASYNC-002` | **Native.** 1.36/kLOC prod on the async corpus; loop-gating proven necessary (§4). |
| `ASYNC-001` | **Native.** Low raw count but the highest-value rule (blocking-in-async); the `async def` gate is proven necessary. |
| `STR-001` | **Keep at MEDIUM/LOW**, corpus-dependent (Aider 2.0 vs chunkhound 0.05 per kLOC prod); honest about refcount==1 amortisation; the loop-gated `+=` control set shows discrimination is load-bearing. |
| `COMPREHENSION-001` | **Keep**, cross-validated against Ruff `PERF401` (§5). |
| `MEM-002` | **Keep** (Info); partial Ruff overlap (`PERF402`). |
| `CALL-002`, `CALL-001`, `LOOP-001` | Keep (Info); low, plausible frequency. |
| `LOOP-003` | Keep (Info) but **note near-zero frequency** (0.12 / 0.02 per kLOC prod) — ship for honesty/education or demote; the `len()` gate keeps it FP-free. |
| `EXECUTOR-001` | Keep (Info); near-zero frequency (1 hit total across 300k LOC). |
| `ARCH-001` | **Keep, but measure before trusting** — the family has abundant structure (§6); thresholds are deferred to W2/W4. |
| `ARCH-002`, `ARCH-003` | **Keep, expected Info/low** (ADR-0007); **not measured in W0** — measure in W4. |

**Reframe (do not ship as naively defined):**

| Rule | Change |
|---|---|
| `ALLOC-001` | Ship only as **loop-invariant construction** (constructed value independent of the loop variable), Info-first; f-string-in-loop is **not** a finding. 98–99.6% of the raw signal is f-strings (§3.1). |
| `LOOP-002` | Drop `print`/`input` from the I/O family (≈72–83% of hits, §3.2); keep file/network/subprocess/socket I/O and consider Info for `print`. |

**Add:**

| Rule | Rationale |
|---|---|
| `PERF-PY-LOOP-009` — try/except inside a loop | Surfaced by the Ruff baseline (§5); Perfpal had no analogue; real footprint (counter Aider 49 / chunkhound 153; Ruff's narrower count 18 / 34). Info severity; honest that this is perflint folklore (the zero-cost `try` claim is version-dependent). |
| `PERF-PY-PANDAS-001` — row-wise pandas (`iterrows`/`itertuples`/`apply(axis=1)`) | Required by ADR-0007 §6. Near-zero on these corpora (4 hits, chunkhound) but they are not pandas-heavy; ships Info. |
| `PERF-PY-PANDAS-002` — `concat` in a loop | Required by ADR-0007 §6. **0 hits** on both corpora — ships Info with the frequency caveat; `.append`/`DataFrame.append` cannot be distinguished from `list.append` syntactically (§9). |

**Deferred to W1/W4:** the durable per-rule disposition table lives in
`docs/architecture/python-backend.md` (ADR-0007 §Consequences). W0 supplies the
evidence; W1 owns the table.

---

## 8. Design implications carried to W1

1. **Loop-invariant gating is the top precision risk** (`ALLOC-001`). W1 must decide
   the invariance test (constructed value independent of the loop variable) before
   `ALLOC-001` exists at all.
2. **The kind lattice is load-bearing, not optional** (`CTL-IN-SET`: 72 set-membership
   controls vs 83 list hits). ADR-0007 §5's on-by-default inference is validated by
   data.
3. **`async def` context tracking is required for `ASYNC-001`** (`CTL-SLEEP-SYNC`:
   166 prod controls vs 0 real hits on Aider).
4. **Loop-context tracking is required for `ASYNC-002` / `LOOP-002` / `ALLOC-003`** —
   all three collapse into noise without a correct loop stack (including the
   comprehension-as-`LoopNode` obligation from ADR-0007).
5. **`print` needs an explicit family decision** before `LOOP-002` ships.
6. **Two rule families are frequency-unproven on this corpus** (pandas, `EXECUTOR-001`)
   — ship Info-only, or add a pandas-heavy corpus in W4.
7. **A production Python import resolver is a T2 concern** — W2 ships best-effort
   resolution and `ARCH-*` stays conservative/Info and *measured* until W4.

---

## 9. Limitations & accepted FP classes (the cranky truth)

- **The counter is not the analyzer.** It is a per-rule *shape* counter; it does not
  resolve names, types, or dataflow beyond the minimal local checks noted here.
- **`LOOP-007` is loop-agnostic in the counter.** It counts *list-literal membership
  comparisons anywhere* (e.g. `key in ["a", "b"]` inside a genexp) plus `x.index(...)`
  in a loop. The raw count is an **upper bound** on the eventual loop-scoped rule.
- **`ARCH-*` numbers are indicative, not decision-grade** (crude resolver; memoized
  longest-path is unsound under cycles). `ARCH-002`/`ARCH-003` are not measured at all
  (§6).
- **`LOOP-009` over-counts vs Ruff `PERF203`** (2.7–4.5×); Ruff's residual exclusions
  were not fully reverse-engineered. Treat as an upper estimate (§5).
- **`LOOP-001` counts one hit per level ≥ 3**, not one per maximal nest entry, so a
  depth-4 nest contributes two hits. Counts are tiny (7 / 15) and the disposition is
  Info/rare, but the number is an upper bound.
- **`ALLOC-001` counts f-string construction** in a loop by design, to expose the
  noise class; that is not the intended rule semantics (§3.1).
- **`ALLOC-003` receiver/scope handling is improved but still approximate:** it accepts
  `Name`/`Attribute`/`Subscript` receivers and treats a receiver reset in an *outer*
  loop frame as a control, but (a) "superlinearity" is never proven, and (b) the
  receiver is reduced to its **leftmost root**, so `self.results` and `self.tmp` share
  the root `self` — a `self.<other> = …` inside the loop can suppress a genuine
  `self.results.append(...)` as `CTL-APPEND-LOCAL`. This is an undisclosed-at-source
  undercount of the flagship; a full attribute-path key would fix it.
- **`+=` classification is honest but coarse:** bare `x += <name>` is recorded as
  *ambiguous* (`CTL-AMBIG-ACC`), not numeric; `x += <binop>` as `CTL-EXPR-ACC`. True
  `STR-001` may therefore be slightly under- or over-counted.
- **Nested-function traversal bias:** `_check_blocking`, `_check_complexity` and
  `_check_recursion` use `ast.walk` over a whole def, so a nested def's calls can
  contribute to its parent's counts. Small, but real.
- **Test/LOC classification is approximate:** `conftest.py` / `/testing/` are handled,
  but other test layouts are not; LOC counts newlines, so a file without a trailing
  newline is under-counted by one.
- **`CTL-LEN-IN-LOOP` also counts the `len()` inside `range(len(x))`** (the iterable
  is evaluated inside the pushed loop frame). A minor upward bias in that one control.
- **`ASYNC-002` counts `await` in `for`/`while`/`async for` bodies only** — not in
  async comprehensions. `CTL-AWAIT-NOLOOP` absorbs the rest.
- **Nested f-strings double-count `ALLOC-001`:** `visit_JoinedStr` recurses into an
  inner `JoinedStr`, so `f"{f'{x}'}"` contributes two hits. Rare in practice.
- **`PANDAS-002`'s `concat` match is name-based** (`concat(...)` on any receiver, not
  only pandas); `PANDAS-001`'s `iterrows`/`itertuples` is likewise name-based. Both
  can fire on non-pandas objects with those method names.
- **Pandas `.append`/`DataFrame.append` is syntactically indistinguishable from
  `list.append`**, so `PANDAS-002` measures only `concat`.
- **Frequency ≠ severity.** A high count can mean "idiomatic and harmless" (f-strings,
  `print`) as easily as "hot spot". Ranking is frequency × plausibility, and
  plausibility is the column that did the real work in §3/§7.
- **Both corpora are small-to-medium and AI-assisted but not AI-*only* in style**
  (Aider hand-edits; chunkhound is 100% AI). A larger, more diverse Python corpus is
  a W4/M2 concern, not a W0 blocker.

---

## 10. Reproduce

```bash
# 1. Fetch the pinned corpora (outside the repo; these tags resolve to the §2 commits)
git clone --depth 1 --branch v0.86.0  https://github.com/Aider-AI/aider.git      aider
git clone --depth 1 --branch v6.0.0   https://github.com/chunkhound/chunkhound.git chunkhound
git -C aider rev-parse HEAD        # a4be6ccd87ebaa59b361f3f028d116ce1761b626
git -C chunkhound rev-parse HEAD   # a8553ae644686d8ee55b94ee74ff560a38e6cc7d

# 2. Run the counter (stdlib only; writes canary_results.json next to itself)
python3 docs/evidence/python-canary/counter.py aider chunkhound

# 3. Ruff baseline (ruff==0.16.9; MIT/Apache-2.0 standalone wheel)
python3 -m venv venv && ./venv/bin/pip install ruff==0.16.9
./venv/bin/ruff check --isolated --no-cache --select PERF --statistics aider chunkhound
./venv/bin/ruff check --isolated --no-cache --select PERF --statistics --exclude tests aider chunkhound

# 4. Ruff PERF203 shape probe (the §5 divergence)
./venv/bin/ruff check --isolated --no-cache --select PERF203 --output-format concise <probe.py>
```

**Determinism note.** The counter walks files in sorted order, sorts every
in-graph traversal, and emits counts into sorted keys. Re-running over the same pinned
checkouts reproduces `canary_results.json` **byte-for-byte** — verified by running
twice and comparing (`sha256` of the two JSON outputs identical). The corpora are
pinned by commit, so the counts are reproducible without a lockfile. Interpreter at
capture: **Python 3.14.7**; ruff **0.16.9**.

---

## 11. Provenance

- **Plan:** `python-backend` (tracks `evidence-beta` **M3**), W0 canary.
- **Inputs:** `docs/adr/ADR-0007-python-tier-strategy.md` (rule-set + inference
  decisions), `docs/specs/evidence-beta-spec.md` §6 (evidence discipline).
- **Artifacts:** `docs/evidence/python-canary/counter.py` (sha256
  `00157c4c841d0b9caf11680e0acd967284b7812a1d14a0f77719364ee79292b5`),
  `docs/evidence/python-canary/canary_results.json`.
- **Tooling versions at capture:** Python 3.14.7, ruff 0.16.9.
- **No product code changed.** No `src/`, `Cargo.toml`, schema, or golden touched.
