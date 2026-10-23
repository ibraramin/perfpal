# W0 canary — JS/TS candidate rule-set frequency evidence (plan `js-ts`)

> **Scope.** `js-ts` **B2-W0** is a *pre-freeze canary*: before any `PERF-JS-*`
> rule is frozen into the catalog, measure whether each candidate actually fires
> on real, pinned JS/TS code — and whether its explicit **silent control** stays
> quiet. This document is the evidence record. The measurement changes no
> production code (`src/`, schema, goldens are untouched); the W0 change itself
> adds only the two grammar pins, the link tests, and this evidence, and updates
> the freeze record (`docs/architecture/js-ts.md` §8, `docs/specs/js-ts.md` §6).
>
> **Why it exists.** The M1 lesson: five C/Rust rules once produced ~89 % of
> self-scan noise. The beta framing is "audits what AI wrote", and JS/TS is the
> dominant AI-generated surface — so the JS catalog must not repeat that mistake.
> The 12-rule set in `docs/specs/js-ts.md` §6 was explicitly **not frozen** at B1;
> this canary is the freeze input, and the dispositions in §9 were **adopted** by
> the B2-W0 freeze. Precision beats breadth.
>
> **Reproduce:** `docs/evidence/js-canary/run.sh` (§10). The counter is
> `tools/js_canary.rs` (sha256
> `cd69c482beba06a9ef5f5f228ebe02e17fae4d22da6280771e18be4ee9783d68`); raw counts
> are `docs/evidence/js-canary/canary_results.json` (sha256
> `7efab4fe7f633ace40a341e64105b7cc1921e64f86e26719e116113f7622a4c2`). Every
> rule id **and** control id is pre-seeded with `0` per repo and in totals, so
> "counted zero times" is visible rather than ambiguous. No timestamps and no
> absolute paths enter the JSON.

---

## 1. Method

The counter is a single deterministic Rust example built against the **already
pinned** runtime and grammars — no new dependency:

| Crate | Pin | Parses |
|---|---|---|
| `tree-sitter` | `=0.26.13` | — |
| `tree-sitter-javascript` | `=0.25.0` | `.js`, `.jsx`, `.mjs`, `.cjs` (`LANGUAGE`) |
| `tree-sitter-typescript` | `=0.23.2` | `.ts`, `.mts`, `.cts` (`LANGUAGE_TYPESCRIPT`), `.tsx` (`LANGUAGE_TSX`) |

Invocation (one or more roots; one root emits a single-repo document, the driver
passes all three):

```bash
cargo run --locked --quiet --example js_canary -- <ROOT_DIR>...
```

Rules of the measurement:

1. **Discovery is deterministic.** Directory entries are sorted by file name;
   symlinks are skipped. Skipped directories: `node_modules`, `bower_components`,
   `dist`, `build`, `out`, `coverage`, `.git`, `.next`, `.nuxt`, `.svelte-kit`,
   `.cache` (all dot-directories are skipped, matching the production discovery
   policy). Skipped files: `*.min.js`, `*.min.mjs`, `*.min.cjs`, `*.bundle.js`,
   `*.d.ts`, `*.d.mts`, `*.d.cts`, and any JS/TS file larger than **2 MiB**.
2. **Parse health is counted, not hidden.** One parser per grammar; a file whose
   root reports `has_error()` increments `parse_failures`. Recovered partial
   trees are still analyzed (the alternative — silently dropping 10 % of one
   corpus — would bias the frequency numbers); the failure is reported
   separately in §2.
3. **Prod/test split.** A file is *test* if any path segment is
   `tests`/`test`/`__tests__`/`__mocks__`/`spec`/`e2e`, or the basename contains
   `.test.`/`.spec.`. Everything else is *prod*. Counts are reported separately;
   `parse_failures` is repo-wide.
4. **Rules count at the deciding construct; loop rules use the nearest enclosing
   loop body.** Loop headers are evaluated in the *enclosing* context (an
   iterable literal in a nested loop header is constructed once per outer
   iteration, so it is inside the outer loop). A nested function body resets loop
   context: a callback defined inside a loop is not executed per iteration
   (documented limitation, §8).
5. **Controls are disjoint from their rule.** A control never shares a construct
   with the rule it gates: literals outside loops, mutation shapes outside loops,
   numeric `+=` in loops, depth-1 functions, sync I/O outside loops, `for...in`
   over an object literal, membership calls outside loops, `await` outside loops,
   sync calls in non-async functions, acyclic modules, and `.delete(...)` calls.

