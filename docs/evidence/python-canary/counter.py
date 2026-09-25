#!/usr/bin/env python3
"""Perfpal Python W0 canary — throwaway pattern counter.

Committed as evidence tooling under docs/evidence/python-canary/ (NOT product
code; not shipped; not built). Counts candidate PERF-PY rule patterns + an
explicit silent control per rule across a corpus of Python files, using the
CPython stdlib `ast` (zero deps, deterministic).

Usage:  python3 counter.py <repo_root> [repo_root ...]
Output: per-repo tables on stdout + canary_results.json next to this file.

Design notes (accuracy caveats are also restated in python-canary.md §9):
  * Every rule id and control id is PRE-SEEDED with 0, so "counted zero times"
    is visible in the JSON (absence is never ambiguous). Totals are merged with a
    plain dict (not Counter addition) so zero-count keys survive into *_total.
  * ALLOC-003 accepts Name/Attribute/Subscript receivers and tests binding
    against the WHOLE enclosing loop stack (an accumulator reset in an outer
    loop frame and grown in an inner one is a control, not a finding).
  * `+=` classification is 3-way honest: string-ish (STR-001), numeric
    constant (CTL-INT-ACC), and ambiguous/expression RHS (CTL-AMBIG-ACC /
    CTL-EXPR-ACC) rather than mislabelling bare names as numeric.
  * The import graph is a deliberately crude proxy for the ARCH family; the
    longest-path figure is NOT sound under cycles (documented, not overclaimed).
"""

import ast
import json
import os
import sys
from collections import Counter, defaultdict

# --------------------------------------------------------------------------
# Rule / control inventories (pre-seeded so zero counts are recorded)
# --------------------------------------------------------------------------
RULE_IDS = [
    "PERF-PY-ALLOC-001", "PERF-PY-ALLOC-003", "PERF-PY-STR-001",
    "PERF-PY-LOOP-001", "PERF-PY-LOOP-002", "PERF-PY-LOOP-003", "PERF-PY-LOOP-007", "PERF-PY-LOOP-009",
    "PERF-PY-CALL-001", "PERF-PY-CALL-002", "PERF-PY-MEM-002", "PERF-PY-N1-001",
    "PERF-PY-COMPREHENSION-001", "PERF-PY-ASYNC-001", "PERF-PY-ASYNC-002",
    "PERF-PY-EXECUTOR-001", "PERF-PY-PANDAS-001", "PERF-PY-PANDAS-002",
    "PERF-PY-ARCH-001", "PERF-PY-ARCH-002", "PERF-PY-ARCH-003",
]
CONTROL_IDS = [
    "CTL-RANGE-PLAIN", "CTL-INT-ACC", "CTL-AMBIG-ACC", "CTL-EXPR-ACC",
    "CTL-STR-NOLOOP", "CTL-LISTADD-NOLOOP",
    "CTL-APPEND-LOCAL", "CTL-APPEND-OUTER-RESET", "CTL-APPEND-MULTI",
    "CTL-LEN-IN-LOOP", "CTL-IN-SET", "CTL-SLEEP-SYNC", "CTL-AWAIT-NOLOOP",
    "CTL-FSTRING-NOLOOP", "CTL-IO-NOLOOP", "CTL-COPY-NOLOOP",
    "CTL-N1-NOLOOP", "CTL-EXECUTOR-NOLOOP", "CTL-TRY-NOLOOP",
    "CTL-COMPILE-IN-LOOP", "CTL-DEPTH2",
]

# Directories excluded everywhere (kept identical across scan + graph passes).
EXCLUDE_DIRS = {".git", "__pycache__", "site-packages", "node_modules", ".venv",
                "venv", "build", "dist", ".mypy_cache", ".pytest_cache", ".ruff_cache"}

