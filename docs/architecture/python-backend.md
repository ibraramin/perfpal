# Perfpal — Python Backend (plan `python-backend`, phase W1)

- **Status:** v1 draft — plan `python-backend`, phase W1 design deliverable (grammar pin + node-kind mapping + comprehension/module-scope/async/notebook decisions + additive-IR field list + the durable rule disposition table). No backend module, no schema change, and no heuristics are implemented by this document.
- **Audience:** Python backend implementers (W2), catalog/schema authors (W3), and proof authors (W4). The node-kind tables below are an *extraction plan*, not a description of running code.
- **Grounding:** [ADR-0007](../adr/ADR-0007-python-tier-strategy.md) (tier strategy; retire-10/keep-13/native-add rule set; inference ships in M3), [ADR-0001](../adr/ADR-0001-parser-strategy.md) (parser-agnostic IR, exact-pin guardrail **G5**, license policy), [modules.md](./modules.md) §4.2 (scanner-backend interface), §5.2 (sanctioned language-addition extension point), §5.3 (semantic resolver — C-only), [ir-v1.md](./ir-v1.md) (the record/event seam this mapping targets), [rust-backend.md](./rust-backend.md) (the sibling backend doc this one mirrors), `docs/evidence/python-canary.md` (the W0 frequency/precision evidence feeding §9).
- **Grammar authority:** every node-kind name and field name below is taken from the grammar's generated `node-types.json` for the pinned crate (`tree-sitter-python` 0.25.0; 217 types, 128 named). The crate also exposes this file as `tree_sitter_python::NODE_TYPES`. Nothing here is invented; where a field name matters to the mapping and was not read, the row says so instead of guessing.

---

## 1. Purpose & scope

Phase W1 closes the pre-conditions for writing the Python scanner backend: (a) prove the pinned runtime + grammar pair is ABI-compatible and must stay so under CI, and (b) record the complete node-kind → IR mapping (plus the comprehension/module-scope/async/notebook decisions and the additive-IR field list) so W2 is a mechanical implementation pass. It also owns the **durable rule disposition table** (§9), resolving the "mirror the full catalog vs retire-10" conflict (R-pf-087/R-pf-088) with per-rule reasons.

**In scope now (W1):**

- The exact pin of `tree-sitter-python` and the ABI-compatibility argument (§2).
- The CI guard: the pair must link and parse a representative Python module with zero `ERROR`/missing nodes (`tests/python_grammar_link.rs`).
- This mapping table (§3), the inference/additive-field plan (§4), import resolution (§5), async (§6), notebooks (§7), tier statement (§8), and the disposition table (§9).

**Deferred (not built by W1):**

- **W2** — `src/scanner/backends/tree_sitter_python/` implementing the same `Extraction` interface as the C/Rust backends; extension dispatch (`.py`/`.pyi` → python); `FileRecord.language = "python"`; the minimal binding/alias inference pass and the best-effort import-resolution pass (§4–§5).
- **W3** — the `PERF-PY-*` catalog (§9), the registry/reporter wiring, and the schema MINOR 1.2 → 1.3 (admitting the `"python"` token).
- **W4** — `examples-python/` proof, precision KPI, benchmark ladder, goldens, docs.

**No new ADR is required for W1 (or for the language addition itself), because the IR contract does not change.** The IR is keyed `(path, name, span)` and shaped by records/events, never by grammar node kinds ([ADR-0001](../adr/ADR-0001-parser-strategy.md); [ir-v1.md §1](./ir-v1.md)). Only **additive, serde-defaulted** IR fields are introduced (§4); a change to the IR *contract* would require an ADR, and W2–W4 must avoid it.

G5 note: the exact pin is the deliverable of this phase, and it is deliberate — both tree-sitter core and its grammars ship breaking node-type changes across releases, so upgrades are events with a node-types diff review, never automatic ([ADR-0001 guardrail 5](../adr/ADR-0001-parser-strategy.md); [ADR-0007 §1](../adr/ADR-0007-python-tier-strategy.md)).

## 2. Pinned pair (ABI compatibility)

| Crate | Version | License | Role |
|---|---|---|---|
| `tree-sitter` (runtime) | `=0.26.13` (pinned in `Cargo.toml`) | MIT | Parser runtime; C header defines the accepted ABI window. |
| `tree-sitter-python` | `=0.25.0` (pinned in `Cargo.toml`) | MIT (official tree-sitter org, `github.com/tree-sitter/tree-sitter-python`) | Python grammar; generated `parser.c` declares its ABI version. |
| `tree-sitter-language` | `0.1.7` (transitive; single shared lock entry) | MIT | `LanguageFn` glue crate; the grammar exposes `pub const LANGUAGE: LanguageFn`, the same glue shape as `tree-sitter-c`/`tree-sitter-rust`. |

