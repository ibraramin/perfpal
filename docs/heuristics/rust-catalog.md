# Perfpal — Rust Performance Heuristic Catalog (`PERF-RS-*`)

- **Status:** LANDED — plan `rust-backend`, phase R3 (workstreams C1 + C2), with the reporter catalog extension in workstream D1. Every entry below was transcribed **read-code-first** from the implemented rule functions (`src/analyzer/heuristics/rust/**`) and their emitted message/evidence, plus the Rust scanner backend (`src/scanner/backends/tree_sitter_rust/mod.rs`).
- **Scope:** the Rust language tier. Findings are **optimization candidates**, never certainties. Rule ids mirror the C catalog's families one-for-one ([catalog-v1.md](./catalog-v1.md)); the Tier 1/2 grouping and Signal/Cost ranks are **inherited** from the C counterpart (R3 implemented the full 23-rule catalog in one phase rather than by tier).
- **Tier:** Rust is **syntactic-tier only** — there is no Rust semantic resolver (`clang-sys`/libclang is C-only, [rust-backend.md §6](../architecture/rust-backend.md)). Every Rust record/event carries `resolver_tier = syntactic`, and Rust files omit `parse_health.resolver`. The confidence upgrades the C resolver performs (ARCH-003 resolved edges, ALLOC-002 proven-`fclose`, STR-003 resolved-format) have **no Rust equivalent**; RS-ARCH-003 keeps a symmetric resolved branch but it is unreachable while no Rust resolver exists.
- **Registry:** the analyzer registry is **language-tagged** (`analyzer::analyze`, `src/analyzer/mod.rs`): `PERF-C-*` entries run only on `FileRecord.language != "rust"` units and `PERF-RS-*` entries only on `"rust"` units, so a name-based C rule can never fire on a same-named Rust construct in a mixed scan. Rule ids are declared as shared consts in `src/analyzer/finding.rs`.
- **Downstream contract:** every finding carries `confidence` and `severity` (metrics.json schema); titles/evidence keys below are the exact strings the rules emit (evidence keys sort alphabetically in the serialized document).

---

## 1. Ranking & confidence model

The ranking axes and confidence model are those of [catalog-v1.md](./catalog-v1.md) §1 (Signal S1–S3 × Cost C1–C3; Tier T1 = C MVP set, T2 = C should-have). The same confidence bands apply:

- **HIGH** — pure syntax, low false-positive classes.
- **MEDIUM** — shape heuristic with known FP classes; FP sources documented per entry.
- **LOW** — approximation involved (size models, cross-file name matching).

Downgrade rules (applied centrally in `analyzer::apply_downgrade_hooks`, never per-rule): any finding whose span intersects a recognized `macro_rules!` alloc-wrapper pattern OR overlaps an event carrying `macro_origin = true` is capped at MEDIUM with `macro_suspect = true`; a finding overlapping a tree-sitter ERROR region is capped at MEDIUM with `error_node_overlap = true`. The central `apply_upgrade_hooks` gate reverts any unbacked `semantic` claim (defensive; no Rust rule emits one today).

Rust-specific `macro_origin` detection (scanner, `FuncScan::macro_origin_of`): true iff an event's span intersects a recognized alloc-wrapper span, **or** the event's `callee_name` equals a local `macro_rules!` definition name (a wrapper USE site). Non-call events match on span only.

---

## 2. Baseline stats (pillar 1a — always emitted, not heuristics)

Unchanged from [catalog-v1.md](./catalog-v1.md) §2, over Rust files: LOC per file, function/parameter/length distributions, cyclomatic + cognitive complexity, allocation-site count per function/file, `static`/`const` global count, and include/module counts. `FunctionRecord.alloc_site_count` for Rust now counts allocation-family call sites via the shared [`RUST_ALLOC_FAMILY`](../architecture/ir-v1.md) matcher (`src/ir/mod.rs`) — the same source of truth RS-ALLOC-001 consumes. Rust cyclomatic complexity = `1 + {if_expression, each match_arm, each loop, &&, ||, try_expression}`; cognitive uses the SonarSource-style approximation (if/loop/try cost `1 + nesting`, `match_arm`/`&&`/`||` cost a flat 1) — `rust-backend.md §8`.

---

## 3. Tier 1 — mirror of the C MVP set (15 heuristics)