**This is frequency evidence, not precision measurement.** The counter is a
syntactic pattern matcher, not the analyzer: its job is to kill implausible rules
before implementation. Precision is measured later (B2 KPI / benchmark ladder).

## 2. Corpus (repro packet)

Three pinned, permissively licensed, real-world repositories with different
shapes: an old callback-style framework (express), a modern ESM async library
(execa), and a large TypeScript monorepo (gemini-cli).

| Corpus | License | Tag | Pinned commit | Prod files | Test files | Prod lines | Test lines | Parse failures |
|---|---|---|---:|---:|---:|---:|---:|---:|
| express | MIT | `v5.1.0` | `cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4` | 51 | 91 | 4,883 | 16,281 | 0 |
| execa | MIT | `v9.6.1` | `84e1f36b2c31a463e3efbd37c7505da9438a05c9` | 219 | 329 | 13,224 | 19,685 | 1 |
| gemini-cli | Apache-2.0 | `v0.65.0` | `6006c14a4decfe4b891c6263ad6ad5992e404b5b` | 1,255 | 993 | 303,008 | 402,004 | 229 |
| **total** | — | — | — | **1,525** | **1,413** | **321,115** | **437,970** | **230** |

Discovery dropped no findings from `node_modules`/`dist`/`.d.ts`/minified files
by design; those paths are absent from the counts. Total failure rate is
**230 / 2,938 files (7.8 %)**, concentrated in gemini-cli (229 / 2,248 = 10.2 %;
execa 1 / 548 = 0.2 %; express 0).

**The 229 gemini-cli failures are one grammar limitation, not 229 broken files.**
220 of them begin at the same `>()` node: the vitest typing idiom
`importOriginal<typeof import('@scope/pkg')>()` (an instantiation-expression call
whose type argument is a `typeof import(...)` query). Minimal repro:

```ts
f<typeof import("x")>();   // ERROR on tree-sitter-typescript 0.23.2
f<typeof y>();             // parses
type X = typeof import("x"); // parses
```

The remaining 9 are spread across test helpers and one production file
(`packages/cli/src/config/settings.ts`, a spread/cast expression). The recovery
tree still yields usable imports/calls, so the files remain in the frequency
denominator — but see §7 and §8: this is an accepted grammar risk (`R-tb-014`),
not a counter bug. **Attribution note:** this 220/229 split is a triage
observation from the counter's parse-error report; the committed JSON stores only
the aggregate `parse_failures` per repo, not a per-file error list.

## 3. Candidate-rule frequency

`prod /kLOC` = hits per 1,000 **production** lines across the whole corpus
(denominator 321.115 kLOC). `total test` is the test-file hit count. Raw per-repo
prod hits are shown so a single repo cannot hide behind the aggregate.

| Candidate | express | execa | gemini-cli | total prod | prod /kLOC | total test |
|---|--:|--:|--:|--:|--:|--:|
| `PERF-JS-ALLOC-001` | 1 | 8 | 1,012 | 1,021 | **3.18** | 275 |
| `PERF-JS-ALLOC-003` | 0 | 0 | 16 | 16 | 0.05 | 1 |
| `PERF-JS-STR-001` | 0 | 0 | 73 | 73 | 0.23 | 11 |
| `PERF-JS-LOOP-001` | 0 | 3 | 162 | 165 | 0.51 | 18 |
| `PERF-JS-LOOP-002` | 0 | 2 | 110 | 112 | 0.35 | 8 |
| `PERF-JS-LOOP-003` | 3 | 0 | 21 | 24 | 0.07 | 3 |
| `PERF-JS-LOOP-007` | 4 | 2 | 171 | 177 | 0.55 | 8 |
| `PERF-JS-ASYNC-001` | 0 | 5 | 390 | 395 | **1.23** | 122 |
| `PERF-JS-ASYNC-002` | 0 | 0 | 270 | 270 | 0.84 | **882** |
| `PERF-JS-CALL-001` | 0 | 3 | 105 | 108 | 0.34 | 9 |
| `PERF-JS-ARCH-001` | 0 | 6 | 0 | 6 | 0.02 | 0 |
| `PERF-JS-TYPE-001` | 3 | 2 | 26 | 31 | 0.10 | 47 |

No rule is zero across the whole corpus, but two are near-zero
(`ALLOC-003` 16, `ARCH-001` 6) and several are zero on two of three repos. The
frequency spread is wide: **ALLOC-001 is 64× ALLOC-003** in production.