**ABI window.** The pinned runtime's C header declares `TREE_SITTER_LANGUAGE_VERSION 15` and `TREE_SITTER_MIN_COMPATIBLE_LANGUAGE_VERSION 13` — i.e. the runtime accepts language ABI **13..=15**. The Python grammar's generated parser (`src/parser.c`) declares `LANGUAGE_VERSION 15` and sets `.abi_version = LANGUAGE_VERSION`. **15 ∈ [13, 15] ⇒ the pinned pair is ABI-compatible**, and this matches [ADR-0007 §1](../adr/ADR-0007-python-tier-strategy.md)'s recorded ABI-15 claim. `parser.set_language(...)` fails only on ABI mismatch, which is precisely what the CI guard detects.

**CI guard.** `tests/python_grammar_link.rs` is the hard link test for this pair. It builds the grammar's `LANGUAGE` into a `tree_sitter::Parser`, asserts the grammar's reported ABI is inside the runtime's accepted window, asserts `set_language` succeeds, parses a representative Python module (module docstring, aliased imports, decorator, typed function, class with base + method, `for`/`while`, comprehensions, f-string, `async def` + `await`, `try`/`except`/`finally`, `with`, lambda, membership), asserts the root kind is `module` and that a recursive walk finds no `ERROR`/missing nodes, and adds a malformed-source test proving the zero-`ERROR` assertion can actually fail. It is intentionally independent of any backend module — the Python backend itself lands in W2.

## 3. Node-kind mapping table (W2 extraction plan)

Target contract recap ([ir-v1.md §3](./ir-v1.md)): `FileRecord`, `ParseHealth`, `IncludeEdge`, `GlobalVar`, `StructDef`, `MacroEvent`, `FunctionRecord`, and the typed events (`CallEvent`, `AssignmentEvent`, `BinaryOpEvent`, `ComparisonEvent`, `AccessChainEvent`, `LoopNode`) over the shared `Shape` descriptor. Spans convert at the boundary exactly as in the C/Rust backends (`Span::from_native`, +1 on entry / −1 on exit) and grammar node kinds never leak past it ([modules.md §4.2](./modules.md), [ir-v1.md §2](./ir-v1.md)).

