# Perfpal — Rust Backend (plan `rust-backend`, phase R0)

- **Status:** v1 draft — plan `rust-backend`, phase R0 research deliverable (grammar pin + node-kind mapping + cfg/attribute rules + macro blind spot). No backend module, no schema change, and no heuristics are implemented by this document.
- **Audience:** Rust backend implementers (R1) and schema authors (R2). The node-kind tables below are an *extraction plan*, not a description of running code.
- **Grounding:** [ADR-0001](../adr/ADR-0001-parser-strategy.md) (parser-agnostic IR, exact-pin guardrail G5, license policy), [modules.md](./modules.md) §4.2 (scanner-backend interface), §5.2 (sanctioned language-addition extension point), §5.3 (semantic resolver — C-only), §6 (Rust decision), [ir-v1.md](./ir-v1.md) (the record/event seam this mapping targets), [metrics.schema.v1.2.json](../schema/metrics.schema.v1.2.json) (language consts; widened at R2), `src/scanner/backends/tree_sitter_c/mod.rs` (the shipped backend whose interface and ParseHealth contract the Rust backend mirrors).
- **Grammar authority:** every node-kind name and field name below is taken from the grammar's generated `node-types.json` for the pinned crate (`tree-sitter-rust` 0.24.2, 169 named kinds). Nothing here is invented; where a field name matters to the mapping and was not read, the row says so instead of guessing.

---

## 1. Purpose & scope

Phase R0 closes the two pre-conditions for writing a Rust scanner backend: (a) prove the pinned runtime + grammar pair is ABI-compatible and must stay so under CI, and (b) record the complete node-kind → IR mapping (plus the conditional-compilation and macro rules) so R1 is a mechanical implementation pass.

**In scope now (R0):**

- The exact pin of `tree-sitter-rust` and the ABI-compatibility argument (§2).
- The CI guard: the pair must link and parse a representative Rust translation unit with zero `ERROR`/missing nodes (`tests/rust_grammar_link.rs`).
- This mapping table (§3), the attribute/`cfg` rules (§4), the `token_tree` blind spot (§5), and the tier statement (§6).

**Deferred (not built by R0):**

- **R1** — `src/scanner/backends/tree_sitter_rust/` implementing the same `Extraction` interface as the C backend; extension-based language dispatch (`.rs` → rust, `.c`/`.h` → c); `FileRecord.language = "rust"`.
- **R2** — schema MINOR bump 1.1 → 1.2 widening `scan.languages` and `files[].language` (the already-priced re-emission, [modules.md §5.2](./modules.md)); any Rust-only IR field added **additively** (serde default) only where a heuristic provably needs it.
- **R3** — the full catalog mirrored as `PERF-RS-*` rule ids; heuristic mapping to Rust idioms.

**No new ADR is required for R0 (or for the language addition itself), because the IR contract does not change.** The IR is keyed `(path, name, span)` and shaped by records/events, never by grammar node kinds ([ADR-0001](../adr/ADR-0001-parser-strategy.md); [ir-v1.md §1](./ir-v1.md)). A new language is exactly the extension point already priced in [modules.md §5.2](./modules.md): a new scanner backend plus a grammar vendored under the ADR-0001 license policy, with `scan.languages` / `files[].language` widening as a **minor** schema re-emission. A new ADR would be required only if the IR contract itself changed — which R0 does not do, and which R1/R2 must avoid (additive, serde-defaulted fields only).

G5 note: the exact pin is the deliverable of this phase, and it is deliberate — both tree-sitter core and its grammars ship breaking node-type changes across releases, so upgrades are events with a node-types diff review, never automatic ([ADR-0001 guardrail 5](../adr/ADR-0001-parser-strategy.md)).

## 2. Pinned pair (ABI compatibility)

| Crate | Version | License | Role |
|---|---|---|---|
| `tree-sitter` (runtime) | `=0.26.13` (pinned in `Cargo.toml`) | MIT | Parser runtime; C header defines the accepted ABI window. |
| `tree-sitter-rust` | `=0.24.2` (pinned in `Cargo.toml`) | MIT (official tree-sitter org, `github.com/tree-sitter/tree-sitter-rust`) | Rust grammar; generated `parser.c` declares its ABI version. |
| `tree-sitter-language` | `0.1.7` (transitive; single shared lock entry) | MIT | `LanguageFn` glue crate; the grammar exposes `pub const LANGUAGE: LanguageFn`, the same glue shape as `tree-sitter-c`. |

**ABI window.** The pinned runtime's C header (`include/tree_sitter/api.h`) declares `TREE_SITTER_LANGUAGE_VERSION 15` and `TREE_SITTER_MIN_COMPATIBLE_LANGUAGE_VERSION 13` — i.e. the runtime accepts language ABI **13..=15**. The Rust grammar's generated parser (`src/parser.c`) declares `LANGUAGE_VERSION 15` and sets `.abi_version = LANGUAGE_VERSION`. **15 ∈ [13, 15] ⇒ the pinned pair is ABI-compatible.** `parser.set_language(...)` fails only on ABI mismatch, which is precisely what the CI guard detects.