# --------------------------------------------------------------------------
# Name sets (kept deliberately conservative to avoid trivial FPs)
# --------------------------------------------------------------------------
CONSTRUCTOR_NAMES = {"list", "dict", "set", "tuple", "bytearray", "deque", "Counter", "defaultdict"}
STR_CALL_NAMES = {"str", "repr", "format", "join", "encode"}

IO_FUNC_NAMES = {"open", "print", "input", "urlopen"}
IO_ATTRS = {"read", "readline", "readlines", "write", "writelines", "send", "recv"}
IO_RECV_ROOTS = {"requests", "httpx", "urllib", "subprocess", "socket", "os"}
IO_RECV_ATTRS = {"get", "post", "put", "delete", "patch", "head", "request", "run", "call",
                 "check_output", "check_call", "Popen", "system", "urlopen"}

N1_ATTRS = {"filter", "filter_by", "fetchone", "fetchmany", "fetchall", "execute"}
N1_HTTP_ROOTS = {"requests", "httpx"}

BLOCKING_FUNC_NAMES = {"open", "input"}
BLOCKING_ATTRS = {"sleep", "read", "readline", "readlines", "write", "writelines",
                  "run", "call", "check_output", "check_call", "Popen", "system"}
BLOCKING_ROOTS = {"requests", "httpx", "urllib", "subprocess", "socket", "os", "time"}

EXECUTOR_NAMES = {"ThreadPoolExecutor", "ProcessPoolExecutor", "Process", "Thread"}
EXECUTOR_ATTRS = {"new_event_loop", "run_in_executor", "create_task", "ensure_future"}

GROW_ATTRS = {"append", "extend", "insert", "add", "update", "setdefault"}
HUB_FANIN_MIN = 12          # ARCH-001 HUB_FANOUT sub-case floor (ARCH-002 is NOT graph-derivable)
DEPTH_MIN = 8               # matches ARCH-001 MAX_INCLUDE_DEPTH
MCCABE_MIN = 15


# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------
def _target_names(t):
    return {n.id for n in ast.walk(t) if isinstance(n, ast.Name)}


def names_bound(node):
    """Names assigned/bound anywhere within `node` (approximate scope)."""
    out = set()
    for n in ast.walk(node):
        if isinstance(n, ast.Assign):
            for t in n.targets:
                out |= _target_names(t)
        elif isinstance(n, (ast.AugAssign, ast.AnnAssign)):
            out |= _target_names(n.target)
        elif isinstance(n, (ast.For, ast.AsyncFor)):
            out |= _target_names(n.target)
        elif isinstance(n, ast.withitem) and n.optional_vars is not None:
            out |= _target_names(n.optional_vars)
        elif isinstance(n, ast.NamedExpr):
            out |= _target_names(n.target)
    return out


def attr_root(node):
    """Leftmost Name of an Attribute/Subscript chain, else None."""
    while isinstance(node, (ast.Attribute, ast.Subscript)):
        node = node.value
    return node.id if isinstance(node, ast.Name) else None


def func_name(node):
    return node.id if isinstance(node, ast.Name) else None


def func_attr(node):
    return node.attr if isinstance(node, ast.Attribute) else None


def qname(node):
    """Human-qualified call name for detail buckets (root.attr or plain func)."""
    fn = func_name(node.func)
    if fn:
        return fn
    attr = func_attr(node.func)
    root = attr_root(node.func)
    if attr:
        return f"{root}.{attr}" if root and root != attr else attr
    return "<call>"


