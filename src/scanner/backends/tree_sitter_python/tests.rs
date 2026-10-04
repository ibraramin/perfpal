//! Python backend unit tests (plan `python-backend` W2). They lock the node-kind
//! assumptions, the synthetic module record, comprehension/depth handling, the
//! inference lattice/alias map, and the import-edge encoding against the
//! scanner's `resolve_python_edges` contract.

use super::*;
use crate::ir::{BindingKind, CalleeKind, IncludeForm, LoopKind, ParseStatus};
use crate::scanner::FunctionUnit;

fn extract_py(source: &str) -> Extraction {
    extract("test.py", source.as_bytes())
}

fn function<'a>(ex: &'a Extraction, name: &str) -> &'a FunctionUnit {
    ex.functions
        .iter()
        .find(|f| f.record.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

/// Synthetic module record: top-level script statements are captured.
#[test]
fn module_record_captures_top_level_loop() {
    let ex = extract_py("for i in range(3):\n    print(i)\n");
    let module = &ex.functions[0];
    assert_eq!(module.record.name, "<module>");
    assert!(module.record.is_module_scope);
    assert!(!module.record.is_async);
    assert_eq!(ex.file.function_count, 1);
    assert_eq!(module.loops.len(), 1);
    assert_eq!(module.loops[0].kind, LoopKind::For);
    assert_eq!(module.loops[0].depth, 1);
    assert_eq!(module.record.max_loop_depth, 1);
    // The `for` iterable `range(3)` is evaluated once at depth 0.
    let range = module
        .calls
        .iter()
        .find(|c| c.callee_name == "range")
        .unwrap();
    assert_eq!(range.meta.in_loop_depth, 0);
    assert!(!range.meta.is_innermost);
}

/// Nested comprehensions are real loops; each clause's iterable is evaluated at
/// that clause's OUTER depth.
#[test]
fn comprehension_loop_depth_and_iterable_outer_depth() {
    let src = "def f(m):\n    xs = [[j for j in range(m)] for i in range(m)]\n    return xs\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let depths: Vec<u32> = f.loops.iter().map(|l| l.depth).collect();
    assert_eq!(depths, [1, 2], "{:?}", f.loops);
    assert!(f.loops.iter().all(|l| l.kind == LoopKind::For));
    assert_eq!(f.record.max_loop_depth, 2);
    // Two `range(m)` iterables: the outer clause's at depth 0, the inner
    // clause's at depth 1 (once per outer iteration).
    let mut depths: Vec<u32> = f
        .calls
        .iter()
        .filter(|c| c.callee_name == "range")
        .map(|c| c.meta.in_loop_depth)
        .collect();
    depths.sort_unstable();
    assert_eq!(depths, [0, 1], "{:?}", f.calls);
}

/// A `for_statement` iterable is evaluated once at the outer depth; the body
/// stays in-loop and innermost.
#[test]
fn for_statement_iterable_is_outside_body_depth() {
    let src = "def f(seq):\n    for x in seq.items():\n        use(x)\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let items = f.calls.iter().find(|c| c.callee_name == "items").unwrap();
    assert_eq!(items.meta.in_loop_depth, 0);
    assert!(!items.meta.is_innermost);
    let use_call = f.calls.iter().find(|c| c.callee_name == "use").unwrap();
    assert_eq!(use_call.meta.in_loop_depth, 1);
    assert!(use_call.meta.is_innermost);
    assert_eq!(f.record.max_loop_depth, 1);
}

/// Alias map applied to the first callee-path segment only.
#[test]
fn alias_map_rewrites_callee_path_first_segment() {
    let src = "import pandas as pd\nfrom pandas import DataFrame as DF\n\n\
               def f():\n    pd.concat([DF()])\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let concat = f.calls.iter().find(|c| c.callee_name == "concat").unwrap();
    assert_eq!(concat.callee_path.as_deref(), Some("pandas.concat"));
    let frame = f.calls.iter().find(|c| c.callee_name == "DF").unwrap();
    assert_eq!(frame.callee_path.as_deref(), Some("pandas.DataFrame"));
}

/// An unaliased `from M import n` binds `n → M.n`.
#[test]
fn unaliased_from_import_resolves_callee_path() {
    let src = "from pandas import DataFrame\n\ndef f():\n    DataFrame()\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let frame = f
        .calls
        .iter()
        .find(|c| c.callee_name == "DataFrame")
        .unwrap();
    assert_eq!(frame.callee_path.as_deref(), Some("pandas.DataFrame"));
}

/// Single-assignment binding lattice: container literals and `+=` inherit the
/// resolved target kind.
#[test]
fn binding_kind_literal_and_augmented() {
    let src = "def f():\n    xs = []\n    xs += [1]\n    s = \"a\"\n    s += \"b\"\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let kinds: Vec<BindingKind> = f.assignments.iter().map(|a| a.binding_kind).collect();
    assert_eq!(
        kinds,
        [
            BindingKind::List,
            BindingKind::List,
            BindingKind::Str,
            BindingKind::Str,
        ],
        "{:?}",
        f.assignments
    );
}

/// Call receiver root of an attribute chain.
#[test]
fn call_receiver_root_is_chain_base() {
    let src = "class C:\n    def f(self):\n        self.results.append(1)\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let append = f.calls.iter().find(|c| c.callee_name == "append").unwrap();
    assert_eq!(append.receiver_root.as_deref(), Some("self"));
}

/// `async def` sets `is_async`.
#[test]
fn async_function_flag() {
    let ex = extract_py("async def fetch():\n    return 1\n");
    let fetch = function(&ex, "fetch");
    assert!(fetch.record.is_async);
}

/// Parameter annotations land in `param_types` (raw text) with `"unknown"` for
/// unannotated parameters.
#[test]
fn param_types_from_annotations() {
    let src = "def f(values: list[int], n):\n    return values\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.record.param_count, 2);
    assert_eq!(f.record.param_types, ["list[int]", "unknown"]);
}

/// Malformed source → `Partial` with outermost error regions; never panics.
#[test]
fn malformed_source_is_partial_with_regions() {
    let ex = extract_py("def broken(:\n    pass\n");
    assert_eq!(ex.parse_health.status, ParseStatus::Partial);
    assert!(ex.parse_health.error_node_count >= 1);
    assert!(!ex.parse_health.error_regions.is_empty());
    assert_eq!(ex.file.language, "python");
}

/// Decorators → `MacroEvent` `Use`; classes → `StructDef` with annotated body
/// members (methods are not members).
#[test]
fn decorator_macro_and_class_struct() {
    let src = "@cached\nclass C:\n    x: int = 1\n    def m(self):\n        pass\n";
    let ex = extract_py(src);
    assert!(
        ex.macros
            .iter()
            .any(|m| m.kind == MacroEventKind::Use && m.name == "cached")
    );
    let class = ex.structs.iter().find(|s| s.name == "C").unwrap();
    assert_eq!(class.members.len(), 1, "{:?}", class.members);
    assert_eq!(class.members[0].name, "x");
    assert_eq!(class.members[0].type_text, "int");
}

/// Import-edge encoding must match `resolve_python_edges` exactly.
#[test]
fn import_edge_encoding() {
    let src =
        "import a.b\nimport a.b as x\nfrom a.b import c\nfrom . import x\nfrom .mod import z\n";
    let ex = extract_py(src);
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|e| e.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(
        raws,
        ["a.b", "a.b", "a.b", ".x", ".mod"],
        "{:?}",
        ex.includes
    );
    assert!(
        ex.includes
            .iter()
            .all(|e| e.form == IncludeForm::Quoted && !e.resolved && !e.is_mod)
    );
    assert_eq!(ex.file.include_counts.quoted, 5);
    assert_eq!(ex.file.include_counts.angle, 0);
}

/// Wildcard from-import and `from x import *` encode as one module edge.
#[test]
fn wildcard_import_single_module_edge() {
    let ex = extract_py("from x import *\n");
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|e| e.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(raws, ["x"]);
}

/// Module-level assignments become `GlobalVar`s; class-body assignments do not.
#[test]
fn module_globals_and_class_exclusion() {
    let src = "LIMIT = 10\nclass C:\n    field: int = 1\n";
    let ex = extract_py(src);
    let names: Vec<&str> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["LIMIT"]);
    assert_eq!(ex.file.global_var_count, 1);
}