**CI guard.** `tests/rust_grammar_link.rs` is the hard link test for this pair. It is the same runtime ABI guard the C backend relies on (`parser.set_language`; there exercised *indirectly* by the C extraction tests, which surface a mismatch as `ParseStatus::Failed`) — made explicit and standalone here. It builds the grammar's `LANGUAGE` into a `tree_sitter::Parser`, asserts the grammar's reported ABI is inside the runtime's accepted window, asserts `set_language` succeeds, parses a representative Rust translation unit (free function, struct, `impl` method, `for` loop, `macro_rules!` definition, `#[inline]` attribute), asserts the root kind is `source_file` and that a recursive walk finds no `ERROR`/missing nodes, and adds a malformed-source test proving the zero-`ERROR` assertion can actually fail. It is intentionally independent of any backend module — the Rust backend itself lands in R1.

`tree-sitter-language` is intentionally a transitive dependency only (no direct perfpal dependency): both grammars pull it in, and the lock resolves a single shared version (`0.1.7`) feeding the runtime and both grammars, so the triple does not diverge today. It is not itself exact-pinned in `Cargo.toml` (ADR-0001 G5 pins the runtime and the grammars); the committed lockfile is the guard, and a deliberate `cargo update` could move it within `0.1.x`.

## 3. Node-kind mapping table (R1 extraction plan)

> **Status: IMPLEMENTED in R1** (`src/scanner/backends/tree_sitter_rust/`).
> The table below is the plan this implementation followed; see §8 for the
> founder-approved decisions that resolved each open item. Explicit R1 gaps:
> `union_item`/`enum_item` are **skipped entirely** (no IR record; R2/R3 work),
> `use` leaf paths are emitted as include edges but are **unresolved** in R1
> (`::` never matches a file; R3 owns module-graph semantics), every Rust
> `FunctionRecord` carries `alloc_site_count = 0` and every event
> `macro_origin = false` (R3 owns the allocation/macro families).

Column 2 names the IR record/event/shape the kind is intended to produce. Where a Rust kind and its C counterpart differ meaningfully, the note says so. Where a field name is deliberately not asserted, the row says the field details are to be confirmed against `node-types.json` at R1.

Target contract recap ([ir-v1.md §3](./ir-v1.md)): `FileRecord`, `ParseHealth`, `IncludeEdge`, `GlobalVar`, `StructDef`, `MacroEvent`, `FunctionRecord`/`FunctionUnit`, and the typed events (`CallEvent`, `AssignmentEvent`, `BinaryOpEvent`, `ComparisonEvent`, `AccessChainEvent`, `LoopNode`) over the shared `Shape` descriptor. Spans convert at the boundary exactly as in the C backend (`ir_span`/`from_native`, +1 on entry / −1 on exit) and grammar node kinds never leak past it ([modules.md §4.2](./modules.md), [ir-v1.md §2](./ir-v1.md)).

### 3.1 File / translation unit

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `source_file` | `FileRecord` + per-file `ParseHealth` container (the walk root) | C's walk root is `translation_unit`; Rust's root kind is `source_file` (marked `root: true`). Children are `_declaration_statement`, `expression_statement`, `shebang`. `FileRecord.language` becomes `"rust"` (R1); the schema const widens at R2. |

### 3.2 Imports / module structure → `IncludeEdge`

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `use_declaration` | `IncludeEdge` (→ `graphs.include_graph`; ARCH-001/002) | Field is `argument` (single, required); child `visibility_modifier` for `pub use`. |
| `use_list` | part of the `use_declaration` path expression (no record of its own) | No fields; positional children (nested `use_list`, `scoped_use_list`, `use_as_clause`, `use_wildcard`, paths). A `use a::{b, c}` fans out to multiple edges in R1. |
| `scoped_use_list` | part of the `use_declaration` path expression | Fields `list` (`use_list`) and `path` (optional). |
| `use_as_clause` | part of the `use_declaration` path | Fields `alias` and `path`. |
| `use_wildcard` | part of the `use_declaration` path | `use foo::*;` — no fields. |
| `mod_item` | `IncludeEdge` | Fields `name` and optional `body` (`declaration_list`). `mod foo;` (no body) references the external file `foo.rs` / `foo/mod.rs` — the include-edge analogue; `mod foo { ... }` is an inline submodule. |
| `extern_crate_declaration` | external-dependency bucket (angle-analogue, excluded from internal coupling) | Fields `name` and optional `alias`; child `crate`. |

**Rust idiom note / open R1 decision — `IncludeEdge.form`.** The IR `form` is `quoted | angle` ([ir-v1.md §3.1](./ir-v1.md)), a filesystem-include distinction. Rust has no quoted/angle syntax and `use` targets are module paths, not files. R1 must therefore fix the mapping convention (e.g. crate-internal paths — `crate::` / `super::` / `self::` / local module names — as the coupling-relevant bucket, external crates as the excluded bucket) and the resolution pass behavior. This is recorded as an open item in §7 rather than asserted here.