# --------------------------------------------------------------------------
# Main visitor
# --------------------------------------------------------------------------
class Canary(ast.NodeVisitor):
    def __init__(self):
        self.counts = Counter({k: 0 for k in RULE_IDS + CONTROL_IDS})
        self.detail = Counter()
        self.examples = {}
        self.cur_file = None
        self.loop_stack = []
        self.async_depth = 0
        self.func_names = []
        self._bound_cache = {}

    def _hit(self, rule, node):
        self.counts[rule] += 1
        ex = self.examples.setdefault(rule, [])
        ln = getattr(node, "lineno", None)
        if len(ex) < 6 and ln and self.cur_file:
            ex.append(f"{self.cur_file}:{ln}")

    def _bound(self, node):
        if id(node) not in self._bound_cache:
            self._bound_cache[id(node)] = names_bound(node)
        return self._bound_cache[id(node)]

    # -- loops --
    def _enter_loop(self, node):
        self.loop_stack.append(node)
        d = len(self.loop_stack)
        if d >= 3:
            self._hit("PERF-PY-LOOP-001", node)
        elif d == 2:
            self.counts["CTL-DEPTH2"] += 1
        # LOOP-009 approximates the Ruff PERF203 shape: a try/except that is a
        # DIRECT statement of a sync for/while body. Ruff's exact exclusions were
        # not fully reverse-engineered, so counts DIVERGE (see §5/§9) — this is a
        # known approximation, not a parity claim.
        for st in getattr(node, "body", []):
            if isinstance(st, (ast.Try, ast.TryStar)) and getattr(st, "handlers", None):
                if isinstance(node, ast.AsyncFor):
                    self.detail["LOOP009::async_for_direct"] += 1
                else:
                    self._hit("PERF-PY-LOOP-009", st)
        self.generic_visit(node)
        self.loop_stack.pop()

    def visit_For(self, node):
        self._visit_for_like(node)

    def visit_AsyncFor(self, node):
        self._visit_for_like(node)

    def visit_While(self, node):
        self._enter_loop(node)

    def _visit_for_like(self, node):
        it = node.iter
        if isinstance(it, ast.Call) and func_name(it.func) == "range":
            first = it.args[0] if it.args else None
            if isinstance(first, ast.Call) and func_name(first.func) == "len":
                self._hit("PERF-PY-LOOP-003", node)
            else:
                self.counts["CTL-RANGE-PLAIN"] += 1
        self._enter_loop(node)

    def visit_Await(self, node):
        if self.loop_stack:
            self._hit("PERF-PY-ASYNC-002", node)
        elif self.async_depth > 0:
            self.counts["CTL-AWAIT-NOLOOP"] += 1
        self.generic_visit(node)

    # -- async context --
    def visit_AsyncFunctionDef(self, node):
        self.async_depth += 1
        self.func_names.append(node.name)
        self._check_blocking(node)
        self._check_recursion(node)
        self._check_complexity(node)
        self.generic_visit(node)
        self.func_names.pop()
        self.async_depth -= 1

    def visit_FunctionDef(self, node):
        self.func_names.append(node.name)
        if self.async_depth == 0:
            for n in ast.walk(node):
                if isinstance(n, ast.Call) and self._is_blocking(n):
                    self.counts["CTL-SLEEP-SYNC"] += 1
        self._check_recursion(node)
        self._check_complexity(node)
        self.generic_visit(node)
        self.func_names.pop()

    def _check_blocking(self, node):
        for n in ast.walk(node):
            if isinstance(n, ast.Call) and self._is_blocking(n):
                self._hit("PERF-PY-ASYNC-001", n)

    def _is_blocking(self, call):
        fn = func_name(call.func)
        attr = func_attr(call.func)
        root = attr_root(call.func)
        if fn in BLOCKING_FUNC_NAMES:
            return True
        if attr in BLOCKING_ATTRS and root in BLOCKING_ROOTS:
            return True
        if attr in {"get", "post", "run", "system"} and root in {"requests", "httpx", "subprocess", "os"}:
            return True
        return False

    def _check_recursion(self, node):
        for n in ast.walk(node):
            if isinstance(n, ast.Call) and func_name(n.func) == node.name:
                self._hit("PERF-PY-CALL-001", node)
                break

    def _check_complexity(self, node):
        if mccabe(node) >= MCCABE_MIN:
            self._hit("PERF-PY-CALL-002", node)

    # -- try/except in loop (LOOP-009, surfaced by the Ruff PERF203 baseline) --
    def visit_Try(self, node):
        self._check_try(node)
    def visit_TryStar(self, node):
        self._check_try(node)

    def _check_try(self, node):
        if getattr(node, "handlers", None):
            if self.loop_stack:
                # the broader "lexically anywhere in a loop" shape (superset of
                # LOOP-009; recorded for comparison, not a rule)
                self.detail["LOOP009::any_in_loop"] += 1
            else:
                self.counts["CTL-TRY-NOLOOP"] += 1
        self.generic_visit(node)

    # -- calls --
    def visit_Call(self, node):
        if func_name(node.func) == "len" and self.loop_stack:
            self.counts["CTL-LEN-IN-LOOP"] += 1
        in_loop = bool(self.loop_stack)
        is_io = self._is_io(node)
        is_n1 = self._is_n1(node)
        is_exec = self._is_executor(node)
        is_copy = self._is_copy(node)
        if is_io:
            if in_loop:
                self._hit("PERF-PY-LOOP-002", node)
                self.detail[f"LOOP002::{qname(node)}"] += 1
            else:
                self.counts["CTL-IO-NOLOOP"] += 1
        if is_n1:
            if in_loop:
                self._hit("PERF-PY-N1-001", node)
                self.detail[f"N1::{qname(node)}"] += 1
            else:
                self.counts["CTL-N1-NOLOOP"] += 1
        if is_exec:
            if in_loop:
                self._hit("PERF-PY-EXECUTOR-001", node)
            else:
                self.counts["CTL-EXECUTOR-NOLOOP"] += 1
        if is_copy:
            if in_loop:
                self._hit("PERF-PY-MEM-002", node)
            else:
                self.counts["CTL-COPY-NOLOOP"] += 1
        self._check_alloc(node)
        self._check_growth_accum(node)
        self._check_membership_call(node)
        self._check_pandas(node)
        self._check_compile_in_loop(node)
        self.generic_visit(node)

    def _is_io(self, node):
        fn, attr, root = func_name(node.func), func_attr(node.func), attr_root(node.func)
        if fn in IO_FUNC_NAMES or attr in IO_ATTRS:
            return True
        if root in IO_RECV_ROOTS and attr in IO_RECV_ATTRS:
            return True
        return False

    def _is_n1(self, node):
        attr, root = func_attr(node.func), attr_root(node.func)
        if attr in N1_ATTRS:
            return True
        if root in N1_HTTP_ROOTS and attr in {"get", "post", "put", "delete"}:
            return True
        return False

    def _is_executor(self, node):
        return func_name(node.func) in EXECUTOR_NAMES or func_attr(node.func) in EXECUTOR_ATTRS

    def _is_copy(self, node):
        attr, root, fn = func_attr(node.func), attr_root(node.func), func_name(node.func)
        if attr == "copy" or (root == "copy" and attr == "deepcopy"):
            return True
        if fn in {"list", "dict", "set", "tuple"} and len(node.args) == 1 and isinstance(node.args[0], ast.Name):
            return True
        return False

    def _check_alloc(self, node):
        # ALLOC-001: fresh empty container constructed inside a loop
        if self.loop_stack and func_name(node.func) in CONSTRUCTOR_NAMES and not node.args and not node.keywords:
            self.counts["PERF-PY-ALLOC-001"] += 1
            self.detail[f"ALLOC001::empty_ctor::{func_name(node.func)}"] += 1

    def _check_growth_accum(self, node):
        # ALLOC-003: accumulator growth inside a loop (receiver-agnostic)
        if not self.loop_stack:
            return
        attr = func_attr(node.func)
        if not (isinstance(node.func, ast.Attribute) and attr in GROW_ATTRS):
            return
        root = attr_root(node.func.value)
        if root is None:
            self._hit("PERF-PY-ALLOC-003", node)
            return
        inner = self._bound(self.loop_stack[-1])
        if root in inner:
            self.counts["CTL-APPEND-LOCAL"] += 1
            return
        outer = set()
        for l in self.loop_stack[:-1]:
            outer |= self._bound(l)
        if root in outer:
            self.counts["CTL-APPEND-OUTER-RESET"] += 1
            return
        self._hit("PERF-PY-ALLOC-003", node)

    def _check_membership_call(self, node):
        if self.loop_stack and func_attr(node.func) == "index":
            self._hit("PERF-PY-LOOP-007", node)

    def _check_pandas(self, node):
        attr = func_attr(node.func)
        if attr in {"iterrows", "itertuples"}:
            self._hit("PERF-PY-PANDAS-001", node)
            self.detail[f"PANDAS001::{attr}"] += 1
        elif attr == "apply" and any(
                k.arg == "axis" and isinstance(k.value, ast.Constant) and k.value.value == 1
                for k in node.keywords):
            self._hit("PERF-PY-PANDAS-001", node)
            self.detail["PANDAS001::apply_axis1"] += 1
        if func_name(node.func) == "concat" or attr == "concat":
            if self.loop_stack:
                self._hit("PERF-PY-PANDAS-002", node)
                self.detail["PANDAS002::concat_in_loop"] += 1
            else:
                self.detail["PANDAS002::concat_outside_loop"] += 1

    def _check_compile_in_loop(self, node):
        if self.loop_stack and func_attr(node.func) == "compile" and attr_root(node.func) == "re":
            self.counts["CTL-COMPILE-IN-LOOP"] += 1

    # -- comparisons: LOOP-007 membership scan --
    def visit_Compare(self, node):
        for op, comp in zip(node.ops, node.comparators):
            if isinstance(op, (ast.In, ast.NotIn)):
                if isinstance(comp, ast.List):
                    self._hit("PERF-PY-LOOP-007", node)
                elif isinstance(comp, (ast.Set, ast.Dict)):
                    self.counts["CTL-IN-SET"] += 1
                elif isinstance(comp, ast.Call) and func_name(comp.func) in {"list", "tuple"}:
                    self._hit("PERF-PY-LOOP-007", node)
                elif isinstance(comp, ast.Call) and func_name(comp.func) in {"set", "frozenset", "dict"}:
                    self.counts["CTL-IN-SET"] += 1
        self.generic_visit(node)

    # -- AugAssign: STR-001 + honest controls --
    def _aug_key(self, rhs):
        if isinstance(rhs, ast.JoinedStr):
            return "PERF-PY-STR-001"
        if isinstance(rhs, ast.Constant):
            v = rhs.value
            if isinstance(v, (int, float, complex)) and not isinstance(v, bool):
                return "CTL-INT-ACC"
            if isinstance(v, (str, bytes)):
                return "PERF-PY-STR-001"
            return "CTL-EXPR-ACC"        # bool / None / Ellipsis etc.
        if isinstance(rhs, ast.Call):
            return "PERF-PY-STR-001" if func_name(rhs.func) in STR_CALL_NAMES else "CTL-EXPR-ACC"
        if isinstance(rhs, (ast.List, ast.Dict, ast.Set)):
            return "PERF-PY-ALLOC-003"
        if isinstance(rhs, ast.Name):
            return "CTL-AMBIG-ACC"
        return "CTL-EXPR-ACC"

    def visit_AugAssign(self, node):
        if isinstance(node.op, ast.Add):
            key = self._aug_key(node.value)
            if key == "PERF-PY-STR-001":
                self._string_add(node)                 # loop-gated; outside → CTL-STR-NOLOOP
            elif key == "PERF-PY-ALLOC-003":
                if self.loop_stack:
                    self.counts["PERF-PY-ALLOC-003"] += 1
                else:
                    self.counts["CTL-LISTADD-NOLOOP"] += 1
            elif self.loop_stack:
                # controls are true near-misses: only counted inside a loop, the
                # same scope STR-001 uses, so §4 compares like with like.
                self.counts[key] += 1
        self.generic_visit(node)

    def _string_add(self, node):
        if self.loop_stack:
            self._hit("PERF-PY-STR-001", node)
        else:
            self.counts["CTL-STR-NOLOOP"] += 1

    def visit_JoinedStr(self, node):
        if self.loop_stack:
            self.counts["PERF-PY-ALLOC-001"] += 1
            self.detail["ALLOC001::fstring"] += 1
        else:
            self.counts["CTL-FSTRING-NOLOOP"] += 1
        self.generic_visit(node)

    def generic_visit(self, node):
        if isinstance(node, (ast.For, ast.AsyncFor)):
            if _is_manual_comprehension(node):
                self._hit("PERF-PY-COMPREHENSION-001", node)
            elif _has_single_append_in_multi(node):
                self.counts["CTL-APPEND-MULTI"] += 1
        super().generic_visit(node)


