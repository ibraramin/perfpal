# B4 real-code canary re-run — implemented `PERF-JS-*` rules

> **Scope.** Plan `js-ts` B4, closing `R-tb-016`: the B2-W0 canary measured *candidate frequency*
> before the rules existed; this run measures the **implemented analyzer** on the same pinned corpora.
> It reports hit counts split by path class, parse health, determinism, and a deterministic hand-verified
> sample per rule. It is **not** a corpus-wide precision claim — a bounded sample cannot replace labeled
> ground truth, and the synthetic 1.0/1.0 KPI remains synthetic-only.
>
> **Reproduce (from the repo root):** build the current tree, then scan each pinned checkout. The checkouts
> are the B2-W0 pins (`docs/evidence/js-canary/run.sh`): express `v5.1.0`
> (`cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4`), execa `v9.6.1`
> (`84e1f36b2c31a463e3efbd37c7505da9438a05c9`), gemini-cli `v0.65.0`
> (`6006c14a4decfe4b891c6263ad6ad5992e404b5b`). All three were verified clean before the run.

## 1. Method

1. **Build:** `CARGO_TARGET_DIR=/var/tmp/perfpal-b4-canary ~/.cargo/bin/cargo build --locked`.
2. **Scan:** `CARGO_TARGET_DIR=/var/tmp/perfpal-b4-canary ~/.cargo/bin/cargo run --locked --quiet -- <corpus> -o <out> --force`.
   Discovery follows the shipped policy (global directory skips; the JS/TS skip names `*.min.js`,
   `*.min.mjs`, `*.min.cjs`, `*.bundle.js`; the 2 MiB cap; `.d.*` skipped), not the counter's separate
   implementation. `.min.ts`/`.min.mts`/`.min.cts` are a known gap (`A-tb-009`).
3. **Split:** `files[].path_class` from `metrics.json`; the headline split is **production** vs
   **non-production** (test/example/bench/fixture/generated/vendor/unknown). The W0 `prod`/`test` split
   used a different path heuristic; the two are not the same. Execa matches exactly (219 production files);
   gemini-cli is 1,255 → 1,110; **express is 51 → 7** because the shipped classifier treats the `lib/`
   production set more narrowly (the W0 heuristic counted more files as production). The B4 numbers are
   the shipped output; do not read the W0 counts as a like-for-like baseline.
4. **Determinism:** a second scan of each corpus, comparing `charts/` byte-for-byte, the bundled schema,
   `metrics.json` after masking `scan.started_at`/`scan.duration_ms`, and `report.md` after masking the two
   provenance values.
5. **Hand verification:** every production finding is reviewed for the rare rules (`ALLOC-003`,
   `ARCH-001`, `TYPE-001`); the common rules use a deterministic sample (sort by
   `(path, start_line, start_col)` and spread the sample across the list; the agent report uses 15 per rule).
   A **TP** matches the rule's construct and mechanism; an **FP** is a listed accepted class, a missed gate,
   or a mis-attribution. No rule change was made from this evidence; any future change re-runs it.

## 2. Corpus and scans

| repo | tag/commit | scanned files | prod files | non-prod files | parse ok/partial/failed | prod code LOC |
|---|---|---|---|---|---|---|
| express | v5.1.0 `cd7d439` | 142 | 7 | 135 | 142/0/0 | 1,129 |
| execa | v9.6.1 `84e1f36` | 548 | 219 | 329 | 547/1/0 | 10,676 |
| gemini-cli | v0.65.0 `6006c14` | 2,297 | 1,110 | 1,187 | 2,068/229/0 | 211,340 |
| **total** | — | **2,987** | **1,336** | **1,651** | **2,757/230/0** | **223,145** |

Languages: express `js`; execa `js,ts`; gemini-cli `js,python,ts,tsx`. The Python findings in gemini-cli
are the existing `PERF-PY-*` family: **128 findings across 23 Python files** (49 Python files were
scanned; 14 `PERF-PY-*` rule ids), all non-production. **No JS rule fired on a Python file**, and the JS
rules fired across `js`/`ts`/`tsx` as intended.

**Path-class caveat (observed).** `path_class` catches `.test.`/`_test.`/`_spec.` but not `.spec.` or
hyphenated test directories, so at least 34 gemini-cli test-ish files are labelled `production` (e.g.
`packages/core/src/utils/fetch.spec.ts`, `packages/cli/test-setup.ts`, `test-utils/*`). Those files carry
98 findings. The production counts below follow the emitted `path_class`; the misclassification is a
separate cross-language follow-up, not a JS rule defect.