### 3.3 Globals → `GlobalVar`

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `static_item` | `GlobalVar` (storage `static`) | Fields `name`, `type`, optional `value`; children `mutable_specifier` (`static mut`) and `visibility_modifier`. |
| `const_item` | `GlobalVar` (storage mapped from the IR `none|static|extern` set) | Fields `name`, `type`, optional `value`; child `visibility_modifier`. Rust `const` is a compile-time constant with no C-storage analogue; R1 maps it to the nearest `storage` value and this may warrant note. |

Rust idiom note: unlike C declarators, the name and type are direct fields, so no leftmost-identifier/declarator unwinding is needed.

### 3.4 Structs / enums / unions → `StructDef` (and open enum gaps)

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `struct_item` | `StructDef` | Fields `name` (`type_identifier`), optional `body` (`field_declaration_list` or `ordered_field_declaration_list`), optional `type_parameters`. Unit structs have no `body`. |
| `field_declaration_list` | `StructDef.members` container | Positional children `attribute_item`, `field_declaration`. |
| `field_declaration` | `StructDef.members[]` | Fields `name` (`field_identifier`) and `type` (`_type`); child `visibility_modifier`. Direct name+type fields — no C-style declarator/pointer-star unwinding. R1 must decide how `IR Member.is_pointer` / `array_dim_literal` are derived from `_type` shapes (`reference_type`, `pointer_type`, `array_type`). |
| `ordered_field_declaration_list` | `StructDef.members[]` | Tuple-struct positional fields: field `type` is **multiple** (`_type`), each occurrence a positional member with **no name**; children `attribute_item`, `visibility_modifier`. The IR member model is name-bearing, so anonymous positional members are an open R1/R2 decision (§7). |
| `union_item` | `StructDef` (nearest IR record) | Fields `body` (`field_declaration_list`, required), `name`, optional `type_parameters`; children `visibility_modifier`, `where_clause`. The IR has no distinct union record; R1 may reuse `StructDef` or propose an additive discriminator. |
| `enum_item` | **no v1 IR record** — R1/R2 decision (§7) | Fields `body` (`enum_variant_list`, required), `name`, optional `type_parameters`. |
| `enum_variant_list` | container for enum variants | Children `attribute_item`, `enum_variant`. |
| `enum_variant` | candidate `StructDef`-like shape for data-carrying variants | Fields `body` (`field_declaration_list` or `ordered_field_declaration_list`, optional), `name` (`identifier`), optional `value` (`_expression`, the discriminant); child `visibility_modifier`. |

Rust idiom note: `struct_item` fields are ordinary `field_declaration` nodes, so the C `struct_specifier`/`field_declaration` shape mostly carries over — but Rust's tuple structs (`ordered_field_declaration_list`) and enums/options have no direct C analogue and no existing IR record, hence the explicit open items rather than invented mappings.

### 3.5 Macros → `MacroEvent`

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `macro_definition` | `MacroEvent { kind: definition }` | Field `name`; children are `macro_rule` nodes (`macro_rules! name { ... }`). |
| `macro_rule` | container inside a definition | Fields `left` (`token_tree_pattern`) and `right` (`token_tree`). |
| `macro_invocation` | candidate `MacroEvent { kind: use }` (R1 decision, §7) | Field is **`macro`** (not `name`), types `identifier`/`scoped_identifier`; child is a required `token_tree`. |
| `token_tree` | **opaque — no items extracted** (§5) | No fields; children are raw tokens (`_literal`, `crate`, `identifier`, `metavariable`, `mutable_specifier`, `primitive_type`, `self`, `super`, `token_repetition`, `token_tree`). Not parsed into items this milestone. |
| `token_tree_pattern` | **opaque — no items extracted** (§5) | No fields; like `token_tree` but the pattern-variant children are `token_binding_pattern`, `token_repetition_pattern`, `token_tree_pattern` (verified in `node-types.json`). |
| `token_repetition` | **opaque — no items extracted** (§5) | No fields; same raw-token children as `token_tree`. |

Rust idiom note: the C backend's curated alloc-wrapper recognizer runs over opaque `#define` bodies; the Rust analogue would run over `macro_rule.right` token text, but macro-generated *items* are invisible in both cases (§5). Whether Rust emits `MacroEvent.kind = use` at invocation sites (the C backend emits only `definition` and `recognized_pattern`) is an R1 decision.

### 3.6 Attributes → consumed / skipped (not IR records)

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `attribute_item` | none — skip/consume while walking | No fields; child `attribute` (required). Outer attribute (`#[...]`) sibling of the annotated item. |
| `inner_attribute_item` | none — skip/consume while walking | No fields; child `attribute`. Inner attribute (`#![...]`), e.g. crate/module level. |
| `attribute` | none — skip/consume | Fields `arguments` (`token_tree`, optional) and `value` (`_expression`, optional); children are the attribute path (`identifier`/`scoped_identifier`/`crate`/`self`/`super`/`metavariable`). `#[cfg(...)]` predicate text lives in `arguments` and is not interpreted. |