All entries `(syntactic)`. Signal/Cost inherited from the C counterpart's ranking.

| ID | Name | Signal | Cost | Default confidence | Severity band |
|---|---|---|---|---|---|
| PERF-RS-ALLOC-001 | Heap allocation inside loop | S1 | C1 | HIGH | hot |
| PERF-RS-ALLOC-002 | `drop()` inside loop | S1 | C1 | HIGH | warn |
| PERF-RS-ALLOC-003 | Growable-buffer additive growth in loop | S1 | C2 | MEDIUM | hot |
| PERF-RS-STR-001 | String accumulation in loop (`push_str`/`push`) | S1 | C1 | HIGH | hot |
| PERF-RS-STR-002 | Length query recomputed in loop | S1 | C1 | HIGH | warn |
| PERF-RS-STR-003 | Unbounded `unsafe` memory operation | S2 | C1 | HIGH | warn |
| PERF-RS-LOOP-001 | Deep loop nesting (≥3) | S1 | C1 | HIGH | warn |
| PERF-RS-LOOP-002 | Expensive setup / I/O call inside loop | S1 | C2 | MEDIUM | hot |
| PERF-RS-LOOP-003 | Character-wise processing in loop | S2 | C1 | MEDIUM | info |
| PERF-RS-LOOP-004 | Integer div/mod with variable divisor in innermost loop | S2 | C1 | MEDIUM | info |
| PERF-RS-CALL-001 | Direct recursion | S2 | C1 | HIGH | info |
| PERF-RS-CALL-002 | High cyclomatic complexity | S2 | C1 | HIGH | warn |
| PERF-RS-ARCH-001 | Module-graph fan-out / cycles | S1 | C1 | HIGH | warn |
| PERF-RS-ARCH-002 | Hot-component aggregation | S1 | C1 | HIGH | hot |
| PERF-RS-ARCH-003 | Cross-module call coupling (name-based) | S2 | C2 | LOW (MEDIUM resolved) | info |

### Detail