## 4. Silent controls

| Control | express | execa | gemini-cli | total prod | prod /kLOC | total test |
|---|--:|--:|--:|--:|--:|--:|
| `CTL-ALLOC-NOLOOP` | 152 | 4,649 | 16,293 | 21,094 | 65.69 | 61,493 |
| `CTL-ALLOC-NOLOOP-MUT` | 0 | 0 | 32 | 32 | 0.10 | 3 |
| `CTL-INT-ACC` | 0 | 1 | 18 | 19 | 0.06 | 7 |
| `CTL-STR-NOLOOP` | 3 | 0 | 156 | 159 | 0.50 | 16 |
| `CTL-DEPTH1` | 12 | 35 | 853 | 900 | 2.80 | 553 |
| `CTL-IO-NOLOOP` | 3 | 5 | 751 | 759 | 2.36 | 1,295 |
| `CTL-FORIN-OBJ` | 0 | 0 | 0 | **0** | 0.00 | 0 |
| `CTL-MEMBERSHIP-NOLOOP` | 9 | 21 | 827 | 857 | 2.67 | 1,108 |
| `CTL-AWAIT-NOLOOP` | 0 | 1,458 | 2,905 | 4,363 | 13.59 | 18,960 |
| `CTL-SYNC-SYNC-FN` | 3 | 7 | 452 | 462 | 1.44 | 421 |
| `CTL-ACYCLIC` | 51 | 213 | 1,255 | 1,519 | 4.73 | 1,413 |
| `CTL-DELETE-MAP` | 2 | 2 | 178 | 182 | 0.57 | 16 |

The loop gates are doing real work: `ALLOC-001` counts 1,021 in-loop literals
against 21,094 outside (20.7×), `LOOP-002` 112 vs 759 (6.8×), `LOOP-007` 177 vs
857 (4.8×), `AWAIT` 395 vs 4,363 (11.0×). `TYPE-001` (31) is 5.9× rarer than the
`.delete(...)` member-call control (182) that tracks it — the counter does not
prove the receiver is a Map/Set, so this is a broad control, not a type check.
`CTL-FORIN-OBJ` is **0 everywhere**: every one of the 24 production `for...in`
statements iterates an identifier, never an object literal (one test-only
array-literal case exists; see §5.2).

## 5. What the numbers say

### 5.1 `ALLOC-001` is the new noise cannon

At **3.18 hits/kLOC** it is the single largest signal in the corpus, and it counts
*every* object/array literal lexically inside a loop body — including `{ i }`,
`[]`, `{ ok: true }`, and short-lived option bags that V8's young generation and
escape analysis routinely absorb. The control shows literals are ubiquitous
(65.7/kLOC outside loops). This is structurally the Python `ALLOC-001` f-string
lesson: the raw number measures a shape, not a cost. **Do not ship it as
"literal in a loop"**; it needs a loop-invariance/escape gate (constructed value
independent of the iteration, or a non-trivial allocation), and even then should
start at Info/MEDIUM.

### 5.2 `LOOP-003` has zero identifiable targets

This is the clearest negative result. The candidate is "`for...in` over an
array". The counter splits every `for...in` by RHS shape (prod / test):

| RHS shape | prod | test |
|---|--:|--:|
| array literal | **0** | 1 |
| object literal | **0** | 0 |
| identifier or other | 24 | 2 |

All 24 production hits are `for (const k in identifier)` — statically
indistinguishable from the entirely legitimate "enumerate this object's keys"
use. The one array-literal occurrence is in a test. Shipping this rule without
type information means flagging 24 ambiguous identifier loops and zero provable
arrays; the named control (`CTL-FORIN-OBJ`) never fires because developers do not
write `for (const k in { ... })`. **CUT for v1**; revisit only if/when a type
tier can classify the RHS.

### 5.3 `CALL-001` is mostly guarded

108 production self-recursive calls, of which **102 (94 %) have a visible
`if`/`return` in the body** — i.e. the spec's own intended rule ("direct
recursion **with no visible base-case guard**") would fire only **6** times in
321 kLOC. The unconditional form is a tree-walk noisemaker; the guarded gate is
load-bearing. Note the heuristic is intentionally coarse: any `return` anywhere
counts as a guard, so the 6 "unguarded" hits are an upper bound for the eventual
rule.

### 5.4 `ASYNC-002` is test-skewed

