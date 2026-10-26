# Perfpal — JavaScript / TypeScript Performance Heuristic Catalog (`PERF-JS-*`)

- **Status:** LANDED — plan `js-ts` B2-W2 (rules + catalog), extended by B3 (TS/TSX backend) and closed by B4
  (docs/contract/canary). Every entry below was transcribed **read-code-first** from the implemented rule
  functions (`src/analyzer/heuristics/js/mod.rs`), their parser-free IR tests
  (`src/analyzer/heuristics/js/tests.rs`), and the backend extraction seam
  (`src/scanner/backends/tree_sitter_javascript/`, parameterized by `Dialect` for TS/TSX).
- **Scope:** the shared JavaScript/TypeScript/TSX language tier. Findings are **optimization candidates**,
  never certainties. Rule ids mirror the C catalog's families ([catalog-v1.md](./catalog-v1.md)); the
  11-rule set was frozen by the B2-W0 canary (`docs/evidence/js-canary.md`) before implementation.
- **Tier:** **syntactic-only** throughout. There is no JS/TS semantic resolver, no type information, no
  `node_modules`/`tsconfig` path resolution, and no framework semantics. Every JS/TS record/event carries
  `resolver_tier = syntactic`; no rule may claim `semantic`. Static analysis is approximate — confirm
  candidates with a profiler/runtime measurement.
- **Registry:** `src/analyzer/mod.rs` partitions `js`/`ts`/`tsx` units to `RuleLanguage::Js`; `RuleLanguage::Ts`
  is reserved and empty in v1, so `PERF-TS-*` ships **zero** rules. The 11 catalog entries in
  `src/reporter/mod.rs` are the same 11 registry ids (`78 = 23 C + 11 JS + 22 PY + 22 RS`).
- **Downstream contract:** every finding carries `confidence` and `severity` (metrics.json schema);
  titles/evidence keys below are the exact strings the rules emit (evidence keys sort alphabetically in the
  serialized document). Every message cites **mechanism → fix → accepted FP class**.

---

## 1. Ranking & confidence model

The ranking axes and confidence model are those of [catalog-v1.md](./catalog-v1.md) §1. Every JS/TS rule is
gated on syntactic inference (`receiver_root`, loop spans, binding-free shape checks), so **all 11 rules cap
at MEDIUM**:

- **HIGH** — pure syntax with a low false-positive class. Not used by this family.
- **MEDIUM** — shape heuristic with a documented FP class; the normal default here.
- **LOW** — approximation involved. Not used by this family.

Central downgrade hooks (applied by `analyzer::apply_downgrade_hooks`, never per-rule): a finding whose
span overlaps a tree-sitter ERROR region is capped at MEDIUM with `error_node_overlap = true`; macro-origin
downgrades are not synthesized for JS/TS (the JS extractor records `macro_origin = false`). ParseHealth is
consulted before scoring; findings are never silently dropped.

---

## 2. Baseline stats (pillar 1a — always emitted, not heuristics)

Unchanged from [catalog-v1.md](./catalog-v1.md) §2, over JS/TS files: LOC per file, function/parameter/length
distributions, cyclomatic + cognitive complexity, allocation-site count, and include/module counts.
`FunctionRecord.alloc_site_count` counts `new` expressions plus object and array literals — the same
`AllocEvent` stream ALLOC-001/ALLOC-003 consume. Cost uses `function_cost_v2` unchanged; only loop/trip
extraction is JS-specific:

- Loop forms: C-style `for`, `for...of`, `for...in`, `while`, `do...while`; `await` does not change shape.
- Literal trip counts only for canonical counted loops `for (let i = 0; i < N; i++)` / `<=` with an integer
  literal `N` (clamped at 1_000_000). Everything else reports `unknown` (honest, not guessed). `for...of`
  over an array literal is deferred.
- `trip_count_source` = `literal` / `partial` / `unknown` exactly as the cross-language v1.7 contract;
  omitted when the function has no loops.

---

## 3. Rule set (11 rules — 5 warn / 6 info)