### 3.1 File / module

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `module` (root) | `FileRecord` + `ParseHealth` + LOC counts | The single translation unit per file. Root kind asserted by the link test. `FileRecord.language = "python"`. |
| `expression_statement` | expression / docstring holder | A bare module/function docstring is an `expression_statement` wrapping a `string`; it is expression-shaped, not a record. |
| `comment` | LOC comment stat | Named kind (there is **no `comment` supertype** issue here: Python lexes one `comment` kind, unlike Rust's `line_comment`/`block_comment`). Feeds comment LOC. |

### 3.2 Imports / module structure → `IncludeEdge`

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `import_statement` | `IncludeEdge` per imported name | Fields `name` (`dotted_name` or `aliased_import`). `import a.b.c` yields an edge to module `a.b.c`. |
| `import_from_statement` | `IncludeEdge` per imported name | Fields `module_name` (`dotted_name` or `relative_import`) and `name` (`dotted_name`/`aliased_import`), plus a positional `wildcard_import` child. |
| `aliased_import` | alias map + edge | Fields `name` (`dotted_name`) and `alias` (`identifier`). Populates the per-file import-alias map (§4). |
| `dotted_name` | module path text | No fields; positional `identifier` children joined by `.`. |
| `relative_import` | relative module path | Leading-dot count + optional `dotted_name`; resolved against the includer's package (§5). |
| `wildcard_import` | `IncludeEdge` with export-unknown | `from x import *` → edge to module `x`; **no name-level edges** (export set unknown) — data-only (§5). |

`IncludeEdge.form`: Python has one import form; reuse `IncludeForm::Quoted` for every Python edge (the C `quoted`/`angle` split has no Python analogue — every Python import is intra-language). `to_path` follows the existing lifecycle: raw text + `resolved=false` at extraction, rewritten by the scanner resolution pass on a hit (§5); an unresolved **internal** import normalizes to `to_path=None` with no finding (mirrors the Rust N1 decision; [ADR-0007 §7](../adr/ADR-0007-python-tier-strategy.md)).

### 3.3 Module-scope bindings → `GlobalVar` + synthetic module function

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| module-level `assignment` (`expression_statement` → `assignment`) | `GlobalVar` | A module-scope target name becomes a `GlobalVar { storage: StorageClass::None }`. |
| module-level `global_statement` | `GlobalVar` (already implied) | `global x` at module scope is a no-op marker; the binding is a `GlobalVar`. |
| module-level statements | **synthetic `FunctionRecord`** | See the decision below — required or script-style AI files yield zero findings. |

**Synthetic module-scope function (decision).** All top-level statements of a module are extracted **as if** they lived in one function body, emitted as a synthetic `FunctionRecord` carrying `is_module_scope = true` (§4) and a reserved name sentinel (`"<module>"`). Rationale: loop/alloc/complexity heuristics key on the enclosing `FunctionRecord`; without this, a top-level `for` loop in a script-style AI file (common) would be invisible. This mirrors the Rust backend's need to anchor top-level events and is an explicit W1 obligation (`state/handoff/latest.md:40`). The sentinel never collides with a real Python identifier (angle brackets are not legal in a name).

### 3.4 Classes → `StructDef`

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `class_definition` | `StructDef { kind: StructKind::Struct }` | Fields `name` (`identifier`), `superclasses` (`argument_list`), `body` (`block`), optional `type_parameters`. Members are **best-effort**: `function_definition`s in the body → methods (not struct members); annotated `assignment`s in the body → `StructMember` with the type text. Unlike C/Rust there is no fixed layout, so TYPE-001/002 never consume this. |
| `block` | class/function body container | Positional statement children; also used for `if`/`for`/`while`/`with`/`try` bodies. |

### 3.5 Decorators → `MacroEvent`

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `decorated_definition` | wrapper (no record of its own) | Field `definition` (`class_definition` or `function_definition`); decorators are positional `decorator` children. |
| `decorator` | `MacroEvent { kind: Use }` | No fields; text is `@name` / `@pkg.name(...)`. The decorated definition is extracted normally, so a `@cache`/`@lru_cache` decorator can be surfaced as a `MacroEvent` for decorator-aware rules (W3 decides whether any rule consumes it). |

### 3.6 Functions → `FunctionRecord`

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `function_definition` | `FunctionRecord` | Flat fields `name` (`identifier`), `parameters`, optional `return_type` (`type`), `body` (`block`), optional `type_parameters`. No C declarator unwinding. `async def` uses the same kind with a leading `async` modifier token → `is_async = true` (§4, §6). |
| `parameters` | container for `param_count` / `param_types` | No fields; positional `identifier`/`typed_parameter`/`default_parameter`/`typed_default_parameter`/`list_splat_pattern`/`dictionary_splat_pattern` children. |
| `typed_parameter` / `typed_default_parameter` | `FunctionRecord.param_types` entry | Annotation `type` field supplies the raw type text; a bare `identifier` contributes `"unknown"`. Parameter annotations are the primary static type evidence (§4). |

`param_types` is best-effort raw text (annotations when present, else `"unknown"`), never a resolved type — matching the inference-confidence ceiling in §4.

### 3.7 Loops → `LoopNode`

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `for_statement` | `LoopNode { kind: For }` | Fields `left` (`pattern`/`pattern_list`), `right` (`expression`/`expression_list`), `body` (`block`), optional `alternative` (`else_clause`). |
| `while_statement` | `LoopNode { kind: While }` | Fields `condition` (`expression`), `body` (`block`), optional `alternative`. |
| `for_in_clause` | `LoopNode { kind: For }` when inside a comprehension | See §3.8; the `right` iterable is evaluated **once** at the outer depth. |

`do` is absent from Python; `LoopKind::Do` is never produced. `LoopNode.induction_vars` carries the `left` target identifier(s).

**`for`-iterable exclusion (reuse the Rust R4 fix).** A Python `for` iterable (`right` of `for_statement` / `for_in_clause`) is evaluated **once**, before the body — exactly like Rust's `for` (unlike C's per-iteration `for` header). `FuncScan::enter_loop` must therefore visit the iterable at the **outer** depth and skip it in the body walk, and the event layer's `in_for_iterable` exclusion applies (`in_loop_depth = 0`, `is_innermost = false`, `statement_role = Unknown` for iterable nodes). This is the comprehension-blindness fix required by [ADR-0007 §Consequences](../adr/ADR-0007-python-tier-strategy.md) and [rust-backend.md §3.8](./rust-backend.md).

### 3.8 Comprehensions → `LoopNode` (not an opaque expression)

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `list_comprehension` | `LoopNode { kind: For }` | Fields `body` (`expression`); positional `for_in_clause` / `if_clause` children. |
| `set_comprehension` | `LoopNode { kind: For }` | Field `body` (`expression`). |
| `dictionary_comprehension` | `LoopNode { kind: For }` | Field `body` (`pair`). |
| `generator_expression` | `LoopNode { kind: For }` | Field `body` (`expression`). |
| `for_in_clause` | nested loop level | Fields `left` (`pattern`/`pattern_list`), `right` (iterable). Each clause adds one nesting depth. |
| `if_clause` | filter (no loop) | No fields; a comprehension filter, contributes to complexity but not loop depth. |