270 production hits vs **882 test hits** — the largest test/prod inversion in the
canary. Test suites legitimately use sync filesystem helpers inside `async`
test callbacks; production code in these three repos mostly uses async APIs or
sync code in sync functions (`CTL-SYNC-SYNC-FN` 462). The rule remains valuable
(event-loop stall is a real bug class), but its raw count is not a prod signal —
and it must be async-function-gated (which the counter is; the control proves the
gate discriminates).

### 5.5 `ARCH-001` is structurally blind in monorepos

Only **6 modules** are in a cycle (all execa). The reason is visible in the graph
breakdown: gemini-cli has 1,255 prod modules but only **9 resolved internal
edges**, because its imports are package aliases (`@google/gemini-cli-core`) —
bare specifiers, which v1 deliberately treats as external. express has 22
internal edges, execa 372. So ARCH-001 works exactly where relative imports
dominate and is nearly invisible in alias-based monorepos. That is the published
v1 limitation (§8), not a reason to overstate the 6.

### 5.6 Rules with thin but plausible signal

`ALLOC-003` (16), `STR-001` (73), `TYPE-001` (31), `LOOP-001` (165) and
`LOOP-002` (112) all fire enough to justify an Info/MEDIUM rule, but not enough
to claim broad impact. `ALLOC-003`'s control is 2× its rule (32 vs 16 mutation
shapes **outside** loops), i.e. these accumulations are often deliberate
immutable snapshots; loop-gating roughly halves the candidate set and must be
kept.

## 6. Import-graph / ARCH evidence

| Metric | express | execa | gemini-cli |
|---|--:|--:|--:|
| Modules (prod / test) | 51 / 91 | 219 / 329 | 1,255 / 993 |
| Internal edges (prod / test) | 22 / 34 | 372 / 791 | 9 / 5 |
| Modules in a cycle (prod / test) | 0 / 0 | **6 / 0** | 0 / 0 |
| Modules acyclic (prod / test) | 51 / 91 | 213 / 329 | 1,255 / 993 |
| `await` inside `for...of` (prod / test) | 0 / 0 | 0 / 8 | 278 / 65 |
| Self-calls guarded / unguarded (prod) | 0 / 0 | 3 / 0 | 99 / 6 |
| `for...in` array / object / other (prod) | 0 / 0 / 3 | 0 / 0 / 0 | 0 / 0 / 21 |

Cycles are counted as modules in a strongly connected component of size > 1
(Kosaraju, iterative; self-imports excluded by SCC definition and by edge
construction). Edges are deduplicated; only `./`/`../` specifiers are resolved,
probing `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.mts`, `.cts`, then
`/index.<ext>`. `ARCH-001` is therefore **repo-wide, not a statement count**, and
the 6 is an upper bound on cycle membership, not a finding count.

## 7. Grammar / AST surprises (pinned grammars)

1. **`for...of` is a `for_in_statement`.** The JS and TS grammars model both
   `for...of` and `for...in` as one node kind; only the `operator` token (`of` vs
   `in`) distinguishes them. Any analyzer that matches on kind alone will
   conflate cross-iteration async with key enumeration.
2. **`typeof import(...)` as a call type argument fails to parse**
   (`tree-sitter-typescript 0.23.2`, ABI 14): 220 of 229 gemini-cli failures.
   Minimal repro in §2. This is a grammar-level limitation the backend must
   accept (or pin a newer grammar only through the ADR-0001 G5 review).
3. **JSX-embedded JS** parses with the JavaScript grammar for `.jsx`, while
   `.tsx` needs `LANGUAGE_TSX`; the two TypeScript grammars are distinct ABI-14
   surfaces, so rule code must stay node-kind-stable across both.
4. **`this.recurse()` is a `member_expression` whose object kind is `this`**, not
   an identifier — method recursion needs that explicit check.
5. **`delete` is a `unary_expression` with an `operator` token**; `delete
   obj["k"]` takes a `subscript_expression` argument and is *not* the candidate
   construct (`delete obj.prop` only).

## 8. Counter limitations (the cranky truth)

- **Not the analyzer.** No name resolution, no types, no dataflow. Every number
  is an upper bound on what the eventual rule can decide.
- **Function-boundary loop reset.** A callback defined inside a loop is analyzed
  with loop depth 0. This undercounts literals/awaits that are truly created per
  iteration *and* overcounts nothing — but it is a deliberate bias, not ground
  truth.
- **`ALLOC-001` cannot judge triviality.** `{}` and a 40-field options object
  count the same. There is no escape/invariance analysis. This is why §5.1
  recommends reframing before implementation.