/// LOC: the single Python `comment` kind feeds the comment count.
#[test]
fn loc_comment_count() {
    let ex = extract_py("# a comment\nx = 1\n# another\n");
    assert_eq!(ex.file.loc.comment, 2);
    assert_eq!(ex.file.loc.code, 1);
}

/// Plain function calls AND member/method callees classify `Direct`;
/// subscript-result callees `Indirect`.
#[test]
fn callee_kind_classification() {
    let src = "def f(handlers, conn):\n    g()\n    conn.execute(q)\n    handlers[0]()\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(
        f.calls
            .iter()
            .find(|c| c.callee_name == "g")
            .unwrap()
            .callee_kind,
        CalleeKind::Direct
    );
    // A method callee is a statically named path → `Direct` (Rust parity),
    // with its chain base recorded as the receiver root.
    let method = f
        .calls
        .iter()
        .find(|c| c.callee_name == "execute")
        .expect("method call present");
    assert_eq!(method.callee_kind, CalleeKind::Direct);
    assert_eq!(method.receiver_root.as_deref(), Some("conn"));
    assert!(
        f.calls
            .iter()
            .any(|c| c.callee_kind == CalleeKind::Indirect),
        "{:?}",
        f.calls
    );
}

/// A member/subscript chain emits ONE `AccessChainEvent` rooted at the base.
#[test]
fn access_chain_event_from_attribute_chain() {
    let src = "def f(s):\n    return s.a[0].b\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.access_chains.len(), 1, "{:?}", f.access_chains);
    assert_eq!(f.access_chains[0].root, "s");
    assert_eq!(f.access_chains[0].elements.len(), 3);
}