See §4 for the sibling rule and why the C guarded-content defect does not recur.

### 3.7 Functions & methods → `FunctionRecord` / `FunctionUnit`

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `function_item` | `FunctionRecord` + `FunctionUnit` | Fields `name`, `parameters`, optional `return_type`, optional `type_parameters`, `body` (`block`, required); children `function_modifiers`, `visibility_modifier`, `where_clause`. **Flat**: the name/params/return/body are direct fields — no C declarator unwinding (`function_declarator` / `pointer_declarator`). |
| `parameters` | container for `param_count` / `param_types` | No fields; positional children `_type`, `attribute_item`, `parameter`, `self_parameter`, `variadic_parameter`. |
| `parameter` | `FunctionRecord.param_types` entry | Fields `pattern` (`_pattern` or `self`) and `type` (`_type`); child `mutable_specifier`. Pattern+type replaces C's type+declarator — param type text comes from the `type` field directly. |
| `self_parameter` | positional receiver; candidate additive receiver field (§7) | **No fields**; children are `lifetime`, `mutable_specifier`, `self` (all positional). The `&self` / `&mut self` / `self` receiver does not carry an identifier, so the C `param_names` model does not fit it. |
| `function_signature_item` | trait method with no body — R1 decision (§7) | Fields `name`, `parameters`, optional `return_type`, optional `type_parameters`; **no `body` field**. Signature-only trait methods therefore have no events. |
| `impl_item` | method container (no record of its own) | Fields `type` (required), optional `trait` (present for `impl Trait for Type`), optional `body` (`declaration_list`), optional `type_parameters`; child `where_clause`. `function_item`s inside `body` are the hot methods. |
| `trait_item` | method container / trait context | Fields `name` (`type_identifier`), `body` (`declaration_list`, required), optional `bounds`, optional `type_parameters`. |
| `declaration_list` | container inside `impl_item` / `trait_item` / `mod_item` | Children are `_declaration_statement` (includes nested `function_item`, `const_item`, `type_item`, etc.). |

Rust idiom note: recursion (CALL-001) and complexity (CALL-002) map cleanly because `function_item` exposes name/body directly; method calls inside `impl` blocks are ordinary `call_expression`s. Whether method receivers/trait context need a *Rust-only additive IR field* to preserve identity or enable a heuristic is an open R2 question (§7) — fields must be serde-default additive only if provably needed.

### 3.8 Loops → `LoopNode`

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `for_expression` | `LoopNode { kind: for }` | Fields `pattern`, `value` (the iterated expression), `body` (`block`); child `label`. |
| `while_expression` | `LoopNode { kind: while }` | Fields `condition` (`_expression`/`let_chain`/`let_condition`) and `body`; child `label`. |
| `loop_expression` | **no v1 `LoopKind` variant** — R1/R2 decision (§7) | Field `body` (`block`, required); child `label`. `loop {}` is Rust's unconditional loop; `LoopKind` is `for|while|do` today, so this needs an additive enum variant or an explicit mapping. |

Rust idiom note: nesting depth (`max_loop_depth`), `in_loop_depth`, and `is_innermost` semantics carry over unchanged once these three kinds are recognized; `do` is absent from Rust, and `loop` is absent from the IR enum.

### 3.9 Events / expressions → typed events and `Shape`