| ID | Family | Name | Default confidence | Severity |
|---|---|---|---|---|
| `PERF-JS-ALLOC-001` | ALLOC | Loop-invariant object/array literal inside loop | MEDIUM | info |
| `PERF-JS-ALLOC-003` | ALLOC | Superlinear `concat`/`slice`/spread accumulation in loop | MEDIUM | info |
| `PERF-JS-STR-001` | STR | String accumulation with `+=` inside loop | MEDIUM | warn |
| `PERF-JS-LOOP-001` | LOOP | Nested loop depth ≥ 2 | MEDIUM | info |
| `PERF-JS-LOOP-002` | LOOP | Synchronous blocking I/O inside loop | MEDIUM | warn |
| `PERF-JS-LOOP-007` | LOOP | Linear membership scan inside loop | MEDIUM | warn |
| `PERF-JS-ASYNC-001` | ASYNC | `await` inside a loop | MEDIUM | warn |
| `PERF-JS-ASYNC-002` | ASYNC | Synchronous blocking call inside `async` function | MEDIUM | warn |
| `PERF-JS-CALL-001` | CALL | Direct recursion without a visible base case | MEDIUM | info |
| `PERF-JS-ARCH-001` | ARCH | Import cycle | MEDIUM | info |
| `PERF-JS-TYPE-001` | TYPE | `delete` on an object property | MEDIUM | info |

`PERF-JS-LOOP-003` (`for...in` over an array) is **CUT for v1**: the B2-W0 canary found zero array-literal
RHS on 321 kLOC of production code (all 24 production `for...in` sites iterate identifiers, statically
indistinguishable from legitimate key enumeration without types). It has no function, constant, or catalog
entry. `PERF-TS-*` is reserved and empty: the canary produced no TS-only candidate, and TS/TSX share the
`PERF-JS-*` ids because the runtime mechanism is identical.

### Detail

**PERF-JS-ALLOC-001 — Loop-invariant object/array literal inside loop** `(syntactic)`
- Detects: an `AllocEvent` of kind `object` or `array` at `in_loop_depth >= 1`, **only after the mandatory
  loop-invariance gate**: silent when any `referenced_names` entry is an induction variable of the nearest
  enclosing loop, silent when an `AssignmentEvent` inside that same loop targets one of those names, and
  silent when no enclosing loop node contains the span.
- Why: V8 allocates the literal per iteration in the young generation; escape analysis rarely elides a
  per-iteration literal. Hoisting the invariant construction or reusing one object is the candidate fix.
- Evidence: `kind` (`object`/`array`), `referenced_names` (comma-joined), `loop_depth`.
- Accepted FP class: a literal created once for another reason but mutated per iteration.
- B2-W0 context: 1,021 production hits vs 21,094 outside-loop controls. The raw "literal in loop" shape is
  the corpus noise cannon; the mandatory invariance gate is what makes the rule shippable.

**PERF-JS-ALLOC-003 — Superlinear `concat`/`slice`/spread accumulation in loop** `(syntactic)`
- Detects two accumulator shapes, both in a loop:
  1. a `concat`/`slice` call whose non-empty `receiver_root` the same loop reassigns
     (`acc = acc.concat(x)`); or
  2. an array `AllocEvent` with `has_spread == true` whose `referenced_names` include a name the nearest
     enclosing loop reassigns (`acc = [...acc, x]`).
- A snapshot call/literal whose root is never reassigned in the loop stays silent (the deliberate
  immutable-snapshot accepted FP class). `this.items = [...this.items, x]` is **not** detected
  (`assigned_within` matches plain identifier targets only).
- **Spread-root attribution limitation:** the IR carries `referenced_names` only, not which name was the
  spread operand. The spread arm reports the **alphabetically first** loop-reassigned referenced name as
  `receiver_root` (the IR sorts the names), not necessarily the actual accumulator. This is documented and
  pinned by an IR test; it is not fixed in v1. `ALLOC-003` member/`this` accumulators are likewise
  undetected.
- Evidence: `receiver_root`, `callee` (call arm) **or** `kind="spread-accumulator"` (spread arm),
  `loop_depth`.
- B2-W0 context: 16 production hits vs 32 outside-loop mutation controls — loop-gating roughly halves the
  candidate set and is load-bearing.

**PERF-JS-STR-001 — String accumulation with `+=` inside loop** `(syntactic)`
- Detects two syntactic shapes in a loop, both gated on a **string/template-literal RHS** (the B2-W0
  canary predicate; numeric and identifier RHS are silent controls):
  1. `+=` with an identifier left operand and a string/template-literal right operand (`s += "x"`);
  2. `x = x + <string|template literal>` **only when the binary expression is the whole right-hand side**
     (same end position as the assignment). `s = f(s + "x")` is therefore silent — the binary is nested in
     a call, not the accumulator assignment.