def _has_single_append_in_multi(loop):
    if len(loop.body) < 2:
        return False
    for stmt in loop.body:
        if isinstance(stmt, ast.Expr) and isinstance(stmt.value, ast.Call) and func_attr(stmt.value.func) in {"append", "add"}:
            return True
    return False


def _is_manual_comprehension(loop):
    if len(loop.body) != 1:
        return False
    stmt = loop.body[0]
    return (isinstance(stmt, ast.Expr) and isinstance(stmt.value, ast.Call)
            and func_attr(stmt.value.func) in {"append", "add"})


def mccabe(fn):
    c = 1
    for n in ast.walk(fn):
        if isinstance(n, (ast.If, ast.For, ast.AsyncFor, ast.While, ast.ExceptHandler,
                          ast.With, ast.AsyncWith, ast.Assert, ast.IfExp)):
            c += 1
        elif isinstance(n, ast.BoolOp):
            c += len(n.values) - 1
        elif isinstance(n, ast.comprehension):
            c += 1 + len(n.ifs)
    return c


# --------------------------------------------------------------------------
# Import graph (ARCH evidence; crude proxy — see §6/§9 of the doc)
# --------------------------------------------------------------------------
def module_index(root):
    idx = {}
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDE_DIRS)
        for f in sorted(filenames):
            if not f.endswith(".py"):
                continue
            full = os.path.join(dirpath, f)
            rel = os.path.relpath(full, root)
            mod = rel[:-3].replace(os.sep, ".")
            if mod.endswith(".__init__"):
                mod = mod[:-len(".__init__")]
            idx[mod] = rel
    return idx