All rows land as events attached to the enclosing `FunctionRecord`, each carrying `span`, `in_loop_depth`, `is_innermost`, `resolver_tier`, `macro_origin` ([ir-v1.md §3.3](./ir-v1.md)).

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `call_expression` | `CallEvent` | Fields `function` (any expression) and `arguments` (`arguments`, required). |
| `arguments` | `CallEvent.args` container | No fields; positional children `_expression`, `attribute_item`. |
| `identifier` (as `function`) | `CallEvent.callee_kind = direct` | Plain function name. |
| `field_expression` (as `function`) | method call — R1 decision for `callee_kind` (§7) | `x.method()`; callee text is a member chain, not a bare name. |
| `scoped_identifier` (as `function`) | path-qualified callee — R1 mapping | Fields `name`, optional `path` (`foo::bar`). |
| `generic_function` (as `function`) | turbofish callee — R1 mapping | Fields `function` and `type_arguments` (`foo::<T>()`). |
| `binary_expression` | `BinaryOpEvent` (arith) / `ComparisonEvent` (relational) | Fields `left`, `operator`, `right`. Operator set includes `+ - * / % & | ^ << >>` and `&& ||`; relational operators (`== != < > <= >=`) map to `ComparisonEvent`. No C `<=>` operator exists. |
| `compound_assignment_expr` | `AssignmentEvent` | Fields `left`, `operator`, `right`; operators `+= -= *= /= %= &= |= ^= <<= >>=`. Distinct from C, where `+=` is an `assignment_expression`; the C backend already treats compound assignment as an assignment (not a `BinaryOpEvent`). |
| `assignment_expression` | `AssignmentEvent` | Fields `left`, `right`. |
| `index_expression` | `Shape { kind: subscript }` / `AccessChainEvent` element | **No fields** — indexed and index expressions are positional `_expression` children. This differs from C's `subscript_expression` (`argument`/`index` fields); R1 must read positionally. |
| `field_expression` | `Shape { kind: member_access }` / `AccessChainEvent` element | Fields `field` (`field_identifier` or `integer_literal`, for tuple access) and `value` (`_expression`). Note the base field is **`value`**, whereas C's `field_expression` base field is `argument`. |
| `let_declaration` | local binding (feeds scope classification / assignment detection) | Fields `pattern`, optional `type`, optional `value`, optional `alternative`; child `mutable_specifier`. Rust locals are not `GlobalVar`s; R1 must decide how `let` feeds `AssignmentEvent`/scope classification. |
| `unary_expression` | `Shape { kind: unary }` | No fields; a single positional `_expression` child (deref/negation/not). |
| `reference_expression` | `Shape { kind: unary }` candidate | Field `value`; child `mutable_specifier` (`&`/`&mut`). Borrow is Rust-specific; R1 decides whether it is a distinct `Shape` variant or folds into unary. |
| `parenthesized_expression` | unwrapped `Shape` | No fields; a single positional `_expression` child — same unwrap intent as the C backend. |
| `type_cast_expression` | `Shape` candidate | Fields `type` and `value` (`as` casts). |
| `try_expression` | `Shape` candidate | No fields; single positional `_expression` child (the `?` operator). |
| `if_expression` | complexity contributor (branch) | Fields `condition`, `consequence`, optional `alternative`. Rust `if` is an expression; R1 maps it to the C `if_statement` complexity role. |
| `match_expression` / `match_arm` | complexity contributor (branch) | `match_expression` fields `value`, `body` (`match_block`); `match_arm` fields `pattern`, `value`. `match` is a Rust-specific branch multiplicity with no direct C analogue — R3 owns how it counts. |

Rust idiom note (memory/`sizeof` family): Rust has **no `sizeof` expression node**. `std::mem::size_of` / `size_of_val` are ordinary `call_expression`s whose `function` is a `scoped_identifier`, so the IR `Shape.sizeof_operand` (the MEM-001 input, [ir-v1.md §3.4](./ir-v1.md)) is populated from the call's argument shape at R3, not from a `sizeof_expression`. Likewise allocation/churn (ALLOC family) and string building (STR family) map to Rust APIs and macros — `Box::new`, `Vec::with_capacity`, `vec!`, `String::with_capacity`, `format!` (a `macro_invocation` with opaque `token_tree`), `push_str`, `to_string`, `clone` — which is an R3 heuristic-mapping concern, not an extraction concern.

### 3.10 Literals / basic shapes / comments

| `tree-sitter-rust` kind | IR target | R1 plan / Rust idiom note |
|---|---|---|
| `identifier` | `Shape { kind: identifier }` | Named token, no fields. |
| `integer_literal` | `Shape { kind: literal }` | |
| `float_literal` | `Shape { kind: literal }` | |
| `string_literal` | `Shape { kind: literal }` | Children `escape_sequence` / `string_content`. |
| `raw_string_literal` | `Shape { kind: literal }` candidate | Child `string_content`; Rust-specific (`r"..."`). |
| `boolean_literal` | `Shape { kind: literal }` | `true`/`false` are lexed as `boolean_literal` (named), not keywords. |
| `char_literal` | `Shape { kind: literal }` | |
| `negative_literal` | literal candidate | Children `integer_literal`/`float_literal`; appears in patterns. |
| `_literal` | supertype selector for the literal shapes above | Supertype over `boolean_literal`, `char_literal`, `float_literal`, `integer_literal`, `raw_string_literal`, `string_literal`. |
| `primitive_type` | type text | Named token (`i32`, `u8`, …); feeds `param_types` / member type text. |
| `type_identifier` | type text | Named token (named types). |
| `field_identifier` | member name | Named token; used by `field_declaration` and `field_expression`. |
| `self` | receiver / `self.x` base | Named token. |
| `line_comment` | LOC comment (scanner-owned stat) | Named, `extra: true`; fields `doc`/`inner`/`outer`. |
| `block_comment` | LOC comment | Named, `extra: true`; same doc/inner/outer fields. |
| `doc_comment` | comment content token | Named token inside doc comments. |

Rust idiom note — **there is no `comment` supertype** (unlike tree-sitter-c, where the C backend keys LOC on the single `"comment"` kind). The Rust backend must match **both** `line_comment` and `block_comment` (and treat `doc_comment` as comment content) to count comment LOC. Both comment kinds are `extra: true`, matching C's comment treatment.