- **`ALLOC-003` requires exact receiver text.** `x = x.concat(...)`,
  `x = x.slice(...)`, and `x = [...x, ...]` are matched by source-text equality;
  aliases (`const a = x; a = a.concat(...)`) and parenthesized receivers are
  missed. It is an undercount.
- **`STR-001` only counts literal RHS.** `s += chunk` and `s = s + chunk` are
  ignored; only `string`/`template_string` literals count, so the number is a
  floor for the broader accumulation shape and a clean signal for the narrow one.
- **`LOOP-002`/`ASYNC-002` use a hard-coded family.** Direct identifiers from the
  sync allowlist plus `*Sync` members rooted at `fs`/`crypto`/`child_process`.
  Destructured imports, aliases (`const f = fs.readFileSync`), and other blocking
  APIs (`Atomics.wait`, synchronous `net` calls) are invisible.
- **`ASYNC-002` is lexical.** "Async function" means an `async` token on the
  function node; promise-returning non-`async` functions and `.then` chains are
  treated as sync.
- **`CALL-001` is name-based.** It matches a bare self-name and, for methods,
  `this.<name>`; indirect recursion, `obj.method()` on another instance, and
  aliased function values are missed. "Guarded" = any `if`/`return` anywhere in
  the body (excluding nested functions), so it over-reports guards.
- **`ARCH-001` is relative-only.** No `tsconfig` paths, package aliases,
  `exports` maps, `.d.ts` resolution, dynamic `import()`, or `node_modules`.
  Unresolved internal imports are dropped (no edge), per the v1 policy; gemini-cli
  is the extreme case (9 edges over 1,255 modules).
- **Partial trees are analyzed.** Files with `has_error()` still contribute rule
  hits from the recovered tree; their parse failures are reported separately.
- **Test classification is heuristic** (path segments / basename conventions).
  A test helper outside those names counts as production.
- **Line counting** counts `\n` and adds one for a non-empty file without a
  trailing newline; it is not a `wc -l` clone.
- **Corpus diversity.** Three repos, one very large; express is old hand-written
  code, execa modern ESM, gemini-cli Google-authored/AI-assisted TypeScript. This
  is a frequency probe, not a representative sample of all JS.
- **Frequency ≠ severity.** `ASYNC-002`'s 882 test hits and `ALLOC-001`'s 1,021
  prod hits are as much "idiomatic and harmless" as "hot spot"; plausibility does
  the real work in §9.

## 9. Dispositions (adopted by the B2-W0 freeze)

| Candidate | Decision | One-line rationale |
|---|---|---|
| `PERF-JS-ALLOC-001` | **KEEP — reframed** | 3.18/kLOC is the biggest signal, but raw "literal in loop" includes benign `{}`/`[]`; ship only with a loop-invariance/escape gate, Info-first. |
| `PERF-JS-ALLOC-003` | **KEEP (Info)** | Only 16 prod hits (one repo), but the O(n²) shape is unambiguous and loop-gating halves it (control 32 outside). |
| `PERF-JS-STR-001` | **KEEP (MEDIUM)** | 73 prod hits; honest MEDIUM because V8 ropes absorb small accumulations, and 159 controls outside loops keep the gate honest. |
| `PERF-JS-LOOP-001` | **KEEP (Info)** | 165 prod depth-≥2 functions vs 900 depth-1 controls; rare enough to be Info, real enough to keep. |
| `PERF-JS-LOOP-002` | **KEEP (MEDIUM)** | 112 in-loop sync calls vs 759 outside; a 6.8× gate and a genuine event-loop tax. |
| `PERF-JS-LOOP-003` | **CUT (v1)** | Zero array-literal RHS; all 24 prod hits are identifier RHS and indistinguishable from legitimate object enumeration without types. |
| `PERF-JS-LOOP-007` | **KEEP — flagship** | 177 in-loop scans (0.55/kLOC) vs 857 outside; the clearest high-value, low-ambiguity loop rule. |
| `PERF-JS-ASYNC-001` | **KEEP — flagship** | 395 prod (1.23/kLOC), 70 % in `for...of`; sequential-latency pattern with a clean fix (`Promise.all`). |
| `PERF-JS-ASYNC-002` | **KEEP — flagship** | 270 prod but 882 tests — a real event-loop-stall rule with a strong async-function gate, not a prod-frequency claim. |
| `PERF-JS-CALL-001` | **KEEP (Info) — guard-gated** | 102/108 self-calls are guarded; only the 6 unguarded prod calls are the candidate, so the guard predicate is mandatory or CUT it. |
| `PERF-JS-ARCH-001` | **KEEP (Info)** | Cheap graph rule; 6 cyclic modules (execa) and near-blind in alias monorepos — publish the relative-only limitation. |
| `PERF-JS-TYPE-001` | **KEEP (Info)** | 31 prod `delete obj.prop` vs 182 `.delete(...)` member-call controls; low frequency, real hidden-class transition, clean silence. |

