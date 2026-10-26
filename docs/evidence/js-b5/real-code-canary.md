# B5 real-code canary re-run — rule-gate fixes verified

> **Scope.** Session B5 of plan `js-ts`, closing the B4 merge-gate blocker `B-tb-004` / `R-tb-017`
> (`PERF-JS-CALL-001`) and the approved follow-ups `R-tb-018` (TYPE-001 fresh-object gate), `R-tb-019`
> (`findings[].symbol` narrowest record), `R-tb-020` (`path_class` `.spec.`/hyphenated test dirs),
> `A-tb-009` (`.min.ts`/`.min.mts`/`.min.cts` skip gap) and the ALLOC-003 spread-arm tightening.
> This is a **re-run of the B4 canary on the same pinned corpora** with the B5 binary; it measures the
> implemented analyzer, not candidate frequency. It is not a corpus-wide precision claim: the samples are
> bounded/deterministic and the synthetic 1.00/1.00 KPI remains synthetic-only.
>
> **Reproduce:** build the final B5 tree, then scan each pinned checkout twice. Pins are the B2-W0
> checkouts (`docs/evidence/js-canary/run.sh`): express `v5.1.0`
> (`cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4`), execa `v9.6.1`
> (`84e1f36b2c31a463e3efbd37c7505da9438a05c9`), gemini-cli `v0.65.0`
> (`6006c14a4decfe4b891c6263ad6ad5992e404b5b`). All three verified clean before the run.
> The pre-fix binary was B4's (`6d6bcdf` tree, `/var/tmp/perfpal-b4-canary/debug/perfpal`); the delta
> tables below are an exact finding-set comparison, not an estimate.

## 1. Corpus, classification and parse health

`path_class` now recognizes `.spec.` basenames and `test-*`/`*-test(s)`/`*-spec(s)` segments (`R-tb-020`).
The production denominator therefore **shrank** versus B4: execa's `test-d/` type-test tree (113 files) and
45 gemini-cli `.spec.`/`test-*` files were previously miscounted as production. Counts below are the shipped
B5 output; do not compare them to B4 as if the denominator were unchanged.

| repo | scanned | production | non-prod | parse ok/partial/failed | prod code LOC |
|---|---:|---:|---:|---|---:|
| express | 142 | 7 | 135 | 142/0/0 | 1,129 |
| execa | 548 | 106 | 442 | 547/1/0 | 5,234 |
| gemini-cli | 2,297 | 1,065 | 1,232 | 2,068/229/0 | 204,799 |
| **total** | **2,987** | **1,178** | **1,809** | **2,757/230/0** | **211,162** |

Non-prod split (gemini-cli): 1,055 test + 169 bench + 6 example + 2 vendor. The B4 caveat stands: 229
partial gemini-cli parses are the pinned TS grammar's `f<typeof import("x")>()` limitation (`R-tb-014`).

## 2. Per-rule production hits (final B5 tree)

`prod/kLOC` uses the B5 production LOC (211.162 kLOC). Full detail is in each corpus's `metrics.json`.

| rule | prod | total | prod/kLOC | B4 prod → B5 prod | observed result |
|---|---:|---:|---:|---:|---|
| `PERF-JS-ALLOC-001` | 485 | 741 | 2.30 | 503 → 485 | intended per-iteration literals; unchanged gate, 18 hits moved to non-prod by `path_class` |
| `PERF-JS-ALLOC-003` | 18 | 22 | 0.09 | 20 → 18 | 17 `slice`/`concat` consumption + 1 `concat` accumulator; the error-message spread FP and a known member-target mis-attribution are gone (§4) |
| `PERF-JS-STR-001` | 35 | 84 | 0.17 | 35 → 35 | intended string/template accumulation in loops |
| `PERF-JS-LOOP-001` | 124 | 189 | 0.59 | 132 → 124 | intended nested loops |
| `PERF-JS-LOOP-002` | 42 | 117 | 0.20 | 48 → 42 | intended sync I/O in loops |
| `PERF-JS-LOOP-007` | 191 | 246 | 0.90 | 195 → 191 | intended membership scans in loops |
| `PERF-JS-ASYNC-001` | 318 | 510 | 1.51 | 369 → 318 | intended sequential `await` in loops |
| `PERF-JS-ASYNC-002` | 134 | 1,181 | 0.63 | 153 → 134 | intended sync-in-async; test skew remains (1,047 non-prod) |
| `PERF-JS-CALL-001` | **2** | **2** | **0.01** | **180 → 2** | **both remaining hits are genuine recursion (100% of production hits); §3** |
| `PERF-JS-ARCH-001` | 4 | 4 | 0.02 | 4 → 4 | intended relative-import cycles (execa); alias blindness unchanged |
| `PERF-JS-TYPE-001` | **2** | **40** | **0.01** | **14 → 2** | **both remaining hits delete from a parameter (long-lived object); §5** |