/// Chained comparisons emit one event per adjacent operand pair.
#[test]
fn chained_comparison_emits_adjacent_pairs() {
    let src = "def f(a, b, c):\n    return a < b < c\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.comparisons.len(), 2, "{:?}", f.comparisons);
}

/// `binary_operator` emits a `BinaryOpEvent` with the operator text.
#[test]
fn binary_operator_event() {
    let src = "def f(a, b):\n    return a // b\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert!(
        f.binary_ops.iter().any(|b| b.op == "//"),
        "{:?}",
        f.binary_ops
    );
}

/// `with open(...) as fh` binds `fh → filehandle` in the lattice.
#[test]
fn with_open_binds_filehandle() {
    let src = "def f(p):\n    with open(p) as fh:\n        fh = fh\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let assignment = f
        .assignments
        .iter()
        .find(|a| a.target.name.as_deref() == Some("fh"))
        .unwrap();
    assert_eq!(assignment.binding_kind, BindingKind::Filehandle);
}

/// A comprehension initializer classifies by its type.
#[test]
fn comprehension_binding_kind() {
    let src = "def f(n):\n    xs = [i for i in range(n)]\n    m = {i: i for i in range(n)}\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let xs = f
        .assignments
        .iter()
        .find(|a| a.target.name.as_deref() == Some("xs"))
        .unwrap();
    assert_eq!(xs.binding_kind, BindingKind::List);
    let m = f
        .assignments
        .iter()
        .find(|a| a.target.name.as_deref() == Some("m"))
        .unwrap();
    assert_eq!(m.binding_kind, BindingKind::Dict);
}

/// FIX 3: every named `await` node emits one `AwaitEvent`; one inside a `for`
/// body has `in_loop_depth >= 1` (innermost), one outside any loop has depth 0.
#[test]
fn await_in_loop_emits_await_event() {
    let src = "async def f(xs):\n    for x in xs:\n        await consume(x)\n    await finish()\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.awaits.len(), 2, "{:?}", f.awaits);
    let in_loop = f
        .awaits
        .iter()
        .find(|a| a.meta.in_loop_depth >= 1)
        .expect("an await inside the for body");
    assert!(in_loop.meta.is_innermost);
    let outside = f
        .awaits
        .iter()
        .find(|a| a.meta.in_loop_depth == 0)
        .expect("an await outside any loop");
    assert!(!outside.meta.is_innermost);
}

/// FIX 2: a `keyword_argument` arg `Shape` carries the keyword NAME and the raw
/// value text, so `apply(axis=1)` is no longer byte-identical to `apply(1)`.
#[test]
fn keyword_argument_carries_name_and_value() {
    let src = "def f(df):\n    df.apply(axis=1)\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let apply = f.calls.iter().find(|c| c.callee_name == "apply").unwrap();
    let kw = apply
        .args
        .iter()
        .find(|a| a.name.as_deref() == Some("axis"))
        .unwrap_or_else(|| panic!("axis keyword arg missing: {:?}", apply.args));
    assert_eq!(kw.value.as_deref(), Some("1"));
    assert_eq!(kw.kind, crate::ir::ShapeKind::Other);
}

/// FIX 6: `from . import mod_b` (no `as`) binds the plain name `mod_b` to the
/// same relative dotted target the `as` branch uses, so `mod_b.helper()`
/// resolves through the alias map.
#[test]
fn bare_relative_import_binds_plain_name_alias() {
    let src = "from . import mod_b\n\ndef f():\n    mod_b.helper()\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    let call = f.calls.iter().find(|c| c.callee_name == "helper").unwrap();
    assert_eq!(call.callee_path.as_deref(), Some(".mod_b.helper"));
}

