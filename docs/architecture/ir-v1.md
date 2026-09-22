# Perfpal — Internal Representation (IR) v1

- **Status:** v1 draft — plan `mvp-architecture`, step 3. Implements the parser-agnostic IR commitment of ADR-0001 (Accepted).
- **Audience:** scanner/analyzer implementers. The IR is *internal*; the external contract is `metrics.json` (`docs/schema/metrics-json-v1.md`).
- **Hard rule (ADR-0001 guardrail 1):** heuristics import **only** this IR module — never tree-sitter node types, never libclang cursors. CI must run the full heuristic suite against fixture IR with no parser bound (guardrail 2).

---

## 1. Design goals

1. **Parser-agnostic.** Entity identity and shapes never reference engine-specific keys (no clang USRs, no tree-sitter node kinds). A future semantic resolver upgrades confidence/tier; it never changes identity or schema.
2. **Just rich enough.** v1 carries exactly what Tier 1 + Tier 2 detection (catalog v1) and pillar-1b aggregation require — an *event stream*, not a syntax tree. Anything else is an extension point, not a field.
3. **Tier-aware.** Every record is tagged with the resolver tier that produced it: `syntactic` (MVP) → `semantic` → `dynamic`.
4. **Honest about parse quality.** ERROR/partial-parse status is first-class; heuristics MUST consult it before scoring (ADR-0001: unparseable regions are reported, never analyzed as fiction).

## 2. Conventions

| Convention | Value |
|---|---|
| Paths | Repo-relative, POSIX `/` separators, UTF-8 |
| Spans | `{start_line, start_col, end_line, end_col}` — **1-based lines, 1-based columns counted in bytes; end_line/end_col are INCLUSIVE (last byte of the match)**. Scanner converts from engine-native 0-based, end-exclusive points at the IR boundary (+1 on entry, −1 on exit). Inclusive ends make span-intersection checks (error_region overlap) unambiguous. |
| Identity key | `(path, name, span)` per ADR-0001; rendered as `path#name#start_line:start_col` |
| Anonymous entities | Empty-string name; span disambiguates |
| Every **event** carries | `resolver_tier` ∈ `syntactic \| semantic \| dynamic` and `macro_origin` (see §3.3 header). File-level records (§3.1) and FunctionRecords (§3.2) are scan-level facts produced by the MVP syntactic pass; their tier is implied `syntactic` until a semantic resolver enriches them, at which point enriched records carry explicit tags. v1 emits only `syntactic`. |
| Macro origin | Boolean `macro_origin` on any record whose span originates from a macro expansion (detected without expansion via the curated macro-pattern recognizer library, ADR-0001). Downstream: caps finding confidence at MEDIUM + sets `macro_suspect=true`. |

## 3. Record model

### 3.1 File-level records

```
FileRecord {
  path, language            // "c" in v1
  loc: {code, comment, blank}
  function_count, global_var_count
  include_counts: {quoted, angle}
}

ParseHealth {               // one per file; consulted before ANY scoring
  status: ok | partial | failed
  error_regions: [span…]    // tree-sitter ERROR / unrecovered regions
  error_node_count: int
  resolver?: absent | ok | failed
                            // semantic-resolver outcome (ADDITIVE v1.1,
                            // plan `semantic-resolver`): absent = resolver
                            // disabled/unavailable; ok = libclang parsed the
                            // TU; failed = TU-level libclang failure. Optional
                            // on the wire (`Option` + `#[serde(default)]`,
                            // omitted when None) — never deletes/degrades
                            // syntactic results (modules.md §4.3)
}

IncludeEdge {
  from_path, to_path        // to_path = null when unresolvable (finding, not failure)
  form: quoted | angle      // angle ⇒ "external" bucket, excluded from internal coupling
  resolved: bool
  is_mod: bool              // ADDITIVE (default false, plan `rust-backend` R3):
                            // true = a Rust `mod foo;` DECLARATION edge; false = a
                            // `use` import edge or a C `#include`. Lets a Rust
                            // module-graph heuristic consume the `use` import graph
                            // only — `mod` declaration edges are not imports and
                            // create parent↔child asymmetry (see PERF-RS-ARCH-001).
  span
}