## 4. Conditional compilation & attributes

**The sibling rule.** In tree-sitter-rust, `#[...]` is an `attribute_item` node and `#![...]` is an `inner_attribute_item` node — both are ordinary **siblings** of the item they annotate, and both are listed as subtypes of `_declaration_statement` alongside `function_item`, `struct_item`, `impl_item`, etc. (verified in `node-types.json`). `#[cfg(...)]` is therefore *not* a wrapper node: the annotated item remains a structured sibling and is extracted normally.

**Consequence — the C guarded-content defect does not recur.** In C, `#ifndef`/`#define` guards nest their content *inside* `preproc_ifdef` nodes, so a direct-children-of-`translation_unit` walk dropped guarded declarations (the defect fixed by plan `guarded-scan-fix`, locked by `tests/guarded_content.rs`). Rust's `#[cfg(...)]` has no such nesting: the annotated item is still a normal item sibling. R1 must therefore only **skip or consume `attribute_item` / `inner_attribute_item` nodes when walking** (they are not items; §3.6) — no guarded-content recovery logic is needed for plain attributes.

**Honest limitation.** tree-sitter is a parser, not a preprocessor: it does not evaluate `cfg` predicates (the predicate text sits in the opaque `argument`/`token_tree` of the `attribute` node). Both arms of a `#[cfg(...)]` / `#[cfg(not(...))]` pair present in the source are extracted, because the backend cannot know which arm the compiler would select. R1 must not claim compiled-only extraction; this mirrors the C backend's existing honesty posture (ParseHealth is consulted before scoring, [ir-v1.md §5](./ir-v1.md)).

## 5. Documented blind spot (`token_tree`)

`macro_rules!` bodies and `cfg_if!`/macro invocation bodies parse as opaque `token_tree` nodes (and `token_tree_pattern` / `token_repetition` inside definitions): raw token children that the grammar does not structure into `function_item`, `struct_item`, `for_expression`, `call_expression`, etc. Consequences:

- **Macro-generated items are invisible this milestone.** A `cfg_if! { if #[cfg(...)] { fn f() { … } } }` or any `macro_rules!`-produced function/struct/loop yields no IR record, so no heuristic can fire on it and the macro-origin downgrade hook has no span to anchor to.
- `macro_definition` still yields a `MacroEvent { kind: definition }` (the definition itself is a structured node), and `macro_invocation` still yields a structured node whose `token_tree` child is opaque — so invocations can be *seen* but not *expanded*.

**Decision:** document the blind spot; do **not** re-parse `token_tree` contents into items in this milestone. This mirrors the C backend's documented macro simplification (`preproc_arg` opacity) and ADR-0001's coverage-over-precision posture. Whether to emit `MacroEvent { kind: use }` at invocation sites is an R1 decision (§7); re-parsing token trees is explicitly out of scope.

## 6. Tier & resolver scope

The Rust backend is **syntactic-tier only** in this milestone. Every Rust record/event carries `resolver_tier = syntactic`, and Rust files report `parse_health.resolver` as absent/`None` (the option is omitted on the wire, [ir-v1.md §3.1](./ir-v1.md)). The semantic resolver (`clang-sys`/libclang) is **C-only** and does not apply to Rust ([modules.md §5.3](./modules.md); [ADR-0001](../adr/ADR-0001-parser-strategy.md) deferred libclang as a C additive resolver). No Rust-specific resolver tier exists, so the confidence upgrades the resolver performs for C (e.g. ARCH-003 resolved edges, ALLOC-002 proven-fclose, STR-003 resolved-format) have no Rust equivalent yet.

Backend/interface contract the Rust backend must mirror (from `src/scanner/backends/tree_sitter_c/mod.rs`, [modules.md §4.2](./modules.md)): given `(path, source bytes)`, emit that file's IR records/events; convert all engine-native spans to IR spans at this boundary (never leak node kinds); and preserve **parse honesty** — `ERROR`/missing nodes become `ParseHealth { status: partial, error_regions, error_node_count }`, a failed parse yields `ParseHealth { status: failed }`, and the function never panics on malformed input. The R1 `Extraction`-shaped output must not copy C-specific logic (declare declarator unwinding, `preproc_*` handling); it reuses only the interface shape and the span/ParseHealth contract.

## 7. Open items deferred to R1–R3

These are recorded so R1/R2/R3 decide them explicitly; none is guessed above.