## 3. Per-rule production hits and sample verification

`prod/kLOC` uses the **scanned production code LOC** above (223.145 kLOC), not the W0 counter's
line denominator. `non-prod` includes test/example/bench.

| rule | express | execa | gemini | total prod | non-prod | prod/kLOC | review scope | observed result |
|---|--:|--:|--:|--:|--:|--:|---|---|
| `PERF-JS-ALLOC-001` | 0 | 3 | 500 | 503 | 238 | 2.254 | 15 sample | intended per-iteration literals; 4/15 are trivial/reset/fallback objects whose cost is likely negligible (the W0 "noise cannon" warning still applies) |
| `PERF-JS-ALLOC-003` | 0 | 0 | 20 | 20 | 4 | 0.090 | all 20 | 19 plausible: 17 `slice`-consumption (string/buffer trimming), 1 spread accumulator, 1 `concat` accumulator; **1 FP**: `[...visited, current]` in an error message, where `current` is loop-reassigned but the spread is not an accumulator |
| `PERF-JS-STR-001` | 0 | 0 | 35 | 35 | 49 | 0.157 | 15 sample | intended string/template accumulation in loops |
| `PERF-JS-LOOP-001` | 0 | 3 | 129 | 132 | 57 | 0.592 | 15 sample | intended nested loops (some inherent matrix/row work) |
| `PERF-JS-LOOP-002` | 0 | 2 | 46 | 48 | 69 | 0.215 | 15 sample | intended sync I/O in loops (some eval/test paths) |
| `PERF-JS-LOOP-007` | 2 | 4 | 189 | 195 | 51 | 0.874 | 15 sample | intended membership scans in loops; no small-collection false positives observed |
| `PERF-JS-ASYNC-001` | 0 | 5 | 364 | 369 | 141 | 1.654 | 15 sample | intended sequential `await` in loops; a few are `await Promise.all(...)` inside a loop / test paths |
| `PERF-JS-ASYNC-002` | 0 | 0 | 153 | 153 | 1,028 | 0.686 | 15 sample | intended sync-in-async shapes; prod sample is largely CLI one-shot/eval paths — **test skew is confirmed** (921 test + 107 bench non-prod) |
| `PERF-JS-CALL-001` | 3 | 1 | 176 | 180 | 32 | 0.807 | all 180 | **not merge-ready:** 150 are strict member/method-name collisions (`this.foo()` inside `foo()`, `new X().foo()` inside `foo()`) plus 2 wrapped `(this.storage as any)` member calls — all with `receiver_root = None`; 3/3 express are FPs, 1/1 execa is a TP; the remaining bare hits include truthiness-guarded recursion and same-name wrapper calls |
| `PERF-JS-ARCH-001` | 0 | 4 | 0 | 4 | 0 | 0.018 | all 4 | intended relative-import cycles (execa); gemini-cli 0 is the expected bare-alias blindness |
| `PERF-JS-TYPE-001` | 1 | 2 | 11 | 14 | 64 | 0.063 | all 14 | 4 plausible; **~10 FPs**: startup `delete process.env.*`, sanitizing a freshly built `mergedConfig` (6), and `delete` on a freshly copied `newPart` (2) — the rule's accepted-FP class does not cover fresh-object property removal |

**CALL-001 detail (the blocking finding).** The implementation requires `callee_kind == Direct`,
`callee_name == enclosing function name`, and `receiver_root.is_none()`. The first classification pass counts
150 strict member calls plus 2 wrapped `(this.storage as any)` member calls; both are the same FP class. The JS extractor's
`receiver_root_of` returns `None` for non-identifier receivers (`this`, `new X()`, wrapped expressions),
so a method call whose method name equals the enclosing method name passes the gate. Examples:
`this.router.handle(...)` inside `handle` (express), `this.scheduler.dispose()` inside `dispose`,
`new ListExtensionsCommand().execute(...)` inside `execute`, `this.commands.get(...)` inside `get`.
A second class is a same-name wrapper (`simulateClick: (…) => simulateClick(...)`) and truthiness-guarded
recursion (`if (definition.properties) return getDefaultsFromSchema(...)`) where no `ComparisonEvent`
precedes the call. **Fix direction (follow-up, not B4):** require the callee path to be exactly the bare
function name (no `.`/`[` receiver), and treat `if`/truthiness guards as visible base cases (or document
the narrower gate explicitly). Re-run this canary and re-baseline the JS/TS goldens after the fix.

## 4. Determinism

