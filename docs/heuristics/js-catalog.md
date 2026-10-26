# Perfpal — JavaScript / TypeScript Performance Heuristic Catalog (`PERF-JS-*`)

- **Status:** LANDED — plan `js-ts` B2-W2 (rules + catalog), extended by B3 (TS/TSX backend), closed by B4
  (docs/contract/canary) and re-verified by B5 (real-code gate fixes to `CALL-001`, `TYPE-001` and
  `ALLOC-003`; no catalog/schema/count change). Every entry below was transcribed **read-code-first** from the implemented rule
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
     enclosing loop reassigns **and whose assignment span contains the literal** (`assigned_from_literal`:
     the assignment's identifier target equals the attributed root) — `acc = [...acc, x]`.
- A snapshot call/literal whose root is never reassigned in the loop stays silent (the deliberate
  immutable-snapshot accepted FP class). `this.items = [...this.items, x]` is **not** detected
  (`assigned_within` matches plain identifier targets only).
- **Spread-arm assignment gate (B5):** the spread literal must be (part of) an assignment RHS to the
  attributed root. A spread nested in an arbitrary expression
  (`throw new Error(\`… ${[...visited, current].join(' -> ')}\`)`) is silent; a member-target accumulator
  (`currentTurn.content.parts = [...]`) is also silent because the assignment target is not a plain
  identifier, so it is no longer mis-attributed to `currentTurn`.
- **Spread-root attribution (residual):** when several loop-reassigned referenced names also satisfy the
  assignment gate, the rule reports the first matching name in the IR's sorted `referenced_names`, not
  necessarily the actual accumulator. Member/`this` accumulators remain undetected (a member-root
  attribution field is a recorded follow-up).
- Evidence: `receiver_root`, `callee` (call arm) **or** `kind="spread-accumulator"` (spread arm),
  `loop_depth`.
- B2-W0 context: 16 production hits vs 32 outside-loop mutation controls — loop-gating roughly halves the
  candidate set and is load-bearing. B5 canary: production 20 → 18 (total 24 → 22) — 17 `slice`/`concat`
  consumption + 1 `concat` accumulator; the synthetic `acc = [...acc, item]` precision fixture still fires
  (`docs/evidence/js-b5/real-code-canary.md` §4).

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
- Detects: a **bare direct** self-call that satisfies ALL gates (B5, `R-tb-017`):
  1. `callee_kind == Direct`, `callee_name == enclosing function name`, and `receiver_root` is `None`;
  2. `callee_path` is **EXACTLY** the bare function name — `this.f`, `new X().f` and `(this.x as any).f`
     are rejected because their callee text is not the bare identifier;
  3. the call is **not** `conditional_guarded` (no `if`/ternary/`catch`/`&&`/`||` ancestor — a visible
     base-case branch keeps it silent);
  4. the enclosing record has `name_is_lexical_binding == true`. A class/object method or a
     property/field arrow derives its name from a **property key**, not a binding, so a bare same-name
     call there resolves to an outer function — delegation, not recursion; and
  5. no `ComparisonEvent` in the same function starts before the call span (a visible base case).
- Why: each recursive frame grows the stack and repeats frame setup; convert to a loop with an accumulator
  or add an explicit base-case guard.
- Evidence: `function`, `loop_depth`.
- Accepted FP class: a same-name wrapper assignment (a local delegate named like the enclosing function
  shadows the recursion), plus a recursive tree walk with small depth.
- Accepted **false-negative** class (documented, B5): recursion whose only guard is an `if`/`catch`
  ancestor is silenced. The B5 canary observed 11 such sites (10 gemini-cli production + 1 execa
  catch-retry); the rule is Info severity and the pre-fix FP rate made the extra recall worthless.
- B2-W0 context: 102 of 108 production self-calls are guarded; only 6 are unguarded, so the visible-base-case
  predicate is load-bearing. The predicate is intentionally coarse (any earlier comparison counts).
- B5 canary: production 180 → 2 (total 212 → 2); both remaining hits are genuine recursion
  (`trackerService.ts` `check` const-arrow DFS and `process-utils.ts` `getAllDescendants` const-arrow tree
  walk) — 100% of production hits (`docs/evidence/js-b5/real-code-canary.md` §3).

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
- Detects: a `UnaryEvent` with `op == "delete"` and a `MemberAccess` operand that is **not** a fresh-object
  receiver (see gate). The subscript form (`delete obj["k"]`) and a bare identifier (`delete x`) are
  excluded.
- **Fresh-object gate (B5, `R-tb-018`):** silent when `UnaryEvent.operand_root_fresh` is true — the
  deleted member's receiver root was bound in the same function, in source order, to an object literal,
  an array literal or `new`. A per-call object is the rule's own suggested rebuild, so no long-lived
  hidden class is polluted. `this`, parameters, globals and unknown roots are never tracked as fresh and
  still fire.
- Why: deleting a property forces a V8 hidden-class → dictionary-mode transition that slows every later
  access. Setting the property to `undefined` or rebuilding the object is the candidate fix.
- Evidence: `property` (empty string when the property name is unavailable).
- Accepted FP class: intentional cache eviction (rare), plus a `process.env.*` delete — the residual
  accepted class (`delete process.env.X` is the real eviction idiom; most subscript forms were already
  excluded).
- B2-W0 context: 31 production hits vs 182 `.delete(...)` member-call controls (the control is broad: it does
  not prove the receiver is a `Map`/`Set`).
- B5 canary: production 14 → 2 (total 78 → 40); both remaining hits delete from a parameter — execa
  `delete error.maxBufferInfo` and `delete state.currentIterable`. Suppressed: 6 `mergedConfig` deletes,
  2 `newPart` deletes and 1 express per-call `opts` delete (plus test-file fresh deletes)
  (`docs/evidence/js-b5/real-code-canary.md` §5).

---

## 4. Real-code evidence (B5 re-run)

The B2-W0 canary measured candidate frequency; the **B4 re-run** measured the implemented rules and the
**B5 re-run closed the B4 merge-gate blockers** on the same pinned corpora (express `v5.1.0`, execa
`v9.6.1`, gemini-cli `v0.65.0`), split by `path_class`. Results and the exact commands live in
[`docs/evidence/js-b5/real-code-canary.md`](../evidence/js-b5/real-code-canary.md); the pre-fix B4 result
stays in [`docs/evidence/js-b4/real-code-canary.md`](../evidence/js-b4/real-code-canary.md). The
measurement is **hits + a hand-verified sample**, not a corpus-wide precision claim: a bounded sample cannot
replace labeled ground truth, and the synthetic 1.0/1.0 KPI remains synthetic-only (`R-tb-016`).

**Observed B5 result (2026-10-26).** The B5 tree fixes the three rule-gate defects; the catalog
(78 = 23 C + 11 JS + 22 PY + 22 RS), the 11-rule set, rule ids/severities, schema v1.8 and dependencies are
unchanged:

- **`PERF-JS-CALL-001` is merge-ready.** Production 180 → 2 (total 212 → 2); both remaining hits are
  genuine recursion. The 210 removed findings are 167 member/wrapped calls + 43 bare calls (11 guarded
  production/target + 32 test delegation). The accepted false-negative class is `if`/`catch`-guarded
  recursion (§3 of the evidence doc).
- **`PERF-JS-TYPE-001`** production 14 → 2, both parameter-object TPs; total 78 → 40. The fresh-object
  gate (§5 of the evidence doc) suppresses the 6 `mergedConfig` + 2 `newPart` + 1 express `opts` class;
  `process.env.*` deletes remain the residual accepted class.
- **`PERF-JS-ALLOC-003`** production 20 → 18 (total 24 → 22): 17 `slice`/`concat` consumption + 1
  `concat` accumulator; the error-message spread FP and the member-target mis-attribution are gone (§4).
  `ALLOC-001` remains noisy but gated (485 production hits; the W0 noise warning stands).
- **`PERF-JS-ASYNC-002`** remains test-skewed (134 production vs 1,047 non-production).
- **`PERF-JS-ARCH-001`** remains structurally blind in alias monorepos (0 on gemini-cli; 4 execa cycles).

Cross-cutting B5 fixes (evidence §6): `path_class` now recognizes `.spec.`/`-spec.` basenames, `test-*`
basenames and `test-*`/`*-test(s)`/`*-spec(s)` segments (production total 1,336 → 1,178; `src/test-data`
stays `fixture` and `latest`/`contest`/`attestation`/`spectrum`/`specification`/`prospect` stay
production); `findings[].symbol` resolves the **narrowest** containing function record (line containment,
width, then id), so 10/11 JS/TS golden findings resolve to a real function and the single `<module>` is the
module-level `ARCH-001` anchor (Python goldens improved the same way).

The re-run does not change the catalog. Any future rule change must re-run that evidence and update this
section; the dispositions in `docs/architecture/js-ts.md` §8 remain the frozen baseline.

---

## 5. Published limitations (must be reflected in the report/docs)

- No type information; no `node_modules`, `tsconfig` `paths`, `exports` maps, or re-export tracing.
- Relative specifiers only; bare specifiers, `node:` and URLs are external. Unresolved internal imports stay
  data-only (no finding).
- Declaration-only `.d.ts`/`.d.mts`/`.d.cts` files and minified bundles (`*.min.js`, `*.min.mjs`,
  `*.min.cjs`, `*.bundle.js`, `*.min.ts`, `*.min.mts`, `*.min.cts` — seven suffixes) are skipped for both
  discovery and single-file targets; an unlabeled minified file under 2 MiB is still parsed.
- JSX is parsed structurally, not semantically; CJS/ESM kind is not inferred; dynamic `require`/`import()`
  with computed specifiers is invisible.
- **Type land is never scored:** interfaces, type aliases, ambient `declare` blocks, overload signatures, and
  `typeof import("…")` type positions emit no events or records. A file may be `Partial` because of a type
  position while emitting no finding from it.
- **Type-only edges are erased:** `import type`/`import typeof`/`export type` produce no include edge, so
  ARCH-001 cannot report a type-only cycle (inline `{ type X, y }` specifiers keep the value edge).
- **Spread-arm attribution** requires the literal to be the RHS of an assignment to the attributed root
  (`assigned_from_literal`); when several candidates match, the first in sorted `referenced_names` is
  reported, not necessarily the spread operand. Member-target accumulators (`x.y = [...]`) are now silent
  rather than mis-attributed; member/`this` accumulators remain undetected (see ALLOC-003).
- **Fresh-object `delete`:** `TYPE-001` suppresses a delete whose receiver root is bound in the same
  function to an object/array literal or `new`; a parameter, global, `this` or unknown receiver still
  fires, and `process.env.*` deletes are the residual accepted class.
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
- B5 real-code re-run (gate fixes): [`docs/evidence/js-b5/real-code-canary.md`](../evidence/js-b5/real-code-canary.md)
- Spec / plan / ADR: [`docs/specs/js-ts.md`](../specs/js-ts.md) ·
  [`docs/plans/js-ts.md`](../plans/js-ts.md) · [`ADR-0011`](../adr/ADR-0011-js-ts-backend.md)
- Cross-language model: [`catalog-v1.md`](./catalog-v1.md) · [`rust-catalog.md`](./rust-catalog.md)