**Decision.** Every comprehension is modelled as a real `LoopNode` (kind `For`) so loop-scoped rules (`LOOP-001/002/003/007/009`, `ALLOC-001/003`, `ASYNC-002`) fire inside it, and so `max_loop_depth`/`is_innermost` are honest. The iterable of each `for_in_clause` is evaluated once at that clause's **outer** depth (the §3.7 fix). Modeling comprehensions as opaque expression nodes would make most loop rules blind or mis-depth them — explicitly listed as a W1 obligation ([ADR-0007 §Consequences](../adr/ADR-0007-python-tier-strategy.md)).

### 3.9 Events / expressions → typed events and `Shape`

All rows land as events attached to the enclosing `FunctionRecord` (including the synthetic module record, §3.3), each carrying `span`, `in_loop_depth`, `is_innermost`, `resolver_tier = syntactic`, `macro_origin` ([ir-v1.md §3.3](./ir-v1.md)).

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `call` | `CallEvent` | Fields `function` (`primary_expression`) and `arguments` (`argument_list` or `generator_expression`). |
| `argument_list` | `CallEvent.args` container | No fields; unnamed positional children (`expression`/`parenthesized_expression`) plus `keyword_argument`/`list_splat`/`dictionary_splat`. There is **no `positional_argument` kind**. |
| `keyword_argument` | `CallEvent.args` entry (keyword) | Fields `name` (`identifier`), `value` (`expression`) — needed for `apply(axis=1)`. |
| `attribute` | `Shape { kind: member_access }` / `AccessChainEvent` element | Fields `object` (`primary_expression`) and `attribute` (`identifier`). **Note the base field is `object`**, not C's `argument`/Rust's `value`. The `object` chain's leftmost name is the receiver root (§4). |
| `subscript` | `Shape { kind: subscript }` / `AccessChainEvent` element | Fields `value` (`primary_expression`) and `subscript` (`expression`/`slice`). Differs from C's positional `subscript_expression`; read field-wise. |
| `identifier` (as `function`) | `CallEvent.callee_kind = Direct` | Plain name; the alias map (§4) may rewrite it to `callee_path`. |
| `attribute` (as `function`) | method call → `callee_path` | `x.method()` / `pd.concat(...)`; callee text is a member chain. |
| `binary_operator` | `BinaryOpEvent` (arith) | Fields `left`, `operator`, `right`; operators `+ - * / // % ** << >> & | ^ @`. Note floor-div `//` is spelled `//` (distinct from a C comment). |
| `comparison_operator` | `ComparisonEvent` | Field `operators` (multiple; `== != < <= > >= <> in not in is is not`); the operands are **unnamed `primary_expression` children** (there is **no `operands` field**). **Python supports chained comparisons** (`a < b < c`) — W2 emits one `ComparisonEvent` per adjacent operand pair. `in` feeds `LOOP-007`. |
| `boolean_operator` | complexity contributor | Fields `left`, `operator` (`and`/`or`), `right`. |
| `unary_operator` | `Shape { kind: unary }` | Fields `operator` (`+ - ~` only) and `argument`. `not` is a **separate named `not_operator`** node (field `argument`), not a `unary_operator` operator. |
| `assignment` | `AssignmentEvent` | Fields `left`, `right`, optional `type` (annotation). |
| `augmented_assignment` | `AssignmentEvent` | Fields `left`, `operator` (`+= -= ...`), `right`. This is the `STR-001` input. |
| `named_expression` | `AssignmentEvent` (walrus) | Fields `name` (`identifier`), `value` (`expression`); `:=` binds a local. |
| `if_statement` | complexity contributor (branch) | Fields `condition`, `consequence` (`block`), optional `alternative` (`elif_clause`/`else_clause`). |
| `conditional_expression` | complexity contributor | Ternary `a if c else b`. |
| `try_statement` | complexity + `LOOP-009` context | Field `body` (`block`); positional `except_clause`/`else_clause`/`finally_clause` children. A `try_statement` inside a loop body is the `PERF-PY-LOOP-009` shape. |
| `with_statement` | resource-context marker | Field `body`; `with_clause` items. Feeds file-handle kind inference (§4). |
| `lambda` | callable expression | **Named node** with fields `parameters` (`lambda_parameters`) and `body` (an anonymous `lambda` keyword token also exists). W2 treats it as an inline callable, not a `FunctionRecord` (matches the C inline-lambda posture). |
| `await` | `AwaitEvent` (additive) | **Named node** wrapping a required `primary_expression` child (an anonymous `await` keyword token also exists) — not a `FunctionRecord`. W2 emits one `AwaitEvent` per named `await` (additive IR, §4) while STILL descending to extract the awaited expression's events normally; `ASYNC-002` consumes `FunctionUnit.awaits` inside a loop body (§6). |