Each corpus scanned twice to separate dirs. Comparisons: `diff -r charts/`; `cmp` on the bundled
schema; `jq -S 'del(.scan.started_at,.scan.duration_ms)'` + `cmp` on `metrics.json`; `report.md` after
masking the `started`/`duration` values.

| repo | charts | schema | metrics (masked) | report.md (masked) | verdict |
|---|---|---|---|---|---|
| express | identical | identical | identical | identical | byte-identical |
| execa | identical | identical | identical | identical | byte-identical |
| gemini-cli | identical | identical | identical | identical | byte-identical |

## 5. Parse health

- express: 142/142 `ok`; no partial/failed.
- execa: 547 `ok`, 1 partial (`test/methods/template.js`, 22 error nodes); no failed.
- gemini-cli: 2,068 `ok`, 229 partial, 0 failed. The partial count matches the W0 record (220
  `f<typeof import("x")>()` type-query cases + 9 other test/helper spreads); the recovered trees still
  emitted usable imports/calls, and no JS finding was invented from a type position.

## 6. Findings and interpretation

1. **`PERF-JS-CALL-001` is not merge-ready.** The member-receiver gate is ineffective for `this`/`new`/wrapped
   receivers, producing 152 clear member-class FPs in 180 production hits (150 strict member calls + 2
   wrapped `(this.storage as any)` calls; plus some truthiness/wrapper bare FPs). This is
   the exact class the B2-W0 canary predicted (the counter's "any return" guard was broader); the real
   analyzer's narrower gate is too narrow to be useful. **Recommendation: block the merge gate until the
   gate is fixed and this canary is re-run.** The rule remains `PERF-JS-CALL-001` (Info), but the fix is a
   rule-code change outside B4's docs/label scope.
2. **`PERF-JS-TYPE-001` has a fresh-object FP class.** Deleting properties from a just-built/just-copied
   object is the rule's own suggested "rebuild" fix, so the hidden-class cost is not the issue; the rule
   should either be gated on a shared/receiver object or the class documented. Info severity; not a
   merge blocker, but the observed FP rate (~10/14) is high.
3. **`PERF-JS-ALLOC-001` remains noisy but gated.** The loop-invariance gate cut the W0 candidate count
   roughly in half (1,012 → 500 on gemini-cli), but trivial option bags/reset literals still fire at
   2.25/kLOC. It is Info/MEDIUM; the W0 "noise cannon" warning stands, and the rule is a candidate for a
   later triviality gate, not a B4 change.
4. **`PERF-JS-ALLOC-003` is mostly the intended `slice`-consumption shape.** 17/20 are string/buffer
   trimming in loops (the O(n²) copy risk is real when the loop count scales with the buffer), 2 are true
   spread/concat accumulators, and 1 is an error-message spread FP. The documented spread-root attribution
   limitation is visible in the `fromGraph.ts`/`modelConfigService.ts` shapes.
5. **`PERF-JS-ASYNC-002` is test-skewed, as predicted.** 153 production vs 1,028 non-production
   (921 test + 107 bench); the async-function gate works, but the raw count is not a production signal.
6. **`ARCH-001` is structurally blind in alias monorepos.** 4 execa cycles; 0 on gemini-cli (bare aliases
   are external by v1 design). This is the published limitation, not a rule defect.
7. **Cross-cutting follow-ups discovered by the canary:** (a) `path_class` does not recognize `.spec.` /
   hyphenated test dirs (98 gemini-cli findings mislabelled production); (b) `findings[].symbol` in
   `metrics.json` reports `<module>` for every JS/TS finding because the aggregator resolves symbols by
   first-hit over the file's synthetic `<module>` record (the ranked action list/key views resolve the
   narrowest function; full-detail rows still show `<module>`). Both are tracked in the track handoff /
   merge gate, not fixed in B4.

## 7. Limitations of this evidence

- The analyzer is syntactic; a "TP" here means the construct is present and the mechanism is plausible,
  not that the code is a confirmed bottleneck. Runtime profiling is the validation step.
- The sample is bounded and deterministic, not exhaustive; a per-rule sample rate is reported with each
  row. The rare rules and `CALL-001`/`TYPE-001` are fully reviewed.
- The corpora are three pinned JavaScript/TypeScript repositories, not a representative sample of all
  JS/TS code. Alias-based monorepos (gemini-cli) are structurally under-connected by the relative-only
  resolver, so `ARCH-001` is a lower bound there.
- The B2-W0 dispositions remain the frozen baseline; this run tests whether the implemented gates behave
  as intended on real code, and records the observed accepted-FP classes. It does not justify changing the
  catalog in B4.