def import_graph(root, idx):
    edges = set()
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDE_DIRS)
        for f in sorted(filenames):
            if not f.endswith(".py"):
                continue
            full = os.path.join(dirpath, f)
            rel = os.path.relpath(full, root)
            src_mod = rel[:-3].replace(os.sep, ".")
            if src_mod.endswith(".__init__"):
                src_mod = src_mod[:-len(".__init__")]
            try:
                tree = ast.parse(open(full, "r", encoding="utf-8", errors="replace").read())
            except SyntaxError:
                continue
            for n in ast.walk(tree):
                targets = []
                if isinstance(n, ast.Import):
                    targets = [a.name for a in n.names]
                elif isinstance(n, ast.ImportFrom):
                    if n.level and n.level > 0:
                        base = src_mod.split(".")[:-(n.level)] if src_mod else []
                        if n.module:
                            base = base + n.module.split(".")
                        targets = [".".join(base)] if base else []
                    elif n.module:
                        targets = [n.module]
                for t in targets:
                    parts = t.split(".")
                    for i in range(len(parts), 0, -1):
                        cand = ".".join(parts[:i])
                        if cand in idx and cand != src_mod:
                            edges.add((src_mod, cand))
                            break
    return edges


def graph_stats(root):
    idx = module_index(root)
    edges = import_graph(root, idx)
    adj = defaultdict(set)
    radj = defaultdict(set)
    for a, b in edges:
        adj[a].add(b)
        radj[b].add(a)

    color = {}
    cycles = 0

    def dfs(u):
        nonlocal cycles
        stack = [(u, iter(sorted(adj[u])))]
        color[u] = 1
        while stack:
            node, it = stack[-1]
            advanced = False
            for v in it:
                if color.get(v) == 1:
                    cycles += 1
                elif color.get(v, 0) == 0:
                    color[v] = 1
                    stack.append((v, iter(sorted(adj[v]))))
                    advanced = True
                    break
            if not advanced:
                color[node] = 2
                stack.pop()

    for m in sorted(idx):
        if color.get(m, 0) == 0 and m in adj:
            dfs(m)

    # Longest path. Memoized for tractability; the memo makes the figure an
    # APPROXIMATION (not a sound bound) when cycles are present — documented in
    # §9. It is used only as indicative ARCH-001 volume, never as a guarantee.
    depth = {}

    def maxdepth(u, seen):
        if u in seen:
            return 0
        if u in depth:
            return depth[u]
        seen = seen | {u}
        best = 0
        for v in sorted(adj[u]):          # sorted ⇒ deterministic under cycles
            best = max(best, 1 + maxdepth(v, seen))
        depth[u] = best
        return best

    depths = {m: maxdepth(m, frozenset()) for m in sorted(idx) if m in adj}
    maxd = max(depths.values(), default=0)
    deep_nodes = sum(1 for d in depths.values() if d >= DEPTH_MIN)

    fanin = {m: len(radj[m]) for m in idx}
    fanout = max(fanin.values(), default=0)
    hub = min((m for m in idx if fanin[m] == fanout), default=None)
    return {
        "modules": len(idx),
        "internal_edges": len(edges),
        "cycles": cycles,
        "max_depth": maxd,
        "deep_nodes_ge_8": deep_nodes,
        "max_hub_fanin": fanout,
        "max_hub": hub,
        "arch001": cycles + deep_nodes,
        "arch001_hub_subcase": 1 if fanout >= HUB_FANIN_MIN else 0,
    }


# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------
def _is_test(rel):
    parts = "/" + rel.replace(os.sep, "/")
    base = os.path.basename(rel)
    return ("/tests/" in parts or "/test/" in parts or "/testing/" in parts
            or base.startswith("test_") or base == "conftest.py")


def scan(root):
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDE_DIRS)
        for f in sorted(filenames):
            if f.endswith(".py"):
                files.append(os.path.join(dirpath, f))
    files.sort()

    visitors = {"prod": Canary(), "test": Canary()}
    loc = {"prod": 0, "test": 0}
    nfiles = {"prod": 0, "test": 0}
    parsed = parse_errors = 0
    for full in files:
        rel = os.path.relpath(full, root)
        cls = "test" if _is_test(rel) else "prod"
        nfiles[cls] += 1
        try:
            src = open(full, "r", encoding="utf-8", errors="replace").read()
            tree = ast.parse(src)
        except SyntaxError:
            parse_errors += 1
            continue
        parsed += 1
        loc[cls] += src.count("\n")
        vis = visitors[cls]
        vis._bound_cache.clear()   # nodes are per-file; id() reuse would corrupt the cache
        vis.cur_file = rel
        vis.visit(tree)

    # ARCH-001 is graph-derivable (cycles + deep chains). ARCH-002 (hot-component
    # aggregation over FunctionRecords) and ARCH-003 (name-based *call* coupling)
    # are NOT derivable from the import graph → recorded as not measured.
    g = graph_stats(root)
    visitors["prod"].counts["PERF-PY-ARCH-001"] = g["arch001"]
    visitors["prod"].counts["PERF-PY-ARCH-002"] = 0
    visitors["prod"].counts["PERF-PY-ARCH-003"] = 0
    visitors["prod"].detail["ARCH002::not-measured"] = 1
    visitors["prod"].detail["ARCH003::not-measured"] = 1

    def counts_for(cls, prefix):
        return {k: v for k, v in sorted(visitors[cls].counts.items()) if k.startswith(prefix)}

    total_counts = {k: 0 for k in RULE_IDS + CONTROL_IDS}
    total_detail = {}
    total_examples = {}
    for cls in visitors:
        for k, v in visitors[cls].counts.items():
            total_counts[k] = total_counts.get(k, 0) + v
        for k, v in visitors[cls].detail.items():
            total_detail[k] = total_detail.get(k, 0) + v
        for k, v in visitors[cls].examples.items():
            total_examples.setdefault(k, [])
            for e in v:
                if len(total_examples[k]) < 6:
                    total_examples[k].append(e)

    return {
        "root": root,
        "files_total": len(files),
        "files_prod": nfiles["prod"],
        "files_test": nfiles["test"],
        "files_parsed": parsed,
        "parse_errors": parse_errors,
        "loc_total": loc["prod"] + loc["test"],
        "loc_prod": loc["prod"],
        "loc_test": loc["test"],
        "rules_total": {k: v for k, v in sorted(total_counts.items()) if k.startswith("PERF-PY-")},
        "rules_prod": counts_for("prod", "PERF-PY-"),
        "rules_test": counts_for("test", "PERF-PY-"),
        "controls_total": {k: v for k, v in sorted(total_counts.items()) if k.startswith("CTL-")},
        "controls_prod": counts_for("prod", "CTL-"),
        "controls_test": counts_for("test", "CTL-"),
        "detail": {k: v for k, v in sorted(total_detail.items())},
        "examples": {k: v for k, v in sorted(total_examples.items())},
        "import_graph": g,
    }