### 3.10 Literals / basic shapes / comments

| `tree-sitter-python` kind | IR target | W2 plan / Python idiom note |
|---|---|---|
| `integer` / `float` | `Shape { kind: literal }` | Named tokens. |
| `true` / `false` / `none` | `Shape { kind: literal }` | Named tokens. |
| `string` | `Shape { kind: literal }` | Positional `string_start`/`string_content`/`string_end`, plus `interpolation` children for f-strings. |
| `interpolation` | f-string hole | Fields `expression`, optional `format_specifier`/`type_conversion`. **Crucial for `ALLOC-001`:** an f-string is a `string`, not a constructor call, so a correct `ALLOC-001` never fires on it (§9). |
| `concatenated_string` | implicit string concat | Adjacent string literals; treated as one literal shape. |
| `list` / `dictionary` / `set` / `tuple` | `Shape { kind: other }` + **binding-kind evidence** | The container literal kind is the strongest static evidence for the binding-kind lattice (§4): `{}`/`[..]`/`set()`/`(..)`. |
| `comment` | LOC comment | See §3.1. |

Python idiom note (memory/`sizeof`): Python has **no `sizeof` and no by-value structs**, so `Shape.sizeof_operand` (MEM-001 input) is never populated, and `TYPE-001`/`TYPE-002` never fire (retired, §9). Allocation/churn (`ALLOC` family) and string building (`STR` family) map to constructors/`+=` in the events above — a W3 heuristic-mapping concern, not extraction.

## 4. Binding/alias inference & additive IR fields

[ADR-0007 §5](../adr/ADR-0007-python-tier-strategy.md) ships a *minimal* inference pass in M3 because dynamic typing is the Python false-positive source and the flagship rules are dishonest without it. The W0 canary validates this directly: `CTL-IN-SET` shows set-membership controls (72) nearly equal list hits (83) on chunkhound, so `LOOP-007` would be ~46% FP without the kind lattice ([python-canary.md §4](./../evidence/python-canary.md)).

**Inference scope (W2):**

1. **Per-file import-alias map** — `import pandas as pd` → `pd → pandas`; `from pandas import DataFrame as DF` → `DF → pandas.DataFrame`. Used to populate `CallEvent.callee_path` (an existing optional field), so `pd.concat(...)` is matchable as `pandas.concat`.
2. **Per-function single-assignment local kind lattice** — `BindingKind` ∈ `{list, dict, set, str, bytes, dataframe, series, ndarray, filehandle, unknown}`. Assigned from container literals, constructor calls, `with open(...)` (filehandle), parameter annotations, and import-alias roots; a name reassigned to a conflicting kind, or never resolved, stays `unknown`. **Single-assignment only** (no dataflow) — the deliberate ceiling.
3. **Call-receiver root** — the leftmost name of an `attribute` chain (`self.results.append` → root `self`; `conn.execute` → root `conn`).
4. **`async` flag** — `async def` sets `FunctionRecord.is_async`.
5. **Parameter annotations** → `FunctionRecord.param_types`.

**Additive IR fields (W2 code; all `#[serde(default)]`, IR-internal, not serialized into `metrics.json`):**

| Struct | New field | Type | Default | Consumer |
|---|---|---|---|---|
| `FunctionRecord` | `is_async` | `bool` | `false` | `ASYNC-001/002` gate |
| `FunctionRecord` | `is_module_scope` | `bool` | `false` | synthetic module record (§3.3) |
| `AssignmentEvent` | `binding_kind` | `BindingKind` (new enum, snake_case) | `unknown` | `LOOP-007`, `STR-001`, `ALLOC-003` |
| `CallEvent` | `receiver_root` | `Option<String>` | `None` (omitted on wire) | `N1-001`, `PANDAS-001/002`, `ASYNC-001` |
| `FunctionUnit` | `awaits` | `Vec<AwaitEvent>` (new additive event) | `[]` | `ASYNC-002` |

