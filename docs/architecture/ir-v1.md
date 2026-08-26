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
}

IncludeEdge {
  from_path, to_path        // to_path = null when unresolvable (finding, not failure)
  form: quoted | angle      // angle ⇒ "external" bucket, excluded from internal coupling
  resolved: bool
  span
}

GlobalVar {
  name, path, span
  storage: none | static | extern
}

StructDef {
  name, path, span          // tag name; "" if anonymous
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
  callee_name               // textual name at call site
  callee_kind: direct | indirect | unknown
                            // direct = named function; indirect = called through a
                            // non-function-name expression (function-pointer candidate, LOOP-006);
                            // unknown = unresolvable shape
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
  kind: for | while | do
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

This one vocabulary is what keeps the IR event-stream-shaped instead of AST-shaped: heuristics pattern-match over typed events + shapes, never over parser node types.

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