**PERF-RS-ALLOC-001 — Heap allocation inside loop** `(syntactic)`
- Rust idiom: Rust allocation is spread across qualified constructors (`Vec::new`, `Box::new`, `Rc::new`, `Arc::new`, `String::new`, `Vec::with_capacity`, `String::with_capacity`, `VecDeque::new`, `HashMap::new`, `HashSet::new`, `BTreeMap::new`, `BTreeSet::new`), methods (`to_vec`, `to_owned`, `to_string`, `collect`, `with_capacity`) and macros (`vec!`, `format!`).
- Detects: any direct or macro `CallEvent` at `in_loop_depth ≥ 1` for which `RUST_ALLOC_FAMILY::matches_site` holds — a qualified path equal to a curated constructor, a bare method name, or a curated macro when `is_macro`. Hot/HIGH; `resolver_tier` inherited from the call event. Evidence `callee`, `loop_depth`, `alloc_site_index` (1-based index among the function's alloc-family sites in declaration order — the same count `alloc_site_count` reports).
- Why: per-iteration allocation is allocator churn that dominates runtime; hoisting the allocation or reusing a buffer is the candidate.
- Accepted FP classes / limitations: **`clone` is deliberately excluded** (ubiquitous, not allocation pressure by itself — an accepted false negative); a `Vec::new()` whose capacity is reserved once still fires (the syntactic tier cannot prove buffer reuse); an allocation generated **inside** a macro body is invisible beyond the macro-invocation event itself (`token_tree` blind spot).

**PERF-RS-ALLOC-002 — `drop()` inside loop** `(syntactic)`
- Rust idiom: Rust has no `free`; the deallocation analogue is an explicit `drop(value)`.
- Detects: a **DIRECT, non-macro** `drop` call at `in_loop_depth ≥ 1`. Default Warn/HIGH. **Co-occurrence escalation:** when the same function also contains any alloc-family site at `in_loop_depth ≥ 1` (an RS-ALLOC-001 site), the finding escalates to Hot with additive evidence `alloc_cooccurrence = "true"` (mirrors the C malloc/free pairing). Evidence `callee`, `loop_depth`[ , `alloc_cooccurrence`].
- Accepted FP classes: `drop` is also the explicit destructor call on a legitimately scoped value; a balanced allocate-per-iteration + drop-per-iteration is still surfaced as churn, never a bug verdict.

**PERF-RS-ALLOC-003 — Growable-buffer additive growth in loop** `(syntactic)` — **REFRAMED**
- Reframing (mandatory): Rust has no `realloc`; the additive-growth analogue of C's `realloc(p, n + k)` is a method call to `reserve` / `reserve_exact`, which grows a growable buffer's capacity by an additive amount (`v.reserve(chunk)` per iteration ⇒ repeated reallocation and copy). Multiplicative capacity doubling is the documented fix.
- Detects: a **DIRECT, non-macro** `reserve` / `reserve_exact` call at `in_loop_depth ≥ 1`. Hot/MEDIUM (MEDIUM because the syntactic tier cannot see whether the reserve is amortized or a one-shot warm-up). Evidence `callee`, `loop_depth`, `growth = "additive"`.
- Accepted FP classes / limitations: a reserve whose argument is a loop-invariant large constant still fires (may be a deliberate one-time pre-allocation); `with_capacity` growth is **not** this rule's shape — it is an allocation handled by RS-ALLOC-001.

**PERF-RS-STR-001 — String accumulation in loop (`push_str`/`push`)** `(syntactic)`
- Rust idiom: appending to a growable string with `s.push_str(...)` / `s.push(...)`.
- Detects: a **DIRECT, non-macro** `push_str` / `push` method call at `in_loop_depth ≥ 1` whose receiver resolves to a **bare identifier** (`s.push_str` ⇒ `s`; UFCS `String::push_str` falls back to an Identifier arg 0). Hot/HIGH. Evidence `callee`, `dest`, `loop_depth`.
- Accepted FP classes / honest limitations: **compound-assignment growth (`s += "x"`) is INVISIBLE to this rule** — it is not a `CallEvent` and `AssignmentEvent` carries no operator field; **`push` also matches `Vec::push` and other container pushes** (the syntactic tier cannot see the receiver's type), so non-string pushes inside a loop fire as well. Candidate-grade, never a bug verdict.

**PERF-RS-STR-002 — Length query recomputed in loop** `(syntactic)`
- Rust idiom: recomputing `s.len()` / `s.is_empty()` / `s.capacity()` per iteration.
- Detects: a **DIRECT, non-macro** `len` / `is_empty` / `capacity` method call at `in_loop_depth ≥ 1` whose receiver is a bare identifier, **suppressed** when an assignment to that same identifier lies inside the innermost enclosing loop (max-depth span-containing loop, earliest-span tie-break; function-span fallback when no enclosing loop is modeled — the C `strlen` reassignment suppression). Warn/HIGH. Evidence `identifier`, `loop_depth`.
- Accepted FP classes: `Vec::len` is O(1) and may be deliberately queried per iteration; a receiver mutated through a **method** (not an assignment) is not detectable.

**PERF-RS-STR-003 — Unbounded `unsafe` memory operation** `(syntactic)` — **REFRAMED**
- Reframing (mandatory): Rust is memory-safe by default, so C's "unbounded string op" (`strcpy`/`sprintf`/`gets`) has no direct analogue. The real unbounded operations live behind `unsafe`: `copy_nonoverlapping`, `copy`, `from_raw_parts`, `from_utf8_unchecked`, `set_len` — length unchecked by the compiler and easy to get wrong.
- Detects: any **DIRECT, non-macro** call in that family, at **any loop depth** (mirrors the C rule's anywhere-firing behavior), matched on `callee_name` or on a `::`-suffix of `callee_path` (`std::slice::from_raw_parts`, `str::from_utf8_unchecked`). Warn/HIGH. Evidence `callee`.
- Accepted FP classes / limitations: a correctly size-checked `unsafe` block still fires (the rule surfaces the unchecked-op surface, not a proven defect); a user-defined method named `copy` also fires (documented name-collision FP class).

**PERF-RS-LOOP-001 — Deeply nested loops (≥3)** `(syntactic)`
- Detects: every `LoopNode` whose `depth` is **exactly** `LOOP_001_MIN_DEPTH` (3) — one finding per maximal nest entry (a depth-5 chain yields one finding at its depth-3 ancestor). Warn/HIGH; threshold `("min_nesting_depth", 3)`. Evidence `nest_depth`, `max_depth` (max depth among inclusive descendants).
- Accepted FP classes / limitations: a deep nest over tiny fixed bounds is legitimate — the syntactic tier cannot bound trip counts; macro-generated loops are invisible (`token_tree` blind spot).

**PERF-RS-LOOP-002 — Expensive setup / I/O call inside loop** `(syntactic)`
- Detects: a Rust call at `in_loop_depth ≥ 1` in one of three curated groups — **macro** `println!`/`print!`/`eprintln!`/`eprint!`/`write!`/`writeln!`/`dbg!` (matched only when `is_macro`); **method** `read_to_string`/`write_all`/`read_to_end`/`flush`/`sync_all`/`recv`/`send`/`recv_from`/`send_to`/`read_line`; **qualified** `File::open`/`File::create`/`fs::read`/`fs::write`/`fs::read_to_string`/`Command::new`/`TcpStream::connect`/`UdpSocket::bind`/`env::var`/`env::args` (matched as `path == entry` or a `::entry` suffix).
- Severity: Hot/MEDIUM by default, lowered to **Warn/MEDIUM** when the call sits inside a conditional branch within the loop (`conditional_guarded`). Evidence `callee`, `loop_depth`, `conditional_guarded`.
- Accepted FP classes: an intentionally interleaved debug/log print stays candidate-grade (Warn when guarded); `send`/`recv` on a non-blocking channel also fire; `token_tree`-opaque macro windows cannot be inspected beyond the invocation event.

**PERF-RS-LOOP-003 — Character-wise processing in loop** `(syntactic)` — **REFRAMED**
- Reframing (mandatory): the C rule caught byte-at-a-time stdio (`fgetc`/`putc`). The Rust analogue is per-character/per-byte iterator or reader work: `chars`, `bytes`, `char_indices`, `byte_indices`, `read_byte`.
- Detects: a **DIRECT, non-macro** method call in that family at `in_loop_depth ≥ 1` whose `statement_role == Primary` (the call IS the loop body's primary statement). Info/MEDIUM. Evidence `callee`, `loop_depth`.
- Accepted FP classes / honest limitations: buffering is often already provided by the reader (`BufReader`), so a single-element read per iteration can be deliberate; `chars()` in a `for` header is not in the loop body and correctly never fires (iteration itself is the intended pattern).

**PERF-RS-LOOP-004 — Integer div/mod with variable divisor in innermost loop** `(syntactic)`
- Detects: a `BinaryOpEvent` with op `/` or `%`, `meta.is_innermost`, and a **non-Literal** divisor. Info/MEDIUM. Evidence `op`, `divisor_kind`[ , `divisor` (when the divisor is an identifier)].
- Why: real integer division runs per iteration; constant divisors are strength-reduced by the compiler (Rust `const`/literal values count).
- Accepted FP classes: constant-foldable **non-literal** divisors (a `const` identifier, `size_of::<T>()`) fire by design — the syntactic tier cannot prove constness; the Info band keeps them candidate-grade.

**PERF-RS-CALL-001 — Direct recursion** `(syntactic)`
- Detects: per function, DIRECT calls whose `callee_name` equals the enclosing function's name; ONE finding at the earliest self-call site by `(start_line, start_col)`. Info/HIGH. Evidence `function`, `self_call_sites`.
- Accepted limitations: name-based (a same-named method on another type in scope can false-fire); recursion **depth** is not bounded (presence-only, catalog deferral); mutual recursion is a later cross-file wave; indirect calls never count.

**PERF-RS-CALL-002 — High cyclomatic complexity** `(syntactic)`
- Detects: `FunctionRecord.cyclomatic_complexity ≥ CALL_002_MIN_COMPLEXITY` (15), reported per function; threshold `("min_complexity", 15)`. Warn/HIGH. Evidence `function`, `complexity`, `threshold`.
- Accepted approximation: Rust complexity counts `if_expression`, each `match_arm`, loops, `&&`/`||`, and `try_expression` (backend formula, `rust-backend.md §8`) — `match`-heavy code legitimately scores high.

**PERF-RS-ARCH-001 — Module-graph fan-out / cycles** `(syntactic)`
- Rust idiom: the C include graph maps onto Rust's **`use` import graph**. An **internal** edge is `resolved && to_path.is_some() && !is_mod` (the scanner resolves internal `use` paths to the owning `.rs` file; external crate paths stay `Angle`; unresolved internal edges normalize to `to_path=None`). **`mod` declaration edges are excluded:** a bare `mod child;` yields parent→child while the child's ordinary `use super::…` yields child→parent, so counting declarations made every ordinary parent/child module pair a false cycle (N1). Module structure remains available in `include_graph` data, but this rule reports on imports only.
- Detects, over internal `use` edges across all Rust units:
  1. **cycles** — one finding per `break_cycles`-broken edge (Warn/HIGH), anchored at that pair's earliest retained directive span; evidence `kind="cycle"`, `coupled_with`.
  2. **deep chains** — transitive import depth over the kept DAG; each file at depth ≥ `ARCH_001_MAX_INCLUDE_DEPTH` (8) emits exactly one finding (Warn/MEDIUM) anchored at the edge targeting its max-depth successor; evidence `kind="deep_chain"`, `include_depth`.
  3. **hubs** — `use` fan-out/fan-in ≥ `ARCH_001_HUB_FANOUT` (12) (Warn/MEDIUM), anchored at the file's first non-colliding directive or the synthetic line-1/col-1 span for a pure importer; evidence `kind="hub"`, `fan_out`, `fan_in`.
- **The C rule's fourth sub-case (unresolved internal `mod`/`use` edges → Info/MEDIUM `kind="unresolved_include"`) is intentionally NOT mirrored.** The longest-file-backed-prefix resolver cannot see items defined at the crate root (`use crate::Config;`), items in inline modules (`mod helper { … }` + `use crate::helper::run;`), or modules behind raw identifiers — so ordinary, rustc-resolvable Rust produced a large false-positive class of "external or build-generated dependency" findings. Rust unresolved edges remain in `include_graph` as data (`to:null`, `resolved=false`); they emit no finding.
- Accepted FP classes: deep/hub thresholds are proxy signals for rebuild and coupling cost, not runtime; macro-generated modules are invisible. `mod`-only structure (declaration fan-out, deep module nesting with no cross-module imports) is intentionally outside this rule.

**PERF-RS-ARCH-002 — Hot-component aggregation** `(syntactic)`
- Pure aggregation over other outputs; no independent detection. Identical IR-only composite over Rust component directories: `complexity_sum ↓ → max_loop_depth ↓ → alloc_site_density ↓ → path_prefix ↑`, **rank-1 component(s) only**, floor `ARCH_002_MIN_COMPLEXITY_SUM` (15). Hot/HIGH, anchored at the component's highest-complexity function. Evidence `path_prefix`, `complexity_sum`, `max_loop_depth`, `alloc_site_density`, `function_count`.
- `finding_density` is deliberately excluded (heuristic purity); `alloc_site_density` reflects the R3 Rust alloc family. **Rank-1 only:** `path_prefix` participates in the tie equality and directory keys are unique, so the tie set is always a single component — there is no multi-way co-fire.

**PERF-RS-ARCH-003 — Cross-module call coupling (name-based)** `(syntactic)`
- Detects: ONE Info finding per language-scoped `cross_file_call_edges` edge — DIRECT calls in a function whose callee name matches a definition in another file of the **same language** (file-local matches suppress; multi-definition ambiguity → lexicographically smallest path). Name-based fallback Info/**LOW** with evidence `callee_name`, `matched_path`.
- Resolved branch (symmetry only): a resolved edge carries MEDIUM/`resolver_tier: semantic` and additive evidence `resolved="true"`; **unreachable while no Rust resolver exists** — the central upgrade gate would revert any unbacked claim.
- Accepted FP classes: same-name functions in different files (the documented LOW-confidence hypothesis class); dedup per `(function, callee name)` keeping the earliest span.

---

## 4. Tier 2 — mirror of the C should-have set (8 heuristics)

All entries `(syntactic)`.

| ID | Name | Signal | Cost | Default confidence | Severity band |
|---|---|---|---|---|---|
| PERF-RS-TYPE-001 | Large struct passed by value | S2 | C2 | LOW | warn |
| PERF-RS-TYPE-002 | Struct member ordering (padding candidate) | S2 | C2 | LOW | info |
| PERF-RS-MEM-001 | `size_of` passed as an element count | S2 | C2 | MEDIUM | warn |
| PERF-RS-MEM-002 | Block copy/clone family in loop | S2 | C1 | MEDIUM | info |
| PERF-RS-LOOP-005 | Global (`static`/`const`) written inside loop | S2 | C2 | MEDIUM | info |
| PERF-RS-LOOP-006 | Indirect call in innermost loop | S2 | C2 | LOW | info |
| PERF-RS-LOOP-007 | Linear membership scan nested in loop | S2 | C2 | LOW | info |
| PERF-RS-LOOP-008 | Member-chain dereference in innermost loop | S2 | C2 | MEDIUM | info |

- **TYPE-001/002** share the language-parameterized LP64 layout model (`estimate_struct_size_with`) so Rust and C size arithmetic stay byte-identical; only the per-member byte mapping differs (Rust type text: `i8`/`u8`/`bool` = 1, `i16`/`u16` = 2, `i32`/`u32`/`f32`/`char` = 4, `i64`/`u64`/`isize`/`usize`/`f64` = 8, `i128`/`u128` = 16, references/pointers = 8). LOW confidence is mandatory — no real layout at MVP. Shared FP classes: tuple-struct positional members are name-bearing synthetics; nested-struct sizing is unmodeled (a struct-typed member counts conservatively as 8 bytes); generic/unknown member types (`Vec<T>`, named types) also count as 8.
- Implementation note (R3) **TYPE-001**: a `FunctionRecord.param_types` entry that is a **bare named type** (trimmed; ASCII alphanumerics/underscore only — excludes `&T`, `*T`, `Vec<T>`, `[T; N]`, `(A, B)`, `a::T`, slices, and any whitespace) resolving to a same-crate `StructDef` with `kind == Struct` whose estimate ≥ `TYPE_001_MIN_SIZE_BYTES` (32). **ONE finding PER FUNCTION** aggregating every qualifying parameter (parameter spans are not modeled, so per-parameter findings would share the function-record span and collapse under deterministic dedup). Warn/LOW; evidence `params`, `structs`, `estimated_sizes`, `threshold`. Definition lookup: the function's **own file wins**, else the lexicographically smallest defining path. Accepted FP classes: `Copy`-cheap large arrays and intentionally-owned values still surface; tuple structs and re-exported aliases are not resolved.
- Implementation note (R3) **TYPE-002**: per `StructDef` with `kind == Struct`, `members.len() ≥ 2`, and (aligned estimate − packed member sum) ≥ `TYPE_002_MIN_PADDING_BYTES` (8). Info/LOW, anchored at the struct definition span; evidence `struct`, `padding_bytes`, `packed_sum`, `aligned_estimate`, `threshold`. **HONEST CAVEAT (mandatory):** Rust's default `repr(Rust)` is free to **reorder** fields automatically, so declared-order padding waste is not necessarily real — the rule is deliberately lower-confidence than the C analogue and is framed as a signal to check whether the type is `#[repr(C)]`/FFI-sensitive. Single-member structs never fire.
- Implementation note (R3) **MEM-001** (REFRAMED): C's `sizeof`-misuse shapes have no byte-count analogue in Rust because allocation goes through typed constructors. The real misuse is passing a **byte count where an element count is expected**: `Vec::with_capacity(size_of::<T>())` reserves `size_of::<T>()` *elements* — `size_of::<T>()` squared bytes. Detection: a DIRECT `with_capacity` call (covers `Vec::with_capacity` / `String::with_capacity`) whose argument list contains either a `Call` shape whose callee name starts with `size_of`/`size_of_val` (the nested call IS expressible in the IR: the backend records the nested callee text in `Shape::name`), or a `Sizeof` shape (fixture/C-style representation) carrying `sizeof_operand`. Warn/MEDIUM, one finding per call; evidence `callee`, `misuse` (`size_of_as_element_count`). Honest limitation: the nested-callee match is name-prefix based, so a user function named `size_of_thing` also matches; a computed byte count hoisted out of the argument (`let n = size_of::<T>(); with_capacity(n)`) is NOT detectable.
- Implementation note (R3) **MEM-002** (REFRAMED): Rust block-copy calls carry **no explicit size argument**, so the C rule's "non-literal size" guard collapses to presence-in-loop. Detects a DIRECT method call in `copy_from_slice`/`copy_nonoverlapping`/`extend_from_slice`/`clone`/`to_vec` at `in_loop_depth ≥ 1`. Info/MEDIUM by default, escalated to **Warn/MEDIUM** in the innermost loop; evidence `callee`, `loop_depth`, `innermost`. Accepted FP classes: per-element small-struct copies are idiomatic and cheap; `clone` on a cheap `Copy` type also fires (no element-size knowledge).
- Implementation note (R3) **LOOP-005**: per `AssignmentEvent` with `scope_class == Global` at `in_loop_depth ≥ 1` and an `Identifier` target. **DEDUP:** one finding per `(target identifier, innermost enclosing loop span)` pair, anchored at the **earliest** qualifying assignment span. Info/MEDIUM (scope classification is name-based against collected `static`/`const` items). Evidence `target`, `loop_depth`. Accepted FP class: accumulator updates that legitimately belong in the loop stay flagged (Info band); framing is optimization candidate, never bug/unsafe.
- Implementation note (R3) **LOOP-006**: per `CallEvent` with `callee_kind == Indirect` at `in_loop_depth ≥ 1` AND `is_innermost`. Info/LOW (no type info confirming the target); evidence `callee`, `loop_depth`. **HONEST LIMITATION (mandated):** the Rust backend classifies a closure or method called through a struct field (`self.callback()`, `ctx.handler(field)`) as **DIRECT**, so that dispatch class does **not** fire here — the existing D2 direct-classification FP class.
- Implementation note (R3) **LOOP-007**: a `ComparisonEvent` at `in_loop_depth ≥ LOOP_007_MIN_DEPTH` (2) where ONE side is a `Subscript` whose `index_ident` matches an induction variable of a span-containing `LoopNode` (Rust `for` patterns populate `induction_vars`) AND the other side is NOT a literal. Info/LOW; evidence `root` (when present), `index_ident`, `loop_depth`. Accepted FP class: comparing against a known-sentinel **variable** is not syntactically distinguishable from comparing against a live key.
- Implementation note (R3) **LOOP-008**: per `AccessChainEvent` with chain depth ≥ `LOOP_008_MIN_CHAIN_DEPTH` (3) at `in_loop_depth ≥ 1` AND `is_innermost`. Info/MEDIUM; evidence `root`, `chain_depth`, `loop_depth`. Accepted FP class (catalog-mandated): trait-object/vtable-style dispatch (`ctx.ops.fun`) is idiomatic Rust design, not a defect — it still reports at Info/MEDIUM, never suppressed.

---

## 5. Rust-specific limitations & blind spots

- **`token_tree` opacity (macro blind spot).** `macro_rules!` bodies and macro-invocation arguments parse as opaque `token_tree` nodes; macro-generated functions, structs, loops and calls yield **no IR record**, so no rule can fire on them and the macro-origin downgrade has no span to anchor to. The definition/invocation nodes themselves are structured and are recorded (`MacroEvent { definition | use }`); invocation bodies are not expanded or re-parsed (`rust-backend.md §5`).
- **No semantic resolver.** Rust is syntactic-tier only; no Rust equivalent of the C resolver's confidence upgrades exists. RS-ARCH-003's resolved branch is present for symmetry but unreachable.
- **D2 callee classification.** `identifier`/`scoped_identifier`/`field_expression`/`generic_function` callees are `Direct` (final-segment `callee_name`), so closure-field and trait-object method calls through a field are invisible to RS-LOOP-006. Documented FP class.
- **Macro-as-call surface.** Every `macro_invocation` yields BOTH a file-level `MacroEvent { kind: Use }` and a function-attached `CallEvent { is_macro: true }` (empty `args`, opaque token tree). Rules that consume `CallEvent`s gate on `is_macro` where a macro must not match a plain function of the same name (e.g. the alloc family's `vec`/`format`).
- **`alloc_site_count`** is populated from the same `RUST_ALLOC_FAMILY` matcher the heuristics use (qualified constructors + method names + `vec!`/`format!` macros), so stats and RS-ALLOC-001 count the same sites.

---

## 6. Severity guidance

Bands and semantics are those of [catalog-v1.md](./catalog-v1.md) §6: **hot** = likely dominant runtime/architecture impact; **warn** = measurable impact or structural risk; **info** = context signal. Severity is a prioritization hint, never a certainty claim; every finding remains an optimization candidate.