1. **Rust-only IR additive fields (R1 research, R2 emit).** Determine whether any Rust-only fact (method receiver, trait context, `mut` binding) is *provably* needed by a heuristic. If yes, it must be a serde-defaulted additive field (same pattern as `ParseHealth.resolver` / `Shape.sizeof_operand`) and land with the R2 minor bump — never a breaking change.
2. **`IncludeEdge.form` mapping (R1).** No `quoted|angle` distinction exists for `use`/`mod`/`extern crate`; fix the internal-vs-external bucketing convention used for coupling and `arch` heuristics.
3. **Callee classification (R1).** Decide `CalleeKind` for method calls (`field_expression` `function`), path-qualified (`scoped_identifier`), and turbofish (`generic_function`) callees.
4. **`LoopKind` coverage (R1/R2).** `loop_expression` has no variant in the current `for|while|do` enum; needs an additive variant or an explicit mapping.
5. **Enums / unions / tuple-struct fields (R1/R2).** The IR has no enum record and `StructDef.members` is name-bearing; decide reuse vs an additive discriminator/field.
6. **`function_signature_item` (R1).** Decide whether a body-less trait method emits a `FunctionRecord` (with no events) or nothing.
7. **`let_declaration` and scope classification (R1).** Decide how Rust local bindings feed `AssignmentEvent.scope_class` and LOOP-005-style heuristics (Rust has no C global-vs-param name model).
8. **Macro invocation events (R1).** Decide whether `macro_invocation` sites emit `MacroEvent { kind: use }` for honesty, given `token_tree` opacity (§5).
9. **Schema 1.2 minor re-emission (R2) — LANDED.** `scan.languages` and `files[].language` were widened from `const "c"` to `enum ["c","rust"]`; the embedded byte-copy + drift test were re-emitted as [metrics.schema.v1.2.json](../schema/metrics.schema.v1.2.json); the v1.1 field set remains valid under v1.2 with the version marker updated (see §8).
10. **`PERF-RS-*` heuristic mapping (R3).** Mirror each C family's intent to Rust idioms — allocation/churn (`Box`/`Vec`/`String`/`Rc` + clone/reserve), string building (`format!`/`push_str`/capacity), loops (nesting, I/O or expensive setup in loops, char-wise string work, variable-divisor arithmetic), recursion/complexity, memory/sizeof analogues (`size_of`, field-layout/padding via `field_declaration_list`, large-struct-by-value), membership scans, pointer/lifetime-chain depth analogues, and architecture (module `use` graph cycles/fan-out, hot components, cross-module call coupling). The `token_tree` blind spot (§5) and accepted false-positive classes must be documented alongside the mapping.

**Unverified / needs-review items in this document (explicit):**

- The exact sub-node layout of `use_declaration` path expressions beyond the field names listed (fan-out enumeration for `use a::{b, c}`) is deferred to R1; only the field names above were read from `node-types.json`.
- `IncludeEdge.form` semantics for Rust, `CalleeKind` for method/scoped/turbofish callees, `LoopKind` for `loop_expression`, and the enum/union/tuple-struct IR targets are **unresolved by design** — listed as open items, not asserted.
- `tree-sitter-rust`'s `node-types.json` was read at the pinned crate version (`0.24.2`); a future grammar upgrade may change kinds/fields and would require a node-types diff review per ADR-0001 guardrail G5.

## 8. R1 implementation notes

Phase R1 landed `src/scanner/backends/tree_sitter_rust/`, hoisted a single
`Extraction` into `src/scanner/backends/mod.rs` (shared by both backends), and
added extension-based language dispatch (`.rs` → Rust, `.c`/`.h` → C) with a
Rust include/module resolution pass in `src/scanner/mod.rs`. The founder
approved eight decisions that resolve the §7 open items; they are implemented
exactly as follows.

| Id | Decision | Resolution |
|---|---|---|
| **D1** | `IncludeEdge.form` mapping | Internal module paths (`crate::`/`self::`/`super::`, plus bare `mod foo;`) → `Quoted`; everything else (`std`/`core`/`alloc`/any crate name/`extern crate`) → `Angle`. `to_path` is the full `::`-joined path text for `use`, the module/crate name for `mod`/`extern crate`. `use` leaves never match a file, so they stay unresolved in R1 (R3 owns module-graph semantics); wildcard leaves drop the trailing `::*`. |
| **D2** | Callee classification | `identifier`/`scoped_identifier`/`field_expression`/`generic_function` as the `function` field → `Direct`; anything else → `Indirect`; missing `function` → `Unknown`. `callee_name` is the final segment (`scoped_identifier`→`name`, `field_expression`→`field`, `generic_function`→recurse). Documented FP class: closure-field invocation (`self.callback()`) classifies `Direct`. |
| **D3** | `LoopKind` coverage | `loop_expression` maps to the additive `LoopKind::Loop` variant (serializes `"loop"`); recorded in [ir-v1.md](./ir-v1.md). No other IR change. |
| **D4** | Structs | `struct_item` → `StructDef` (named members from `field_declaration_list`); tuple structs (`ordered_field_declaration_list`) → `StructDef` with synthetic positional member names `"0"`, `"1"`, …. `union_item`/`enum_item` are skipped entirely — R2/R3 work. |
| **D5** | Signatures | `function_signature_item` (body-less trait method) is skipped, mirroring C where prototypes are not functions. |
| **D6** | let / assignment | `let_declaration` is NOT an `AssignmentEvent`; `assignment_expression` and `compound_assignment_expr` ARE. `scope_class` is `Global` iff the target root identifier is a collected `static`/`const` name, `Param` iff a parameter name (including `self`), else `Local`; a non-identifier target is `Unknown`. |
| **D7** | Macros | `macro_definition` → `MacroEvent { kind: Definition }`; every `macro_invocation` (including inside function bodies — gathered by a whole-tree pass) → `MacroEvent { kind: Use }`, no expansion. |
| **D8** | Stats honesty | `FunctionRecord.alloc_site_count = 0` and every event's `macro_origin = false` for Rust; R3 owns the allocation family and `token_tree` bodies are opaque. |