**Additive `AwaitEvent` stream:** `await` uses the additive `FunctionUnit.awaits: Vec<AwaitEvent>` stream (each carries the shared `EventMeta`: span, loop depth, innermost, resolver tier, macro origin), while the per-file import-alias map folds into the existing `CallEvent.callee_path` (no field is added for it). **Confidence rule:** a finding that used any inference above is capped at **MEDIUM** and its `resolver_tier` stays `syntactic` — rules must never claim `semantic` ([ADR-0007 §5](../adr/ADR-0007-python-tier-strategy.md)); the central `apply_upgrade_hooks` gate reverts unbacked claims. **No schema bump comes from inference** — only the `"python"` language token widens the schema (1.2 → 1.3, W3).

## 5. Import-graph resolution (best-effort) & `__init__.py`

Per [ADR-0007 §7](../adr/ADR-0007-python-tier-strategy.md), resolution is **best-effort**:

- **Resolve** relative imports (`from . import x`, `from ..pkg import y`) and scan-root packages to real files, using the includer's package path and the scan root(s).
- **Unresolved internal imports stay data-only with NO finding** (`to_path = None`, `resolved = false`) — this mirrors the Rust N1 decision that removed an entire FP stream, and it gates `ARCH-001`.
- **External imports** (stdlib/third-party, e.g. `import pandas`) keep their raw module text in `to_path` with `resolved = false` (they are not in-repo edges).

**`__init__.py` re-export hub (explicit decision).** A package `__init__.py` is extracted as a normal module; its `from .sub import Name` / `from .sub import *` statements produce `IncludeEdge`s to `sub.py` (the hub→sub edges), and those edges still resolve. **W2 keeps MODULE-level edges only:** a consumer's `from pkg import Name` resolves to `pkg/__init__.py` (the module dependency), NOT through to the re-exporting submodule. **Name-level re-export tracing through an `__init__.py` hub is explicitly DEFERRED to the T2 `python-resolver` milestone**; wildcard re-exports and ambiguous multi-source re-exports stay **data-only** (no edge, no finding). Rationale: guessing the re-export target produces the exact FP stream the Rust N1 rule killed.

## 6. `async def` handling

- `async def` parses as `function_definition` with a leading `async` modifier token → `FunctionRecord.is_async = true` (§4).
- `await x` parses as a **named `await` node** wrapping a required `primary_expression` child (an anonymous `await` keyword token also exists). `ASYNC-002` (await in loop) consumes the additive `FunctionUnit.awaits` stream (§4) — one `AwaitEvent` per `await`, with `meta.in_loop_depth >= 1` marking an await inside a loop body — so the rule never reads engine node kinds.
- `ASYNC-001` (blocking call in `async def`) fires on a call in the blocking family (`time.sleep`, `requests.*`, sync DB/`open`) **inside a `is_async` function**. The `async def` gate is the highest-leverage gate in the canary: `CTL-SLEEP-SYNC` shows 166 production blocking calls in **sync** functions vs **0** real hits on Aider — ungated, `ASYNC-001` is pure noise ([python-canary.md §4](./../evidence/python-canary.md)).

## 7. Notebooks (`.ipynb`) — decision

**`.ipynb` files are OUT OF SCOPE for M3 and are documented as an accepted limitation** (W4 accepted-limitations page). Rationale: (a) a notebook is JSON, so extraction requires a separate cell-decode layer plus a line-offset mapping onto a synthesized source unit, which complicates `ParseHealth` honesty (a cell can fail while the file parses) and span determinism; (b) `.py` coverage dominates AI Python repos (the W0 corpora: 1,014 `.py` files, 0 parse errors); (c) the single-binary/zero-dependency model forbids a notebook runtime. Dispatcher behavior: `.ipynb` is **not** recognized as Python (`language_of` returns `None`), so it is counted as an unrecognized file, never mis-parsed. A future `python-notebook` plan may add cell parsing with offset mapping.

## 8. Tier & resolver scope

The Python backend is **syntactic-tier only** (T1: AST + binding awareness). Every Python record/event carries `resolver_tier = syntactic`, and Python files report `parse_health.resolver` as absent/`None`. The semantic resolver (`clang-sys`/libclang) is **C-only** and does not apply to Python ([ADR-0007 §4, §8](../adr/ADR-0007-python-tier-strategy.md); [modules.md §5.3](./modules.md)). T2 (cross-module resolver) is the future `python-resolver` plan; T3 (runtime profiles) is CONFIRMED-1.

Backend/interface contract to mirror (from `src/scanner/backends/tree_sitter_c/mod.rs`, [modules.md §4.2](./modules.md)): given `(path, source bytes)`, emit that file's IR records/events; convert all engine-native spans at the boundary (never leak node kinds); preserve **parse honesty** — `ERROR`/missing nodes become `ParseHealth { status: partial, error_regions, error_node_count }`, a failed parse yields `ParseHealth { status: failed }`, and the function never panics on malformed input.