- Silent on a canonical loop with `trip_count < 8` (the tiny-bounded-loop accepted FP class); an unknown
  trip fires.
- Why: the accumulation builds a cons-string chain; V8 ropes mitigate the per-pass cost, but flattening at
  first use is linear and repeated concatenation is quadratic. Collect parts and `join("")` once.
- Evidence: `target`, `loop_depth`.
- B2-W0 context: 73 production hits vs 159 outside-loop controls; only the narrow literal-RHS shape is
  admitted.

**PERF-JS-LOOP-001 — Nested loop depth ≥ 2** `(syntactic)`
- Detects: **one finding per function**, anchored at the earliest-span loop with `depth >= 2`. `max_depth`
  counts only loops whose span is contained in that anchored loop's span (it is not a function-wide maximum).
- Why: JS uses a depth floor of **2** (not the C catalog's 3): a two-deep nest already multiplies iteration
  superlinearly. Flattening, hoisting invariant work, or an early exit is the candidate fix.
- Evidence: `nest_depth` (anchored loop depth), `max_depth` (contained maximum).
- Accepted FP class: matrix work where `n²` is inherent.
- B2-W0 context: 165 production hits vs 900 depth-1 controls.

**PERF-JS-LOOP-002 — Synchronous blocking I/O inside loop** `(syntactic)`
- Detects: a call at `in_loop_depth >= 1` whose callee name is in the curated **23-name** synchronous Node
  API set: `readFileSync`, `writeFileSync`, `appendFileSync`, `copyFileSync`, `mkdirSync`, `readdirSync`,
  `statSync`, `lstatSync`, `existsSync`, `unlinkSync`, `renameSync`, `rmSync`, `cpSync`, `execSync`,
  `spawnSync`, `execFileSync`, `openSync`, `closeSync`, `readSync`, `writeSync`, `pbkdf2Sync`, `scryptSync`,
  `randomFillSync`.
- Why: one syscall/serialization per iteration blocks the single event-loop thread. Batch, use the async
  API (`fs/promises`, async `exec`/`spawn`), or make one call outside the loop.
- Evidence: `callee`, `loop_depth`.
- Accepted FP class: a startup-only loop.
- B2-W0 context: 112 production hits vs 759 outside-loop controls.

**PERF-JS-LOOP-007 — Linear membership scan inside loop** `(syntactic, flagship)`
- Detects: `indexOf`, `includes`, `find`, `findIndex`, `some`, `every`, `filter` at `in_loop_depth >= 1`
  with a non-empty `receiver_root`. Silent on a canonical loop with `trip_count < 16` (small-collection
  accepted FP class); an unknown trip fires.
- Why: the collection is scanned linearly on every iteration, making the loop `O(n²)`. Build a `Set`/`Map`
  once and do a keyed lookup.
- Evidence: `callee`, `receiver_root`, `loop_depth`.
- B2-W0 context: 177 production hits vs 857 outside-loop controls.

**PERF-JS-ASYNC-001 — `await` inside a loop** `(syntactic, flagship)`
- Detects: an `AwaitEvent` at `in_loop_depth >= 1`, **only inside a function with `is_async`**. One finding
  per await event.
- Why: awaiting each item sequentially serializes one microtask round-trip per iteration; collect the
  promises and `await Promise.all(items.map(...))`.
- Evidence: `loop_depth`.
- Accepted FP class: deliberately sequential work (rate limiting, ordering).
- B2-W0 context: 395 production hits (278 in `for...of`) vs 4,363 await controls outside loops.

**PERF-JS-ASYNC-002 — Synchronous blocking call inside `async` function** `(syntactic, flagship)`
- Detects: a known synchronous Node API (same 23-name set as LOOP-002) **anywhere** in a function with
  `is_async`; there is **no loop requirement** (any depth fires).
- Why: the blocking call stalls the event-loop thread for every concurrent task; use the async equivalent.
- Evidence: `callee`, `loop_depth`.
- Accepted FP class: a CLI single-shot path.
- B2-W0 context: 270 production hits vs **882 test hits** — the largest test/prod inversion in the canary.
  The rule is valuable but test-skewed; the async-function gate is mandatory and the control proves it
  discriminates.

**PERF-JS-CALL-001 — Direct recursion without a visible base case** `(syntactic)`
- Detects: a **bare direct** self-call (`callee_kind == Direct`, `callee_name == enclosing function name`,
  no receiver root) that fires **only when no comparison event in the function starts before the call
  span**. Any earlier comparison is accepted as a visible base case. **B4 canary caveat (`R-tb-017`):** the
  intended "member/`this` calls are excluded" gate is not effective — `receiver_root_of` returns `None`
  for non-identifier receivers, so `this.foo()` inside `foo()` and `new X().foo()` inside `foo()` still
  pass. 150 strict (+2 wrapped) of 180 production hits on the pinned corpora are this class; the fix is a merge-gate follow-up,
  not a catalog change.
- Why: each recursive frame grows the stack and repeats frame setup; convert to a loop with an accumulator
  or add an explicit base-case guard.
- Evidence: `function`, `loop_depth`.
- Accepted FP class: a recursive tree walk with small depth.
- B2-W0 context: 102 of 108 production self-calls are guarded; only 6 are unguarded, so the visible-base-case
  predicate is load-bearing. The predicate is intentionally coarse (any earlier comparison counts).

**PERF-JS-ARCH-001 — Import cycle** `(syntactic)`
- Detects: cycles over **resolved relative-import edges only** (`edge.resolved` with `to_path`), one finding
  per `break_cycles` broken edge, anchored at that pair's earliest retained directive span.
- Why: module-initialization order becomes observable and bundlers cannot tree-shake the cycle cleanly;
  break the lowest-sorted edge into a shared leaf module.
- Evidence: `kind="cycle"`, `coupled_with`.
- Accepted FP class: type-only import cycles (TypeScript erases them; the backend emits no edge for
  `import type`/`import typeof`/`export type`).
- B2-W0 context: 6 cyclic modules (all execa). The relative-only graph is structurally blind in
  alias-based monorepos — gemini-cli has 1,255 production modules but only 9 resolved internal edges because
  its imports are bare package aliases, which v1 treats as external. This is a published limitation, not a
  reason to overstate the 6.

**PERF-JS-TYPE-001 — `delete` on an object property** `(syntactic)`
- Detects: a `UnaryEvent` with `op == "delete"` and a `MemberAccess` operand. The subscript form
  (`delete obj["k"]`) and a bare identifier (`delete x`) are excluded.
- Why: deleting a property forces a V8 hidden-class → dictionary-mode transition that slows every later
  access. Setting the property to `undefined` or rebuilding the object is the candidate fix.
- Evidence: `property` (empty string when the property name is unavailable).
- Accepted FP class: intentional cache eviction (rare).
- B2-W0 context: 31 production hits vs 182 `.delete(...)` member-call controls (the control is broad: it does
  not prove the receiver is a `Map`/`Set`).

---

## 4. Real-code evidence (B4 re-run)

The B2-W0 canary measured candidate frequency; the **B4 re-run measures the implemented rules** on the same
pinned corpora (express `v5.1.0`, execa `v9.6.1`, gemini-cli `v0.65.0`), split by `path_class`, with a
deterministic hand-verified sample per rule. Results and the exact commands live in
[`docs/evidence/js-b4/real-code-canary.md`](../evidence/js-b4/real-code-canary.md). The measurement is
**hits + sample FP rate**, not a corpus-wide precision claim: a bounded sample cannot replace labeled
ground truth, and the synthetic 1.0/1.0 KPI remains synthetic-only (`R-tb-016`).

**Observed B4 result (2026-10-26).** All 11 rules fired across the pinned corpora; the canary's main
finding is a rule-gate defect, not a catalog change:

- **`PERF-JS-CALL-001` is not merge-ready.** 180 production hits; 150 are member/method-name collisions
  (`this.foo()` inside `foo()`, `new X().foo()` inside `foo()`) because the JS extractor's
  `receiver_root_of` returns `None` for non-identifier receivers, so the `receiver_root.is_some()` gate
  never sees them. 3/3 express and the sampled gemini-cli member calls are FPs; 1/1 execa is a genuine
  unguarded self-call. The fix belongs in the rule gate (`callee_path == function name` and/or a
  `receiver_root_of` fix for `this`/`new`) and is a **merge-gate follow-up**, not a B4 change.
- **`PERF-JS-TYPE-001`** has a fresh-object FP class: deleting a property from a just-created/copied object
  (the rule's own suggested rebuild) is ~10 of 14 production hits. Info severity; document or gate later.
- **`PERF-JS-ALLOC-001`** remains noisy but gated: 503 production hits, mostly real per-iteration
  literals, with trivial option bags/reset literals in the sample. Info/MEDIUM; the W0 noise warning
  stands.
- **`PERF-JS-ALLOC-003`** is mostly the intended `slice`-consumption shape (17/20); one error-message
  spread is an FP. The spread-root attribution limitation is visible.
- **`PERF-JS-ASYNC-002`** is test-skewed as predicted (153 production vs 1,028 non-production).
- **`PERF-JS-ARCH-001`** remains structurally blind in alias monorepos (0 on gemini-cli; 4 execa cycles).

Cross-cutting canary findings (tracked in the handoff, not rule defects): `path_class` does not recognize
`.spec.` / hyphenated test dirs, and `findings[].symbol` in `metrics.json` reports `<module>` for every
JS/TS finding because the aggregator resolves symbols by first-hit over the synthetic `<module>` record.
The ranked action list/key views resolve the narrowest function; full-detail rows still show `<module>`
until `A-tb-008`.

The re-run does not change the catalog. Any future rule change must re-run that evidence and update this
section; the dispositions in `docs/architecture/js-ts.md` §8 remain the frozen baseline.

---

## 5. Published limitations (must be reflected in the report/docs)

- No type information; no `node_modules`, `tsconfig` `paths`, `exports` maps, or re-export tracing.
- Relative specifiers only; bare specifiers, `node:` and URLs are external. Unresolved internal imports stay
  data-only (no finding).
- Declaration-only `.d.ts`/`.d.mts`/`.d.cts` files and minified bundles (`*.min.js`, `*.min.mjs`,
  `*.min.cjs`, `*.bundle.js`) are skipped; an unlabeled minified file under 2 MiB is still parsed.
  `.min.ts`/`.min.mts`/`.min.cts` are **not** in the skip list (known gap, `A-tb-009`).
- JSX is parsed structurally, not semantically; CJS/ESM kind is not inferred; dynamic `require`/`import()`
  with computed specifiers is invisible.
- **Type land is never scored:** interfaces, type aliases, ambient `declare` blocks, overload signatures, and
  `typeof import("…")` type positions emit no events or records. A file may be `Partial` because of a type
  position while emitting no finding from it.
- **Type-only edges are erased:** `import type`/`import typeof`/`export type` produce no include edge, so
  ARCH-001 cannot report a type-only cycle (inline `{ type X, y }` specifiers keep the value edge).
- **Spread-root attribution** reports the alphabetically first loop-reassigned referenced name, not the
  spread operand (see ALLOC-003); member/`this` accumulators are undetected.
- **Class bodies are not event owners:** methods and arrow-function fields are their own records; non-arrow
  field-initializer expressions and `class_static_block` bodies are not event-scanned.
- **Pinned TS grammar limitation:** `f<typeof import("x")>()` fails to parse on
  `tree-sitter-typescript 0.23.2` (`R-tb-014`); the recovered tree is still analyzed honestly.
- Static analysis is approximate — profile to confirm.

---

## 6. References

- Frozen dispositions: [`docs/architecture/js-ts.md`](../architecture/js-ts.md) §8
- Node-kind → IR map and implementation record: [`docs/architecture/js-ts.md`](../architecture/js-ts.md) §6,
  §13, §16
- B2-W0 frequency evidence: [`docs/evidence/js-canary.md`](../evidence/js-canary.md) ·
  `docs/evidence/js-canary/run.sh` · `tools/js_canary.rs`
- B4 real-code re-run: [`docs/evidence/js-b4/real-code-canary.md`](../evidence/js-b4/real-code-canary.md)
- Spec / plan / ADR: [`docs/specs/js-ts.md`](../specs/js-ts.md) ·
  [`docs/plans/js-ts.md`](../plans/js-ts.md) · [`ADR-0011`](../adr/ADR-0011-js-ts-backend.md)
- Cross-language model: [`catalog-v1.md`](./catalog-v1.md) · [`rust-catalog.md`](./rust-catalog.md)