Additional implementation notes:

- **Parser honesty.** `extract` is total; `set_language` failure or a `None`
  parse yields `ParseHealth { status: failed, .. }` with an empty
  `language: "rust"` file record. Otherwise zero `ERROR`/missing nodes → `ok`,
  any → `partial` with outermost error regions and a total error-node count.
- **Recursive file-level walk.** `walk_file_level` stops descending at
  `function_item` bodies but DOES descend into `impl_item`/`trait_item`/
  `mod_item` bodies and `declaration_list`s, so methods and nested-module items
  are collected while function-local items stay function-local. Associated
  `const`/`static` items inside an `impl`/`trait` body are excluded from the
  file-level `GlobalVar` set (they are not module globals; R1 review M1).
- **Rust module resolution (reviews m2 + N1 + rounds 3–4).** Resolves `mod`
  DECLARATIONS only. MODULE-dir-relative, matching rustc: a submodule file
  `<dir>/<stem>.rs` owns the `<dir>/<stem>/` subdirectory, so `src/a.rs` +
  `mod b;` → `src/a/b.rs` or `src/a/b/mod.rs` (a bare `src/b.rs` is
  error[E0583]). CRATE-ROOT files instead resolve submodules in their
  CONTAINING directory: `lib.rs`/`main.rs`, any `mod.rs` module file, and any
  DIRECT child of the Cargo crate-root directories
  `src/bin`/`examples`/`tests`/`benches` — matched by TRAILING SEGMENT so
  nested/workspace packages work too (`crates/foo/src/bin/bar.rs` + `mod baz;`
  → `crates/foo/src/bin/baz.rs`; `crates/foo/tests/x.rs` + `mod common;` →
  `crates/foo/tests/common/mod.rs`; `src/bin/foo.rs` + `mod bar;` →
  `src/bin/bar.rs`, NOT `src/bin/foo/bar.rs`). A `mod` miss yields
  `to_path=null`, `resolved=false` (data, not error).
  - **Inline modules:** nested `mod` declarations carry a `/`-joined prefix
    (`mod outer { mod inner; }` → raw `outer/inner`; never `::`), resolved as
    `<module_dir>/outer/inner.rs` / `<module_dir>/outer/inner/mod.rs`.
  - **`use` edges (founder decision)**: internal `use` paths are CLASSIFIED but
    NOT resolved — they keep their raw `::` path with `resolved=false`
    (`use crate::util::helper` stays `Quoted`, `to_path="crate::util::helper"`).
    Only `Angle` (external) edges are also left unresolved with raw text kept.
  - Repo-root single-file targets (`foo.rs`) are ambiguous (not under a known
    crate-root dir), so the file-stem rule applies: `foo` → `foo/`.
- **Complexity formula.** cyclomatic = `1 + {if_expression, each match_arm,
  each loop, &&, ||, try_expression}`; cognitive = `if_expression` / loops /
  `try_expression` cost `1 + nesting` and increase nesting, while `match_arm`
  and `&&`/`||` cost a flat `1`.
- **Attributes.** `#[...]`/`#![...]` are ordinary siblings, not wrappers; the
  annotated item always extracts (locked by tests), and both arms of a
  `#[cfg(...)]` pair present in source are extracted since the predicate is
  opaque (honest limitation, §4).
- **Known limitation — crate-root directory detection is a path-suffix
  approximation (R1 review, accepted).** A directory whose final segment is
  `src/bin`/`examples`/`tests`/`benches` is treated as a Cargo crate-root
  directory, so a module directory that literally ends in one of those names
  (e.g. `src/tests/`) is mis-classified as a crate root (false negative:
  `<dir>/<stem>/` submodules missed; false positive: a sibling `<dir>/<stem>.rs`
  matched). Distinguishing the two needs Cargo.toml-aware package-root
  detection, which is deferred to R3 (module-graph work). No R1 test exercises
  this layout.

**R2 schema note (plan `rust-backend` R2).** The metrics.json schema MINOR bump
1.1 → 1.2 admits `"rust"` in both `scan.languages` and `files[].language`
(widened from `const "c"` to `enum ["c","rust"]`); internal `use` edges remain
raw `::` paths with `resolved=false` (metrics.json `include_graph` admits
`Quoted` edges, so they are carried as unresolved internal edges, exactly as in
R1), and Rust module-graph semantics are still R3.