## 3. `PERF-JS-CALL-001` — the merge blocker, closed

The B4 finding: 150 strict member calls + 2 wrapped member calls out of 180 production hits (~84%) because
`receiver_root_of` returns `None` for non-identifier roots. B5 adds three rule-local gates (no
`receiver_root_of` change, so ALLOC-003/LOOP-007 receiver semantics are untouched):

1. `callee_path` must be **exactly** the function name — a bare identifier, no `.`/`[` receiver. This rejects
   `this.foo()`, `new X().foo()`, and `(this.x as any).foo()`.
2. `!conditional_guarded` — an `if`/ternary/`catch`/`&&`/`||` ancestor is a visible base-case branch (the
   truthiness-guard FP class and the canary's retry recursion).
3. `FunctionRecord::name_is_lexical_binding` (new additive IR field) — a class/object **method** or
   property/field **arrow** derives its record name from a property key, which is not a lexical binding;
   a bare same-name call inside it resolves to an outer function and is **delegation, not recursion**.

**Exact delta** (old B4 binary vs B5 binary, finding-set diff on the same files):

| corpus | B4 total (prod) | B5 total (prod) | removed | removed class |
|---|---:|---:|---:|---|
| express | 3 (3) | 0 (0) | 3 | 3 member calls (`this.router.handle` in `handle`, …) |
| execa | 1 (1) | 0 (0) | 1 | bare recursion inside a `catch` (retry shape) — conditional-guard FN |
| gemini-cli | 208 (176) | 2 (2) | 206 | 164 member/wrapped, 10 if/for-guarded bare recursion, 32 non-prod property/param delegation |
| **total** | **212 (180)** | **2 (2)** | **210** | 167 member/wrapped + 43 bare (11 guarded production/target + 32 test delegation) |

> **152 vs 167 reconciliation.** B4's hand count classified **152** calls as the strict member class
> (150 strict member + 2 wrapped). The B5 exact diff classifies **167** removed hits as "non-bare callee
> text" mechanically (call-slice does not start with `name(`/`name<`). The 15-call difference is other
> non-bare shapes (parenthesized/wrapped callees and similar), not a different finding set: both numbers
> come from the same 212 → 2 delta, and 167 + 11 + 32 = 210.

**The two remaining production hits (both verified true positives):**

| path:line | function | shape |
|---|---|---|
| `packages/core/src/services/trackerService.ts:237` | `check` (const arrow inside `validateNoCircularDependencies`) | `await check(depId)` over a dependency graph; the arrow's name is a `const` binding |
| `packages/core/src/utils/process-utils.ts:74` | `getAllDescendants` (const arrow) | `await getAllDescendants(p)` over a process tree; the arrow's name is a `const` binding |

**Accepted false-negative class (documented, not fixed):** recursion whose only guard is an `if`/`catch`
ancestor is silenced. The canary observed 11 such hits (10 gemini-cli production + 1 execa): e.g.
`makeWritableRecursive` (recursive call inside `if (stats.isDirectory())` + `for`), `walk` (inside
`if (entry.isDirectory())`), `getDefaultsFromSchema` (truthiness guard), and execa's catch-retry. This is
the deliberate "narrower gate" tradeoff: the rule is Info severity, and the B4 FP rate made the previous
recall worthless. See `js-catalog.md` → published limitations.

## 4. `PERF-JS-ALLOC-003` — spread-arm tightening

The spread arm now additionally requires the array literal to be the RHS of an assignment to the attributed
root (`assigned_from_literal`), so a spread inside an arbitrary expression cannot fire.

- Removed 2 production hits: the known error-message FP
  (`throw new Error(\`… ${[...visited, current].join(' -> ')}\`)` in `modelConfigService.ts`, where
  `current` is loop-reassigned but the literal is not an accumulator) and `fromGraph.ts:57`, whose
  assignment target is `currentTurn.content.parts` while the rule attributed `currentTurn` — the evidence
  claim `'currentTurn = [...currentTurn, …]'` was already false (B4 documented this attribution limitation).
- Remaining production hits are the intended `slice`/`concat` accumulation shapes; the synthetic
  `alloc_003_spread.js` accumulator (`acc = [...acc, item]`) still fires (precision suite green).
- Member-target accumulators are now silent rather than mis-attributed; restoring them needs a
  member-root attribution field, recorded as a follow-up.

## 5. `PERF-JS-TYPE-001` — fresh-object gate

`UnaryEvent::operand_root_fresh` (new additive IR field) is true when the deleted member's receiver root was
bound in the same function, in source order, to an object literal, array literal or `new`. The rule skips
that case: a per-call object is the rule's own suggested rebuild, so no long-lived hidden class is polluted.

- Production 14 → **2**, both plausible TPs: `execa/lib/io/max-buffer.js:61` (`delete error.maxBufferInfo`
  on the error parameter) and `execa/lib/transform/run-async.js:12` (`delete state.currentIterable` on the
  state parameter).
- Suppressed production FPs: 6 `mcpUtils.ts` deletes from a freshly built `mergedConfig`, 2 `geminiChat.ts`
  deletes from a copied `newPart`, 1 express `clearCookie` delete from a per-call `opts` literal. Three
  further B4 "production" hits were `test-setup.ts` files that `path_class` now classifies as test.
- `process.env.*` deletes remain firing (residual accepted class; most are subscript-form and were already
  excluded). Total 78 → 40; the 38 suppressed are the fresh-object class.

## 6. Cross-cutting fixes

- **`findings[].symbol`** (`R-tb-019`): the aggregator now picks the narrowest containing function record
  (line containment, width, then id) instead of first-hit. In the JS/TS goldens, 10 of 11 findings resolve
  to the real function and exactly one per tree stays `<module>` — the module-level `ARCH-001` cycle anchor.
  Python goldens improved the same way (nested/module attribution); the other three trees are unchanged.
- **`path_class`** (`R-tb-020`): `.spec.`/`-spec.` basenames, `test-*` basenames, and
  `test-*`/`*-test(s)`/`*-spec(s)` segments now classify as `test`. Reclassified 113 execa `test-d/` files
  and 45 gemini-cli files; production totals 1,336 → 1,178. `src/test-data` keeps `fixture` (test DATA is
  more specific than test code) and production lookalikes (`latest`, `contest`, `attestation`, `spectrum`,
  `specification`, `prospect`) stay production (locked by unit tests).
- **`.min.ts`/`.min.mts`/`.min.cts`** (`A-tb-009`): added to the JS-family skip list (4 → 7 suffixes) for
  discovery and single-file targets; tests cover both paths.
- **p6 Atlas comparison**: `atlas-q2.html`/`atlas-weight.html` joined `GOLDEN_ARTIFACTS` (17 → 19) and the
  regenerator's provenance-free list, so all six trees' Atlas output is now byte-compared.

## 7. Determinism

Each corpus scanned twice into separate directories with the final B5 binary. Comparisons: `diff -r charts/`;
`cmp` on the bundled schema; `jq -S 'del(.scan.started_at,.scan.duration_ms)'` + `cmp` on `metrics.json`.

| repo | charts | schema | metrics (masked) | verdict |
|---|---|---|---|---|
| express | identical | identical | identical | byte-identical modulo provenance |
| execa | identical | identical | identical | byte-identical modulo provenance |
| gemini-cli | identical | identical | identical | byte-identical modulo provenance |

The plan DoD's `examples-js`/`examples-ts` double-runs also passed (`docs/evidence/js-b5/gate.txt`).

## 8. Limitations of this evidence

- Bounded deterministic samples, not labeled ground truth; TP/FP labels are construct+mechanism judgements
  by the implementer, with every CALL-001/TYPE-001 production hit and every ALLOC-003 hit reviewed in full.
- `path_class` moved ~158 files out of production, so B4 → B5 count deltas are **not** a pure rule effect;
  §2/§3/§5 separate the gate effect (finding-set diff on the same files) from the denominator change.
- Alias monorepos remain structurally under-connected (`ARCH-001` lower bound); no type information;
  static analysis is approximate — a runtime profile validates a candidate.