/// FIX 7a: an `if_clause` filter adds complexity but NOT loop depth.
#[test]
fn comprehension_if_clause_adds_complexity_not_depth() {
    let src = "def f(n):\n    xs = [i for i in range(n) if i > 0]\n    return xs\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.loops.len(), 1, "{:?}", f.loops);
    assert_eq!(f.record.max_loop_depth, 1, "the filter adds no loop depth");
    // 1 (base) + for_in_clause + if_clause.
    assert_eq!(f.record.cyclomatic_complexity, 3, "{:?}", f.record);
}

/// FIX 7b: a single multi-clause comprehension `[x for a in A for b in B]`
/// nests two levels deep.
#[test]
fn multi_clause_comprehension_depth_two() {
    let src = "def f(A, B):\n    return [x for a in A for b in B]\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.loops.len(), 2, "{:?}", f.loops);
    assert_eq!(f.record.max_loop_depth, 2);
}

/// FIX 7c: comma-separated imports emit one `IncludeEdge` per name.
#[test]
fn comma_imports_emit_one_edge_per_name() {
    let ex = extract_py("import a, b\nfrom . import x, y\n");
    let raws: Vec<&str> = ex
        .includes
        .iter()
        .map(|e| e.to_path.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(raws, ["a", "b", ".x", ".y"], "{:?}", ex.includes);
}

/// FIX 7d: `alloc_site_count` counts a container/type constructor call in a
/// function body (here `list(...)`); the non-constructing `range(...)` is not
/// counted.
#[test]
fn alloc_site_count_counts_container_constructor() {
    let src = "def f(n):\n    xs = list(range(n))\n    return xs\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.record.alloc_site_count, 1, "{:?}", f.calls);
}

/// W3: comparison operators are captured per adjacent pair, including the
/// two-token `not in` / `is not` forms normalized with a single space.
#[test]
fn comparison_operator_text_captured_and_normalized() {
    let ex = extract_py("def f(x, seq):\n    return x not in seq\n");
    let f = function(&ex, "f");
    assert_eq!(f.comparisons.len(), 1, "{:?}", f.comparisons);
    assert_eq!(f.comparisons[0].operator, "not in");

    let ex2 = extract_py("def f(x, a, b, c):\n    return a < b <= c\n");
    let f2 = function(&ex2, "f");
    let ops: Vec<&str> = f2.comparisons.iter().map(|c| c.operator.as_str()).collect();
    assert_eq!(ops, ["<", "<="], "{:?}", f2.comparisons);
}

/// W3: a lone `seq[i]` emits one `AccessChainEvent` (one `Subscript` element),
/// while a `seq[i].j` chain keeps its full element list — so chain-depth
/// consumers (e.g. LOOP-008's `>= 3` floor on multi-element chains) are not
/// regressed by the lone-subscript widening.
#[test]
fn lone_subscript_and_deep_chain_element_counts() {
    let ex = extract_py("def f(seq, i):\n    a = seq[i]\n    b = seq[i].j\n");
    let f = function(&ex, "f");
    let lens: Vec<usize> = f.access_chains.iter().map(|c| c.elements.len()).collect();
    assert!(
        lens.contains(&1),
        "lone subscript emits: {:?}",
        f.access_chains
    );
    assert!(
        lens.contains(&2),
        "two-element chain kept: {:?}",
        f.access_chains
    );
    assert!(
        f.access_chains.iter().all(|c| c.elements.len() < 3),
        "no chain reaches LOOP-008 depth here: {:?}",
        f.access_chains
    );
}

/// W3: a `try_statement` emits one `TryEvent`; the one inside the loop carries
/// `in_loop_depth >= 1`, the one outside any loop carries 0.
#[test]
fn try_event_captures_loop_context() {
    let src = "def f(xs):\n    for x in xs:\n        try:\n            use(x)\n        except ValueError:\n            pass\n    try:\n        pass\n    except Exception:\n        pass\n";
    let ex = extract_py(src);
    let f = function(&ex, "f");
    assert_eq!(f.try_blocks.len(), 2, "{:?}", f.try_blocks);
    assert!(f.try_blocks.iter().any(|t| t.meta.in_loop_depth >= 1));
    assert!(f.try_blocks.iter().any(|t| t.meta.in_loop_depth == 0));
}

/// W3: `augmented_assignment` additionally emits a `BinaryOpEvent` carrying the
/// operator (`+=`), the target and value shapes — STR-001's in-place signal.
#[test]
fn augmented_assignment_emits_binary_op_event() {
    let ex = extract_py("def f(x):\n    s += x\n");
    let f = function(&ex, "f");
    let aug = f
        .binary_ops
        .iter()
        .find(|b| b.op == "+=")
        .expect("augmented binop event");
    assert_eq!(aug.left.name.as_deref(), Some("s"));
    assert_eq!(aug.right.name.as_deref(), Some("x"));
}