## 9. Durable rule disposition table

This table resolves the "mirror the full 23-rule catalog vs retire-10" conflict (R-pf-087/R-pf-088): the retirement set is **not** arbitrary — each retired rule is retired because its *premise* does not transfer to CPython (cited), and each kept rule carries a canary-measured reason. The target lenient set is **13 transferred/reframed mirrors + Python-native additions**. Confirmed by [ADR-0007 §6](../adr/ADR-0007-python-tier-strategy.md) and the W0 canary ([python-canary.md §7](./../evidence/python-canary.md)). Severity is the intended Python severity; W3 finalizes.

### 9.1 Retired (10) — premise does not transfer to CPython

| Source rule (C/Rust) | Verdict | Reason |
|---|---|---|
| `PERF-*-TYPE-001` | **RETIRE** | No by-value struct layout; class instances are references, not sized aggregates. |
| `PERF-*-TYPE-002` | **RETIRE** | No struct padding/alignment to reason about. |
| `PERF-*-MEM-001` | **RETIRE** | No `sizeof` expression in Python (`Shape.sizeof_operand` never populated, §3.10). |
| `PERF-*-ALLOC-002` | **RETIRE** | No `free`; CPython refcounting + GC own deallocation. |
| `PERF-*-STR-002` | **RETIRE** | `len()` is O(1) (object header); `CTL-LEN-IN-LOOP` shows it is common-but-harmless (117/334 prod hits) — shipping it would be ~2.5/kLOC of dead advice. |
| `PERF-*-STR-003` | **RETIRE** | Memory safety is guaranteed by the runtime; no resolved-format-length premise. |
| `PERF-*-LOOP-004` | **RETIRE** | Interpreter-bound; the per-iteration div/mod premise is a bytecode-era micro-opt (`*` vs `%`), invisible at the AST level and dominated by dispatch. |
| `PERF-*-LOOP-005` | **RETIRE** | A "global store" is a module-dict write, not a cache-hostile memory access. |
| `PERF-*-LOOP-006` | **RETIRE** | Every Python call is dynamic dispatch; the rule would fire on all code. |
| `PERF-*-LOOP-008` | **RETIRE** | Attribute chains are idiomatic and PEP 659-cached; not a cost signal. |

### 9.2 Transferred / reframed mirrors (13)

| Source rule | Python id | Verdict | Reason / canary evidence |
|---|---|---|---|
| `ALLOC-001` | `PERF-PY-ALLOC-001` | **REFRAME** | Ship **only as loop-invariant construction** (value independent of the loop variable), Info-first; **f-string-in-loop is NOT a finding** — 98–99.6% of raw hits were f-strings (§3.10, canary §3.1). |
| `ALLOC-003` | `PERF-PY-ALLOC-003` | **TRANSFER — flagship** | Superlinear accumulation; dense on both corpora (7.0/6.0 per kLOC prod), no Ruff analogue. Receiver root + `CTL-APPEND-LOCAL`/`CTL-APPEND-OUTER-RESET` gates are load-bearing. |
| `STR-001` | `PERF-PY-STR-001` | **TRANSFER/REFRAME** | `str +=` in loop, **MEDIUM/LOW**, honest about CPython refcount==1 amortisation; corpus-dependent (Aider 2.0 vs chunkhound 0.05 per kLOC). `kind` gate (`str`) is load-bearing vs `CTL-INT-ACC`/`CTL-AMBIG-ACC`. |
| `LOOP-001` | `PERF-PY-LOOP-001` | **TRANSFER** | Nesting ≥ 3; Info, rare (7/15). `CTL-DEPTH2` keeps it rare. |
| `LOOP-002` | `PERF-PY-LOOP-002` | **REFRAME** | **Drop `print`/`input`** (72–83% of hits); keep file/network/subprocess/socket I/O; `print` at most Info. Honest that this is partly folklore. |
| `LOOP-003` | `PERF-PY-LOOP-003` | **TRANSFER** | `range(len(seq))` → `enumerate`; Info, near-zero (0.12/0.02 per kLOC); the `len()` gate keeps it FP-free (`CTL-RANGE-PLAIN`). |
| `LOOP-007` | `PERF-PY-LOOP-007` | **TRANSFER — flagship** | Membership/index scan; must be **loop-scoped** and **kind-gated** to list (exclude `set`/`dict` — `CTL-IN-SET` 72 vs 83). |
| `MEM-002` | `PERF-PY-MEM-002` | **TRANSFER (Info)** | Container copy/`deepcopy` in a loop; partial Ruff overlap (`PERF402`). |
| `CALL-001` | `PERF-PY-CALL-001` | **TRANSFER (Info)** | Recursion framed as `RecursionError`/depth, not C stack cost; low frequency. |
| `CALL-002` | `PERF-PY-CALL-002` | **TRANSFER (Info)** | Cyclomatic ≥ 15; low-but-plausible (2.44/1.83 per kLOC). |
| `ARCH-001` | `PERF-PY-ARCH-001` | **TRANSFER, measure first** | Import cycles / deep chains; abundant structure but thresholds not decision-grade in W0 — defer thresholds to W2/W4. |
| `ARCH-002` | `PERF-PY-ARCH-002` | **TRANSFER, Info/low** | Hot-component aggregation; expected noisy on Python; **not measured in W0** — measure in W4. |
| `ARCH-003` | `PERF-PY-ARCH-003` | **TRANSFER, Info/low** | Name-based call coupling is noisier on Python (duck typing, ubiquitous `get`/`save`/`run`); **not measured in W0** — measure in W4. |