def _per_kloc(n, loc):
    return round(n / (loc / 1000), 3) if loc else 0.0


if __name__ == "__main__":
    results = {}
    for root in sys.argv[1:]:
        r = scan(root)
        name = os.path.basename(root.rstrip("/"))
        results[name] = r
        print(f"\n===== {name} =====")
        print(f"files {r['files_total']} (prod {r['files_prod']} / test {r['files_test']}) "
              f"parsed {r['files_parsed']} parse_errors {r['parse_errors']} "
              f"loc total {r['loc_total']} (prod {r['loc_prod']})")
        print(f"{'rule':28s} {'total':>7s} {'/kLOC':>8s} {'prod':>7s} {'/kLOC_prod':>11s}")
        for k, val in r["rules_total"].items():
            pv = r["rules_prod"].get(k, 0)
            print(f"  {k:26s} {val:7d} {_per_kloc(val, r['loc_total']):8.3f} {pv:7d} {_per_kloc(pv, r['loc_prod']):11.3f}")
        print("-- controls (must NOT be a rule hit) --")
        for k, val in r["controls_total"].items():
            pv = r["controls_prod"].get(k, 0)
            print(f"  {k:26s} {val:7d} {_per_kloc(val, r['loc_total']):8.3f} {pv:7d} {_per_kloc(pv, r['loc_prod']):11.3f}")
        print("-- detail --")
        for k, val in r["detail"].items():
            print(f"  {k:34s} {val:7d}")
        print("-- examples (hand-check) --")
        for k, exs in r["examples"].items():
            print(f"  {k}:")
            for e in exs:
                print(f"      {e}")
        print(f"-- import graph -- {r['import_graph']}")
    print("\nJSON:")
    print(json.dumps(results, indent=2, sort_keys=True))
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "canary_results.json"), "w") as fh:
        json.dump(results, fh, indent=2, sort_keys=True)