GlobalVar {
  name, path, span
  storage: none | static | extern
}

StructDef {
  name, path, span          // tag name; "" if anonymous
  kind: struct | union | enum
                            // ADDITIVE (default struct, plan `rust-backend` R3):
                            // only `struct` is a layout-member aggregate; `union`
                            // reuses the member model; `enum` carries NO members
                            // (variants are not layout fields) — a size/padding
                            // heuristic must check `kind == struct` before summing
                            // members. The C backend always emits `struct`.
  members: [{name, type_text, array_dim_literal, is_pointer}]
                            // array_dim_literal = int|null (approximate size model, TYPE-001/002)
                            // is_pointer = bool (additive, default false): declarator carries a
                            // pointer star; stars never reach type_text, so the size model needs
                            // the explicit flag (TYPE-001/002)
}

MacroEvent {
  kind: definition | use | recognized_pattern
  name, path, span
  pattern_id                // curated recognizer id, when kind = recognized_pattern
}
```

### 3.2 Function record (structural facts = baseline stats)

```
FunctionRecord {
  id                        // "src/foo.c#process_items#42:5"
  name, path, span
  param_count
  param_types: [string]     // raw type text per parameter (approximate, no layout);
                            // feeds TYPE-001 "struct passed by value" detection
  cyclomatic_complexity     // branch-count approximation on AST (catalog §2)
  cognitive_complexity      // nesting-weighted variant
  max_loop_depth
  alloc_site_count          // malloc-family call sites in body
  length_lines
}
```

Function length is deliberately a stat, not a finding (catalog §2 note); it feeds ARCH-002 aggregation.

### 3.3 Events (attached to their enclosing FunctionRecord)

All events carry: `span`, `in_loop_depth` (0 = not in loop), `is_innermost` (inside the deepest enclosing loop), `resolver_tier`, `macro_origin`.

```
CallEvent {
  callee_name               // textual name at call site (final segment)
  callee_kind: direct | indirect | unknown
                            // direct = named function; indirect = called through a
                            // non-function-name expression (function-pointer candidate, LOOP-006);
                            // unknown = unresolvable shape
  callee_path?              // ADDITIVE (plan `rust-backend` R3): FULL callee path text at the
                            // call site (e.g. `Vec::with_capacity`, `Box::new`, `x.clone`,
                            // `std::fs::read`); `Option` + serde default, omitted when None.
                            // The C backend leaves it None (no path concept); the Rust backend
                            // populates it — the API-name rule families match curated names
                            // against this path where a qualified spelling is needed.
  is_macro: bool            // ADDITIVE (plan `rust-backend` R3, default false): true only for
                            // Rust macro-invocation call events (the approved macro-as-call
                            // surface); a macro name is matched only when this is true, so a
                            // plain function named `vec`/`format` is never counted.
  args: [Shape…]            // ALLOC family (callee ∈ alloc set), MEM-001 sizeof shapes,
                            // MEM-002 size-arg literalness, LOOP-002/003 callee membership
  statement_role: primary | nested | unknown
                            // position of the call within the loop body: the loop's primary
                            // statement (LOOP-003 fidelity) vs buried in a branch/expression
  conditional_guarded: bool // call sits inside a conditional branch within the loop (LOOP-002 severity hint)
}

AssignmentEvent {
  target: Shape             // identifier | member-chain | dereference
  scope_class: local | param | global | unknown
                            // global classification is NAME-BASED against GlobalVar records (LOOP-005, MEDIUM confidence)
}

BinaryOpEvent {
  op                        // "+ - * / % …"
  left, right: Shape        // right.literal-ness decides LOOP-004 (variable divisor)
}

ComparisonEvent {
  left, right: Shape        // subscript-with-index vs outer-loop induction var ⇒ LOOP-007
}

AccessChainEvent {
  root: string              // base identifier
  elements: [Shape…]        // member/subscript sequence; chain depth ≥ 3 ⇒ LOOP-008
}