### 9.3 Python-native additions

| Python id | Rule | Severity | Reason / evidence |
|---|---|---|---|
| `PERF-PY-LOOP-009` | `try`/`except` inside a loop | Info | Surfaced by the Ruff `PERF203` baseline; real footprint; honest that the zero-cost-`try` claim is version-dependent. |
| `PERF-PY-PANDAS-001` | row-wise pandas (`iterrows`/`itertuples`/`apply(axis=1)`) | Info | Required by ADR-0007 §6; near-zero on the W0 corpora (4 hits) — not pandas-heavy. Receiver-root gated. |
| `PERF-PY-PANDAS-002` | `concat`/`DataFrame.append` in a loop | Info | Required by ADR-0007 §6; **0 hits** on both corpora — ships with the frequency caveat; `.append` indistinguishable from `list.append` syntactically. |
| `PERF-PY-ALLOC-004` | `list()`/`dict()`/`set()` materialisation churn (eager re-materialisation of an iterable, esp. in a loop) | Info | Required by ADR-0007 §6 / plan native list. Mechanism: `list(gen)`/`dict(pairs)` forces an eager O(n) allocation + copy and defeats laziness. **Not measured in W0** (the canary had no row for it) — ships Info and is measured in W4. |
| `PERF-PY-N1-001` | DB/HTTP call in a loop (N+1) | **Flagship** | Native; real production footprint (chunkhound 66 prod; `CTL-N1-NOLOOP` 332). Receiver-root + call-name family gated. |
| `PERF-PY-ASYNC-001` | blocking call inside `async def` | **High-value** | Highest-value native rule; `async def` gate proven necessary (`CTL-SLEEP-SYNC` 166 vs 0). |
| `PERF-PY-ASYNC-002` | `await` in a loop | Native | 1.36/kLOC prod on the async corpus; loop-gating proven necessary (`CTL-AWAIT-NOLOOP`). |
| `PERF-PY-COMPREHENSION-001` | manual loop collapsible to a comprehension | Info | Cross-validated against Ruff `PERF401`; single-statement gate (`CTL-APPEND-MULTI`) is load-bearing. |
| `PERF-PY-EXECUTOR-001` | executor/event-loop creation per iteration | Info | Near-zero frequency (1 hit / 300k LOC); ships Info. |

### 9.4 Cut / demoted (explicit non-goals)

| Candidate | Verdict | Reason |
|---|---|---|
| import-in-loop | **CUT** | Modules are cached in `sys.modules`; the cost claim is folklore. |
| `re.compile` outside a loop | **CUT** | CPython's 512-entry `re` cache makes it folklore; **0 hits** in both corpora (`CTL-COMPILE-IN-LOOP`). |
| exception-driven control flow | **Info only** | Style/robustness, not a quantified performance cost; not shipped as a perf rule. |

## 10. Blind spots & accepted limitations (carried to W2/W4)

- **Inference is single-assignment and best-effort** — a name reused to a different kind degrades to `unknown`; flagships are MEDIUM-confidence, never `semantic`.
- **Attribute-path key is root-only** — `self.results` vs `self.tmp` share root `self` (the canary's disclosed undercount; `python-canary.md §9`). A full attribute-path key is a possible W2 refinement if `ALLOC-003` precision demands it.
- **`PANDAS-002`/`PANDAS-001` matching is name-based** — can fire on non-pandas objects with the same method names (accepted FP class).
- **`ARCH-*` thresholds are unmeasured** on Python; they ship conservative/Info and are **measured in W4** (not asserted).
- **`.ipynb` excluded** (§7); **`python-resolver` (T2)** and **runtime profiles (T3)** are out of scope.
- **Decorators** yield `MacroEvent`s but no rule consumes them yet (W3 decides).