**Net result: 11 keep (1 reframed, 1 guard-gated), 1 cut (`LOOP-003`), 0 defer.**
No rule is zero across the corpus, so no candidate dies for total absence; the
cut is an *identifiability* failure, not a frequency failure. `ALLOC-003` and
`ARCH-001` are kept at near-zero frequency (Info) because they are cheap and
unambiguous; if the B2 precision ratchet cannot hold 1.0/1.0 on them, they are
the first two to demote to DEFER.

## 10. Reproduce

```bash
# Everything: clone/reuse pinned checkouts, run twice, cmp, publish counts.
docs/evidence/js-canary/run.sh

# Optional knobs
WORKDIR=/var/tmp/opencode/js-canary \
CARGO_TARGET_DIR=/var/tmp/perfpal-w0-b \
docs/evidence/js-canary/run.sh
```

The driver:

1. clones `--depth 1 --branch <tag>` into `${WORKDIR:-/var/tmp/opencode/js-canary}/<id>`
   (reuses an existing checkout);
2. verifies `git rev-parse HEAD` equals the §2 pin, **fails on mismatch**, and rejects a dirty checkout;
3. runs the counter twice to temp files and `cmp`s them
   (byte-identical determinism proof);
4. prints the corpus table, rule totals and control totals (python3 optional);
5. publishes run 1 to `docs/evidence/js-canary/canary_results.json` only after
   the determinism proof and summary step succeed.

Manual equivalent:

```bash
git clone --depth 1 --branch v5.1.0  https://github.com/expressjs/express.git      /var/tmp/opencode/js-canary/express
git clone --depth 1 --branch v9.6.1  https://github.com/sindresorhus/execa.git     /var/tmp/opencode/js-canary/execa
git clone --depth 1 --branch v0.65.0 https://github.com/google-gemini/gemini-cli.git /var/tmp/opencode/js-canary/gemini-cli
git -C /var/tmp/opencode/js-canary/express    rev-parse HEAD   # cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4
git -C /var/tmp/opencode/js-canary/execa      rev-parse HEAD   # 84e1f36b2c31a463e3efbd37c7505da9438a05c9
git -C /var/tmp/opencode/js-canary/gemini-cli rev-parse HEAD   # 6006c14a4decfe4b891c6263ad6ad5992e404b5b

CARGO_TARGET_DIR=/var/tmp/perfpal-w0-b cargo run --locked --quiet --example js_canary -- \
  /var/tmp/opencode/js-canary/express \
  /var/tmp/opencode/js-canary/execa \
  /var/tmp/opencode/js-canary/gemini-cli > /tmp/canary_a.json
# run again > /tmp/canary_b.json ; cmp /tmp/canary_a.json /tmp/canary_b.json
jq -e . docs/evidence/js-canary/canary_results.json
```

**Determinism.** Sorted discovery, sorted JSON keys (`serde_json` default
`BTreeMap`, `preserve_order` OFF), sorted graph traversal, no timestamps, no
absolute paths. Two consecutive runs are byte-identical (the driver enforces it
with `cmp`).

## 11. Evidence gates actually run

| Claim | Command | Result |
|---|---|---|
| Driver + corpus + determinism | `docs/evidence/js-canary/run.sh` | exit 0; `cmp` OK; corpus 2,938 files / 759,085 lines |
| Raw JSON is valid | `jq -e . docs/evidence/js-canary/canary_results.json` | exit 0 |
| Formatting | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-b ~/.cargo/bin/cargo fmt --all -- --check` | exit 0 |
| Lint (all targets) | `CARGO_TARGET_DIR=/var/tmp/perfpal-w0-b ~/.cargo/bin/cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| Counter artifact | `sha256sum tools/js_canary.rs` | `cd69c482beba06a9ef5f5f228ebe02e17fae4d22da6280771e18be4ee9783d68` |
| Results artifact | `sha256sum docs/evidence/js-canary/canary_results.json` | `7efab4fe7f633ace40a341e64105b7cc1921e64f86e26719e116113f7622a4c2` |