LoopNode {
  kind: for | while | do | loop
                            // `loop` is Rust's unconditional `loop_expression`
                            // (ADDITIVE variant, plan `rust-backend` R1); the C
                            // backend never emits it
  depth                     // nesting depth (LOOP-001 threshold, default ≥ 3)
  induction_vars: [string]  // syntactic best-effort for-loop induction identifiers (feeds LOOP-007)
  span                      // loops are records too: they anchor nesting chains
}
```

### 3.4 Shape descriptor (shared expression vocabulary)

```
Shape {
  kind: literal | identifier | call | subscript | member_access | unary | binary | sizeof | other
  name?                     // identifier text or callee name
  value?                    // literal value when kind = literal
  index_ident?              // subscript index identifier when kind = subscript
  depth?                    // chain depth when kind = member_access
  sizeof_operand?           // raw source text of a sizeof operand, parens stripped as written
                            // (`sizeof(p)` ⇒ "p", `sizeof(struct S)` ⇒ "struct S") — additive,
                            // MEM-001 input; absent on non-sizeof shapes
}
```

Additive-change note (P5 Wave A): `Shape.sizeof_operand` and
`StructMember.is_pointer` are ADDITIVE v1 fields — both optional/defaulted
on the wire (`Option` / `#[serde(default)]`), so documents written before
them deserialize unchanged. They add detection fidelity (MEM-001 operand
classification; TYPE-001/002 pointer-member sizing) without touching any
existing field's meaning; the minor-bump schema policy (open rule_id
pattern) covers them.

Additive-change note (semantic-resolver Wave A, 2026-09-02):
`ParseHealth.resolver` is an ADDITIVE v1 field in the same pattern —
`Option<ResolverHealth>` with `#[serde(default, skip_serializing_if)]`, so
pre-resolver documents deserialize unchanged and `None` is OMITTED on the
wire (omit-don't-null). It is pure honesty reporting from the resolver seam
(modules.md §4.3): heuristic-visible state, identity keys `(path, name,
span)`, and record shapes are untouched. Schema surface: optional
`parse_health.resolver` in metrics.schema.v1.1.json (minor bump,
metrics-json-v1.md §5).

Additive-change note (semantic-resolver Wave B2, 2026-09-05):
`CallSemantic.fopened_handle` and `CallSemantic.literal_args` are ADDITIVE
v1 fields in the same pattern — `bool` / `Vec<(u32, String)>` with
`#[serde(default, skip_serializing_if)]`, so documents written before them
deserialize unchanged and absent facts stay off the wire
(omit-don't-null). They are IR-internal enrichment proven by the resolver
seam (fopen-assigned `FILE *` handles; single-string-literal format
initializers): identity keys `(path, name, span)` and record shapes are
untouched, and no metrics.json DTO changes (evidence keys are open maps;
findings carry what output needs). Invariant: the resolver sets
`Some(CallSemantic)` only when at least one fact is proven — never an
empty struct — which the analyzer's `apply_upgrade_hooks` gate depends on.

This one vocabulary is what keeps the IR event-stream-shaped instead of AST-shaped: heuristics pattern-match over typed events + shapes, never over parser node types.

Additive-change note (rust-backend R1, 2026-09-22):
`LoopKind::Loop` is an ADDITIVE enum variant — Rust's unconditional
`loop {}` expression, serialized `"loop"` by the existing
`#[serde(rename_all = "lowercase")]`. It follows the same
backwards-compatible pattern as the additive fields above: C-produced
documents never contain it, pre-Rust consumers that do not match on it are
unaffected, and no existing variant's meaning changes. It is the only IR
change the Rust scanner backend (R1) makes; the rest of the Rust scope is a
new parser backend plus extension-based language dispatch, both behind the
unchanged record model of §3.

Additive-change note (rust-backend R3, 2026-09-22):
`StructKind` + `StructDef.kind`, `CallEvent.callee_path`,
`CallEvent.is_macro` and `IncludeEdge.is_mod` are ADDITIVE v1 surface in the
same pattern as the fields above. `StructKind` is a serde-defaulted enum
(`Struct` = the C backend's only kind, `Union`/`Enum` produced by the Rust
backend); `callee_path` is `Option<String>` with `#[serde(default,
skip_serializing_if)]` (omit-don't-null; the C backend leaves it `None`);
`is_macro` is `bool` with `#[serde(default)]` (default false = a real call);
`is_mod` is `bool` with `#[serde(default)]` (default false; the Rust backend
sets it on `mod foo;` declaration edges so module-graph heuristics can consume
the `use` import graph only — see `PERF-RS-ARCH-001`).
Two further points:

- **`RUST_ALLOC_FAMILY`** (`src/ir/mod.rs`) is a shared producer/consumer
  const — a `RustAllocFamily` of 12 qualified constructor paths, 5 method
  names and 2 macro names, with `matches_site(callee_path, callee_name,
  is_macro)` and `all()`. The Rust scanner backend uses it to compute
  `FunctionRecord.alloc_site_count` and to recognize alloc-wrapper macro
  bodies; the Rust ALLOC heuristics use it to match call sites — one constant,
  so stats and findings count the SAME sites. `clone` is deliberately
  excluded (noise control; accepted false negative).
- **IR-only, no schema bump.** None of this surface is serialized into
  `metrics.json`: it is internal enrichment consumed by the analyzer, not a
  document DTO. Identity keys `(path, name, span)` and record shapes are
  untouched, and the serde-default pattern keeps documents written before it
  deserializing unchanged.

## 4. Coverage check — every catalog heuristic maps to IR records

| Heuristic(s) | Consumes |
|---|---|
| ALLOC-001/002/003 | CallEvent (alloc/free/realloc sets) × LoopNode ancestry via `in_loop_depth`; ALLOC-003 reads arg Shapes (`n+k`) |
| STR-001/002/003 | CallEvent (strcat/strlen/unbounded-copy sets); STR-001 AND STR-002 cross-check AssignmentEvents on the destination/base identifier (destination not reassigned per iteration ⇒ accumulation) |
| LOOP-001 | LoopNode.depth |
| LOOP-002/003 | CallEvent membership sets + `conditional_guarded`; LOOP-003 additionally requires `statement_role = primary` |
| LOOP-004 | BinaryOpEvent (`/`,`%`) + Shape literalness + `is_innermost` |
| CALL-001 | FunctionRecord + CallEvents matching own name |
| CALL-002 | FunctionRecord.cyclomatic_complexity vs threshold |
| ARCH-001 | IncludeEdge graph (fan-out/in, cycles, unresolved) |
| ARCH-002 | Aggregation over FunctionRecords + findings (no independent detection) |
| ARCH-003 | Cross-file CallEvent edges by name (file-local matches first) — LOW confidence until semantic tier |
| TYPE-001/002 | StructDef.members + array_dim_literal AND FunctionRecord.param_types (approximate size model; param_types supplies the "passed by value" half of TYPE-001) |
| MEM-001/002 | CallEvent args Shapes (sizeof misuse; memcpy-family size literalness) |
| LOOP-005 | AssignmentEvent.scope_class = global + `in_loop_depth` |
| LOOP-006 | CallEvent.callee_kind = indirect + `is_innermost` |
| LOOP-007 | ComparisonEvent Shapes + LoopNode.induction_vars |
| LOOP-008 | AccessChainEvent.depth ≥ 3 + `is_innermost` |

Baseline stats (§2) come straight off FileRecord + FunctionRecord fields.

## 5. Confidence downgrade hooks (from catalog §1)

The IR exposes exactly three signals findings need for downgrades:

1. `macro_origin=true` on any record intersecting the matched span → cap MEDIUM, `macro_suspect=true`.
2. Cross-file name-based edges (ARCH-003) are inherently LOW until a semantic resolver exists.
3. `ParseHealth.error_regions` intersecting the matched span → cap MEDIUM (C11/C17 grammar-gap monitor).

## 6. Extension seams (explicitly NOT built in v1)

- **Semantic tier:** libclang resolver would enrich existing records (types, true scopes, resolved callees) and raise tier tags — no new record kinds required for current heuristics.
- **Dynamic tier:** runtime profiles attach as parallel event streams keyed by the same `(path, name, span)` identity.
- **New languages:** FileRecord.language + per-language event emitters; record model is language-neutral by construction.

## 7. Validation requirements (for implementation phase)

- Fixture IR corpus runs the full heuristic suite with **no parser bound** (ADR-0001 guardrail 2).
- Span round-trip tests: engine-native points → IR spans conversion locked by unit tests.
- ParseHealth gating test: findings MUST be suppressed/downgraded inside `error_regions`.
